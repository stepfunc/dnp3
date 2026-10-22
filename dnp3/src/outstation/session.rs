use std::borrow::BorrowMut;

use tracing::Instrument;
use xxhash_rust::xxh64::xxh64;

use crate::app::attr::Attribute;
use crate::app::control::CommandStatus;
use crate::app::format::write::HeaderWriter;
use crate::app::gen::all::AllObjectsVariation;
use crate::app::gen::count::CountVariation;
use crate::app::gen::ranged::RangedVariation;
use crate::app::parse::count::CountSequence;
use crate::app::parse::parser::{HeaderCollection, HeaderDetails, ObjectHeader, Request};
use crate::app::variations::{
    Group34Var1, Group34Var2, Group34Var3, Group50Var1, Group50Var3, Group52Var1, Group52Var2,
};
use crate::app::*;
use crate::decode::DecodeLevel;
use crate::link::error::LinkError;
use crate::link::header::BroadcastConfirmMode;
use crate::link::EndpointAddress;
use crate::master::EventClasses;
use crate::outstation::config::{Feature, OutstationConfig};
use crate::outstation::control::collection::{ControlCollection, ControlTransaction};
use crate::outstation::control::select::SelectState;
use crate::outstation::database::{DatabaseHandle, ResponseInfo};
use crate::outstation::deferred::DeferredRead;
use crate::outstation::response::{Prepared, ResponseSlot};
use crate::outstation::task::{ConfigurationChange, OutstationMessage};
use crate::outstation::traits::*;
use crate::transport::{
    FragmentAddr, FragmentInfo, RequestGuard, Transport, TransportReader, TransportRequest,
    TransportRequestError, TransportWriter,
};
use crate::util::channel::Receiver;
use crate::util::deadline::deadline;
use crate::util::phys::PhysLayer;

use crate::app::gen::prefixed::PrefixedVariation;
use crate::app::parse::bit::BitSequence;
use crate::app::parse::prefix::Prefix;
use crate::app::parse::traits::{FixedSizeVariation, Index};
use crate::util::session::{Enabled, RunError, StopReason};

#[derive(Copy, Clone)]
enum TimeoutStatus {
    Yes,
    No,
}

#[derive(Copy, Clone)]
enum ControlType {
    Select,
    Operate,
    DirectOperate,
    DirectOperateNoAck,
}

impl ControlType {
    fn expects_response(self) -> bool {
        match self {
            ControlType::Select | ControlType::Operate | ControlType::DirectOperate => true,
            ControlType::DirectOperateNoAck => false,
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
struct ResponseSeries {
    ecsn: Sequence,
    fin: bool,
}

impl ResponseSeries {
    fn new(ecsn: Sequence, fin: bool) -> Self {
        Self { ecsn, fin }
    }
}

struct RetryCounter {
    retries: Option<usize>,
}

impl RetryCounter {
    fn new(limit: Option<usize>) -> Self {
        Self { retries: limit }
    }

    fn decrement(&mut self) -> bool {
        match self.retries {
            None => true,
            Some(x) => match x.checked_sub(1) {
                Some(remaining) => {
                    self.retries = Some(remaining);
                    true
                }
                None => false,
            },
        }
    }
}

impl ResponseInfo {
    fn get_response_series(&self, ecsn: Sequence) -> Option<ResponseSeries> {
        if self.need_confirm() {
            Some(ResponseSeries::new(ecsn, self.complete))
        } else {
            None
        }
    }
}

#[derive(Copy, Clone)]
struct LastValidRequest {
    seq: Sequence,
    request_hash: u64,
    series: Option<ResponseSeries>,
}

impl LastValidRequest {
    fn new(seq: Sequence, request_hash: u64, series: Option<ResponseSeries>) -> Self {
        LastValidRequest {
            seq,
            request_hash,
            series,
        }
    }
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
}
pub(crate) struct SessionParameters {
    max_read_headers_per_request: u16,
    sol_tx_buffer_size: BufferSize<249, 2048>,
    unsol_tx_buffer_size: BufferSize<249, 2048>,
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
        }
    }
}

impl From<OutstationConfig> for SessionParameters {
    fn from(x: OutstationConfig) -> Self {
        SessionParameters {
            // the same minimum the database enforces for READs that aren't deferred
            max_read_headers_per_request: x
                .max_read_request_headers
                .map(|x| x.max(OutstationConfig::DEFAULT_MAX_READ_REQUEST_HEADERS))
                .unwrap_or(OutstationConfig::DEFAULT_MAX_READ_REQUEST_HEADERS),
            sol_tx_buffer_size: x.solicited_buffer_size,
            unsol_tx_buffer_size: x.unsolicited_buffer_size,
        }
    }
}

#[derive(Copy, Clone)]
enum UnsolicitedState {
    /// need to perform NULL unsolicited
    NullRequired,
    Ready(Option<tokio::time::Instant>),
}

/// What a request does to the selection, following IEEE 1815-2012 4.4.4 Rule 4 and Table 4-9.
/// Ending a selection early can only make an OPERATE fail, while keeping one too long could
/// execute a control, so every request ends it except the few cases that keep it
enum SelectionAction {
    /// a CONFIRM, which isn't a request, or a retry of the SELECT
    Keep,
    /// a SELECT with the selection's sequence number but different contents
    Discard,
    /// any other request, which ends the selection, giving it to an OPERATE to check
    Take(Option<SelectState>),
}

/// What processing a non-read request needs to know about the selection
#[derive(Copy, Clone)]
struct SelectContext {
    /// the selection that this request ended
    previous: Option<SelectState>,
    /// the request's hash, which a SELECT records to recognize its retry
    request_hash: u64,
}

impl SelectContext {
    /// for requests that can't be a SELECT or OPERATE
    const NONE: Self = Self {
        previous: None,
        request_hash: 0,
    };
}

/// state that mutates while the session runs
struct SessionState {
    restart_iin_asserted: bool,
    enabled_unsolicited_classes: EventClasses,
    last_valid_request: Option<LastValidRequest>,
    select: Option<SelectState>,
    unsolicited: UnsolicitedState,
    unsolicited_seq: Sequence,
    deferred_read: DeferredRead,
    last_recorded_time: Option<tokio::time::Instant>,
    last_broadcast_type: Option<BroadcastConfirmMode>,
}

impl SessionState {
    fn new(max_read_headers: u16) -> Self {
        Self {
            enabled_unsolicited_classes: EventClasses::none(),
            restart_iin_asserted: true,
            last_valid_request: None,
            select: None,
            unsolicited: UnsolicitedState::NullRequired,
            unsolicited_seq: Sequence::default(),
            deferred_read: DeferredRead::new(max_read_headers),
            last_recorded_time: None,
            last_broadcast_type: None,
        }
    }

    // reset items that should reset between communication (TCP) sessions
    fn reset(&mut self) {
        self.last_valid_request = None;
        self.select = None;
        self.deferred_read.clear();
    }
}

pub(crate) struct OutstationSession {
    enabled: Enabled,
    messages: Receiver<OutstationMessage>,
    /// the response to the last valid request, if it had one
    sol: ResponseSlot,
    unsol: ResponseSlot,
    config: SessionConfig,
    destination: FragmentAddr,
    state: SessionState,
    application: Box<dyn OutstationApplication>,
    info: Box<dyn OutstationInformation>,
    control_handler: Box<dyn ControlHandler>,
    next_link_status: Option<tokio::time::Instant>,
}

enum Confirm {
    Yes(FragmentAddr),
    Timeout,
    NewRequest,
}

/// How the confirm wait for a response ended
#[derive(Copy, Clone)]
enum ConfirmOutcome {
    /// the master confirmed the response, so its events were delivered
    Confirmed,
    /// the response wasn't confirmed, so its events must be reported again
    Unconfirmed,
}

#[derive(Copy, Clone)]
enum UnsolicitedResult {
    Confirmed,
    Timeout,
    ReturnToIdle,
}

#[derive(Copy, Clone)]
enum UnsolicitedWaitResult {
    Timeout,
    ReadNext,
    Complete(UnsolicitedResult),
}

enum FragmentType<'a> {
    MalformedRequest(u64, ObjectParseError),
    NewRead(u64, HeaderCollection<'a>),
    RepeatRead(u64, HeaderCollection<'a>),
    NewNonRead(u64, HeaderCollection<'a>),
    RepeatNonRead(u64),
    Broadcast(BroadcastConfirmMode),
    SolicitedConfirm(Sequence),
    UnsolicitedConfirm(Sequence),
}

/// How the idle state answers a request
enum IdleResponse {
    /// record the request as the last valid request and publish its response, if it has one
    New(LastValidRequest, Option<Prepared>),
    /// a repeated non-read request: send the published response again exactly as it was, and wait
    /// for its confirm again if it asked for one
    Repeat,
}

#[derive(Copy, Clone)]
enum ConfirmAction {
    Confirmed(FragmentAddr),
    NewRequest,
    EchoLastResponse(FragmentAddr),
    ContinueWait,
}

#[derive(Copy, Clone)]
enum NextIdleAction {
    NoSleep,
    SleepUntilEvent,
    SleepUnit(tokio::time::Instant),
}

impl NextIdleAction {
    fn select_earliest(self, instant: Option<tokio::time::Instant>) -> Self {
        match instant {
            None => self,
            Some(instant) => match self {
                NextIdleAction::NoSleep => self,
                NextIdleAction::SleepUntilEvent => Self::SleepUnit(instant),
                NextIdleAction::SleepUnit(other) => {
                    Self::SleepUnit(tokio::time::Instant::min(instant, other))
                }
            },
        }
    }
}

impl OutstationSession {
    #[expect(
        clippy::too_many_arguments,
        reason = "a session takes each part of its configuration separately"
    )]
    pub(crate) fn new(
        initial_state: Enabled,
        messages: Receiver<OutstationMessage>,
        destination: FragmentAddr,
        config: SessionConfig,
        param: SessionParameters,
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
            config,
            sol: ResponseSlot::new(param.sol_tx_buffer_size),
            unsol: ResponseSlot::new(param.unsol_tx_buffer_size),
            state: SessionState::new(param.max_read_headers_per_request),
            application,
            info: information,
            control_handler,
            next_link_status,
            destination,
        }
    }

