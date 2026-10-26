// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Outstation I/O orchestration and protocol session handling.
//!
//! [`OutstationTask`] owns the reader, responder, and database handle, and coordinates request
//! intake, confirm waits, and unsolicited responses. [`OutstationSession`] owns protocol state
//! and application callbacks, and processes requests and formats responses. Session methods
//! receive transport or response storage when needed. Keeping these fields separate lets the
//! session run while an incoming fragment borrows the reader or a prepared response borrows
//! its slot.
//!
//! [`OutstationTask`]: crate::outstation::session::task::OutstationTask
//! [`OutstationSession`]: crate::outstation::session::OutstationSession

mod broadcast;
mod confirm;
mod control;
mod deferred;
mod non_read;
mod request;
pub(crate) mod response;
mod state;
pub(crate) mod task;
mod unsolicited;
mod write;

use std::borrow::BorrowMut;

use tracing::Instrument;
use xxhash_rust::xxh64::xxh64;

use self::response::{Prepared, ResponseSlot, Staged};
use self::task::{ConfigurationChange, OutstationMessage, OutstationTask};
use crate::app::attr::Attribute;
use crate::app::control::CommandStatus;
use crate::app::format::write::HeaderWriter;
use crate::app::gen::count::CountVariation;
use crate::app::parse::count::CountSequence;
use crate::app::parse::parser::{HeaderCollection, ObjectHeader, Request};
use crate::app::parse::range::RangedSequence;
use crate::app::variations::{
    Group34Var1, Group34Var2, Group34Var3, Group50Var1, Group50Var3, Group52Var1, Group52Var2,
};
use crate::app::*;
use crate::decode::DecodeLevel;
use crate::link::error::LinkError;
use crate::link::header::BroadcastConfirmMode;
use crate::link::EndpointAddress;
use crate::outstation::config::{Feature, OutstationConfig};
use crate::outstation::control::collection::{
    ControlCollection, ControlLimit, ControlMode, ControlTransaction,
};
use crate::outstation::control::select::SelectState;
use crate::outstation::database::{DatabaseHandle, EventClass};
use crate::outstation::target::{DeadBands, FreezeTarget, Selector, WriteHeader, WriteTarget};
use crate::outstation::traits::*;
use crate::phys::PhysLayer;
use crate::transport::{
    FragmentAddr, FragmentInfo, RequestGuard, Transport, TransportReader, TransportRequest,
    TransportRequestError, TransportWriter,
};
use crate::util::channel::Receiver;
use crate::util::deadline::deadline;

use crate::app::parse::bit::BitSequence;
use crate::app::parse::prefix::Prefix;
use crate::app::parse::traits::{FixedSizeVariation, Index};
use crate::util::lifecycle::{Enabled, RunError, StopReason};
use crate::util::skipped::Skipped;

use self::state::{
    Accepted, Confirm, ConfirmAction, ConfirmOutcome, ControlRequest, ExpectedConfirm,
    FragmentType, Intake, LastValidRequest, NextIdleAction, Reads, Received, ResponseKind,
    ResponseSeries, RetryCounter, SessionState, SolicitedWait, TimeoutStatus, UnsolicitedState,
    UnsolicitedWait, UnsolicitedWaitResult,
};

/// The header of a solicited response that reports IIN2.2
fn parameter_error(seq: Sequence) -> ResponseHeader {
    ResponseHeader::solicited(seq, Iin::default() | Iin2::PARAMETER_ERROR)
}

pub(crate) struct SessionConfig {
    decode_level: DecodeLevel,
    confirm_timeout: Timeout,
    select_timeout: Timeout,
    broadcast: Feature,
    unsolicited: Feature,
    respond_to_any_master: Feature,
    max_unsolicited_retries: Option<usize>,
    unsolicited_retry_delay: std::time::Duration,
    keep_alive_timeout: Option<std::time::Duration>,
    max_controls_per_request: Option<u16>,
    max_read_headers_per_request: u16,
}

