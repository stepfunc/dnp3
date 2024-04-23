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
use crate::outstation::task::{ConfigurationChange, OutstationMessage};
use crate::outstation::traits::*;
use crate::transport::{
    FragmentAddr, FragmentInfo, RequestGuard, TransportReader, TransportRequest,
    TransportRequestError, TransportWriter,
};
use crate::util::buffer::Buffer;
use crate::util::channel::Receiver;
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

#[derive(Copy, Clone)]
struct Response {
    header: ResponseHeader,
    size: usize,
}

impl Response {
    fn new(header: ResponseHeader, size: usize) -> Self {
        Self { header, size }
    }

    fn empty_solicited(seq: Sequence, iin: Iin) -> Self {
        let header = ResponseHeader::new(
            ControlField::response(seq, true, true, false),
            ResponseFunction::Response,
            iin,
        );
        Self::new(header, 0)
    }

    fn seq(&self) -> Sequence {
        self.header.control.seq
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
            Some(x) => {
                if x == 0 {
                    false
                } else {
                    self.retries = Some(x - 1);
                    true
                }
            }
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
    response: Option<Response>,
    series: Option<ResponseSeries>,
}

impl LastValidRequest {
    fn new(
        seq: Sequence,
        request_hash: u64,
        response: Option<Response>,
        series: Option<ResponseSeries>,
    ) -> Self {
        LastValidRequest {
            seq,
            request_hash,
            response,
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
            max_read_headers_per_request: x
                .max_read_request_headers
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
    sol_tx_buffer: Buffer,
    unsol_tx_buffer: Buffer,
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
    RepeatRead(u64, Option<Response>, HeaderCollection<'a>),
    NewNonRead(u64, HeaderCollection<'a>),
    RepeatNonRead(u64, Option<Response>),
    Broadcast(BroadcastConfirmMode),
    SolicitedConfirm(Sequence),
    UnsolicitedConfirm(Sequence),
}

#[derive(Copy, Clone)]
enum ConfirmAction {
    Confirmed(FragmentAddr),
    NewRequest,
    EchoLastResponse(FragmentAddr, Option<Response>),
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
    #[allow(clippy::too_many_arguments)]
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
            .map(|delay| tokio::time::Instant::now() + delay);

        Self {
            enabled: initial_state,
            messages,
            config,
            sol_tx_buffer: param.sol_tx_buffer_size.create_buffer(),
            unsol_tx_buffer: param.unsol_tx_buffer_size.create_buffer(),
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

    pub(crate) fn enabled(&self) -> Enabled {
        self.enabled
    }

    /// used when the there is no running IO to process outstation messages
    pub(crate) async fn process_next_message(&mut self) -> Result<(), StopReason> {
        self.handle_next_message().await
    }

    pub(crate) async fn run(
        &mut self,
        io: &mut PhysLayer,
        reader: &mut TransportReader,
        writer: &mut TransportWriter,
        database: &mut DatabaseHandle,
    ) -> RunError {
        loop {
            if let Err(err) = self.run_idle_state(io, reader, writer, database).await {
                self.state.reset();
                return err;
            }
        }
    }

    async fn write_unsolicited(
        &mut self,
        io: &mut PhysLayer,
        writer: &mut TransportWriter,
        mut response: Response,
        database: &DatabaseHandle,
    ) -> Result<Response, LinkError> {
        response.header.iin |= self.get_response_iin(database);

        self.repeat_unsolicited(io, writer, response).await?;

        Ok(response)
    }

    async fn repeat_unsolicited(
        &mut self,
        io: &mut PhysLayer,
        writer: &mut TransportWriter,
        response: Response,
    ) -> Result<(), LinkError> {
        let mut cursor = self.unsol_tx_buffer.write_cursor();
        let _ = response.header.write(&mut cursor);

        let len = std::cmp::max(cursor.written().len(), response.size);

        writer
            .write(
                io,
                self.config.decode_level,
                self.destination,
                self.unsol_tx_buffer.get(len).unwrap(),
            )
            .await
    }

    async fn write_solicited(
        &mut self,
        io: &mut PhysLayer,
        writer: &mut TransportWriter,
        respond_to: FragmentAddr,
        mut response: Response,
        database: &DatabaseHandle,
    ) -> Result<Response, LinkError> {
        response.header.iin |= self.get_response_iin(database);

        // Determine if we need to ask for confirmation due to broadcast
        if let Some(BroadcastConfirmMode::Mandatory) = self.state.last_broadcast_type {
            if !response.header.control.con {
                response.header.control.con = true;
            }
        }

        self.repeat_solicited(io, respond_to, writer, response)
            .await?;

        Ok(response)
    }

    async fn repeat_solicited(
        &mut self,
        io: &mut PhysLayer,
        respond_to: FragmentAddr,
        writer: &mut TransportWriter,
        response: Response,
    ) -> Result<(), LinkError> {
        let mut cursor = self.sol_tx_buffer.write_cursor();
        let _ = response.header.write(&mut cursor);

        let len = std::cmp::max(cursor.written().len(), response.size);

        writer
            .write(
                io,
                self.config.decode_level,
                respond_to,
                self.sol_tx_buffer.get(len).unwrap(),
            )
            .await
    }

    async fn run_idle_state(
        &mut self,
        io: &mut PhysLayer,
        reader: &mut TransportReader,
        writer: &mut TransportWriter,
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

    async fn check_unsolicited(
        &mut self,
        io: &mut PhysLayer,
        reader: &mut TransportReader,
        writer: &mut TransportWriter,
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
                        let retry_at = self.new_unsolicited_retry_deadline();
                        self.state.unsolicited = UnsolicitedState::Ready(Some(retry_at));
                        Ok(NextIdleAction::SleepUnit(retry_at))
                    }
                    Some(UnsolicitedResult::Confirmed) => {
                        database
                            .clear_written_events(self.application.as_mut())
                            .await;
                        self.state.unsolicited = UnsolicitedState::Ready(None);
                        Ok(NextIdleAction::NoSleep)
                    }
                }
            }
        }
    }

    async fn check_link_status(
        &mut self,
        io: &mut PhysLayer,
        writer: &mut TransportWriter,
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

    async fn perform_null_unsolicited(
        &mut self,
        io: &mut PhysLayer,
        reader: &mut TransportReader,
        writer: &mut TransportWriter,
        database: &mut DatabaseHandle,
    ) -> Result<UnsolicitedResult, RunError> {
        let header = ResponseHeader::new(
            ControlField::unsolicited_response(self.state.unsolicited_seq.increment()),
            ResponseFunction::UnsolicitedResponse,
            Iin::default(),
        );
        self.perform_unsolicited_response_series(
            database,
            Response::new(header, 0),
            true,
            io,
            reader,
            writer,
        )
        .await
    }

    async fn maybe_perform_unsolicited(
        &mut self,
        io: &mut PhysLayer,
        reader: &mut TransportReader,
        writer: &mut TransportWriter,
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

    fn write_unsolicited_data(&mut self, database: &mut DatabaseHandle) -> Option<Response> {
        let mut cursor = self.unsol_tx_buffer.write_cursor();
        let _ = cursor.skip(ResponseHeader::LENGTH);
        let count = database.write_unsolicited(self.state.enabled_unsolicited_classes, &mut cursor);

        if count == 0 {
            return None;
        }

        let seq = self.state.unsolicited_seq.increment();
        let header = ResponseHeader::new(
            ControlField::unsolicited_response(seq),
            ResponseFunction::UnsolicitedResponse,
            Iin::default(),
        );
        Some(Response::new(header, cursor.written().len()))
    }

    async fn perform_unsolicited_response_series(
        &mut self,
        database: &mut DatabaseHandle,
        response: Response,
        is_null: bool,
        io: &mut PhysLayer,
        reader: &mut TransportReader,
        writer: &mut TransportWriter,
    ) -> Result<UnsolicitedResult, RunError> {
        let response = self
            .write_unsolicited(io, writer, response, database)
            .await?;

        // enter unsolicited confirm wait state
        self.info.enter_unsolicited_confirm_wait(response.seq());

        let mut retry_count = RetryCounter::new(self.config.max_unsolicited_retries);

        // For null responses, we want to regenerate a response everytime (see section 5.1.1.1.1 Rule 2)
        if is_null {
            retry_count = RetryCounter::new(Some(0));
        }

        let mut deadline = self.new_confirm_deadline();

        loop {
            match self
                .wait_for_unsolicited_confirm(
                    response.seq(),
                    deadline,
                    io,
                    reader,
                    writer,
                    database,
                )
                .instrument(tracing::info_span!(
                    "UnsolConfirmWait",
                    "seq" = response.seq().value()
                ))
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

                    self.info.unsolicited_confirm_timeout(response.seq(), retry);

                    if !retry {
                        return Ok(UnsolicitedResult::Timeout);
                    }

                    // perform a retry
                    self.repeat_unsolicited(io, writer, response).await?;

                    // update the deadline
                    deadline = self.new_confirm_deadline();
                }
            }
        }
    }

    async fn wait_for_unsolicited_confirm(
        &mut self,
        uns_ecsn: Sequence,
        deadline: tokio::time::Instant,
        io: &mut PhysLayer,
        reader: &mut TransportReader,
        writer: &mut TransportWriter,
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
                self.state.deferred_read.clear();
                self.write_error_response(io, from, writer, err, database)
                    .await?;
                return Ok(UnsolicitedWaitResult::ReadNext);
            }
        };

        match self.classify(info, request) {
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
                self.process_broadcast(info.id, database, mode, request)
                    .await;
                Ok(UnsolicitedWaitResult::ReadNext)
            }
            FragmentType::MalformedRequest(_, err) => {
                self.state.deferred_read.clear();

                let seq = request.header.control.seq;
                let iin = Iin::default() | Iin2::from(err);
                self.write_solicited(
                    io,
                    writer,
                    info.addr,
                    Response::empty_solicited(seq, iin),
                    database,
                )
                .await?;
                Ok(UnsolicitedWaitResult::ReadNext)
            }
            FragmentType::NewNonRead(hash, objects) => {
                self.state.deferred_read.clear();
                let mut response = self
                    .handle_non_read(
                        database,
                        request.header.function,
                        request.header.control.seq,
                        info.id,
                        objects,
                    )
                    .await;
                if let Some(response) = &mut response {
                    *response = self
                        .write_solicited(io, writer, info.addr, *response, database)
                        .await?;
                }
                self.state.last_valid_request = Some(LastValidRequest::new(
                    request.header.control.seq,
                    hash,
                    response,
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
            FragmentType::NewRead(hash, headers) => {
                tracing::info_span!("deferring READ request");
                self.state
                    .deferred_read
                    .set(hash, request.header.control.seq, info, headers);
                Ok(UnsolicitedWaitResult::ReadNext)
            }
            FragmentType::RepeatRead(hash, _, headers) => {
                tracing::info_span!("deferring READ request");
                self.state
                    .deferred_read
                    .set(hash, request.header.control.seq, info, headers);
                Ok(UnsolicitedWaitResult::ReadNext)
            }
            FragmentType::RepeatNonRead(_, last_response) => {
                if let Some(last_response) = last_response {
                    self.repeat_solicited(io, info.addr, writer, last_response)
                        .await?
                }
                self.state.deferred_read.clear();
                Ok(UnsolicitedWaitResult::ReadNext)
            }
        }
    }

    async fn read_until(
        &mut self,
        io: &mut PhysLayer,
        reader: &mut TransportReader,
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

    async fn handle_deferred_read(
        &mut self,
        io: &mut PhysLayer,
        reader: &mut TransportReader,
        writer: &mut TransportWriter,
        database: &mut DatabaseHandle,
    ) -> Result<(), RunError> {
        if let Some(x) = self.state.deferred_read.select(database) {
            tracing::info!("handling deferred READ request");
            let (response, mut series) = self.format_read_response(database, true, x.seq, x.iin2);
            let response = self
                .write_solicited(io, writer, x.info.addr, response, database)
                .await?;
            self.state.last_valid_request =
                Some(LastValidRequest::new(x.seq, x.hash, Some(response), series));

            // check if an extra confirmation was added due to broadcast
            if response.header.control.con && series.is_none() {
                series = Some(ResponseSeries::new(response.header.control.seq, true));
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

    async fn handle_one_request_from_idle(
        &mut self,
        io: &mut PhysLayer,
        reader: &mut TransportReader,
        writer: &mut TransportWriter,
        database: &mut DatabaseHandle,
    ) -> Result<(), RunError> {
        let mut guard = reader.pop_request(self.required_master_address());
        match guard.get() {
            Some(TransportRequest::Request(info, request)) => {
                self.on_link_activity();
                if let Some(mut result) = self
                    .process_request_from_idle(info, request, database)
                    .await
                {
                    // optional response
                    if let Some(response) = &mut result.response {
                        *response = self
                            .write_solicited(io, writer, info.addr, *response, database)
                            .await?;

                        // check if an extra confirmation was added due to broadcast
                        if response.header.control.con && result.series.is_none() {
                            result.series =
                                Some(ResponseSeries::new(response.header.control.seq, true));
                        }
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
            Some(TransportRequest::LinkLayerMessage) => {
                self.on_link_activity();
            }
            Some(TransportRequest::Error(from, err)) => {
                self.on_link_activity();
                self.write_error_response(io, from, writer, err, database)
                    .await?;
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
    ) -> Option<LastValidRequest> {
        self.info.process_request_from_idle(request.header);

        let seq = request.header.control.seq;

        match self.classify(info, request) {
            FragmentType::MalformedRequest(hash, err) => {
                let response = Response::empty_solicited(seq, Iin::default() | Iin2::from(err));
                Some(LastValidRequest::new(seq, hash, Some(response), None))
            }
            FragmentType::NewRead(hash, objects) => {
                let (response, series) = self.format_first_read_response(database, seq, objects);
                Some(LastValidRequest::new(seq, hash, Some(response), series))
            }
            FragmentType::RepeatRead(hash, _, objects) => {
                // this deviates a bit from the spec, the specification says to
                // also reply to duplicate READ requests from idle, but this
                // is plainly wrong since it can't possibly handle a multi-fragmented
                // response correctly. Answering a repeat READ with a fresh response is harmless
                let (response, series) = self.format_first_read_response(database, seq, objects);
                Some(LastValidRequest::new(seq, hash, Some(response), series))
            }
            FragmentType::NewNonRead(hash, objects) => {
                let response = self
                    .handle_non_read(database, request.header.function, seq, info.id, objects)
                    .await;
                Some(LastValidRequest::new(seq, hash, response, None))
            }
            FragmentType::RepeatNonRead(hash, last_response) => {
                // If we have a pending select, update the sequence number
                if let Some(select) = &mut self.state.select {
                    select.update_frame_id(info.id);
                }

                // per the spec, we just echo the last response
                Some(LastValidRequest::new(seq, hash, last_response, None))
            }
            FragmentType::Broadcast(mode) => {
                self.process_broadcast(info.id, database, mode, request)
                    .await;
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

    async fn write_error_response(
        &mut self,
        io: &mut PhysLayer,
        respond_to: FragmentAddr,
        writer: &mut TransportWriter,
        err: TransportRequestError,
        database: &DatabaseHandle,
    ) -> Result<(), RunError> {
        let seq = match err {
            TransportRequestError::HeaderParseError(err) => match err {
                HeaderParseError::UnknownFunction(seq, _) => Some(seq),
                HeaderParseError::InsufficientBytes => None,
            },
            TransportRequestError::RequestValidationError(seq, _) => Some(seq),
        };

        if let Some(seq) = seq {
            let iin = Iin::default() | Iin2::NO_FUNC_CODE_SUPPORT;
            self.write_solicited(
                io,
                writer,
                respond_to,
                Response::empty_solicited(seq, iin),
                database,
            )
            .await?;
        }
        Ok(())
    }

    fn format_first_read_response(
        &mut self,
        database: &mut DatabaseHandle,
        seq: Sequence,
        object_headers: HeaderCollection,
    ) -> (Response, Option<ResponseSeries>) {
        let iin2 = database.select(&object_headers);
        self.format_read_response(database, true, seq, iin2)
    }

    fn format_read_response(
        &mut self,
        database: &mut DatabaseHandle,
        fir: bool,
        seq: Sequence,
        iin2: Iin2,
    ) -> (Response, Option<ResponseSeries>) {
        let (len, info) = {
            let mut cursor = self.sol_tx_buffer.write_cursor();
            let _ = cursor.skip(ResponseHeader::LENGTH);
            let info = database.write_response_headers(&mut cursor);
            (cursor.written().len(), info)
        };

        let header = ResponseHeader::new(
            ControlField::response(seq, fir, info.complete, info.need_confirm()),
            ResponseFunction::Response,
            Iin::default() | iin2,
        );
        (Response::new(header, len), info.get_response_series(seq))
    }

    async fn handle_non_read(
        &mut self,
        database: &mut DatabaseHandle,
        function: FunctionCode,
        seq: Sequence,
        frame_id: u32,
        object_headers: HeaderCollection<'_>,
    ) -> Option<Response> {
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
                self.handle_controls(ControlType::Select, database, seq, frame_id, object_headers)
                    .await
            }
            FunctionCode::Operate => {
                self.handle_controls(
                    ControlType::Operate,
                    database,
                    seq,
                    frame_id,
                    object_headers,
                )
                .await
            }
            FunctionCode::DirectOperate => {
                self.handle_controls(
                    ControlType::DirectOperate,
                    database,
                    seq,
                    frame_id,
                    object_headers,
                )
                .await
            }
            FunctionCode::DirectOperateNoResponse => {
                self.handle_controls(
                    ControlType::DirectOperateNoAck,
                    database,
                    seq,
                    frame_id,
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
                Some(Response::empty_solicited(
                    seq,
                    Iin::default() | Iin2::NO_FUNC_CODE_SUPPORT,
                ))
            }
        };

        if let Some(response) = &mut result {
            response.header.iin |= Self::get_iin2(function, object_headers);
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
    ) -> Response {
        let mut iin2 = Iin2::default();
        let mut empty = true;

        for header in object_headers.iter() {
            empty = false;
            iin2 = self.handle_single_write_header(header, database).await;
        }

        if empty {
            tracing::warn!("empty WRITE request");
        }

        Response::empty_solicited(seq, Iin::default() | iin2)
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
            return Iin2::NO_FUNC_CODE_SUPPORT;
        }

        // next, ask the application to validate and persist the value
        if !self.application.write_device_attr(attr).get().await {
            // validation of the value failed
            return Iin2::PARAMETER_ERROR;
        }

        // this should never fail b/c we already validated the attribute is writable
        let _ = db.transaction(|db| db.inner.get_attr_map().write(attr));
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
            return Iin2::NO_FUNC_CODE_SUPPORT;
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
            HeaderDetails::OneByteCount(_, CountVariation::Group50Var1(seq)) => {
                self.handle_write_abs_time(seq)
            }
            HeaderDetails::OneByteCount(_, CountVariation::Group50Var3(seq)) => {
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
                Iin2::NO_FUNC_CODE_SUPPORT
            }
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
            Err(RequestError::NotSupported) => Iin2::NO_FUNC_CODE_SUPPORT,
            Err(RequestError::ParameterError) => Iin2::PARAMETER_ERROR,
            Ok(()) => Iin2::default(),
        }
    }

    fn handle_delay_measure(&mut self, seq: Sequence) -> Response {
        let g52v2 = Group52Var2 {
            time: self.application.get_processing_delay_ms(),
        };

        let mut cursor = self.sol_tx_buffer.write_cursor();
        let _ = cursor.skip(ResponseHeader::LENGTH);
        let mut writer = HeaderWriter::new(&mut cursor);
        writer.write_count_of_one(g52v2).unwrap();

        let header = ResponseHeader::new(
            ControlField::response(seq, true, true, false),
            ResponseFunction::Response,
            Iin::default(),
        );
        Response::new(header, cursor.written().len())
    }

    fn handle_record_current_time(&mut self, seq: Sequence) -> Response {
        self.state.last_recorded_time = Some(tokio::time::Instant::now());
        Response::empty_solicited(seq, Iin::default())
    }

    fn handle_restart(&mut self, seq: Sequence, delay: Option<RestartDelay>) -> Response {
        let delay = match delay {
            None => {
                return Response::empty_solicited(seq, Iin::default() | Iin2::NO_FUNC_CODE_SUPPORT);
            }
            Some(x) => x,
        };

        // respond with the delay
        let mut cursor = self.sol_tx_buffer.write_cursor();
        let _ = cursor.skip(ResponseHeader::LENGTH);
        let mut writer = HeaderWriter::new(&mut cursor);
        match delay {
            RestartDelay::Seconds(value) => {
                writer
                    .write_count_of_one(Group52Var1 { time: value })
                    .unwrap();
            }
            RestartDelay::Milliseconds(value) => {
                writer
                    .write_count_of_one(Group52Var2 { time: value })
                    .unwrap();
            }
        }

        let header = ResponseHeader::new(
            ControlField::response(seq, true, true, false),
            ResponseFunction::Response,
            Iin::default(),
        );

        Response::new(header, cursor.written().len())
    }

    async fn handle_direct_operate(
        &mut self,
        database: &mut DatabaseHandle,
        seq: Sequence,
        controls: ControlCollection<'_>,
    ) -> Response {
        // Handle each operate and write the response
        let (result, len) = {
            let mut cursor = self.sol_tx_buffer.write_cursor();
            let _ = cursor.skip(ResponseHeader::LENGTH);

            let max_controls_per_request = self.config.max_controls_per_request;
            let result = ControlTransaction::execute(
                self.control_handler.borrow_mut(),
                database,
                |tx, db| {
                    controls.operate_with_response(
                        &mut cursor,
                        OperateType::DirectOperate,
                        tx,
                        db,
                        max_controls_per_request,
                    )
                },
            )
            .await;

            (result, cursor.written().len())
        };

        // Calculate IIN and return it
        let mut iin = Iin::default();

        if let Ok(CommandStatus::NotSupported) = result {
            iin |= Iin2::PARAMETER_ERROR;
        }

        let header = ResponseHeader::new(
            ControlField::single_response(seq),
            ResponseFunction::Response,
            iin,
        );
        Response::new(header, len)
    }

    fn handle_enable_or_disable_unsolicited(
        &mut self,
        enable: bool,
        seq: Sequence,
        object_headers: HeaderCollection,
    ) -> Response {
        fn to_string(enable: bool) -> &'static str {
            if enable {
                "ENABLE"
            } else {
                "DISABLE"
            }
        }

        if self.config.unsolicited.is_disabled() {
            tracing::warn!("received {} unsolicited request, but unsolicited support is disabled by configuration", to_string(enable));
            return Response::empty_solicited(seq, Iin::default() | Iin2::NO_FUNC_CODE_SUPPORT);
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
                    iin2 |= Iin2::NO_FUNC_CODE_SUPPORT;
                }
            }
        }

        Response::empty_solicited(seq, Iin::default() | iin2)
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
        frame_id: u32,
        object_headers: HeaderCollection<'_>,
    ) -> Option<Response> {
        let controls = match ControlCollection::from(object_headers) {
            Err(err) => {
                tracing::warn!(
                    "ignoring select request containing non-control object header {} - {}",
                    err.variation,
                    err.qualifier
                );

                let err = Some(Response::empty_solicited(
                    seq,
                    Iin::default() | Iin2::PARAMETER_ERROR,
                ));

                return match ct {
                    ControlType::Select => err,
                    ControlType::Operate => err,
                    ControlType::DirectOperate => err,
                    ControlType::DirectOperateNoAck => None,
                };
            }
            Ok(controls) => controls,
        };

        match ct {
            ControlType::Select => {
                Some(self.handle_select(database, seq, frame_id, controls).await)
            }
            ControlType::Operate => {
                Some(self.handle_operate(database, seq, frame_id, controls).await)
            }
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
        frame_id: u32,
        controls: ControlCollection<'_>,
    ) -> Response {
        // Handle each select and write the response
        let (result, len) = {
            let mut cursor = self.sol_tx_buffer.write_cursor();
            let _ = cursor.skip(ResponseHeader::LENGTH);

            let max_controls_per_request = self.config.max_controls_per_request;
            let result: Result<CommandStatus, scursor::WriteError> = ControlTransaction::execute(
                self.control_handler.borrow_mut(),
                database,
                |tx, db| {
                    controls.select_with_response(&mut cursor, tx, db, max_controls_per_request)
                },
            )
            .await;

            (result, cursor.written().len())
        };

        // Record the select state
        if let Ok(CommandStatus::Success) = result {
            self.state.select = Some(SelectState::new(
                seq,
                frame_id,
                tokio::time::Instant::now(),
                controls.hash(),
            ))
        }

        // Calculate IIN and return response
        let mut iin = Iin::default();

        if let Ok(CommandStatus::NotSupported) = result {
            iin |= Iin2::PARAMETER_ERROR;
        }

        let header = ResponseHeader::new(
            ControlField::single_response(seq),
            ResponseFunction::Response,
            iin,
        );
        Response::new(header, len)
    }

    async fn handle_operate(
        &mut self,
        database: &mut DatabaseHandle,
        seq: Sequence,
        frame_id: u32,
        controls: ControlCollection<'_>,
    ) -> Response {
        // Handle each operate and write the response
        let (status, len) = {
            let mut cursor = self.sol_tx_buffer.write_cursor();
            let _ = cursor.skip(ResponseHeader::LENGTH);

            // determine if we have a matching SELECT
            let status = match self.state.select {
                Some(s) => {
                    match s.match_operate(
                        self.config.select_timeout,
                        seq,
                        frame_id,
                        controls.hash(),
                    ) {
                        Err(status) => {
                            controls.respond_with_status(&mut cursor, status).unwrap();
                            status
                        }
                        Ok(()) => {
                            let max_controls_per_request = self.config.max_controls_per_request;
                            ControlTransaction::execute(
                                self.control_handler.borrow_mut(),
                                database,
                                |tx, db| {
                                    controls
                                        .operate_with_response(
                                            &mut cursor,
                                            OperateType::SelectBeforeOperate,
                                            tx,
                                            db,
                                            max_controls_per_request,
                                        )
                                        .unwrap()
                                },
                            )
                            .await
                        }
                    }
                }
                None => {
                    let status = CommandStatus::NoSelect;
                    controls.respond_with_status(&mut cursor, status).unwrap();
                    status
                }
            };

            (status, cursor.written().len())
        };

        // Calculate IIN and return it
        let mut iin = Iin::default();

        if status == CommandStatus::NotSupported {
            iin |= Iin2::PARAMETER_ERROR;
        }

        let header = ResponseHeader::new(
            ControlField::single_response(seq),
            ResponseFunction::Response,
            iin,
        );
        Response::new(header, len)
    }

    fn handle_freeze(
        &mut self,
        database: &mut DatabaseHandle,
        seq: Sequence,
        object_headers: HeaderCollection,
        freeze_type: FreezeType,
    ) -> Response {
        let mut iin = Iin::default();

        for header in object_headers.iter() {
            iin |= self.handle_freeze_header(database, freeze_type, header.details);
        }

        Response::empty_solicited(seq, iin)
    }

    fn handle_freeze_at_time(
        &mut self,
        database: &mut DatabaseHandle,
        seq: Sequence,
        object_headers: HeaderCollection,
    ) -> Response {
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
                _ => {
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
                                header.details,
                            );
                        }
                    }
                }
            }
        }

        Response::empty_solicited(seq, iin)
    }

    fn handle_freeze_header(
        &mut self,
        database: &mut DatabaseHandle,
        freeze_type: FreezeType,
        details: HeaderDetails,
    ) -> Iin2 {
        match details {
            HeaderDetails::AllObjects(AllObjectsVariation::Group20Var0) => self
                .application
                .freeze_counter(FreezeIndices::All, freeze_type, database)
                .map_or_else(|err| err.into(), |_| Iin2::default()),
            HeaderDetails::OneByteStartStop(start, stop, RangedVariation::Group20Var0) => self
                .application
                .freeze_counter(
                    FreezeIndices::Range(start as u16, stop as u16),
                    freeze_type,
                    database,
                )
                .map_or_else(|err| err.into(), |_| Iin2::default()),
            HeaderDetails::TwoByteStartStop(start, stop, RangedVariation::Group20Var0) => self
                .application
                .freeze_counter(FreezeIndices::Range(start, stop), freeze_type, database)
                .map_or_else(|err| err.into(), |_| Iin2::default()),
            _ => Iin2::NO_FUNC_CODE_SUPPORT,
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
        frame_id: u32,
        database: &mut DatabaseHandle,
        mode: BroadcastConfirmMode,
        request: Request<'_>,
    ) {
        self.state.last_broadcast_type = Some(mode);
        let action = self
            .process_broadcast_get_action(frame_id, database, request)
            .await;
        self.info
            .broadcast_received(request.header.function, action)
    }

    async fn process_broadcast_get_action(
        &mut self,
        frame_id: u32,
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
                    frame_id,
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
        tokio::time::Instant::now() + self.config.unsolicited_retry_delay
    }

    async fn sol_confirm_wait(
        &mut self,
        io: &mut PhysLayer,
        reader: &mut TransportReader,
        writer: &mut TransportWriter,
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

                    database
                        .clear_written_events(self.application.as_mut())
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
                    database.reset();
                    return Ok(());
                }
                Confirm::NewRequest => {
                    tracing::info!("aborting solicited response due to new request");
                    database.reset();
                    return Ok(());
                }
            }
        }
    }

    async fn wait_for_sol_confirm(
        &mut self,
        io: &mut PhysLayer,
        reader: &mut TransportReader,
        writer: &mut TransportWriter,
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
                        ConfirmAction::EchoLastResponse(respond_to, response) => {
                            if let Some(response) = response {
                                self.repeat_solicited(io, respond_to, writer, response)
                                    .await?;
                            }
                            // per the spec, we restart the confirm timer
                            deadline = self.new_confirm_deadline();
                        }
                    }
                }
            }
        }
    }