    fn required_master_address(&self) -> Option<EndpointAddress> {
        match self.config.respond_to_any_master {
            Feature::Enabled => None,
            Feature::Disabled => Some(self.destination.link),
        }
    }

    pub(crate) fn change_master_address(&mut self, address: EndpointAddress) {
        self.destination.link = address;
    }

    pub(crate) fn enabled(&self) -> Enabled {
        self.enabled
    }

    /// used when the there is no running IO to process outstation messages
    pub(crate) async fn process_next_message(&mut self) -> Result<(), StopReason> {
        self.handle_next_message().await
    }

    pub(crate) async fn run<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        reader: &mut TransportReader<T>,
        writer: &mut TransportWriter<T>,
        database: &mut DatabaseHandle,
    ) -> RunError {
        loop {
            if let Err(err) = self.run_idle_state(io, reader, writer, database).await {
                // teardown (including self.state.reset()) is handled by SessionTeardown
                // in OutstationTask::run, so it also covers a cancelled run future
                return err;
            }
        }
    }

    /// reset the per-connection session state between communication sessions
    pub(crate) fn reset_session_state(&mut self) {
        self.state.reset();
        self.sol.clear();
        self.unsol.clear();
    }

    async fn write_unsolicited<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        writer: &mut TransportWriter<T>,
        mut response: Prepared,
        database: &DatabaseHandle,
    ) -> Result<ResponseHeader, LinkError> {
        response.header_mut().iin |= self.get_response_iin(database);

        writer
            .write(
                io,
                self.config.decode_level,
                self.destination,
                self.unsol.publish(response),
            )
            .await?;

        Ok(response.header())
    }

    async fn repeat_unsolicited<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        writer: &mut TransportWriter<T>,
    ) -> Result<(), LinkError> {
        if let Some(response) = self.unsol.ready() {
            writer
                .write(io, self.config.decode_level, self.destination, response)
                .await?;
        }
        Ok(())
    }

    async fn write_solicited<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        writer: &mut TransportWriter<T>,
        respond_to: FragmentAddr,
        mut response: Prepared,
        database: &DatabaseHandle,
    ) -> Result<ResponseHeader, LinkError> {
        self.finish_solicited_header(response.header_mut(), database);

        writer
            .write(
                io,
                self.config.decode_level,
                respond_to,
                self.sol.publish(response),
            )
            .await?;

        Ok(response.header())
    }

    /// Send a response with no objects that doesn't replace the response to the last valid request
    async fn write_transient_solicited<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        writer: &mut TransportWriter<T>,
        respond_to: FragmentAddr,
        mut header: ResponseHeader,
        database: &DatabaseHandle,
    ) -> Result<(), LinkError> {
        self.finish_solicited_header(&mut header, database);

        writer
            .write(io, self.config.decode_level, respond_to, &header.to_bytes())
            .await
    }

    fn finish_solicited_header(&mut self, header: &mut ResponseHeader, database: &DatabaseHandle) {
        header.iin |= self.get_response_iin(database);

        // Determine if we need to ask for confirmation due to broadcast
        if let Some(BroadcastConfirmMode::Mandatory) = self.state.last_broadcast_type {
            header.control.con = true;
        }
    }

    async fn repeat_solicited<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        respond_to: FragmentAddr,
        writer: &mut TransportWriter<T>,
    ) -> Result<(), LinkError> {
        if let Some(response) = self.sol.ready() {
            writer
                .write(io, self.config.decode_level, respond_to, response)
                .await?;
        }
        Ok(())
    }

    async fn run_idle_state<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        reader: &mut TransportReader<T>,
        writer: &mut TransportWriter<T>,
        database: &mut DatabaseHandle,
    ) -> Result<(), RunError> {
        // handle a request fragment if present
        self.handle_one_request_from_idle(io, reader, writer, database)
            .await?;

        // check to see if we should perform unsolicited
        let next_action = self.check_unsolicited(io, reader, writer, database).await?;

        // handle a deferred read request if it was produced during unsolicited
        self.handle_deferred_read(io, reader, writer, database)
            .await?;

        // check to see if we should perform a link status check
        self.check_link_status(io, writer).await?;

        let next_action = next_action.select_earliest(self.next_link_status);

        // wait for an event
        tokio::select! {
            frame_read = reader.read(io, self.config.decode_level) => {
                // make sure an I/O error didn't occur, ending the session
                frame_read?;
            }
            _ = database.wait_for_change() => {
                // wake for unsolicited here
            }
            res = self.sleep_until(next_action) => {
                res?
                // just wake up
            }
        }

        Ok(())
    }

    async fn check_unsolicited<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        reader: &mut TransportReader<T>,
        writer: &mut TransportWriter<T>,
        database: &mut DatabaseHandle,
    ) -> Result<NextIdleAction, RunError> {
        if self.config.unsolicited.is_disabled() {
            return Ok(NextIdleAction::SleepUntilEvent);
        }

        match self.state.unsolicited {
            UnsolicitedState::NullRequired => {
                // perform NULL unsolicited
                match self
                    .perform_null_unsolicited(io, reader, writer, database)
                    .await?
                {
                    UnsolicitedResult::Timeout | UnsolicitedResult::ReturnToIdle => {
                        self.end_confirm_wait(database, ConfirmOutcome::Unconfirmed)
                            .await;
                        self.state.unsolicited = UnsolicitedState::NullRequired;
                        Ok(NextIdleAction::NoSleep)
                    }
                    UnsolicitedResult::Confirmed => {
                        self.state.unsolicited = UnsolicitedState::Ready(None);
                        Ok(NextIdleAction::NoSleep)
                    }
                }
            }
            UnsolicitedState::Ready(deadline) => {
                if let Some(deadline) = deadline {
                    if tokio::time::Instant::now() < deadline {
                        return Ok(NextIdleAction::SleepUnit(deadline)); // not ready yet
                    }
                }

                // perform regular unsolicited
                match self
                    .maybe_perform_unsolicited(io, reader, writer, database)
                    .await?
                {
                    None => {
                        // there was nothing to send
                        Ok(NextIdleAction::SleepUntilEvent)
                    }
                    Some(UnsolicitedResult::Timeout) | Some(UnsolicitedResult::ReturnToIdle) => {
                        self.end_confirm_wait(database, ConfirmOutcome::Unconfirmed)
                            .await;
                        let retry_at = self.new_unsolicited_retry_deadline();
                        self.state.unsolicited = UnsolicitedState::Ready(Some(retry_at));
                        Ok(NextIdleAction::SleepUnit(retry_at))
                    }
                    Some(UnsolicitedResult::Confirmed) => {
                        self.end_confirm_wait(database, ConfirmOutcome::Confirmed)
                            .await;
                        self.state.unsolicited = UnsolicitedState::Ready(None);
                        Ok(NextIdleAction::NoSleep)
                    }
                }
            }
        }
    }

    /// Settle the events written into a response once its confirm wait ends: delete them if the
    /// master confirmed it, otherwise return them to the unselected state to be reported again
    async fn end_confirm_wait(&mut self, database: &mut DatabaseHandle, outcome: ConfirmOutcome) {
        match outcome {
            ConfirmOutcome::Confirmed => {
                database
                    .clear_written_events(self.application.as_mut())
                    .await
            }
            ConfirmOutcome::Unconfirmed => database.reset(),
        }
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

    async fn perform_null_unsolicited<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        reader: &mut TransportReader<T>,
        writer: &mut TransportWriter<T>,
        database: &mut DatabaseHandle,
    ) -> Result<UnsolicitedResult, RunError> {
        let header = ResponseHeader::new(
            ControlField::unsolicited_response(self.state.unsolicited_seq.increment()),
            ResponseFunction::UnsolicitedResponse,
            Iin::default(),
        );
        self.perform_unsolicited_response_series(
            database,
            Prepared::Empty(header),
            true,
            io,
            reader,
            writer,
        )
        .await
    }

    async fn maybe_perform_unsolicited<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        reader: &mut TransportReader<T>,
        writer: &mut TransportWriter<T>,
        database: &mut DatabaseHandle,
    ) -> Result<Option<UnsolicitedResult>, RunError> {
        if !self.state.enabled_unsolicited_classes.any() {
            return Ok(None);
        }

        match self.write_unsolicited_data(database) {
            None => Ok(None),
            Some(res) => {
                let result = self
                    .perform_unsolicited_response_series(database, res, false, io, reader, writer)
                    .await?;
                Ok(Some(result))
            }
        }
    }

    fn write_unsolicited_data(&mut self, database: &mut DatabaseHandle) -> Option<Prepared> {
        let classes = self.state.enabled_unsolicited_classes;
        let count = self
            .unsol
            .build(|cursor| Ok(database.write_unsolicited(classes, cursor)))
            .unwrap_or_default();

        if count == 0 {
            return None;
        }

        let seq = self.state.unsolicited_seq.increment();
        let header = ResponseHeader::new(
            ControlField::unsolicited_response(seq),
            ResponseFunction::UnsolicitedResponse,
            Iin::default(),
        );
        Some(Prepared::Stored(header))
    }

    async fn perform_unsolicited_response_series<T: Transport>(
        &mut self,
        database: &mut DatabaseHandle,
        response: Prepared,
        is_null: bool,
        io: &mut PhysLayer,
        reader: &mut TransportReader<T>,
        writer: &mut TransportWriter<T>,
    ) -> Result<UnsolicitedResult, RunError> {
        let seq = self
            .write_unsolicited(io, writer, response, database)
            .await?
            .control
            .seq;

        // enter unsolicited confirm wait state
        self.info.enter_unsolicited_confirm_wait(seq);

        let mut retry_count = RetryCounter::new(self.config.max_unsolicited_retries);

        // For null responses, we want to regenerate a response everytime (see section 5.1.1.1.1 Rule 2)
        if is_null {
            retry_count = RetryCounter::new(Some(0));
        }

        let mut deadline = self.new_confirm_deadline();

        loop {
            match self
                .wait_for_unsolicited_confirm(seq, deadline, io, reader, writer, database)
                .instrument(tracing::info_span!("UnsolConfirmWait", "seq" = seq.value()))
                .await?
            {
                UnsolicitedWaitResult::ReadNext => {
                    // just go to next iteration without changing the deadline
                }
                UnsolicitedWaitResult::Complete(result) => return Ok(result),
                UnsolicitedWaitResult::Timeout => {
                    let mut retry = retry_count.decrement();

                    // If a deferred read is pending, we want to exit
                    if self.state.deferred_read.is_set() {
                        retry = false;
                    }

                    self.info.unsolicited_confirm_timeout(seq, retry);

                    if !retry {
                        return Ok(UnsolicitedResult::Timeout);
                    }

                    // perform a retry
                    self.repeat_unsolicited(io, writer).await?;

                    // update the deadline
                    deadline = self.new_confirm_deadline();
                }
            }
        }
    }

    async fn wait_for_unsolicited_confirm<T: Transport>(
        &mut self,
        uns_ecsn: Sequence,
        deadline: tokio::time::Instant,
        io: &mut PhysLayer,
        reader: &mut TransportReader<T>,
        writer: &mut TransportWriter<T>,
        database: &mut DatabaseHandle,
    ) -> Result<UnsolicitedWaitResult, RunError> {
        if let TimeoutStatus::Yes = self.read_until(io, reader, deadline).await? {
            return Ok(UnsolicitedWaitResult::Timeout);
        }

        let mut guard = reader.pop_request(self.required_master_address());
        let (info, request) = match guard.get() {
            None => return Ok(UnsolicitedWaitResult::ReadNext),
            Some(TransportRequest::Request(info, request)) => {
                self.on_link_activity();
                (info, request)
            }
            Some(TransportRequest::LinkLayerMessage) => {
                self.on_link_activity();
                return Ok(UnsolicitedWaitResult::ReadNext);
            }
            Some(TransportRequest::Error(from, err)) => {
                if let Some(seq) = Self::error_response_seq(err) {
                    // a request with an unknown function code ends the selection
                    self.state.select = None;
                    self.state.deferred_read.clear();
                    self.write_error_response(io, from, writer, seq, database)
                        .await?;
                }
                return Ok(UnsolicitedWaitResult::ReadNext);
            }
        };

        let fragment = self.classify(info, request);
        let selection = match self.selection_for(
            &fragment,
            request.header.function,
            request.header.control.seq,
        ) {
            SelectionAction::Discard => return Ok(UnsolicitedWaitResult::ReadNext),
            SelectionAction::Keep => None,
            SelectionAction::Take(selection) => selection,
        };

        match fragment {
            FragmentType::UnsolicitedConfirm(seq) => {
                if seq == uns_ecsn {
                    self.state.last_broadcast_type = None;
                    self.info.unsolicited_confirmed(seq);
                    Ok(UnsolicitedWaitResult::Complete(
                        UnsolicitedResult::Confirmed,
                    ))
                } else {
                    tracing::warn!(
                        "ignoring unsolicited confirm with wrong sequence number ({})",
                        seq.value()
                    );
                    Ok(UnsolicitedWaitResult::ReadNext)
                }
            }
            FragmentType::SolicitedConfirm(_) => {
                if let Some(BroadcastConfirmMode::Mandatory) = self.state.last_broadcast_type {
                    self.state.last_broadcast_type = None
                } else {
                    tracing::warn!("ignoring solicited confirm");
                }
                Ok(UnsolicitedWaitResult::ReadNext)
            }
            FragmentType::Broadcast(mode) => {
                self.state.deferred_read.clear();
                self.process_broadcast(database, mode, request).await;
                Ok(UnsolicitedWaitResult::ReadNext)
            }
            FragmentType::MalformedRequest(_, err) => {
                self.state.deferred_read.clear();

                let seq = request.header.control.seq;
                let iin = Iin::default() | Iin2::from(err);
                self.write_transient_solicited(
                    io,
                    writer,
                    info.addr,
                    ResponseHeader::solicited(seq, iin),
                    database,
                )
                .await?;
                Ok(UnsolicitedWaitResult::ReadNext)
            }
            FragmentType::NewNonRead(hash, objects) => {
                self.state.deferred_read.clear();
                let select = SelectContext {
                    previous: selection,
                    request_hash: hash,
                };
                let response = self
                    .handle_non_read(
                        database,
                        request.header.function,
                        request.header.control.seq,
                        select,
                        objects,
                    )
                    .await;
                match response {
                    Some(response) => {
                        self.write_solicited(io, writer, info.addr, response, database)
                            .await?;
                    }
                    None => self.sol.clear(),
                }
                self.state.last_valid_request = Some(LastValidRequest::new(
                    request.header.control.seq,
                    hash,
                    None,
                ));

                // Cancel unsolicited series if it's a DISABLE_UNSOLICITED
                if request.header.function == FunctionCode::DisableUnsolicited {
                    return Ok(UnsolicitedWaitResult::Complete(
                        UnsolicitedResult::ReturnToIdle,
                    ));
                }

                Ok(UnsolicitedWaitResult::ReadNext)
            }
            FragmentType::NewRead(hash, headers) | FragmentType::RepeatRead(hash, headers) => {
                tracing::info_span!("deferring READ request");
                self.state
                    .deferred_read
                    .set(hash, request.header.control.seq, info, headers);
                Ok(UnsolicitedWaitResult::ReadNext)
            }
            FragmentType::RepeatNonRead(_) => {
                self.repeat_solicited(io, info.addr, writer).await?;
                self.state.deferred_read.clear();
                Ok(UnsolicitedWaitResult::ReadNext)
            }
        }
    }

    async fn read_until<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        reader: &mut TransportReader<T>,
        deadline: tokio::time::Instant,
    ) -> Result<TimeoutStatus, RunError> {
        let decode_level = self.config.decode_level;
        tokio::select! {
             res = self.sleep_until(NextIdleAction::SleepUnit(deadline)) => {
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
                NextIdleAction::SleepUnit(x) => tokio::time::sleep_until(x).await,
                NextIdleAction::SleepUntilEvent => crate::util::future::forever().await,
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

    async fn handle_deferred_read<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        reader: &mut TransportReader<T>,
        writer: &mut TransportWriter<T>,
        database: &mut DatabaseHandle,
    ) -> Result<(), RunError> {
        if let Some(x) = self.state.deferred_read.select(database) {
            tracing::info!("handling deferred READ request");
            let (response, mut series) = self.format_read_response(database, true, x.seq, x.iin2);
            let header = self
                .write_solicited(io, writer, x.info.addr, response, database)
                .await?;
            self.state.last_valid_request = Some(LastValidRequest::new(x.seq, x.hash, series));

            // check if an extra confirmation was added due to broadcast
            if header.control.con && series.is_none() {
                series = Some(ResponseSeries::new(header.control.seq, true));
            }

            if let Some(series) = series {
                // enter the solicited confirm wait state
                self.sol_confirm_wait(io, reader, writer, database, series)
                    .instrument(tracing::info_span!(
                        "SolConfirmWait",
                        "ecsn" = series.ecsn.value()
                    ))
                    .await?;
            }
            self.state.deferred_read.clear();
        }

        Ok(())
    }

    async fn handle_one_request_from_idle<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        reader: &mut TransportReader<T>,
        writer: &mut TransportWriter<T>,
        database: &mut DatabaseHandle,
    ) -> Result<(), RunError> {
        let mut guard = reader.pop_request(self.required_master_address());
        match guard.get() {
            Some(TransportRequest::Request(info, request)) => {
                self.on_link_activity();
                match self
                    .process_request_from_idle(info, request, database)
                    .await
                {
                    None => {}
                    Some(IdleResponse::Repeat) => {
                        self.repeat_solicited(io, info.addr, writer).await?;

                        // a response that asked for a confirm asks for it again
                        if let Some(header) = self.sol.header().filter(|h| h.control.con) {
                            drop(guard);
                            let series = ResponseSeries::new(header.control.seq, true);
                            self.sol_confirm_wait(io, reader, writer, database, series)
                                .instrument(tracing::info_span!(
                                    "SolConfirmWait",
                                    "ecsn" = series.ecsn.value()
                                ))
                                .await?;
                        }
                    }
                    Some(IdleResponse::New(mut result, response)) => {
                        // optional response
                        match response {
                            Some(response) => {
                                let header = self
                                    .write_solicited(io, writer, info.addr, response, database)
                                    .await?;

                                // check if an extra confirmation was added due to broadcast
                                if header.control.con && result.series.is_none() {
                                    result.series =
                                        Some(ResponseSeries::new(header.control.seq, true));
                                }
                            }
                            None => self.sol.clear(),
                        }

                        self.state.last_valid_request = Some(result);

                        // maybe start a response series
                        if let Some(series) = result.series {
                            drop(guard);
                            // enter the solicited confirm wait state
                            self.sol_confirm_wait(io, reader, writer, database, series)
                                .instrument(tracing::info_span!(
                                    "SolConfirmWait",
                                    "ecsn" = series.ecsn.value()
                                ))
                                .await?;
                        }
                    }
                }
            }
            Some(TransportRequest::LinkLayerMessage) => {
                self.on_link_activity();
            }
            Some(TransportRequest::Error(from, err)) => {
                self.on_link_activity();
                if let Some(seq) = Self::error_response_seq(err) {
                    // a request with an unknown function code ends the selection
                    self.state.select = None;
                    self.write_error_response(io, from, writer, seq, database)
                        .await?;
                }
            }
            None => (),
        }

        Ok(())
    }

    async fn process_request_from_idle(
        &mut self,
        info: FragmentInfo,
        request: Request<'_>,
        database: &mut DatabaseHandle,
    ) -> Option<IdleResponse> {
        let seq = request.header.control.seq;
        let fragment = self.classify(info, request);
        let selection = match self.selection_for(&fragment, request.header.function, seq) {
            SelectionAction::Discard => return None,
            SelectionAction::Keep => None,
            SelectionAction::Take(selection) => selection,
        };

        self.info.process_request_from_idle(request.header);

        match fragment {
            FragmentType::MalformedRequest(hash, err) => {
                let response = Prepared::empty_solicited(seq, Iin::default() | Iin2::from(err));
                Some(IdleResponse::New(
                    LastValidRequest::new(seq, hash, None),
                    Some(response),
                ))
            }
            FragmentType::NewRead(hash, objects) | FragmentType::RepeatRead(hash, objects) => {
                // IEEE 1815-2012 Table 6-1 (Idle, row 8) says to resend the previous response to a
                // repeated READ. We deliberately answer it as a new READ instead: the fresh response
                // carries current values, and events that were never confirmed are still buffered,
                // so they are reported again
                let (response, series) = self.format_first_read_response(database, seq, objects);
                Some(IdleResponse::New(
                    LastValidRequest::new(seq, hash, series),
                    Some(response),
                ))
            }
            FragmentType::NewNonRead(hash, objects) => {
                let select = SelectContext {
                    previous: selection,
                    request_hash: hash,
                };
                let response = self
                    .handle_non_read(database, request.header.function, seq, select, objects)
                    .await;
                Some(IdleResponse::New(
                    LastValidRequest::new(seq, hash, None),
                    response,
                ))
            }
            FragmentType::RepeatNonRead(_) => {
                // IEEE 1815-2012 Table 6-1: send the same response, byte for byte
                Some(IdleResponse::Repeat)
            }
            FragmentType::Broadcast(mode) => {
                self.process_broadcast(database, mode, request).await;
                None
            }
            FragmentType::SolicitedConfirm(seq) => {
                tracing::warn!(
                    "ignoring solicited CONFIRM from idle state with seq: {}",
                    seq.value()
                );
                None
            }
            FragmentType::UnsolicitedConfirm(seq) => {
                tracing::warn!(
                    "ignoring unsolicited CONFIRM from idle state with seq: {}",
                    seq.value()
                );
                None
            }
        }
    }

    /// The sequence number to answer a request that couldn't be accepted with, if it gets an answer
    fn error_response_seq(err: TransportRequestError) -> Option<Sequence> {
        match err {
            TransportRequestError::HeaderParseError(err) => match err {
                HeaderParseError::UnknownFunction(seq, _) => Some(seq),
                HeaderParseError::InsufficientBytes => None,
            },
            TransportRequestError::RequestValidationError(_, err) => {
                // IEEE 1815-2012 clause 6 discards a fragment that isn't a valid request
                tracing::warn!("ignoring invalid request: {err}");
                None
            }
        }
    }

    async fn write_error_response<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        respond_to: FragmentAddr,
        writer: &mut TransportWriter<T>,
        seq: Sequence,
        database: &DatabaseHandle,
    ) -> Result<(), LinkError> {
        let iin = Iin::default() | Iin2::NO_FUNC_CODE_SUPPORT;
        self.write_transient_solicited(
            io,
            writer,
            respond_to,
            ResponseHeader::solicited(seq, iin),
            database,
        )
        .await
    }

    fn format_first_read_response(
        &mut self,
        database: &mut DatabaseHandle,
        seq: Sequence,
        object_headers: HeaderCollection,
    ) -> (Prepared, Option<ResponseSeries>) {
        let iin2 = database.select(&object_headers);
        self.format_read_response(database, true, seq, iin2)
    }

    fn format_read_response(
        &mut self,
        database: &mut DatabaseHandle,
        fir: bool,
        seq: Sequence,
        iin2: Iin2,
    ) -> (Prepared, Option<ResponseSeries>) {
        let info = match self
            .sol
            .build(|cursor| Ok(database.write_response_headers(cursor)))
        {
            Ok(info) => info,
            Err(err) => {
                tracing::error!("unable to write a READ response: {err:?}");
                return (Prepared::parameter_error(seq), None);
            }
        };

        let header = ResponseHeader::new(
            ControlField::response(seq, fir, info.complete, info.need_confirm()),
            ResponseFunction::Response,
            Iin::default() | iin2,
        );
        (Prepared::Stored(header), info.get_response_series(seq))
    }

    async fn handle_non_read(
        &mut self,
        database: &mut DatabaseHandle,
        function: FunctionCode,
        seq: Sequence,
        select: SelectContext,
        object_headers: HeaderCollection<'_>,
    ) -> Option<Prepared> {
        let mut result = match function {
            FunctionCode::Write => Some(self.handle_write(seq, object_headers, database).await),
            // these function don't process objects
            FunctionCode::DelayMeasure => Some(self.handle_delay_measure(seq)),
            FunctionCode::RecordCurrentTime => Some(self.handle_record_current_time(seq)),
            FunctionCode::ColdRestart => {
                let delay = self.application.cold_restart();
                Some(self.handle_restart(seq, delay))
            }
            FunctionCode::WarmRestart => {
                let delay = self.application.warm_restart();
                Some(self.handle_restart(seq, delay))
            }
            // controls
            FunctionCode::Select => {
                self.handle_controls(ControlType::Select, database, seq, select, object_headers)
                    .await
            }
            FunctionCode::Operate => {
                self.handle_controls(ControlType::Operate, database, seq, select, object_headers)
                    .await
            }
            FunctionCode::DirectOperate => {
                self.handle_controls(
                    ControlType::DirectOperate,
                    database,
                    seq,
                    select,
                    object_headers,
                )
                .await
            }
            FunctionCode::DirectOperateNoResponse => {
                self.handle_controls(
                    ControlType::DirectOperateNoAck,
                    database,
                    seq,
                    select,
                    object_headers,
                )
                .await
            }
            FunctionCode::ImmediateFreeze => {
                Some(self.handle_freeze(database, seq, object_headers, FreezeType::ImmediateFreeze))
            }
            FunctionCode::ImmediateFreezeNoResponse => {
                self.handle_freeze(database, seq, object_headers, FreezeType::ImmediateFreeze);
                None
            }
            FunctionCode::FreezeClear => {
                Some(self.handle_freeze(database, seq, object_headers, FreezeType::FreezeAndClear))
            }
            FunctionCode::FreezeClearNoResponse => {
                self.handle_freeze(database, seq, object_headers, FreezeType::FreezeAndClear);
                None
            }
            FunctionCode::FreezeAtTime => {
                Some(self.handle_freeze_at_time(database, seq, object_headers))
            }
            FunctionCode::FreezeAtTimeNoResponse => {
                self.handle_freeze_at_time(database, seq, object_headers);
                None
            }
            FunctionCode::EnableUnsolicited => {
                Some(self.handle_enable_or_disable_unsolicited(true, seq, object_headers))
            }
            FunctionCode::DisableUnsolicited => {
                Some(self.handle_enable_or_disable_unsolicited(false, seq, object_headers))
            }

            _ => {
                tracing::warn!("unsupported function code: {:?}", function);
                Some(Prepared::empty_solicited(
                    seq,
                    Iin::default() | Iin2::NO_FUNC_CODE_SUPPORT,
                ))
            }
        };

        if let Some(response) = &mut result {
            response.header_mut().iin |= Self::get_iin2(function, object_headers);
        }

        result
    }

    fn get_iin2(function: FunctionCode, object_headers: HeaderCollection) -> Iin2 {
        if function.get_function_info().objects_allowed {
            return Iin2::default();
        }

        if object_headers.is_empty() {
            Iin2::default()
        } else {
            tracing::warn!("ignoring object headers in {:?} request", function);
            Iin2::PARAMETER_ERROR
        }
    }

    async fn handle_write(
        &mut self,
        seq: Sequence,
        object_headers: HeaderCollection<'_>,
        database: &DatabaseHandle,
    ) -> Prepared {
        let mut iin2 = Iin2::default();
        let mut empty = true;

        for header in object_headers.iter() {
            empty = false;
            iin2 |= self.handle_single_write_header(header, database).await;
        }

        if empty {
            tracing::warn!("empty WRITE request");
        }

        Prepared::empty_solicited(seq, Iin::default() | iin2)
    }

    fn handle_write_iin(&mut self, bits: BitSequence) -> Iin2 {
        let mut iin2 = Iin2::default();
        for (value, index) in bits.iter() {
            if index == 7 {
                // restart IIN
                if value {
                    tracing::warn!("ignoring request to write IIN 1.7 to TRUE");
                    iin2 |= Iin2::PARAMETER_ERROR;
                } else {
                    // clear the restart bit
                    self.state.restart_iin_asserted = false;
                    self.info.clear_restart_iin();
                }
            } else {
                tracing::warn!("ignoring write of IIN index {} to value {}", index, value);
                iin2 |= Iin2::PARAMETER_ERROR;
            }
        }
        iin2
    }

    async fn handle_write_attr(&mut self, attr: Attribute<'_>, db: &DatabaseHandle) -> Iin2 {
        // first validate that attribute can be written
        if let Err(err) = db.transaction(|db| db.inner.get_attr_map().can_write(attr)) {
            tracing::warn!("Unable to WRITE attribute: {err}");
            return err.into();
        }

        // next, ask the application to validate and persist the value
        if !self.application.write_device_attr(attr).get().await {
            // validation of the value failed
            return Iin2::PARAMETER_ERROR;
        }

        // this can't fail today: nothing removes an attribute or changes its type once defined
        if let Err(err) = db.transaction(|db| db.inner.get_attr_map().write(attr)) {
            tracing::error!("Unable to store the WRITE of an attribute: {err}");
            return Iin2::PARAMETER_ERROR;
        }
        Iin2::default()
    }

    fn handle_write_abs_time(&mut self, seq: CountSequence<Group50Var1>) -> Iin2 {
        if let Some(value) = seq.single() {
            match self.application.write_absolute_time(value.time) {
                Err(err) => err.into(),
                Ok(()) => Iin2::default(),
            }
        } else {
            tracing::warn!("request lacks a single g50v1");
            Iin2::PARAMETER_ERROR
        }
    }

    async fn handle_write_analog_deadbands<I, V>(
        &mut self,
        items: CountSequence<'_, Prefix<I, V>>,
        db: &DatabaseHandle,
    ) -> Iin2
    where
        I: Index,
        V: FixedSizeVariation + Into<f64>,
    {
        if !self.application.support_write_analog_dead_bands() {
            return Iin2::OBJECT_UNKNOWN;
        }

        self.application.begin_write_analog_dead_bands();

        let iin = db.transaction(|db| {
            let mut iin2 = Iin2::default();
            for (index, deadband) in items
                .iter()
                .map(|x| (x.index.widen_to_u16(), x.value.into()))
            {
                if db.inner.set_analog_deadband(index, deadband) {
                    self.application.write_analog_dead_band(index, deadband);
                } else {
                    iin2 |= Iin2::PARAMETER_ERROR
                }
            }
            iin2
        });

        self.application.end_write_analog_dead_bands().get().await;

        iin
    }

    async fn handle_single_write_header(
        &mut self,
        header: ObjectHeader<'_>,
        db: &DatabaseHandle,
    ) -> Iin2 {
        match header.details {
            HeaderDetails::OneByteStartStop(_, _, RangedVariation::Group0(_, Some(attr))) => {
                self.handle_write_attr(attr, db).await
            }
            HeaderDetails::TwoByteStartStop(_, _, RangedVariation::Group0(_, Some(attr))) => {
                self.handle_write_attr(attr, db).await
            }
            HeaderDetails::OneByteStartStop(_, _, RangedVariation::Group80Var1(bits)) => {
                self.handle_write_iin(bits)
            }
            HeaderDetails::TwoByteStartStop(_, _, RangedVariation::Group80Var1(bits)) => {
                self.handle_write_iin(bits)
            }
            HeaderDetails::OneByteCount(_, CountVariation::Group50Var1(seq)) => {
                self.handle_write_abs_time(seq)
            }
            HeaderDetails::TwoByteCount(_, CountVariation::Group50Var1(seq)) => {
                self.handle_write_abs_time(seq)
            }
            HeaderDetails::OneByteCount(_, CountVariation::Group50Var3(seq)) => {
                self.handle_write_at_last_recorded_time(seq)
            }
            HeaderDetails::TwoByteCount(_, CountVariation::Group50Var3(seq)) => {
                self.handle_write_at_last_recorded_time(seq)
            }
            // analog deadbands
            HeaderDetails::OneByteCountAndPrefix(_, PrefixedVariation::Group34Var1(seq)) => {
                self.handle_write_analog_deadbands(seq, db).await
            }
            HeaderDetails::OneByteCountAndPrefix(_, PrefixedVariation::Group34Var2(seq)) => {
                self.handle_write_analog_deadbands(seq, db).await
            }
            HeaderDetails::OneByteCountAndPrefix(_, PrefixedVariation::Group34Var3(seq)) => {
                self.handle_write_analog_deadbands(seq, db).await
            }
            HeaderDetails::TwoByteCountAndPrefix(_, PrefixedVariation::Group34Var1(seq)) => {
                self.handle_write_analog_deadbands(seq, db).await
            }
            HeaderDetails::TwoByteCountAndPrefix(_, PrefixedVariation::Group34Var2(seq)) => {
                self.handle_write_analog_deadbands(seq, db).await
            }
            HeaderDetails::TwoByteCountAndPrefix(_, PrefixedVariation::Group34Var3(seq)) => {
                self.handle_write_analog_deadbands(seq, db).await
            }
            _ => {
                tracing::warn!(
                    "WRITE not supported with qualifier: {} and variation: {}",
                    header.details.qualifier(),
                    header.variation
                );
                if Self::is_writable(header.variation) {
                    Iin2::PARAMETER_ERROR
                } else {
                    Iin2::OBJECT_UNKNOWN
                }
            }
        }
    }

    /// variations that WRITE supports with some qualifier
    fn is_writable(variation: Variation) -> bool {
        match variation {
            Variation::Group0(var) => var != 255,
            Variation::Group34Var1
            | Variation::Group34Var2
            | Variation::Group34Var3
            | Variation::Group50Var1
            | Variation::Group50Var3
            | Variation::Group80Var1 => true,
            _ => false,
        }
    }

    fn handle_write_at_last_recorded_time(&mut self, seq: CountSequence<Group50Var3>) -> Iin2 {
        let value = if let Some(value) = seq.single() {
            value
        } else {
            tracing::warn!("request didn't have a single g50v3");
            return Iin2::PARAMETER_ERROR;
        };

        let last_recorded_time = if let Some(last_recorded_time) = self.state.last_recorded_time {
            last_recorded_time
        } else {
            tracing::warn!("no previous RECORD_CURRENT_TIME");
            return Iin2::PARAMETER_ERROR;
        };

        let now = tokio::time::Instant::now();
        let delay = if let Some(delay) = now.checked_duration_since(last_recorded_time) {
            delay
        } else {
            tracing::warn!("clock rollback detected (Tokio error)");
            return Iin2::PARAMETER_ERROR;
        };

        let timestamp = if let Some(timestamp) = value.time.checked_add(delay) {
            timestamp
        } else {
            tracing::warn!("calculating current time overflown");
            return Iin2::PARAMETER_ERROR;
        };

        self.state.last_recorded_time = None;
        match self.application.write_absolute_time(timestamp) {
            Err(err) => err.into(),
            Ok(()) => Iin2::default(),
        }
    }

    fn handle_delay_measure(&mut self, seq: Sequence) -> Prepared {
        let g52v2 = Group52Var2 {
            time: self.application.get_processing_delay_ms(),
        };

        self.single_object_response(seq, g52v2)
    }

    /// Respond with a single object, or with IIN2.2 if it can't be written
    fn single_object_response<V>(&mut self, seq: Sequence, item: V) -> Prepared
    where
        V: FixedSizeVariation,
    {
        match self
            .sol
            .build(|cursor| HeaderWriter::new(cursor).write_count_of_one(item))
        {
            Ok(()) => Prepared::Stored(ResponseHeader::solicited(seq, Iin::default())),
            Err(err) => {
                tracing::error!("unable to write {item} to the response buffer: {err:?}");
                Prepared::parameter_error(seq)
            }
        }
    }

    fn handle_record_current_time(&mut self, seq: Sequence) -> Prepared {
        self.state.last_recorded_time = Some(tokio::time::Instant::now());
        Prepared::empty_solicited(seq, Iin::default())
    }

    fn handle_restart(&mut self, seq: Sequence, delay: Option<RestartDelay>) -> Prepared {
        let delay = match delay {
            None => {
                return Prepared::empty_solicited(seq, Iin::default() | Iin2::NO_FUNC_CODE_SUPPORT);
            }
            Some(x) => x,
        };

        // respond with the delay
        match delay {
            RestartDelay::Seconds(value) => {
                self.single_object_response(seq, Group52Var1 { time: value })
            }
            RestartDelay::Milliseconds(value) => {
                self.single_object_response(seq, Group52Var2 { time: value })
            }
        }
    }

    async fn handle_direct_operate(
        &mut self,
        database: &mut DatabaseHandle,
        seq: Sequence,
        controls: ControlCollection<'_>,
    ) -> Prepared {
        // Handle each operate and write the response
        let max_controls_per_request = self.config.max_controls_per_request;
        let result =
            ControlTransaction::execute(self.control_handler.borrow_mut(), database, |tx, db| {
                self.sol.build(|cursor| {
                    controls.operate_with_response(
                        cursor,
                        OperateType::DirectOperate,
                        tx,
                        db,
                        max_controls_per_request,
                    )
                })
            })
            .await;

        Self::control_response(seq, result)
    }

    /// Echo the controls into an unpublished response, without executing them, to check that the
    /// real response fits. The echo is the same size whatever the status.
    fn control_response_fits(&mut self, controls: &ControlCollection) -> bool {
        self.sol
            .build(|cursor| controls.respond_with_status(cursor, CommandStatus::Success))
            .is_ok()
    }

    /// `result` is the overall status of the controls built into the response, or the error
    /// writing them
    fn control_response(
        seq: Sequence,
        result: Result<CommandStatus, scursor::WriteError>,
    ) -> Prepared {
        let status = match result {
            Ok(x) => x,
            Err(_) => {
                tracing::warn!("control response doesn't fit in the response buffer");
                return Prepared::parameter_error(seq);
            }
        };

        let mut iin = Iin::default();

        if status == CommandStatus::NotSupported {
            iin |= Iin2::PARAMETER_ERROR;
        }

        Prepared::Stored(ResponseHeader::solicited(seq, iin))
    }

    fn handle_enable_or_disable_unsolicited(
        &mut self,
        enable: bool,
        seq: Sequence,
        object_headers: HeaderCollection,
    ) -> Prepared {
        fn to_string(enable: bool) -> &'static str {
            if enable {
                "ENABLE"
            } else {
                "DISABLE"
            }
        }

        if self.config.unsolicited.is_disabled() {
            tracing::warn!("received {} unsolicited request, but unsolicited support is disabled by configuration", to_string(enable));
            return Prepared::empty_solicited(seq, Iin::default() | Iin2::NO_FUNC_CODE_SUPPORT);
        }

        let mut iin2 = Iin2::default();

        for header in object_headers.iter() {
            match header.details {
                HeaderDetails::AllObjects(AllObjectsVariation::Group60Var2) => {
                    self.state.enabled_unsolicited_classes.class1 = enable;
                }
                HeaderDetails::AllObjects(AllObjectsVariation::Group60Var3) => {
                    self.state.enabled_unsolicited_classes.class2 = enable;
                }
                HeaderDetails::AllObjects(AllObjectsVariation::Group60Var4) => {
                    self.state.enabled_unsolicited_classes.class3 = enable;
                }
                _ => {
                    tracing::warn!("received {} unsolicited request for unsupported qualifier ({}) and variation ({})", to_string(enable), header.details.qualifier(), header.variation);
                    iin2 |= match header.variation {
                        Variation::Group60Var2
                        | Variation::Group60Var3
                        | Variation::Group60Var4 => Iin2::PARAMETER_ERROR,
                        _ => Iin2::OBJECT_UNKNOWN,
                    };
                }
            }
        }

        Prepared::empty_solicited(seq, Iin::default() | iin2)
    }

    async fn handle_direct_operate_no_ack(
        &mut self,
        database: &mut DatabaseHandle,
        controls: ControlCollection<'_>,
    ) {
        let max_controls_per_request = self.config.max_controls_per_request;
        ControlTransaction::execute(self.control_handler.borrow_mut(), database, |tx, db| {
            controls.operate_no_ack(tx, db, max_controls_per_request)
        })
        .await;
    }

    async fn handle_controls(
        &mut self,
        ct: ControlType,
        database: &mut DatabaseHandle,
        seq: Sequence,
        select: SelectContext,
        object_headers: HeaderCollection<'_>,
    ) -> Option<Prepared> {
        // a control object with an unsupported qualifier fails to parse, so this is an object
        // that the control functions don't support
        let controls = match ControlCollection::from(object_headers) {
            Err(err) => {
                tracing::warn!(
                    "ignoring control request containing non-control object header {} - {}",
                    err.variation,
                    err.qualifier
                );
                return ct.expects_response().then(|| {
                    Prepared::empty_solicited(seq, Iin::default() | Iin2::OBJECT_UNKNOWN)
                });
            }
            Ok(controls) => controls,
        };

        if ct.expects_response() && !self.control_response_fits(&controls) {
            tracing::warn!(
                "ignoring control request whose response doesn't fit in the response buffer"
            );
            return Some(Prepared::parameter_error(seq));
        }

        match ct {
            ControlType::Select => Some(
                self.handle_select(database, seq, select.request_hash, controls)
                    .await,
            ),
            ControlType::Operate => Some(
                self.handle_operate(database, seq, select.previous, controls)
                    .await,
            ),
            ControlType::DirectOperate => {
                Some(self.handle_direct_operate(database, seq, controls).await)
            }
            ControlType::DirectOperateNoAck => {
                self.handle_direct_operate_no_ack(database, controls).await;
                None
            }
        }
    }

    async fn handle_select(
        &mut self,
        database: &mut DatabaseHandle,
        seq: Sequence,
        request_hash: u64,
        controls: ControlCollection<'_>,
    ) -> Prepared {
        // Handle each select and write the response
        let max_controls_per_request = self.config.max_controls_per_request;
        let result =
            ControlTransaction::execute(self.control_handler.borrow_mut(), database, |tx, db| {
                self.sol.build(|cursor| {
                    controls.select_with_response(cursor, tx, db, max_controls_per_request)
                })
            })
            .await;

        // Record the select state
        if let Ok(CommandStatus::Success) = result {
            self.state.select = Some(SelectState::new(
                seq,
                request_hash,
                tokio::time::Instant::now(),
                controls.hash(),
            ))
        }

        Self::control_response(seq, result)
    }

    async fn handle_operate(
        &mut self,
        database: &mut DatabaseHandle,
        seq: Sequence,
        selection: Option<SelectState>,
        controls: ControlCollection<'_>,
    ) -> Prepared {
        // the OPERATE has already ended the selection, so it is used at most once
        let matched = selection
            .ok_or(CommandStatus::NoSelect)
            .and_then(|s| s.match_operate(self.config.select_timeout, seq, controls.hash()));

        // Handle each operate and write the response
        let result = match matched {
            Err(status) => self
                .sol
                .build(|cursor| controls.respond_with_status(cursor, status).map(|_| status)),
            Ok(()) => {
                let max_controls_per_request = self.config.max_controls_per_request;
                ControlTransaction::execute(
                    self.control_handler.borrow_mut(),
                    database,
                    |tx, db| {
                        self.sol.build(|cursor| {
                            controls.operate_with_response(
                                cursor,
                                OperateType::SelectBeforeOperate,
                                tx,
                                db,
                                max_controls_per_request,
                            )
                        })
                    },
                )
                .await
            }
        };

        Self::control_response(seq, result)
    }

    fn handle_freeze(
        &mut self,
        database: &mut DatabaseHandle,
        seq: Sequence,
        object_headers: HeaderCollection,
        freeze_type: FreezeType,
    ) -> Prepared {
        let mut iin = Iin::default();

        for header in object_headers.iter() {
            iin |= self.handle_freeze_header(database, freeze_type, header);
        }

        Prepared::empty_solicited(seq, iin)
    }

    fn handle_freeze_at_time(
        &mut self,
        database: &mut DatabaseHandle,
        seq: Sequence,
        object_headers: HeaderCollection,
    ) -> Prepared {
        let mut iin = Iin::default();

        let mut timing: Option<FreezeInterval> = None;

        for header in object_headers.iter() {
            match header.details {
                HeaderDetails::OneByteCount(_, CountVariation::Group50Var2(seq)) => {
                    match seq.single() {
                        None => iin.iin2.set(Iin2::PARAMETER_ERROR),
                        Some(x) => {
                            timing = Some(x.into());
                        }
                    }
                }
                HeaderDetails::TwoByteCount(_, CountVariation::Group50Var2(seq)) => {
                    match seq.single() {
                        None => iin.iin2.set(Iin2::PARAMETER_ERROR),
                        Some(x) => {
                            timing = Some(x.into());
                        }
                    }
                }
                HeaderDetails::AllObjects(_)
                | HeaderDetails::OneByteStartStop(..)
                | HeaderDetails::TwoByteStartStop(..)
                | HeaderDetails::OneByteCount(..)
                | HeaderDetails::TwoByteCount(..)
                | HeaderDetails::OneByteCountAndPrefix(..)
                | HeaderDetails::TwoByteCountAndPrefix(..)
                | HeaderDetails::TwoByteFreeFormat(..) => {
                    // other variations require that there be a preceding g50v2
                    match timing {
                        None => {
                            tracing::warn!(
                                "freeze-at-time on {} w/o preceding g50v2",
                                header.variation
                            );
                            iin.iin2.set(Iin2::PARAMETER_ERROR);
                        }
                        Some(x) => {
                            iin |= self.handle_freeze_header(
                                database,
                                FreezeType::FreezeAtTime(x),
                                header,
                            );
                        }
                    }
                }
            }
        }

        Prepared::empty_solicited(seq, iin)
    }

    fn handle_freeze_header(
        &mut self,
        database: &mut DatabaseHandle,
        freeze_type: FreezeType,
        header: ObjectHeader,
    ) -> Iin2 {
        match header.details {
            HeaderDetails::AllObjects(AllObjectsVariation::Group20Var0) => self
                .application
                .freeze_counter(FreezeIndices::All, freeze_type, database)
                .map_or_else(|err| err.into(), |_| Iin2::default()),
            HeaderDetails::OneByteStartStop(start, stop, RangedVariation::Group20Var0) => self
                .application
                .freeze_counter(
                    FreezeIndices::Range(start.into(), stop.into()),
                    freeze_type,
                    database,
                )
                .map_or_else(|err| err.into(), |_| Iin2::default()),
            HeaderDetails::TwoByteStartStop(start, stop, RangedVariation::Group20Var0) => self
                .application
                .freeze_counter(FreezeIndices::Range(start, stop), freeze_type, database)
                .map_or_else(|err| err.into(), |_| Iin2::default()),
            _ => match header.variation {
                Variation::Group20Var0 => Iin2::PARAMETER_ERROR,
                _ => Iin2::OBJECT_UNKNOWN,
            },
        }
    }

    fn get_response_iin(&mut self, database: &DatabaseHandle) -> Iin {
        let mut iin = Iin::default();

        // Restart IIN bit
        if self.state.restart_iin_asserted {
            iin |= Iin1::RESTART
        }

        // Events available
        let events_info = database.get_events_info();
        if events_info.unwritten_classes.class1 {
            iin |= Iin1::CLASS_1_EVENTS;
        }
        if events_info.unwritten_classes.class2 {
            iin |= Iin1::CLASS_2_EVENTS;
        }
        if events_info.unwritten_classes.class3 {
            iin |= Iin1::CLASS_3_EVENTS;
        }

        // Buffer overflow
        if events_info.is_overflown {
            iin |= Iin2::EVENT_BUFFER_OVERFLOW;
        }

        // Broadcast bit
        if let Some(mode) = self.state.last_broadcast_type {
            iin |= Iin1::BROADCAST;

            if mode != BroadcastConfirmMode::Mandatory {
                self.state.last_broadcast_type = None;
            }
        }

        // Application-controlled IIN bits
        iin |= self.application.get_application_iin();

        iin
    }

    async fn process_broadcast(
        &mut self,
        database: &mut DatabaseHandle,
        mode: BroadcastConfirmMode,
        request: Request<'_>,
    ) {
        self.state.last_broadcast_type = Some(mode);
        let action = self.process_broadcast_get_action(database, request).await;
        self.info
            .broadcast_received(request.header.function, action)
    }

    async fn process_broadcast_get_action(
        &mut self,
        database: &mut DatabaseHandle,
        request: Request<'_>,
    ) -> BroadcastAction {
        if self.config.broadcast.is_disabled() {
            tracing::warn!(
                "ignoring broadcast request (broadcast support disabled): {:?}",
                request.header.function
            );
            return BroadcastAction::IgnoredByConfiguration;
        }

        let objects = match request.objects {
            Ok(x) => x,
            Err(err) => {
                tracing::warn!(
                    "ignoring broadcast message with bad object headers: {}",
                    err
                );
                return BroadcastAction::BadObjectHeaders;
            }
        };

        let seq = request.header.control.seq;

        match request.header.function {
            FunctionCode::Write => {
                self.handle_write(seq, objects, database).await;
                BroadcastAction::Processed
            }
            FunctionCode::DirectOperateNoResponse => {
                self.handle_controls(
                    ControlType::DirectOperateNoAck,
                    database,
                    seq,
                    SelectContext::NONE,
                    objects,
                )
                .await;
                BroadcastAction::Processed
            }
            FunctionCode::ImmediateFreezeNoResponse => {
                self.handle_freeze(database, seq, objects, FreezeType::ImmediateFreeze);
                BroadcastAction::Processed
            }
            FunctionCode::FreezeClearNoResponse => {
                self.handle_freeze(database, seq, objects, FreezeType::FreezeAndClear);
                BroadcastAction::Processed
            }
            FunctionCode::FreezeAtTimeNoResponse => {
                self.handle_freeze_at_time(database, seq, objects);
                BroadcastAction::Processed
            }
            FunctionCode::RecordCurrentTime => {
                self.handle_record_current_time(seq);
                BroadcastAction::Processed
            }
            FunctionCode::DisableUnsolicited => {
                self.handle_enable_or_disable_unsolicited(false, seq, objects);
                BroadcastAction::Processed
            }
            FunctionCode::EnableUnsolicited => {
                self.handle_enable_or_disable_unsolicited(true, seq, objects);
                BroadcastAction::Processed
            }
            _ => {
                tracing::warn!(
                    "unsupported broadcast function: {:?}",
                    request.header.function
                );
                BroadcastAction::UnsupportedFunction(request.header.function)
            }
        }
    }

    fn new_confirm_deadline(&self) -> tokio::time::Instant {
        self.config.confirm_timeout.deadline_from_now()
    }

    fn new_unsolicited_retry_deadline(&self) -> tokio::time::Instant {
        deadline(
            tokio::time::Instant::now(),
            self.config.unsolicited_retry_delay,
        )
    }

    async fn sol_confirm_wait<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        reader: &mut TransportReader<T>,
        writer: &mut TransportWriter<T>,
        database: &mut DatabaseHandle,
        mut series: ResponseSeries,
    ) -> Result<(), RunError> {
        self.info.enter_solicited_confirm_wait(series.ecsn);

        loop {
            match self
                .wait_for_sol_confirm(io, reader, writer, series.ecsn)
                .await?
            {
                Confirm::Yes(respond_to) => {
                    self.state.last_broadcast_type = None;

                    self.end_confirm_wait(database, ConfirmOutcome::Confirmed)
                        .await;

                    if series.fin {
                        // done with response series
                        return Ok(());
                    }
                    // format the next response in the series
                    series.ecsn.increment();
                    let (response, next) =
                        self.format_read_response(database, false, series.ecsn, Iin2::default());
                    self.write_solicited(io, writer, respond_to, response, database)
                        .await?;
                    match next {
                        None => return Ok(()),
                        Some(next) => {
                            series = next;
                        }
                    }
                }
                Confirm::Timeout => {
                    tracing::warn!("confirm timeout");
                    self.end_confirm_wait(database, ConfirmOutcome::Unconfirmed)
                        .await;
                    return Ok(());
                }
                Confirm::NewRequest => {
                    tracing::info!("aborting solicited response due to new request");
                    self.end_confirm_wait(database, ConfirmOutcome::Unconfirmed)
                        .await;
                    return Ok(());
                }
            }
        }
    }

    async fn wait_for_sol_confirm<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        reader: &mut TransportReader<T>,
        writer: &mut TransportWriter<T>,
        ecsn: Sequence,
    ) -> Result<Confirm, RunError> {
        let mut deadline = self.new_confirm_deadline();
        loop {
            match self.read_until(io, reader, deadline).await? {
                TimeoutStatus::Yes => {
                    self.info.solicited_confirm_timeout(ecsn);
                    return Ok(Confirm::Timeout);
                }
                // process data
                TimeoutStatus::No => {
                    let mut guard = reader.pop_request(self.required_master_address());
                    match self.expect_sol_confirm(ecsn, &mut guard) {
                        ConfirmAction::ContinueWait => {
                            // we ignored whatever the request was and logged it elsewhere
                            // just go back to the loop and read another fragment
                        }
                        ConfirmAction::Confirmed(respond_to) => {
                            self.info.solicited_confirm_received(ecsn);
                            return Ok(Confirm::Yes(respond_to));
                        }
                        ConfirmAction::NewRequest => {
                            self.info.solicited_confirm_wait_new_request();
                            // retain the fragment so that it can be processed from the idle state
                            guard.retain();
                            return Ok(Confirm::NewRequest);
                        }
                        ConfirmAction::EchoLastResponse(respond_to) => {
                            self.repeat_solicited(io, respond_to, writer).await?;
                            // per the spec, we restart the confirm timer
                            deadline = self.new_confirm_deadline();
                        }
                    }
                }
            }
        }
    }

    fn expect_sol_confirm<T: Transport>(
        &mut self,
        ecsn: Sequence,
        request: &mut RequestGuard<'_, T>,
    ) -> ConfirmAction {
        let (info, request) = match request.get() {
            Some(TransportRequest::Request(info, request)) => {
                self.on_link_activity();
                (info, request)
            }
            Some(TransportRequest::LinkLayerMessage) => {
                self.on_link_activity();
                return ConfirmAction::ContinueWait;
            }
            Some(TransportRequest::Error(_, err)) => {
                self.on_link_activity();
                return match Self::error_response_seq(err) {
                    Some(_) => ConfirmAction::NewRequest,
                    None => ConfirmAction::ContinueWait,
                };
            }
            None => return ConfirmAction::ContinueWait,
        };

        let fragment = self.classify(info, request);
        if self.is_discarded_select(
            &fragment,
            request.header.function,
            request.header.control.seq,
        ) {
            return ConfirmAction::ContinueWait;
        }

        match fragment {
            FragmentType::MalformedRequest(_, _) => ConfirmAction::NewRequest,
            FragmentType::NewRead(_, _) => ConfirmAction::NewRequest,
            FragmentType::RepeatRead(_, _) => {
                // a request ends the selection; the READ that started this wait already did
                self.state.select = None;
                ConfirmAction::EchoLastResponse(info.addr)
            }
            FragmentType::NewNonRead(_, _) => ConfirmAction::NewRequest,
            // this should never happen, but if it does, new request is probably best course of action
            FragmentType::RepeatNonRead(_) => ConfirmAction::NewRequest,
            FragmentType::Broadcast(_) => ConfirmAction::NewRequest,
            FragmentType::SolicitedConfirm(seq) => {
                if seq == ecsn {
                    ConfirmAction::Confirmed(info.addr)
                } else {
                    self.info
                        .wrong_solicited_confirm_seq(ecsn, request.header.control.seq);
                    tracing::warn!(
                        "ignoring confirm with wrong sequence number: {}",
                        seq.value()
                    );
                    ConfirmAction::ContinueWait
                }
            }
            FragmentType::UnsolicitedConfirm(seq) => {
                self.info.unexpected_confirm(true, seq);
                tracing::warn!("ignoring unsolicited confirm with seq: {}", seq.value());
                ConfirmAction::ContinueWait
            }
        }
    }

    fn classify<'a>(&self, info: FragmentInfo, request: Request<'a>) -> FragmentType<'a> {
        if request.header.function == FunctionCode::Confirm {
            return if request.header.control.uns {
                FragmentType::UnsolicitedConfirm(request.header.control.seq)
            } else {
                FragmentType::SolicitedConfirm(request.header.control.seq)
            };
        }

        if let Some(mode) = info.broadcast {
            return FragmentType::Broadcast(mode);
        }

        // we need to calculate a digest to deduplicate
        let this_hash = xxh64(request.raw_fragment, 0);

        let object_headers = match request.objects {
            Ok(x) => x,
            Err(err) => return FragmentType::MalformedRequest(this_hash, err),
        };

        // detect duplicate requests
        if let Some(last) = self.state.last_valid_request {
            if last.seq == request.header.control.seq && last.request_hash == this_hash {
                return if request.header.function == FunctionCode::Read {
                    FragmentType::RepeatRead(this_hash, object_headers)
                } else {
                    FragmentType::RepeatNonRead(this_hash)
                };
            }
        }

        if request.header.function == FunctionCode::Read {
            FragmentType::NewRead(this_hash, object_headers)
        } else {
            FragmentType::NewNonRead(this_hash, object_headers)
        }
    }

    /// Table 4-9: a SELECT with the selection's sequence number but different contents is discarded
    fn is_discarded_select(
        &self,
        fragment: &FragmentType,
        function: FunctionCode,
        seq: Sequence,
    ) -> bool {
        let discarded = matches!(fragment, FragmentType::NewNonRead(_, _))
            && function == FunctionCode::Select
            && self.state.select.is_some_and(|select| select.seq() == seq);
        if discarded {
            tracing::warn!("discarding SELECT that repeats the selection's sequence number with different contents");
        }
        discarded
    }

    /// Decide what a request does to the selection, ending it unless it's kept
    fn selection_for(
        &mut self,
        fragment: &FragmentType,
        function: FunctionCode,
        seq: Sequence,
    ) -> SelectionAction {
        if self.is_discarded_select(fragment, function, seq) {
            return SelectionAction::Discard;
        }
        match fragment {
            FragmentType::SolicitedConfirm(_) | FragmentType::UnsolicitedConfirm(_) => {
                return SelectionAction::Keep
            }
            FragmentType::RepeatNonRead(hash) => {
                if let Some(select) = self.state.select {
                    if select.request_hash() == *hash {
                        return SelectionAction::Keep;
                    }
                }
            }
            FragmentType::MalformedRequest(_, _)
            | FragmentType::NewRead(_, _)
            | FragmentType::RepeatRead(_, _)
            | FragmentType::NewNonRead(_, _)
            | FragmentType::Broadcast(_) => {}
        }
        SelectionAction::Take(self.state.select.take())
    }

    fn on_link_activity(&mut self) {
        self.next_link_status = self
            .config
            .keep_alive_timeout
            .map(|timeout| deadline(tokio::time::Instant::now(), timeout));
    }
}