impl From<OutstationConfig> for SessionConfig {
    fn from(config: OutstationConfig) -> Self {
        SessionConfig {
            decode_level: config.decode_level,
            confirm_timeout: config.confirm_timeout,
            select_timeout: config.select_timeout,
            broadcast: config.features.broadcast,
            unsolicited: config.features.unsolicited,
            respond_to_any_master: config.features.respond_to_any_master,
            max_unsolicited_retries: config.max_unsolicited_retries,
            unsolicited_retry_delay: config.unsolicited_retry_delay,
            keep_alive_timeout: config.keep_alive_timeout,
            max_controls_per_request: config.max_controls_per_request,
            max_read_headers_per_request: config.read_header_limit(),
        }
    }
}

/// What the session answers the master with: the transport writer, and a slot for each kind of
/// response, which keeps the response published last to send again. The slots live outside the
/// session, so a response prepared in one can be held while the session processes its request.
pub(crate) struct Responder<T: Transport> {
    writer: TransportWriter<T>,
    /// the response to the last valid request, if it had one
    sol: ResponseSlot,
    unsol: ResponseSlot,
}

impl<T: Transport> Responder<T> {
    pub(crate) fn new(writer: TransportWriter<T>, config: &OutstationConfig) -> Self {
        Self {
            writer,
            sol: ResponseSlot::new(config.solicited_buffer_size),
            unsol: ResponseSlot::new(config.unsolicited_buffer_size),
        }
    }

    /// reset the writer and drop both responses between communication sessions
    pub(crate) fn reset(&mut self) {
        self.writer.reset();
        self.sol.clear();
        self.unsol.clear();
    }
}

/// Protocol state and request handling, borrowed independently of receive and response storage.
pub(crate) struct OutstationSession {
    enabled: Enabled,
    messages: Receiver<OutstationMessage>,
    config: SessionConfig,
    destination: FragmentAddr,
    state: SessionState,
    application: Box<dyn OutstationApplication>,
    info: Box<dyn OutstationInformation>,
    control_handler: Box<dyn ControlHandler>,
    next_link_status: Option<tokio::time::Instant>,
}

impl OutstationSession {
    pub(crate) fn new(
        initial_state: Enabled,
        messages: Receiver<OutstationMessage>,
        destination: FragmentAddr,
        config: SessionConfig,
        application: Box<dyn OutstationApplication>,
        information: Box<dyn OutstationInformation>,
        control_handler: Box<dyn ControlHandler>,
    ) -> Self {
        let next_link_status = config
            .keep_alive_timeout
            .map(|delay| deadline(tokio::time::Instant::now(), delay));

        Self {
            enabled: initial_state,
            messages,
            state: SessionState::new(config.max_read_headers_per_request),
            config,
            application,
            info: information,
            control_handler,
            next_link_status,
            destination,
        }
    }

    pub(crate) fn change_master_address(&mut self, address: EndpointAddress) {
        self.destination.link = address;
    }

    pub(crate) fn enabled(&self) -> Enabled {
        self.enabled
    }

    /// reset the per-connection session state between communication sessions
    pub(crate) fn reset_session_state(&mut self) {
        self.state.reset();
    }

    /// used when the there is no running IO to process outstation messages
    pub(crate) async fn process_next_message(&mut self) -> Result<(), StopReason> {
        self.handle_next_message().await
    }

