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
use crate::outstation::control::collection::{
    ControlCollection, ControlLimit, ControlMode, ControlTransaction,
};
use crate::outstation::control::select::SelectState;
use crate::outstation::database::DatabaseHandle;
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

/// A control request, with what its function needs to know about the selection
#[derive(Copy, Clone)]
enum ControlRequest {
    Select {
        /// the request's hash, which the selection records to recognize the SELECT's retry
        request_hash: u64,
    },
    Operate {
        /// the selection that this request ended
        selection: Option<SelectState>,
    },
    DirectOperate,
    DirectOperateNoAck,
}

impl ControlRequest {
    fn expects_response(self) -> bool {
        match self {
            ControlRequest::Select { .. }
            | ControlRequest::Operate { .. }
            | ControlRequest::DirectOperate => true,
            ControlRequest::DirectOperateNoAck => false,
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

#[derive(Copy, Clone)]
struct LastValidRequest {
    seq: Sequence,
    request_hash: u64,
}

impl LastValidRequest {
    fn new(seq: Sequence, request_hash: u64) -> Self {
        LastValidRequest { seq, request_hash }
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
            max_read_headers_per_request: x.read_header_limit(),
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

/// a broadcast whose IIN1.0 hasn't been cleared
#[derive(Copy, Clone)]
struct PendingBroadcast {
    mode: BroadcastConfirmMode,
    /// a solicited response has carried IIN1.0 for this broadcast
    solicited_reported: bool,
    /// an unsolicited response has carried IIN1.0 for this broadcast
    unsolicited_reported: bool,
}

#[derive(Copy, Clone)]
enum ResponseKind {
    Solicited,
    Unsolicited,
}

/// IIN1.0 for the last broadcast received. It ends with the first response that reports it, or,
/// when the broadcast requires a confirm, once the master confirms a response that reported it.
#[derive(Default)]
struct BroadcastIin(Option<PendingBroadcast>);

impl BroadcastIin {
    fn received(&mut self, mode: BroadcastConfirmMode) {
        self.0 = Some(PendingBroadcast {
            mode,
            solicited_reported: false,
            unsolicited_reported: false,
        });
    }

    /// Report the broadcast in a response: IIN1.0, and in a solicited response, CON if the
    /// broadcast requires a confirm
    fn report(&mut self, header: &mut ResponseHeader, kind: ResponseKind) {
        let Some(pending) = &mut self.0 else {
            return;
        };

        header.iin |= Iin1::BROADCAST;

        if pending.mode != BroadcastConfirmMode::Mandatory {
            self.0 = None;
            return;
        }

        match kind {
            ResponseKind::Solicited => {
                header.control.con = true;
                pending.solicited_reported = true;
            }
            ResponseKind::Unsolicited => pending.unsolicited_reported = true,
        }
    }

    /// The master confirmed a response of this kind. Returns true if that ended the IIN1.0.
    fn confirmed(&mut self, kind: ResponseKind) -> bool {
        let reported = self.0.is_some_and(|pending| match kind {
            ResponseKind::Solicited => pending.solicited_reported,
            ResponseKind::Unsolicited => pending.unsolicited_reported,
        });
        if reported {
            self.0 = None;
        }
        reported
    }
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
    broadcast: BroadcastIin,
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
            broadcast: BroadcastIin::default(),
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

/// What the unsolicited confirm wait expects (IEEE 1815-2012 Table 6-1 rows 20-34)
struct UnsolicitedWait {
    /// the sequence number of the unsolicited response
    seq: Sequence,
    /// the confirm of a solicited response sent during the wait, if one asked for it
    solicited: Option<ExpectedConfirm>,
}

/// A solicited response's confirm, expected until the confirm timeout. The timeout is checked
/// when a confirm arrives, rather than with a timer
#[derive(Copy, Clone)]
struct ExpectedConfirm {
    ecsn: Sequence,
    deadline: tokio::time::Instant,
}

#[derive(Copy, Clone)]
enum UnsolicitedWaitResult {
    Timeout,
    ReadNext,
    Complete(UnsolicitedResult),
}

/// A request, classified against the last valid request
#[derive(Copy, Clone)]
enum FragmentType<'a> {
    MalformedRequest(u64, ObjectParseError),
    NewRead(u64, HeaderCollection<'a>),
    RepeatRead(u64, HeaderCollection<'a>),
    NewNonRead(u64, HeaderCollection<'a>),
    RepeatNonRead(u64),
    Broadcast(BroadcastConfirmMode),
}

impl FragmentType<'_> {
    fn is_read(&self) -> bool {
        match self {
            FragmentType::NewRead(..) | FragmentType::RepeatRead(..) => true,
            FragmentType::MalformedRequest(..)
            | FragmentType::NewNonRead(..)
            | FragmentType::RepeatNonRead(_)
            | FragmentType::Broadcast(_) => false,
        }
    }
}

/// A fragment from the master, after [`OutstationSession::receive`] filtered and classified it
enum Intake<'a> {
    /// nothing to answer: no fragment, a link-layer message, a fragment that isn't a valid
    /// request, or a SELECT that the selection discards
    Nothing,
    /// a request with an unknown function code, answered with IIN2.0
    UnknownFunction(FragmentAddr, Sequence),
    /// the confirm of a solicited or an unsolicited response
    Confirm(FragmentAddr, RequestHeader),
    /// a request, for the state to answer or defer
    Request(Received<'a>),
}

#[derive(Copy, Clone)]
struct Received<'a> {
    info: FragmentInfo,
    request: Request<'a>,
    fragment: FragmentType<'a>,
}

/// What a state does with a READ
#[derive(Copy, Clone)]
enum Reads {
    /// answer it now
    Answer,
    /// keep it until the session returns to idle (IEEE 1815-2012 Table 6-1 row 25)
    Defer,
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

    fn pop_request<'a, T: Transport>(
        &self,
        reader: &'a mut TransportReader<T>,
    ) -> RequestGuard<'a, T> {
        reader.pop_request(
            self.required_master_address(),
            self.config.decode_level.application,
        )
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
        self.state
            .broadcast
            .report(response.header_mut(), ResponseKind::Unsolicited);

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
        self.state.broadcast.report(header, ResponseKind::Solicited);
    }

    /// The confirm that a written solicited response waits for: its own, or a mandatory
    /// broadcast's
    fn awaited_confirm(header: ResponseHeader) -> Option<ResponseSeries> {
        header
            .control
            .con
            .then(|| ResponseSeries::new(header.control.seq, header.control.fin))
    }

    /// Answer a request that becomes the last valid request, publishing its response if it has
    /// one. Returns the confirm that the response waits for.
    async fn respond<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        writer: &mut TransportWriter<T>,
        respond_to: FragmentAddr,
        request: LastValidRequest,
        response: Option<Prepared>,
        database: &DatabaseHandle,
    ) -> Result<Option<ResponseSeries>, LinkError> {
        let confirm = match response {
            Some(response) => Self::awaited_confirm(
                self.write_solicited(io, writer, respond_to, response, database)
                    .await?,
            ),
            None => {
                self.sol.clear();
                None
            }
        };
        self.state.last_valid_request = Some(request);
        Ok(confirm)
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

        // IEEE 1815-2012 Table 6-1 rows 22 and 33 would go on to wait for the solicited confirm
        // after the unsolicited response's wait ends. This wait never does: confirming a
        // solicited response from here only clears a broadcast's IIN1.0, while the solicited
        // confirm wait would settle the unsolicited response's events
        let mut wait = UnsolicitedWait {
            seq,
            solicited: None,
        };

        loop {
            match self
                .wait_for_unsolicited_confirm(&mut wait, deadline, io, reader, writer, database)
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
        wait: &mut UnsolicitedWait,
        deadline: tokio::time::Instant,
        io: &mut PhysLayer,
        reader: &mut TransportReader<T>,
        writer: &mut TransportWriter<T>,
        database: &mut DatabaseHandle,
    ) -> Result<UnsolicitedWaitResult, RunError> {
        if let TimeoutStatus::Yes = self.read_until(io, reader, deadline).await? {
            return Ok(UnsolicitedWaitResult::Timeout);
        }

        let guard = self.pop_request(reader);
        let received = match self.receive(&guard) {
            Intake::Nothing => return Ok(UnsolicitedWaitResult::ReadNext),
            Intake::UnknownFunction(from, seq) => {
                // like a new request, it ends the wait for a solicited confirm, as it ends the
                // solicited confirm wait. Its IIN2.0 answer isn't the response to a valid request,
                // so the outstation doesn't wait for that answer's confirm
                wait.solicited = None;
                self.answer_unknown_function(io, writer, from, seq, database)
                    .await?;
                return Ok(UnsolicitedWaitResult::ReadNext);
            }
            Intake::Confirm(_, header) => {
                return Ok(self.confirm_in_unsolicited_wait(wait, header));
            }
            Intake::Request(received) => received,
        };

        let selection = self.take_selection(&received.fragment);
        let disable = matches!(received.fragment, FragmentType::NewNonRead(..))
            && received.request.header.function == FunctionCode::DisableUnsolicited;

        // Table 6-1 rows 24-29: a request ends the wait for a solicited confirm, and an answer that
        // asks for one starts it again
        let confirm = self
            .accept(io, writer, database, received, selection, Reads::Defer)
            .await?;
        wait.solicited = confirm.map(|series| ExpectedConfirm {
            ecsn: series.ecsn,
            deadline: self.new_confirm_deadline(),
        });

        if disable {
            // DISABLE_UNSOLICITED ends the unsolicited response series
            return Ok(UnsolicitedWaitResult::Complete(
                UnsolicitedResult::ReturnToIdle,
            ));
        }

        Ok(UnsolicitedWaitResult::ReadNext)
    }

    fn confirm_in_unsolicited_wait(
        &mut self,
        wait: &mut UnsolicitedWait,
        header: RequestHeader,
    ) -> UnsolicitedWaitResult {
        let seq = header.control.seq;
        if header.control.uns {
            if seq == wait.seq {
                self.state.broadcast.confirmed(ResponseKind::Unsolicited);
                self.info.unsolicited_confirmed(seq);
                return UnsolicitedWaitResult::Complete(UnsolicitedResult::Confirmed);
            }
            tracing::warn!(
                "ignoring unsolicited confirm with wrong sequence number ({})",
                seq.value()
            );
        } else {
            // Table 6-1 rows 20, 21 and 30: only the expected confirm, before its timeout
            let now = tokio::time::Instant::now();
            wait.solicited.take_if(|expected| now >= expected.deadline);
            if wait
                .solicited
                .take_if(|expected| expected.ecsn == seq)
                .is_some()
            {
                self.state.broadcast.confirmed(ResponseKind::Solicited);
            } else {
                tracing::warn!("ignoring solicited confirm with seq: {}", seq.value());
            }
        }
        UnsolicitedWaitResult::ReadNext
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
            let response = self.format_read_response(database, true, x.seq, x.iin2);
            let confirm = self
                .respond(
                    io,
                    writer,
                    x.info.addr,
                    LastValidRequest::new(x.seq, x.hash),
                    Some(response),
                    database,
                )
                .await?;

            if let Some(series) = confirm {
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
        let guard = self.pop_request(reader);
        let confirm = match self.receive(&guard) {
            Intake::Nothing => None,
            Intake::UnknownFunction(from, seq) => {
                self.answer_unknown_function(io, writer, from, seq, database)
                    .await?;
                None
            }
            Intake::Confirm(_, header) => {
                self.info.process_request_from_idle(header);
                let kind = if header.control.uns {
                    "unsolicited"
                } else {
                    "solicited"
                };
                tracing::warn!(
                    "ignoring {kind} CONFIRM from idle state with seq: {}",
                    header.control.seq.value()
                );
                None
            }
            Intake::Request(received) => {
                let selection = self.take_selection(&received.fragment);
                self.info.process_request_from_idle(received.request.header);
                self.accept(io, writer, database, received, selection, Reads::Answer)
                    .await?
            }
        };
        drop(guard);

        if let Some(series) = confirm {
            // enter the solicited confirm wait state
            self.sol_confirm_wait(io, reader, writer, database, series)
                .instrument(tracing::info_span!(
                    "SolConfirmWait",
                    "ecsn" = series.ecsn.value()
                ))
                .await?;
        }

        Ok(())
    }

    /// Take the master's next fragment. This is where every request enters the session, in any
    /// state: any fragment from the master is link activity, and a request is classified once,
    /// against the last valid request and the selection.
    fn receive<'a, T: Transport>(&mut self, guard: &'a RequestGuard<'_, T>) -> Intake<'a> {
        let (info, request) = match guard.get() {
            None => return Intake::Nothing,
            Some(TransportRequest::LinkLayerMessage) => {
                self.on_link_activity();
                return Intake::Nothing;
            }
            Some(TransportRequest::Error(from, err)) => {
                self.on_link_activity();
                return match Self::error_response_seq(err) {
                    Some(seq) => Intake::UnknownFunction(from, seq),
                    None => Intake::Nothing,
                };
            }
            Some(TransportRequest::Request(info, request)) => {
                self.on_link_activity();
                (info, request)
            }
        };

        if request.header.function == FunctionCode::Confirm {
            return Intake::Confirm(info.addr, request.header);
        }

        let fragment = self.classify(info, request);
        if self.is_discarded_select(
            &fragment,
            request.header.function,
            request.header.control.seq,
        ) {
            return Intake::Nothing;
        }

        Intake::Request(Received {
            info,
            request,
            fragment,
        })
    }

    /// Accept a request in idle or the unsolicited confirm wait (IEEE 1815-2012 Table 6-1 rows
    /// 4-9 and 24-29): answer it, or defer a READ. A request that isn't a READ discards the
    /// deferred READ. Returns the confirm that the answer waits for.
    async fn accept<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        writer: &mut TransportWriter<T>,
        database: &mut DatabaseHandle,
        received: Received<'_>,
        selection: Option<SelectState>,
        reads: Reads,
    ) -> Result<Option<ResponseSeries>, LinkError> {
        let Received {
            info,
            request,
            fragment,
        } = received;
        let seq = request.header.control.seq;

        if !fragment.is_read() {
            self.state.deferred_read.clear();
        }

        match fragment {
            FragmentType::NewRead(hash, objects) | FragmentType::RepeatRead(hash, objects) => {
                match reads {
                    Reads::Answer => {
                        // IEEE 1815-2012 Table 6-1 (Idle, row 8) says to resend the previous
                        // response to a repeated READ. We deliberately answer it as a new READ
                        // instead: the fresh response carries current values, and events that were
                        // never confirmed are still buffered, so they are reported again
                        let response = self.format_first_read_response(database, seq, objects);
                        self.respond(
                            io,
                            writer,
                            info.addr,
                            LastValidRequest::new(seq, hash),
                            Some(response),
                            database,
                        )
                        .await
                    }
                    Reads::Defer => {
                        self.state.deferred_read.set(hash, seq, info, objects);
                        Ok(None)
                    }
                }
            }
            FragmentType::MalformedRequest(hash, err) => {
                // IEEE 1815-2012 clause 6.1: a request whose objects don't parse is still the last
                // request, answered with IIN2
                let response = Prepared::empty_solicited(seq, Iin::default() | Iin2::from(err));
                self.respond(
                    io,
                    writer,
                    info.addr,
                    LastValidRequest::new(seq, hash),
                    Some(response),
                    database,
                )
                .await
            }
            FragmentType::NewNonRead(hash, objects) => {
                let response = self
                    .handle_non_read(
                        database,
                        request.header.function,
                        seq,
                        hash,
                        selection,
                        objects,
                    )
                    .await;
                self.respond(
                    io,
                    writer,
                    info.addr,
                    LastValidRequest::new(seq, hash),
                    response,
                    database,
                )
                .await
            }
            FragmentType::RepeatNonRead(_) => {
                // IEEE 1815-2012 Table 6-1: send the same response, byte for byte, which waits for
                // its confirm again if it asked for one
                self.repeat_solicited(io, info.addr, writer).await?;
                Ok(self.sol.header().and_then(Self::awaited_confirm))
            }
            FragmentType::Broadcast(mode) => {
                self.process_broadcast(database, mode, request).await;
                Ok(None)
            }
        }
    }

    /// Answer a request with an unknown function code with IIN2.0. It ends the selection and
    /// discards a deferred READ, but isn't a valid request, so the last valid request and its
    /// response stay (TB2016-001).
    async fn answer_unknown_function<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        writer: &mut TransportWriter<T>,
        respond_to: FragmentAddr,
        seq: Sequence,
        database: &DatabaseHandle,
    ) -> Result<(), LinkError> {
        self.state.select = None;
        self.state.deferred_read.clear();
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

    fn format_first_read_response(
        &mut self,
        database: &mut DatabaseHandle,
        seq: Sequence,
        object_headers: HeaderCollection,
    ) -> Prepared {
        let iin2 = database.select(&object_headers);
        self.format_read_response(database, true, seq, iin2)
    }

    fn format_read_response(
        &mut self,
        database: &mut DatabaseHandle,
        fir: bool,
        seq: Sequence,
        iin2: Iin2,
    ) -> Prepared {
        let info = match self
            .sol
            .build(|cursor| Ok(database.write_response_headers(cursor)))
        {
            Ok(info) => info,
            Err(err) => {
                tracing::error!("unable to write a READ response: {err:?}");
                return Prepared::parameter_error(seq);
            }
        };

        let header = ResponseHeader::new(
            ControlField::response(seq, fir, info.complete, info.need_confirm()),
            ResponseFunction::Response,
            Iin::default() | iin2,
        );
        Prepared::Stored(header)
    }

    async fn handle_non_read(
        &mut self,
        database: &mut DatabaseHandle,
        function: FunctionCode,
        seq: Sequence,
        request_hash: u64,
        selection: Option<SelectState>,
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
                self.handle_controls(
                    ControlRequest::Select { request_hash },
                    database,
                    seq,
                    object_headers,
                )
                .await
            }
            FunctionCode::Operate => {
                self.handle_controls(
                    ControlRequest::Operate { selection },
                    database,
                    seq,
                    object_headers,
                )
                .await
            }
            FunctionCode::DirectOperate => {
                self.handle_controls(ControlRequest::DirectOperate, database, seq, object_headers)
                    .await
            }
            FunctionCode::DirectOperateNoResponse => {
                self.handle_controls(
                    ControlRequest::DirectOperateNoAck,
                    database,
                    seq,
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
        if function.objects_allowed() {
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
            HeaderDetails::Range(_, _, RangedVariation::Group0(_, attr)) => {
                self.handle_write_attr(attr, db).await
            }
            HeaderDetails::Range(_, _, RangedVariation::Group80Var1(bits)) => {
                self.handle_write_iin(bits)
            }
            HeaderDetails::Count(_, _, CountVariation::Group50Var1(seq)) => {
                self.handle_write_abs_time(seq)
            }
            HeaderDetails::Count(_, _, CountVariation::Group50Var3(seq)) => {
                self.handle_write_at_last_recorded_time(seq)
            }
            // analog deadbands
            HeaderDetails::CountAndPrefix8(_, PrefixedVariation::Group34Var1(seq)) => {
                self.handle_write_analog_deadbands(seq, db).await
            }
            HeaderDetails::CountAndPrefix8(_, PrefixedVariation::Group34Var2(seq)) => {
                self.handle_write_analog_deadbands(seq, db).await
            }
            HeaderDetails::CountAndPrefix8(_, PrefixedVariation::Group34Var3(seq)) => {
                self.handle_write_analog_deadbands(seq, db).await
            }
            HeaderDetails::CountAndPrefix16(_, PrefixedVariation::Group34Var1(seq)) => {
                self.handle_write_analog_deadbands(seq, db).await
            }
            HeaderDetails::CountAndPrefix16(_, PrefixedVariation::Group34Var2(seq)) => {
                self.handle_write_analog_deadbands(seq, db).await
            }
            HeaderDetails::CountAndPrefix16(_, PrefixedVariation::Group34Var3(seq)) => {
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
        let result = self
            .execute_controls(
                database,
                ControlMode::Operate(OperateType::DirectOperate),
                &controls,
            )
            .await;

        Self::control_response(seq, result)
    }

    /// Execute the controls and build the response that echoes them, returning their overall
    /// status
    async fn execute_controls(
        &mut self,
        database: &mut DatabaseHandle,
        mode: ControlMode,
        controls: &ControlCollection<'_>,
    ) -> Result<CommandStatus, scursor::WriteError> {
        let max_controls_per_request = self.config.max_controls_per_request;
        ControlTransaction::execute(self.control_handler.borrow_mut(), database, |tx, db| {
            self.sol.build(|cursor| {
                controls.execute_with_response(
                    cursor,
                    mode,
                    tx,
                    db,
                    ControlLimit::new(max_controls_per_request),
                )
            })
        })
        .await
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
            controls.operate_no_ack(tx, db, ControlLimit::new(max_controls_per_request))
        })
        .await;
    }

    async fn handle_controls(
        &mut self,
        request: ControlRequest,
        database: &mut DatabaseHandle,
        seq: Sequence,
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
                return request.expects_response().then(|| {
                    Prepared::empty_solicited(seq, Iin::default() | Iin2::OBJECT_UNKNOWN)
                });
            }
            Ok(controls) => controls,
        };

        if request.expects_response() && !self.control_response_fits(&controls) {
            tracing::warn!(
                "ignoring control request whose response doesn't fit in the response buffer"
            );
            return Some(Prepared::parameter_error(seq));
        }

        match request {
            ControlRequest::Select { request_hash } => Some(
                self.handle_select(database, seq, request_hash, controls)
                    .await,
            ),
            ControlRequest::Operate { selection } => Some(
                self.handle_operate(database, seq, selection, controls)
                    .await,
            ),
            ControlRequest::DirectOperate => {
                Some(self.handle_direct_operate(database, seq, controls).await)
            }
            ControlRequest::DirectOperateNoAck => {
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
        let result = self
            .execute_controls(database, ControlMode::Select, &controls)
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
                self.execute_controls(
                    database,
                    ControlMode::Operate(OperateType::SelectBeforeOperate),
                    &controls,
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
                HeaderDetails::Count(_, _, CountVariation::Group50Var2(seq)) => {
                    match seq.single() {
                        None => iin.iin2.set(Iin2::PARAMETER_ERROR),
                        Some(x) => {
                            timing = Some(x.into());
                        }
                    }
                }
                HeaderDetails::AllObjects(_)
                | HeaderDetails::Range(..)
                | HeaderDetails::Count(..)
                | HeaderDetails::CountAndPrefix8(..)
                | HeaderDetails::CountAndPrefix16(..)
                | HeaderDetails::FreeFormat(_)
                | HeaderDetails::Read(_) => {
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
            HeaderDetails::Range(_, range, RangedVariation::Group20Var0) => self
                .application
                .freeze_counter(
                    FreezeIndices::Range(range.get_start(), range.get_stop()),
                    freeze_type,
                    database,
                )
                .map_or_else(|err| err.into(), |_| Iin2::default()),
            _ => match header.variation {
                Variation::Group20Var0 => Iin2::PARAMETER_ERROR,
                _ => Iin2::OBJECT_UNKNOWN,
            },
        }
    }

    fn get_response_iin(&self, database: &DatabaseHandle) -> Iin {
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
        self.state.broadcast.received(mode);
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
                self.handle_controls(ControlRequest::DirectOperateNoAck, database, seq, objects)
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
                    self.state.broadcast.confirmed(ResponseKind::Solicited);

                    self.end_confirm_wait(database, ConfirmOutcome::Confirmed)
                        .await;

                    if series.fin {
                        // done with response series
                        return Ok(());
                    }
                    // format the next response in the series
                    series.ecsn.increment();
                    let response =
                        self.format_read_response(database, false, series.ecsn, Iin2::default());
                    let header = self
                        .write_solicited(io, writer, respond_to, response, database)
                        .await?;
                    match Self::awaited_confirm(header) {
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
                    let mut guard = self.pop_request(reader);
                    match self.expect_sol_confirm(ecsn, &guard) {
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
        guard: &RequestGuard<'_, T>,
    ) -> ConfirmAction {
        match self.receive(guard) {
            Intake::Nothing => ConfirmAction::ContinueWait,
            Intake::UnknownFunction(_, _) => ConfirmAction::NewRequest,
            Intake::Confirm(from, header) => {
                let seq = header.control.seq;
                if header.control.uns {
                    self.info.unexpected_confirm(true, seq);
                    tracing::warn!("ignoring unsolicited confirm with seq: {}", seq.value());
                    ConfirmAction::ContinueWait
                } else if seq == ecsn {
                    ConfirmAction::Confirmed(from)
                } else {
                    self.info.wrong_solicited_confirm_seq(ecsn, seq);
                    tracing::warn!(
                        "ignoring confirm with wrong sequence number: {}",
                        seq.value()
                    );
                    ConfirmAction::ContinueWait
                }
            }
            Intake::Request(received) => match received.fragment {
                FragmentType::RepeatRead(_, _) => {
                    // a request ends the selection; the READ that started this wait already did
                    self.state.select = None;
                    ConfirmAction::EchoLastResponse(received.info.addr)
                }
                // a new request ends the wait, and is answered from idle
                FragmentType::MalformedRequest(_, _)
                | FragmentType::NewRead(_, _)
                | FragmentType::NewNonRead(_, _)
                | FragmentType::RepeatNonRead(_)
                | FragmentType::Broadcast(_) => ConfirmAction::NewRequest,
            },
        }
    }

    fn classify<'a>(&self, info: FragmentInfo, request: Request<'a>) -> FragmentType<'a> {
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

    /// End the selection, giving it to an OPERATE to check, unless the request is a retry of the
    /// SELECT. IEEE 1815-2012 4.4.4 Rule 4 and Table 4-9: ending a selection early can only make
    /// an OPERATE fail, while keeping one too long could execute a control, so every request ends
    /// it except a retry of the SELECT and a SELECT that it discards. A confirm isn't a request.
    fn take_selection(&mut self, fragment: &FragmentType) -> Option<SelectState> {
        if let FragmentType::RepeatNonRead(hash) = fragment {
            if self
                .state
                .select
                .is_some_and(|select| select.request_hash() == *hash)
            {
                return None;
            }
        }
        self.state.select.take()
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
            #[expect(
                deprecated,
                reason = "the mapping must cover every variant until removal"
            )]
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