impl From<ObjectParseError> for Iin2 {
    fn from(err: ObjectParseError) -> Self {
        match err {
            ObjectParseError::InsufficientBytes => Iin2::PARAMETER_ERROR,
            ObjectParseError::InvalidQualifierForVariation(_, _) => Iin2::PARAMETER_ERROR,
            ObjectParseError::InvalidRange(_, _) => Iin2::PARAMETER_ERROR,
            ObjectParseError::UnknownGroupVariation(_, _) => Iin2::OBJECT_UNKNOWN,
            ObjectParseError::UnsupportedQualifierCode(_) => Iin2::PARAMETER_ERROR,
            ObjectParseError::UnknownQualifier(_) => Iin2::PARAMETER_ERROR,
            ObjectParseError::ZeroLengthOctetData => Iin2::PARAMETER_ERROR,
            ObjectParseError::BadAttribute(_) => Iin2::PARAMETER_ERROR,
            ObjectParseError::BadEncoding => Iin2::PARAMETER_ERROR,
            ObjectParseError::UnsupportedFreeFormatCount(_) => Iin2::PARAMETER_ERROR,
        }
    }
}

impl From<Group34Var1> for f64 {
    fn from(x: Group34Var1) -> Self {
        x.value.into()
    }
}

impl From<Group34Var2> for f64 {
    fn from(x: Group34Var2) -> Self {
        x.value.into()
    }
}

impl From<Group34Var3> for f64 {
    fn from(x: Group34Var3) -> Self {
        x.value.into()
    }
}
