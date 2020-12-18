use crate::app::enums::{CommandStatus, FunctionCode};
use crate::app::format::write::start_response;
use crate::app::gen::ranged::RangedVariation;
use crate::app::header::{
    Control, RequestHeader, ResponseFunction, ResponseHeader, IIN, IIN1, IIN2,
};
use crate::app::parse::error::ObjectParseError;
use crate::app::parse::parser::{HeaderCollection, HeaderDetails, Request};
use crate::app::parse::DecodeLogLevel;
use crate::app::sequence::Sequence;
use crate::app::variations::{Group52Var1, Group52Var2};
use crate::entry::EndpointAddress;
use crate::link::error::LinkError;
use crate::link::header::BroadcastConfirmMode;
use crate::outstation::config::{BufferSize, Feature};
use crate::outstation::control::collection::{ControlCollection, ControlTransaction};
use crate::outstation::database::{DatabaseHandle, ResponseInfo};
use crate::outstation::traits::{
    BroadcastAction, ControlHandler, OperateType, OutstationApplication, OutstationInformation,
    RestartDelay,
};
use crate::transport::{
    FragmentInfo, RequestGuard, TransportReader, TransportRequest, TransportWriter,
};
use crate::util::buffer::Buffer;
use crate::util::cursor::WriteError;

use crate::outstation::config::OutstationConfig;
use crate::util::io::IOStream;
use std::borrow::BorrowMut;
use xxhash_rust::xxh64::xxh64;

use crate::app::gen::all::AllObjectsVariation;
use crate::master::request::EventClasses;
use crate::outstation::control::select::SelectState;
use crate::outstation::deferred::DeferredRead;
use crate::outstation::task::OutstationMessage;
use crate::util::task::{Receiver, RunError, Shutdown};

#[derive(Copy, Clone)]
enum Timeout {
    Yes,
    No,
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
    response_length: Option<usize>,
    series: Option<ResponseSeries>,
}

impl LastValidRequest {
    fn new(
        seq: Sequence,
        request_hash: u64,
        response_length: Option<usize>,
        series: Option<ResponseSeries>,
    ) -> Self {
        LastValidRequest {
            seq,
            request_hash,
            response_length,
            series,
        }
    }
}

pub(crate) struct SessionConfig {
    level: DecodeLogLevel,
    master_address: EndpointAddress,
    confirm_timeout: std::time::Duration,
    select_timeout: std::time::Duration,
    broadcast: Feature,
    unsolicited: Feature,
    max_unsolicited_retries: Option<usize>,
    unsolicited_retry_delay: std::time::Duration,
    keep_alive_timeout: Option<std::time::Duration>,
}

impl From<OutstationConfig> for SessionConfig {
    fn from(config: OutstationConfig) -> Self {
        SessionConfig {
            level: config.log_level,
            master_address: config.master_address,
            confirm_timeout: config.confirm_timeout,
            select_timeout: config.select_timeout,
            broadcast: config.features.broadcast,
            unsolicited: config.features.unsolicited,
            max_unsolicited_retries: config.max_unsolicited_retries,
            unsolicited_retry_delay: config.unsolicited_retry_delay,
            keep_alive_timeout: config.keep_alive_timeout,
        }
    }
}

#[derive(Copy, Clone)]
enum UnsolicitedState {
    /// need to perform NULL unsolicited, possibly waiting for a retry deadline
    NullRequired(Option<crate::tokio::time::Instant>),
    Ready(Option<crate::tokio::time::Instant>),
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
}

impl SessionState {
    fn new(max_read_headers: u16) -> Self {
        Self {
            enabled_unsolicited_classes: EventClasses::none(),
            restart_iin_asserted: true,
            last_valid_request: None,
            select: None,
            unsolicited: UnsolicitedState::NullRequired(None),
            unsolicited_seq: Sequence::default(),
            deferred_read: DeferredRead::new(max_read_headers),
        }
    }
}

pub(crate) struct OutstationSession {
    receiver: Receiver<OutstationMessage>,
    sol_tx_buffer: Buffer,
    unsol_tx_buffer: Buffer,
    config: SessionConfig,
    state: SessionState,
    application: Box<dyn OutstationApplication>,
    info: Box<dyn OutstationInformation>,
    control_handler: Box<dyn ControlHandler>,
    next_link_status: Option<crate::tokio::time::Instant>,
}