    async fn check_link_status<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        writer: &mut TransportWriter<T>,
    ) -> Result<(), RunError> {
        if let Some(next) = self.next_link_status {
            // Wait until we need to send the link status
            if next > tokio::time::Instant::now() {
                return Ok(());
            }

            writer
                .send_link_status_request(io, self.config.decode_level, self.destination)
                .await?;

            self.on_link_activity();
        }

        Ok(())
    }

    async fn read_until<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        reader: &mut TransportReader<T>,
        deadline: tokio::time::Instant,
    ) -> Result<TimeoutStatus, RunError> {
        let decode_level = self.config.decode_level;
        tokio::select! {
             res = self.sleep_until(NextIdleAction::SleepUntil(deadline)) => {
                 res?;
                 Ok(TimeoutStatus::Yes)
             }
             res = reader.read(io, decode_level) => {
                 res?;
                 Ok(TimeoutStatus::No)
             }
        }
    }

    async fn sleep_until(&mut self, next_action: NextIdleAction) -> Result<(), RunError> {
        async fn sleep_only(next_action: NextIdleAction) {
            match next_action {
                NextIdleAction::NoSleep => {}
                NextIdleAction::SleepUntil(x) => tokio::time::sleep_until(x).await,
                NextIdleAction::SleepUntilEvent => std::future::pending().await,
            }
        }

        loop {
            tokio::select! {
                 _ = sleep_only(next_action) => {
                        return Ok(());
                 }
                 res = self.handle_next_message() => {
                     res?;
                 }
            }
        }
    }

    async fn handle_next_message(&mut self) -> Result<(), StopReason> {
        match self.messages.receive().await? {
            OutstationMessage::Shutdown => Err(StopReason::Shutdown),
            OutstationMessage::Enable => {
                tracing::info!("enable communication");
                self.enabled = Enabled::Yes;
                Ok(())
            }
            OutstationMessage::Disable => {
                tracing::info!("disable communication");
                self.enabled = Enabled::No;
                Err(StopReason::Disable)
            }
            OutstationMessage::Configuration(change) => {
                self.handle_config_change(change);
                Ok(())
            }
        }
    }

    fn handle_config_change(&mut self, message: ConfigurationChange) {
        match message {
            ConfigurationChange::SetDecodeLevel(level) => {
                tracing::info!("decode level changed to: {:?}", level);
                self.config.decode_level = level;
            }
        }
    }

    fn pop_request<'a, T: Transport>(
        &self,
        reader: &'a mut TransportReader<T>,
    ) -> RequestGuard<'a, T> {
        reader.pop_request(
            self.required_master_address(),
            self.config.decode_level.application,
        )
    }

    fn required_master_address(&self) -> Option<EndpointAddress> {
        match self.config.respond_to_any_master {
            Feature::Enabled => None,
            Feature::Disabled => Some(self.destination.link),
        }
    }

    fn on_link_activity(&mut self) {
        self.next_link_status = self
            .config
            .keep_alive_timeout
            .map(|timeout| deadline(tokio::time::Instant::now(), timeout));
    }
}

impl<T: Transport> OutstationTask<T> {
    async fn run_session(&mut self, io: &mut PhysLayer) -> RunError {
        loop {
            if let Err(err) = self.run_idle_state(io).await {
                // teardown (including self.session.state.reset()) is handled by SessionTeardown
                // in OutstationTask::run, so it also covers a cancelled run future
                return err;
            }
        }
    }

    async fn run_idle_state(&mut self, io: &mut PhysLayer) -> Result<(), RunError> {
        // handle a request fragment if present, and each request that ends the solicited confirm
        // wait its response enters
        while let SolicitedWait::RequestPending = self.handle_one_request_from_idle(io).await? {}

        // check to see if we should perform unsolicited
        let next_action = self.check_unsolicited(io).await?;

        // handle a deferred read request if it was produced during unsolicited. A request that
        // ends its solicited confirm wait is processed by the next pass, before any unsolicited
        // response
        if let SolicitedWait::RequestPending = self.handle_deferred_read(io).await? {
            return Ok(());
        }

        // check to see if we should perform a link status check
        self.session
            .check_link_status(io, &mut self.responder.writer)
            .await?;

        let next_action = next_action.select_earliest(self.session.next_link_status);

        // wait for an event
        tokio::select! {
            frame_read = self.reader.read(io, self.session.config.decode_level) => {
                // make sure an I/O error didn't occur, ending the session
                frame_read?;
            }
            _ = self.database.wait_for_change() => {
                // wake for unsolicited here
            }
            res = self.session.sleep_until(next_action) => {
                res?
                // just wake up
            }
        }

        Ok(())
    }
}