    fn expect_sol_confirm(&mut self, ecsn: Sequence, request: &mut RequestGuard) -> ConfirmAction {
        let (info, request) = match request.get() {
            Some(TransportRequest::Request(info, request)) => {
                self.on_link_activity();
                (info, request)
            }
            Some(TransportRequest::LinkLayerMessage) => {
                self.on_link_activity();
                return ConfirmAction::ContinueWait;
            }
            Some(TransportRequest::Error(_, _)) => {
                self.on_link_activity();
                return ConfirmAction::NewRequest;
            }
            None => return ConfirmAction::ContinueWait,
        };

        match self.classify(info, request) {
            FragmentType::MalformedRequest(_, _) => ConfirmAction::NewRequest,
            FragmentType::NewRead(_, _) => ConfirmAction::NewRequest,
            FragmentType::RepeatRead(_, response, _) => {
                ConfirmAction::EchoLastResponse(info.addr, response)
            }
            FragmentType::NewNonRead(_, _) => ConfirmAction::NewRequest,
            // this should never happen, but if it does, new request is probably best course of action
            FragmentType::RepeatNonRead(_, _) => ConfirmAction::NewRequest,
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
                    FragmentType::RepeatRead(this_hash, last.response, object_headers)
                } else {
                    FragmentType::RepeatNonRead(this_hash, last.response)
                };
            }
        }

        if request.header.function == FunctionCode::Read {
            FragmentType::NewRead(this_hash, object_headers)
        } else {
            FragmentType::NewNonRead(this_hash, object_headers)
        }
    }

    fn on_link_activity(&mut self) {
        self.next_link_status = self
            .config
            .keep_alive_timeout
            .map(|timeout| tokio::time::Instant::now() + timeout);
    }
}

impl From<ObjectParseError> for Iin2 {
    fn from(err: ObjectParseError) -> Self {
        match err {
            ObjectParseError::InsufficientBytes => Iin2::PARAMETER_ERROR,
            ObjectParseError::InvalidQualifierForVariation(_, _) => Iin2::NO_FUNC_CODE_SUPPORT,
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
        x.value as f64
    }
}

impl From<Group34Var2> for f64 {
    fn from(x: Group34Var2) -> Self {
        x.value as f64
    }
}

impl From<Group34Var3> for f64 {
    fn from(x: Group34Var3) -> Self {
        x.value as f64
    }
}