enum Confirm {
    Yes,
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
    RepeatRead(u64, Option<usize>, HeaderCollection<'a>),
    NewNonRead(u64, HeaderCollection<'a>),
    RepeatNonRead(u64, Option<usize>),
    Broadcast(BroadcastConfirmMode),
    SolicitedConfirm(Sequence),
    UnsolicitedConfirm(Sequence),
}

#[derive(Copy, Clone)]
enum ConfirmAction {
    Confirmed,
    NewRequest(RequestHeader),
    EchoLastResponse(Option<usize>),
    ContinueWait,
}

impl OutstationSession {
    // TODO
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        receiver: Receiver<OutstationMessage>,
        config: SessionConfig,
        max_read_headers_per_request: u16,
        sol_tx_buffer_size: BufferSize,
        unsol_tx_buffer_size: BufferSize,
        application: Box<dyn OutstationApplication>,
        information: Box<dyn OutstationInformation>,
        control_handler: Box<dyn ControlHandler>,
    ) -> Self {
        let next_link_status = config
            .keep_alive_timeout
            .map(|delay| crate::tokio::time::Instant::now() + delay);

        Self {
            receiver,
            config,
            sol_tx_buffer: sol_tx_buffer_size.create_buffer(),
            unsol_tx_buffer: unsol_tx_buffer_size.create_buffer(),
            state: SessionState::new(max_read_headers_per_request),
            application,
            info: information,
            control_handler,
            next_link_status,
        }
    }

    pub(crate) async fn run<T>(
        &mut self,
        io: &mut T,
        reader: &mut TransportReader,
        writer: &mut TransportWriter,
        database: &mut DatabaseHandle,
    ) -> Result<(), RunError>
    where
        T: IOStream,
    {
        loop {
            self.run_idle_state(io, reader, writer, database).await?;
        }
    }

    async fn write_unsolicited<T>(
        &self,
        io: &mut T,
        writer: &mut TransportWriter,
        length: usize,
    ) -> Result<(), LinkError>
    where
        T: IOStream,
    {
        writer
            .write(
                io,
                self.config.level,
                self.config.master_address.wrap(),
                self.unsol_tx_buffer.get(length).unwrap(),
            )
            .await
    }

    async fn write_solicited<T>(
        &self,
        io: &mut T,
        writer: &mut TransportWriter,
        length: usize,
    ) -> Result<(), LinkError>
    where
        T: IOStream,
    {
        writer
            .write(
                io,
                self.config.level,
                self.config.master_address.wrap(),
                self.sol_tx_buffer.get(length).unwrap(),
            )
            .await
    }

    async fn run_idle_state<T>(
        &mut self,
        io: &mut T,
        reader: &mut TransportReader,
        writer: &mut TransportWriter,
        database: &mut DatabaseHandle,
    ) -> Result<(), RunError>
    where
        T: IOStream,
    {
        // handle a deferred read request if it exists
        self.handle_deferred_read(io, reader, writer, database)
            .await?;

        // handle a request fragment if present
        self.handle_one_request_from_idle(io, reader, writer, database)
            .await?;

        // check to see if we should perform unsolicited
        let deadline = self.check_unsolicited(io, reader, writer, database).await?;

        // check to see if we should perform a link status check
        self.check_link_status(io, writer).await?;

        let deadline = match deadline {
            Some(deadline) => match self.next_link_status {
                Some(link_deadline) => {
                    Some(crate::tokio::time::Instant::min(deadline, link_deadline))
                }
                None => Some(deadline),
            },
            None => self.next_link_status,
        };

        // wait for an event
        crate::tokio::select! {
            frame_read = reader.read(io) => {
                // make sure an I/O error didn't occur, ending the session
                frame_read?;
            }
            _ = database.wait_for_change() => {
                // wake for unsolicited here
            }
            _ = self.sleep_until(deadline) => {
                // just wake up
            }
        }

        Ok(())
    }

    async fn check_unsolicited<T>(
        &mut self,
        io: &mut T,
        reader: &mut TransportReader,
        writer: &mut TransportWriter,
        database: &mut DatabaseHandle,
    ) -> Result<Option<crate::tokio::time::Instant>, RunError>
    where
        T: IOStream,
    {
        if self.config.unsolicited.is_disabled() {
            return Ok(None);
        }

        match self.state.unsolicited {
            UnsolicitedState::NullRequired(deadline) => {
                if let Some(deadline) = deadline {
                    if crate::tokio::time::Instant::now() < deadline {
                        return Ok(Some(deadline)); // not ready yet
                    }
                }

                // perform NULL unsolicited
                match self
                    .perform_null_unsolicited(io, reader, writer, database)
                    .await?
                {
                    UnsolicitedResult::Timeout | UnsolicitedResult::ReturnToIdle => {
                        let retry_at = self.new_unsolicited_retry_deadline();
                        self.state.unsolicited = UnsolicitedState::NullRequired(Some(retry_at));
                        Ok(Some(retry_at))
                    }
                    UnsolicitedResult::Confirmed => {
                        self.state.unsolicited = UnsolicitedState::Ready(None);
                        Ok(None)
                    }
                }
            }
            UnsolicitedState::Ready(deadline) => {
                if let Some(deadline) = deadline {
                    if crate::tokio::time::Instant::now() < deadline {
                        return Ok(Some(deadline)); // not ready yet
                    }
                }

                // perform regular unsolicited
                match self
                    .maybe_perform_unsolicited(io, reader, writer, database)
                    .await?
                {
                    None => {
                        // there was nothing to send
                        Ok(None)
                    }
                    Some(UnsolicitedResult::Timeout) | Some(UnsolicitedResult::ReturnToIdle) => {
                        let retry_at = self.new_unsolicited_retry_deadline();
                        self.state.unsolicited = UnsolicitedState::Ready(Some(retry_at));
                        Ok(Some(retry_at))
                    }
                    Some(UnsolicitedResult::Confirmed) => {
                        database.clear_written_events();
                        self.state.unsolicited = UnsolicitedState::Ready(None);
                        Ok(None)
                    }
                }
            }
        }
    }

    async fn check_link_status<T>(
        &mut self,
        io: &mut T,
        writer: &mut TransportWriter,
    ) -> Result<(), RunError>
    where
        T: IOStream,
    {
        if let Some(next) = self.next_link_status {
            // Wait until we need to send the link status
            if next > crate::tokio::time::Instant::now() {
                return Ok(());
            }

            writer
                .write_link_status_request(io, self.config.master_address.wrap())
                .await?;

            self.on_link_activity();
        }

        Ok(())
    }

    async fn perform_null_unsolicited<T>(
        &mut self,
        io: &mut T,
        reader: &mut TransportReader,
        writer: &mut TransportWriter,
        database: &mut DatabaseHandle,
    ) -> Result<UnsolicitedResult, RunError>
    where
        T: IOStream,
    {
        let seq = self.state.unsolicited_seq.increment();
        let length = self.write_null_unsolicited_response(seq);
        self.perform_unsolicited_response_series(database, seq, length, io, reader, writer)
            .await
    }

    async fn maybe_perform_unsolicited<T>(
        &mut self,
        io: &mut T,
        reader: &mut TransportReader,
        writer: &mut TransportWriter,
        database: &mut DatabaseHandle,
    ) -> Result<Option<UnsolicitedResult>, RunError>
    where
        T: IOStream,
    {
        if !self.state.enabled_unsolicited_classes.any() {
            return Ok(None);
        }

        match self.write_unsolicited_data(database) {
            None => Ok(None),
            Some((seq, length)) => {
                let result = self
                    .perform_unsolicited_response_series(database, seq, length, io, reader, writer)
                    .await?;
                Ok(Some(result))
            }
        }
    }

    fn write_unsolicited_data(
        &mut self,
        database: &mut DatabaseHandle,
    ) -> Option<(Sequence, usize)> {
        let iin = self.get_response_iin() + IIN2::default();
        let mut cursor = self.unsol_tx_buffer.write_cursor();
        let _ = cursor.skip(ResponseHeader::LENGTH);

        let count = database.write_unsolicited(self.state.enabled_unsolicited_classes, &mut cursor);

        if count == 0 {
            return None;
        }

        let seq = self.state.unsolicited_seq.increment();

        let header = ResponseHeader::new(
            Control::unsolicited_response(seq),
            ResponseFunction::UnsolicitedResponse,
            iin,
        );
        cursor.at_start(|cur| header.write(cur)).unwrap();
        Some((seq, cursor.position()))
    }

    async fn perform_unsolicited_response_series<T>(
        &mut self,
        database: &mut DatabaseHandle,
        uns_ecsn: Sequence,
        length: usize,
        io: &mut T,
        reader: &mut TransportReader,
        writer: &mut TransportWriter,
    ) -> Result<UnsolicitedResult, RunError>
    where
        T: IOStream,
    {
        self.write_unsolicited(io, writer, length).await?;

        // enter unsolicited confirm wait state
        self.info.enter_unsolicited_confirm_wait(uns_ecsn);

        let mut retry_count = RetryCounter::new(self.config.max_unsolicited_retries);

        let mut deadline = self.new_confirm_deadline();

        loop {
            match self
                .wait_for_unsolicited_confirm(uns_ecsn, deadline, io, reader, writer, database)
                .await?
            {
                UnsolicitedWaitResult::ReadNext => {
                    // just go to next iteration without changing the deadline
                }
                UnsolicitedWaitResult::Complete(result) => return Ok(result),
                UnsolicitedWaitResult::Timeout => {
                    let retry = retry_count.decrement();
                    self.info.unsolicited_confirm_timeout(uns_ecsn, retry);

                    if !retry {
                        return Ok(UnsolicitedResult::Timeout);
                    }

                    // perform a retry
                    self.write_unsolicited(io, writer, length).await?;

                    // update the deadline
                    deadline = self.new_confirm_deadline();
                }
            }
        }
    }

    async fn wait_for_unsolicited_confirm<T>(
        &mut self,
        uns_ecsn: Sequence,
        deadline: crate::tokio::time::Instant,
        io: &mut T,
        reader: &mut TransportReader,
        writer: &mut TransportWriter,
        database: &mut DatabaseHandle,
    ) -> Result<UnsolicitedWaitResult, RunError>
    where
        T: IOStream,
    {
        if let Timeout::Yes = self.read_until(io, reader, deadline).await? {
            return Ok(UnsolicitedWaitResult::Timeout);
        }

        let mut guard = reader.pop_request(self.config.level);
        let (info, request) = match guard.get() {
            None => return Ok(UnsolicitedWaitResult::ReadNext),
            Some(TransportRequest::Request(info, request)) => {
                self.on_link_activity();
                (info, request)
            }
            Some(TransportRequest::LinkLayerMessage(_)) => {
                self.on_link_activity();
                return Ok(UnsolicitedWaitResult::ReadNext);
            }
        };

        match self.classify(info, request) {
            FragmentType::UnsolicitedConfirm(seq) => {
                if seq == uns_ecsn {
                    self.info.unsolicited_confirmed(seq);
                    Ok(UnsolicitedWaitResult::Complete(
                        UnsolicitedResult::Confirmed,
                    ))
                } else {
                    log::warn!("ignoring unsolicited confirm with wrong sequence number ({}) while expecting ({})", seq.value(), uns_ecsn.value());
                    Ok(UnsolicitedWaitResult::ReadNext)
                }
            }
            FragmentType::SolicitedConfirm(_) => {
                log::warn!("ignoring solicited confirm while waiting for unsolicited confirm");
                Ok(UnsolicitedWaitResult::ReadNext)
            }
            FragmentType::Broadcast(mode) => {
                self.state.deferred_read.clear();
                self.process_broadcast(database, mode, request);
                Ok(UnsolicitedWaitResult::ReadNext)
            }
            FragmentType::MalformedRequest(_, err) => {
                self.state.deferred_read.clear();
                let length =
                    self.write_empty_solicited_response(request.header.control.seq, err.into());
                self.write_solicited(io, writer, length).await?;
                Ok(UnsolicitedWaitResult::ReadNext)
            }
            FragmentType::NewNonRead(_hash, objects) => {
                self.state.deferred_read.clear();
                if let Some(length) = self.handle_non_read(
                    database,
                    request.header.function,
                    request.header.control.seq,
                    info.id,
                    objects,
                ) {
                    self.write_solicited(io, writer, length).await?;
                }
                Ok(UnsolicitedWaitResult::ReadNext)
            }
            FragmentType::NewRead(hash, headers) => {
                self.state
                    .deferred_read
                    .set(hash, request.header.control.seq, info, headers);
                Ok(UnsolicitedWaitResult::ReadNext)
            }
            FragmentType::RepeatRead(hash, _, headers) => {
                self.state
                    .deferred_read
                    .set(hash, request.header.control.seq, info, headers);
                Ok(UnsolicitedWaitResult::ReadNext)
            }
            FragmentType::RepeatNonRead(_, _) => {
                // TODO - send the response?

                self.state.deferred_read.clear();
                Ok(UnsolicitedWaitResult::ReadNext)
            }
        }
    }

    async fn read_until<T>(
        &mut self,
        io: &mut T,
        reader: &mut TransportReader,
        deadline: crate::tokio::time::Instant,
    ) -> Result<Timeout, RunError>
    where
        T: IOStream,
    {
        loop {
            crate::tokio::select! {
                 res = self.sleep_until(Some(deadline)) => {
                     res?;
                     return Ok(Timeout::Yes);
                 }
                 res = reader.read(io) => {
                     res?;
                     return Ok(Timeout::No);
                 }
            }
        }
    }

    async fn sleep_until(
        &mut self,
        instant: Option<crate::tokio::time::Instant>,
    ) -> Result<(), Shutdown> {
        async fn sleep_only(instant: Option<crate::tokio::time::Instant>) {
            match instant {
                Some(x) => crate::tokio::time::delay_until(x).await,
                None => {
                    // sleep forever
                    crate::util::future::forever().await;
                }
            }
        }

        loop {
            crate::tokio::select! {
                 _ = sleep_only(instant) => {
                        return Ok(());
                 }
                 message = self.receiver.next() => {
                     self.handle_message(message?);
                 }
            }
        }
    }

    fn handle_message(&mut self, message: OutstationMessage) {
        match message {
            OutstationMessage::SetDecodeLogLevel(level) => {
                self.config.level = level;
            }
        }
    }

    async fn handle_deferred_read<T>(
        &mut self,
        io: &mut T,
        reader: &mut TransportReader,
        writer: &mut TransportWriter,
        database: &mut DatabaseHandle,
    ) -> Result<(), RunError>
    where
        T: IOStream,
    {
        if let Some(x) = self.state.deferred_read.select(database) {
            let (length, series) = self.write_read_response(database, true, x.seq, x.iin2);
            self.state.last_valid_request =
                Some(LastValidRequest::new(x.seq, x.hash, Some(length), series));
            self.write_solicited(io, writer, length).await?;
            if let Some(series) = series {
                // enter the solicited confirm wait state
                self.sol_confirm_wait(io, reader, writer, database, series)
                    .await?;
            }
        }

        Ok(())
    }

    async fn handle_one_request_from_idle<T>(
        &mut self,
        io: &mut T,
        reader: &mut TransportReader,
        writer: &mut TransportWriter,
        database: &mut DatabaseHandle,
    ) -> Result<(), RunError>
    where
        T: IOStream,
    {
        let mut guard = reader.pop_request(self.config.level);
        match guard.get() {
            Some(TransportRequest::Request(info, request)) => {
                self.on_link_activity();
                if let Some(result) = self.process_request_from_idle(info, request, database) {
                    self.state.last_valid_request = Some(result);

                    // optional response
                    if let Some(length) = result.response_length {
                        self.write_solicited(io, writer, length).await?;
                    }

                    // maybe start a response series
                    if let Some(series) = result.series {
                        drop(guard);
                        // enter the solicited confirm wait state
                        self.sol_confirm_wait(io, reader, writer, database, series)
                            .await?;
                    }
                }
            }
            Some(TransportRequest::LinkLayerMessage(_)) => {
                self.on_link_activity();
            }
            None => (),
        }

        Ok(())
    }

    fn process_request_from_idle(
        &mut self,
        info: FragmentInfo,
        request: Request,
        database: &mut DatabaseHandle,
    ) -> Option<LastValidRequest> {
        self.info.process_request_from_idle(request.header);

        let seq = request.header.control.seq;

        match self.classify(info, request) {
            FragmentType::MalformedRequest(hash, err) => {
                let length = self.write_empty_solicited_response(seq, err.into());
                Some(LastValidRequest::new(seq, hash, Some(length), None))
            }
            FragmentType::NewRead(hash, objects) => {
                let (length, series) = self.write_first_read_response(database, seq, objects);
                Some(LastValidRequest::new(seq, hash, Some(length), series))
            }
            FragmentType::RepeatRead(hash, _, objects) => {
                // this deviates a bit from the spec, the specification says to
                // also reply to duplicate READ requests from idle, but this
                // is plainly wrong since it can't possibly handle a multi-fragmented
                // response correctly. Answering a repeat READ with a fresh response is harmless
                let (length, series) = self.write_first_read_response(database, seq, objects);
                Some(LastValidRequest::new(seq, hash, Some(length), series))
            }
            FragmentType::NewNonRead(hash, objects) => {
                let length =
                    self.handle_non_read(database, request.header.function, seq, info.id, objects);
                Some(LastValidRequest::new(seq, hash, length, None))
            }
            FragmentType::RepeatNonRead(hash, last_response_length) => {
                // per the spec, we just echo the last response
                Some(LastValidRequest::new(seq, hash, last_response_length, None))
            }
            FragmentType::Broadcast(mode) => {
                self.process_broadcast(database, mode, request);
                None
            }
            FragmentType::SolicitedConfirm(seq) => {
                log::warn!(
                    "ignoring solicited CONFIRM from idle state with seq: {}",
                    seq.value()
                );
                None
            }
            FragmentType::UnsolicitedConfirm(seq) => {
                log::warn!(
                    "ignoring unsolicited CONFIRM from idle state with seq: {}",
                    seq.value()
                );
                None
            }
        }
    }

    fn write_empty_solicited_response(&mut self, seq: Sequence, iin2: IIN2) -> usize {
        let iin = IIN::new(self.get_response_iin(), iin2);
        let header = ResponseHeader::new(
            Control::response(seq, true, true, false),
            ResponseFunction::Response,
            iin,
        );
        let mut cursor = self.sol_tx_buffer.write_cursor();
        header.write(&mut cursor).unwrap();
        cursor.written().len()
    }

    fn write_null_unsolicited_response(&mut self, seq: Sequence) -> usize {
        let iin = IIN::new(self.get_response_iin(), IIN2::default());
        let header = ResponseHeader::new(
            Control::unsolicited_response(seq),
            ResponseFunction::UnsolicitedResponse,
            iin,
        );
        let mut cursor = self.unsol_tx_buffer.write_cursor();
        header.write(&mut cursor).unwrap();
        cursor.written().len()
    }

    fn write_first_read_response(
        &mut self,
        database: &mut DatabaseHandle,
        seq: Sequence,
        object_headers: HeaderCollection,
    ) -> (usize, Option<ResponseSeries>) {
        let iin2 = database.select(&object_headers);
        self.write_read_response(database, true, seq, iin2)
    }

    fn write_read_response(
        &mut self,
        database: &mut DatabaseHandle,
        fir: bool,
        seq: Sequence,
        iin2: IIN2,
    ) -> (usize, Option<ResponseSeries>) {
        let iin1 = self.get_response_iin();
        let mut cursor = self.sol_tx_buffer.write_cursor();
        cursor.skip(ResponseHeader::LENGTH).unwrap();
        let info = database.write_response_headers(&mut cursor);
        let header = ResponseHeader::new(
            Control::response(seq, fir, info.complete, info.need_confirm()),
            ResponseFunction::Response,
            iin1 + iin2,
        );
        cursor.at_start(|cur| header.write(cur)).unwrap();
        (cursor.written().len(), info.get_response_series(seq))
    }

    fn handle_non_read(
        &mut self,
        database: &mut DatabaseHandle,
        function: FunctionCode,
        seq: Sequence,
        frame_id: u32,
        object_headers: HeaderCollection,
    ) -> Option<usize> {
        let iin2 = Self::get_iin2(function, object_headers);

        match function {
            FunctionCode::Write => Some(self.handle_write(seq, object_headers)),
            // these function don't process objects
            FunctionCode::DelayMeasure => Some(self.handle_delay_measure(seq, iin2)),
            FunctionCode::ColdRestart => {
                let delay = self.application.cold_restart();
                Some(self.handle_restart(seq, delay, iin2))
            }
            FunctionCode::WarmRestart => {
                let delay = self.application.warm_restart();
                Some(self.handle_restart(seq, delay, iin2))
            }
            // controls
            FunctionCode::Select => {
                Some(self.handle_select(database, seq, frame_id, object_headers))
            }
            FunctionCode::Operate => {
                Some(self.handle_operate(database, seq, frame_id, object_headers))
            }
            FunctionCode::DirectOperate => {
                Some(self.handle_direct_operate(database, seq, object_headers))
            }
            FunctionCode::DirectOperateNoResponse => {
                self.handle_direct_operate_no_ack(database, object_headers);
                None
            }
            FunctionCode::EnableUnsolicited => {
                Some(self.handle_enable_or_disable_unsolicited(true, seq, object_headers))
            }
            FunctionCode::DisableUnsolicited => {
                Some(self.handle_enable_or_disable_unsolicited(false, seq, object_headers))
            }

            _ => {
                log::warn!("unsupported function code: {:?}", function);
                Some(self.write_empty_solicited_response(seq, IIN2::NO_FUNC_CODE_SUPPORT))
            }
        }
    }

    fn get_iin2(function: FunctionCode, object_headers: HeaderCollection) -> IIN2 {
        if function.get_function_info().objects_allowed {
            return IIN2::default();
        }

        if object_headers.is_empty() {
            IIN2::default()
        } else {
            log::warn!("Ignoring object headers in {:?} request", function);
            IIN2::PARAMETER_ERROR
        }
    }

    fn handle_write(&mut self, seq: Sequence, object_headers: HeaderCollection) -> usize {
        let mut iin2 = IIN2::default();

        for header in object_headers.iter() {
            match header.details {
                HeaderDetails::OneByteStartStop(_, _, RangedVariation::Group80Var1(seq)) => {
                    for (value, index) in seq.iter() {
                        if index == 7 {
                            // restart IIN
                            if value {
                                log::warn!("cannot write IIN 1.7 to TRUE");
                                iin2 |= IIN2::PARAMETER_ERROR;
                            } else {
                                // clear the restart bit
                                self.state.restart_iin_asserted = false;
                                self.info.clear_restart_iin();
                            }
                        } else {
                            log::warn!("ignoring write of IIN index {} to value {}", index, value);
                            iin2 |= IIN2::PARAMETER_ERROR;
                        }
                    }
                }
                _ => {
                    log::warn!(
                        "WRITE not supported with qualifier: {} and variation: {}",
                        header.details.qualifier(),
                        header.variation
                    );
                    iin2 |= IIN2::NO_FUNC_CODE_SUPPORT;
                }
            }
        }

        self.write_empty_solicited_response(seq, iin2)
    }

    fn handle_delay_measure(&mut self, seq: Sequence, iin2: IIN2) -> usize {
        let iin = self.get_response_iin() + iin2;

        let g52v2 = Group52Var2 {
            time: self.application.get_processing_delay_ms(),
        };

        let mut cursor = self.sol_tx_buffer.write_cursor();
        let mut writer = start_response(
            Control::response(seq, true, true, false),
            ResponseFunction::Response,
            iin,
            &mut cursor,
        )
        .unwrap();

        writer.write_count_of_one(g52v2).unwrap();
        cursor.written().len()
    }

    fn handle_restart(&mut self, seq: Sequence, delay: Option<RestartDelay>, iin2: IIN2) -> usize {
        let delay = match delay {
            None => {
                return self.write_empty_solicited_response(seq, iin2 | IIN2::NO_FUNC_CODE_SUPPORT)
            }
            Some(x) => x,
        };

        let iin = self.get_response_iin() + iin2;

        // respond with the delay
        let mut cursor = self.sol_tx_buffer.write_cursor();
        let mut writer = start_response(
            Control::response(seq, true, true, false),
            ResponseFunction::Response,
            iin,
            &mut cursor,
        )
        .unwrap();

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

        cursor.written().len()
    }

    fn handle_direct_operate(
        &mut self,
        database: &mut DatabaseHandle,
        seq: Sequence,
        object_headers: HeaderCollection,
    ) -> usize {
        let controls = match ControlCollection::from(object_headers) {
            Err(err) => {
                log::warn!(
                    "ignoring control request containing non-control object header {} - {}",
                    err.variation,
                    err.qualifier
                );
                return self.write_empty_solicited_response(seq, IIN2::PARAMETER_ERROR);
            }
            Ok(controls) => controls,
        };

        let iin = self.get_response_iin() + IIN2::default();
        let mut cursor = self.sol_tx_buffer.write_cursor();
        ResponseHeader::new(
            Control::single_response(seq),
            ResponseFunction::Response,
            iin,
        )
        .write(&mut cursor)
        .unwrap();

        let mut control_tx = ControlTransaction::new(self.control_handler.borrow_mut());

        let _ = database.transaction(|database| {
            controls.operate_with_response(
                &mut cursor,
                OperateType::DirectOperate,
                &mut control_tx,
                database,
            )
        });

        cursor.written().len()
    }

    fn handle_enable_or_disable_unsolicited(
        &mut self,
        enable: bool,
        seq: Sequence,
        object_headers: HeaderCollection,
    ) -> usize {
        fn to_string(enable: bool) -> &'static str {
            if enable {
                "ENABLE"
            } else {
                "DISABLE"
            }
        }

        if self.config.unsolicited.is_disabled() {
            log::warn!("received {} unsolicited request, but unsolicited support is disabled by configuration", to_string(enable));
            return self.write_empty_solicited_response(seq, IIN2::NO_FUNC_CODE_SUPPORT);
        }

        let mut iin2 = IIN2::default();

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
                    log::warn!("received {} unsolicited request for unsupported qualifier ({}) and variation ({})", to_string(enable), header.details.qualifier(), header.variation);
                    iin2 |= IIN2::NO_FUNC_CODE_SUPPORT;
                }
            }
        }

        self.write_empty_solicited_response(seq, iin2)
    }

    fn handle_direct_operate_no_ack(
        &mut self,
        database: &mut DatabaseHandle,
        object_headers: HeaderCollection,
    ) {
        let controls = match ControlCollection::from(object_headers) {
            Err(err) => {
                log::warn!(
                    "ignoring control request containing non-control object header {} - {}",
                    err.variation,
                    err.qualifier
                );
                return;
            }
            Ok(controls) => controls,
        };

        let mut control_tx = ControlTransaction::new(self.control_handler.borrow_mut());

        let _ = database.transaction(|database| controls.operate_no_ack(&mut control_tx, database));
    }

    fn handle_select(
        &mut self,
        database: &mut DatabaseHandle,
        seq: Sequence,
        frame_id: u32,
        object_headers: HeaderCollection,
    ) -> usize {
        let controls = match ControlCollection::from(object_headers) {
            Err(err) => {
                log::warn!(
                    "ignoring select request containing non-control object header {} - {}",
                    err.variation,
                    err.qualifier
                );
                return self.write_empty_solicited_response(seq, IIN2::PARAMETER_ERROR);
            }
            Ok(controls) => controls,
        };

        let iin = self.get_response_iin() + IIN2::default();
        let mut cursor = self.sol_tx_buffer.write_cursor();
        ResponseHeader::new(
            Control::single_response(seq),
            ResponseFunction::Response,
            iin,
        )
        .write(&mut cursor)
        .unwrap();

        let mut transaction = ControlTransaction::new(self.control_handler.borrow_mut());

        let result: Result<CommandStatus, WriteError> = database.transaction(|database| {
            controls.select_with_response(&mut cursor, &mut transaction, database)
        });

        if let Ok(CommandStatus::Success) = result {
            self.state.select = Some(SelectState::new(
                seq,
                frame_id,
                crate::tokio::time::Instant::now(),
                object_headers.hash(),
            ))
        }

        cursor.written().len()
    }

    fn handle_operate(
        &mut self,
        database: &mut DatabaseHandle,
        seq: Sequence,
        frame_id: u32,
        object_headers: HeaderCollection,
    ) -> usize {
        let controls = match ControlCollection::from(object_headers) {
            Err(err) => {
                log::warn!(
                    "ignoring OPERATE request containing non-control object header {} - {}",
                    err.variation,
                    err.qualifier
                );
                return self.write_empty_solicited_response(seq, IIN2::PARAMETER_ERROR);
            }
            Ok(controls) => controls,
        };

        let iin = self.get_response_iin() + IIN2::default();
        let mut cursor = self.sol_tx_buffer.write_cursor();
        ResponseHeader::new(
            Control::single_response(seq),
            ResponseFunction::Response,
            iin,
        )
        .write(&mut cursor)
        .unwrap();

        // determine if we have a matching SELECT
        match self.state.select {
            Some(s) => {
                match s.match_operate(
                    self.config.select_timeout,
                    seq,
                    frame_id,
                    object_headers.hash(),
                ) {
                    Err(status) => {
                        let _ = controls.respond_with_status(&mut cursor, status);
                    }
                    Ok(()) => {
                        let mut control_tx =
                            ControlTransaction::new(self.control_handler.borrow_mut());
                        let _ = database.transaction(|db| {
                            controls.operate_with_response(
                                &mut cursor,
                                OperateType::SelectBeforeOperate,
                                &mut control_tx,
                                db,
                            )
                        });
                    }
                }
            }
            None => {
                let _ = controls.respond_with_status(&mut cursor, CommandStatus::NoSelect);
            }
        }

        cursor.written().len()
    }

    fn get_response_iin(&self) -> IIN1 {
        let mut iin1 = IIN1::default();
        if self.state.restart_iin_asserted {
            iin1 |= IIN1::RESTART
        }
        iin1
    }

    fn process_broadcast(
        &mut self,
        database: &mut DatabaseHandle,
        _mode: BroadcastConfirmMode,
        request: Request,
    ) {
        let action = self.process_broadcast_get_action(database, request);
        self.info
            .broadcast_received(request.header.function, action)
    }

    fn process_broadcast_get_action(
        &mut self,
        database: &mut DatabaseHandle,
        request: Request,
    ) -> BroadcastAction {
        if self.config.broadcast.is_disabled() {
            log::warn!(
                "ignoring broadcast request (broadcast support disabled): {:?}",
                request.header.function
            );
            return BroadcastAction::IgnoredByConfiguration;
        }

        let objects = match request.objects {
            Ok(x) => x,
            Err(err) => {
                log::warn!(
                    "ignoring broadcast message with bad object headers: {}",
                    err
                );
                return BroadcastAction::BadObjectHeaders;
            }
        };

        match request.header.function {
            FunctionCode::DirectOperateNoResponse => {
                self.handle_direct_operate_no_ack(database, objects);
                BroadcastAction::Processed
            }
            _ => {
                log::warn!(
                    "unsupported broadcast function: {:?}",
                    request.header.function
                );
                BroadcastAction::UnsupportedFunction(request.header.function)
            }
        }
    }

    fn new_confirm_deadline(&self) -> crate::tokio::time::Instant {
        crate::tokio::time::Instant::now() + self.config.confirm_timeout
    }

    fn new_unsolicited_retry_deadline(&self) -> crate::tokio::time::Instant {
        crate::tokio::time::Instant::now() + self.config.unsolicited_retry_delay
    }

    async fn sol_confirm_wait<T>(
        &mut self,
        io: &mut T,
        reader: &mut TransportReader,
        writer: &mut TransportWriter,
        database: &mut DatabaseHandle,
        mut series: ResponseSeries,
    ) -> Result<(), RunError>
    where
        T: IOStream,
    {
        self.info.enter_solicited_confirm_wait(series.ecsn);

        loop {
            match self
                .wait_for_sol_confirm(io, reader, writer, series.ecsn)
                .await?
            {
                Confirm::Yes => {
                    database.clear_written_events();
                    if series.fin {
                        // done with response series
                        return Ok(());
                    }
                    // format the next response in the series
                    series.ecsn.increment();
                    let (length, next) =
                        self.write_read_response(database, false, series.ecsn, IIN2::default());
                    self.write_solicited(io, writer, length).await?;
                    match next {
                        None => return Ok(()),
                        Some(next) => {
                            series = next;
                        }
                    }
                }
                Confirm::Timeout => {
                    database.reset();
                    return Ok(());
                }
                Confirm::NewRequest => {
                    database.reset();
                    return Ok(());
                }
            }
        }
    }

    async fn wait_for_sol_confirm<T>(
        &mut self,
        io: &mut T,
        reader: &mut TransportReader,
        writer: &mut TransportWriter,
        ecsn: Sequence,
    ) -> Result<Confirm, RunError>
    where
        T: IOStream,
    {
        let mut deadline = self.new_confirm_deadline();
        loop {
            match self.read_until(io, reader, deadline).await? {
                Timeout::Yes => {
                    self.info.solicited_confirm_timeout(ecsn);
                    return Ok(Confirm::Timeout);
                }
                // process data
                Timeout::No => {
                    let mut guard = reader.pop_request(self.config.level);
                    match self.expect_sol_confirm(ecsn, &mut guard) {
                        ConfirmAction::ContinueWait => {
                            // we ignored whatever the request was and logged it elsewhere
                            // just go back to the loop and read another fragment
                        }
                        ConfirmAction::Confirmed => {
                            self.info.solicited_confirm_received(ecsn);
                            return Ok(Confirm::Yes);
                        }
                        ConfirmAction::NewRequest(header) => {
                            self.info.solicited_confirm_wait_new_request(header);
                            // retain the fragment so that it can be processed from the idle state
                            guard.retain();
                            return Ok(Confirm::NewRequest);
                        }
                        ConfirmAction::EchoLastResponse(length) => {
                            if let Some(length) = length {
                                self.write_solicited(io, writer, length).await?;
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
            Some(TransportRequest::LinkLayerMessage(_)) => {
                self.on_link_activity();
                return ConfirmAction::ContinueWait;
            }
            None => return ConfirmAction::ContinueWait,
        };

        match self.classify(info, request) {
            FragmentType::MalformedRequest(_, _) => ConfirmAction::NewRequest(request.header),
            FragmentType::NewRead(_, _) => ConfirmAction::NewRequest(request.header),
            FragmentType::RepeatRead(_, response_length, _) => {
                ConfirmAction::EchoLastResponse(response_length)
            }
            FragmentType::NewNonRead(_, _) => ConfirmAction::NewRequest(request.header),
            // this should never happen, but if it does, new request is probably best course of action
            FragmentType::RepeatNonRead(_, _) => ConfirmAction::NewRequest(request.header),
            FragmentType::Broadcast(_) => ConfirmAction::NewRequest(request.header),
            FragmentType::SolicitedConfirm(seq) => {
                if seq == ecsn {
                    ConfirmAction::Confirmed
                } else {
                    self.info
                        .wrong_solicited_confirm_seq(ecsn, request.header.control.seq);
                    log::warn!("received solicited confirm with wrong sequence number, expected: {} received: {}", ecsn.value(), seq.value());
                    ConfirmAction::ContinueWait
                }
            }
            FragmentType::UnsolicitedConfirm(seq) => {
                self.info.unexpected_confirm(true, seq);
                log::warn!(
                    "ignoring unsolicited CONFIRM while waiting for solicited confirm, seq: {}",
                    seq.value()
                );
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
                    FragmentType::RepeatRead(this_hash, last.response_length, object_headers)
                } else {
                    FragmentType::RepeatNonRead(this_hash, last.response_length)
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
        self.next_link_status = match self.config.keep_alive_timeout {
            Some(timeout) => Some(crate::tokio::time::Instant::now() + timeout),
            None => None,
        }
    }
}

impl From<ObjectParseError> for IIN2 {
    fn from(err: ObjectParseError) -> Self {
        // TODO - review these
        match err {
            ObjectParseError::InsufficientBytes => IIN2::PARAMETER_ERROR,
            ObjectParseError::InvalidQualifierForVariation(_, _) => IIN2::NO_FUNC_CODE_SUPPORT,
            ObjectParseError::InvalidRange(_, _) => IIN2::PARAMETER_ERROR,
            ObjectParseError::UnknownGroupVariation(_, _) => IIN2::OBJECT_UNKNOWN,
            ObjectParseError::UnsupportedQualifierCode(_) => IIN2::PARAMETER_ERROR,
            ObjectParseError::UnknownQualifier(_) => IIN2::PARAMETER_ERROR,
            ObjectParseError::ZeroLengthOctetData => IIN2::PARAMETER_ERROR,
        }
    }
}
