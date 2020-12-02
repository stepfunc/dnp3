use crate::app::enums::{CommandStatus, FunctionCode};
use crate::app::format::write::start_response;
use crate::app::gen::ranged::RangedVariation;
use crate::app::header::{Control, ResponseFunction, ResponseHeader, IIN, IIN1, IIN2};
use crate::app::parse::error::ObjectParseError;
use crate::app::parse::parser::{HeaderCollection, HeaderDetails, Request};
use crate::app::parse::DecodeLogLevel;
use crate::app::sequence::Sequence;
use crate::app::variations::{Group52Var1, Group52Var2};
use crate::entry::EndpointAddress;
use crate::link::error::LinkError;
use crate::link::header::BroadcastConfirmMode;
use crate::outstation::control::collection::ControlCollection;
use crate::outstation::database::{DatabaseConfig, DatabaseHandle, ResponseInfo};
use crate::outstation::traits::{ControlHandler, OperateType, OutstationApplication, RestartDelay};
use crate::outstation::SelfAddressSupport;
use crate::tokio::io::{AsyncRead, AsyncWrite};
use crate::transport::{FragmentInfo, Timeout, TransportReader, TransportWriter};
use crate::util::buffer::Buffer;
use crate::util::cursor::WriteError;

use std::borrow::BorrowMut;
use std::time::Duration;
use xxhash_rust::xxh64::xxh64;

#[derive(Copy, Clone, PartialEq)]
struct ResponseSeries {
    ecsn: Sequence,
    last_response: bool,
    master: EndpointAddress,
}

impl ResponseSeries {
    fn new(ecsn: Sequence, last_response: bool, master: EndpointAddress) -> Self {
        Self {
            ecsn,
            last_response,
            master,
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
enum NextState {
    Idle,
    SolConfirmWait(ResponseSeries),
}

impl ResponseInfo {
    fn to_next_state(&self, ecsn: Sequence, source: EndpointAddress) -> NextState {
        if self.need_confirm() {
            NextState::SolConfirmWait(ResponseSeries::new(ecsn, self.complete, source))
        } else {
            NextState::Idle
        }
    }
}

#[derive(Copy, Clone)]
struct LastValidRequest {
    seq: Sequence,
    request_hash: u64,
    response_length: usize,
    next: NextState,
}

impl LastValidRequest {
    fn new(seq: Sequence, request_hash: u64, response_length: usize, next: NextState) -> Self {
        LastValidRequest {
            seq,
            request_hash,
            response_length,
            next,
        }
    }
}

struct SessionConfig {
    level: DecodeLogLevel,
    master_address: EndpointAddress,
    confirm_timeout: Duration,
    select_timeout: Duration,
}

/// records when a select occurs
#[derive(Copy, Clone)]
struct SelectState {
    /// sequence number of the SELECT
    seq: Sequence,
    /// frame count of the select, makes it easier to ensure that OPERATE directly follows SELECT
    /// without requests in between
    frame_id: u32,
    /// time at which the SELECT occurred
    time: crate::tokio::time::Instant,
    /// the hash of the object headers
    object_hash: u64,
}

impl SelectState {
    fn new(
        seq: Sequence,
        frame_id: u32,
        time: crate::tokio::time::Instant,
        object_hash: u64,
    ) -> Self {
        Self {
            seq,
            frame_id,
            time,
            object_hash,
        }
    }

    fn match_operate(
        &self,
        timeout: Duration,
        seq: Sequence,
        frame_id: u32,
        object_hash: u64,
    ) -> Result<(), CommandStatus> {
        let elapsed = crate::tokio::time::Instant::now().checked_duration_since(self.time);

        // check the sequence number
        if self.seq.next() != seq.value() {
            log::warn!("received OPERATE with non-consecutive sequence number");
            return Err(CommandStatus::NoSelect);
        }

        // check the frame_id to ensure there was no requests in between the SELECT and OPERATE
        if self.frame_id.wrapping_add(1) != frame_id {
            log::warn!("received OPERATE without prior SELECT");
            return Err(CommandStatus::NoSelect);
        }

        // check the object hash
        if self.object_hash != object_hash {
            log::warn!("received OPERATE with different header than SELECT");
            return Err(CommandStatus::NoSelect);
        }

        // check the time last
        match elapsed {
            None => {
                log::error!("current time is less than time of SELECT, clock error?");
                return Err(CommandStatus::Timeout);
            }
            Some(elapsed) => {
                if elapsed > timeout {
                    log::warn!("received valid OPERATE after SELECT timeout");
                    return Err(CommandStatus::Timeout);
                }
            }
        }

        Ok(())
    }
}

/// state that mutates while the session runs
struct SessionState {
    restart_iin_asserted: bool,
    last_valid_request: Option<LastValidRequest>,
    select_state: Option<SelectState>,
}

impl SessionState {
    fn new() -> Self {
        Self {
            restart_iin_asserted: true,
            last_valid_request: None,
            select_state: None,
        }
    }
}

pub(crate) struct OutstationSession {
    tx_buffer: Buffer,
    config: SessionConfig,
    database: DatabaseHandle,
    state: SessionState,
    application: Box<dyn OutstationApplication>,
    control_handler: Box<dyn ControlHandler>,
}

pub struct OutstationTask {
    session: OutstationSession,
    reader: TransportReader,
    writer: TransportWriter,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct OutstationConfig {
    pub tx_buffer_size: usize,
    pub rx_buffer_size: usize,
    pub outstation_address: EndpointAddress,
    pub master_address: EndpointAddress,
    pub self_address_support: SelfAddressSupport,
    pub log_level: DecodeLogLevel,
    pub confirm_timeout: Duration,
    pub select_timeout: Duration,
}

impl OutstationConfig {
    #[allow(clippy::too_many_arguments)] // TODO
    pub fn new(
        tx_buffer_size: usize,
        rx_buffer_size: usize,
        outstation_address: EndpointAddress,
        master_address: EndpointAddress,
        self_address_support: SelfAddressSupport,
        log_level: DecodeLogLevel,
        confirm_timeout: Duration,
        select_timeout: Duration,
    ) -> Self {
        OutstationConfig {
            tx_buffer_size,
            rx_buffer_size,
            outstation_address,
            self_address_support,
            master_address,
            log_level,
            confirm_timeout,
            select_timeout,
        }
    }

    fn to_session_config(&self) -> SessionConfig {
        SessionConfig {
            level: self.log_level,
            master_address: self.master_address,
            confirm_timeout: self.confirm_timeout,
            select_timeout: self.select_timeout,
        }
    }
}

impl OutstationTask {
    /// create an `OutstationTask` and return it along with a `DatabaseHandle` for updating it
    pub fn create(
        config: OutstationConfig,
        database: DatabaseConfig,
        application: Box<dyn OutstationApplication>,
        control_handler: Box<dyn ControlHandler>,
    ) -> (Self, DatabaseHandle) {
        let handle = DatabaseHandle::new(database);
        let (reader, writer) = crate::transport::create_outstation_transport_layer(
            config.outstation_address,
            config.self_address_support,
            config.rx_buffer_size,
        );
        let task = Self {
            session: OutstationSession::new(
                config.to_session_config(),
                config.tx_buffer_size,
                handle.clone(),
                application,
                control_handler,
            ),
            reader,
            writer,
        };
        (task, handle)
    }

    /// run the outstation task asynchronously until a `LinkError` occurs
    pub async fn run<T>(&mut self, io: &mut T) -> Result<(), LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        self.session
            .run(io, &mut self.reader, &mut self.writer)
            .await
    }
}

enum Confirm {
    Yes,
    Timeout,
    NewRequest,
}

enum FragmentType<'a> {
    MalformedRequest(u64, ObjectParseError),
    NewRead(u64, HeaderCollection<'a>),
    RepeatRead(u64, usize, HeaderCollection<'a>),
    NewNonRead(u64, HeaderCollection<'a>),
    RepeatNonRead(u64, usize),
    Broadcast(BroadcastConfirmMode),
    SolicitedConfirm,
    UnsolicitedConfirm,
}

#[derive(Copy, Clone)]
enum ConfirmAction {
    Confirmed,
    NewRequest,
    EchoLastResponse(usize),
    ContinueWait,
}

impl OutstationSession {
    pub(crate) const MIN_TX_BUFFER_SIZE: usize = 249; // 1 link frame
    pub(crate) const MIN_RX_BUFFER_SIZE: usize = 249; // 1 link frame

    fn new(
        config: SessionConfig,
        tx_buffer_size: usize,
        database: DatabaseHandle,
        application: Box<dyn OutstationApplication>,
        control_handler: Box<dyn ControlHandler>,
    ) -> Self {
        let tx_buffer_size = if tx_buffer_size < Self::MIN_TX_BUFFER_SIZE {
            log::warn!("Minimum TX buffer size is {}. Defaulting to this value because the provided value ({}) is too low.", Self::MIN_TX_BUFFER_SIZE, tx_buffer_size);
            Self::MIN_TX_BUFFER_SIZE
        } else {
            tx_buffer_size
        };

        Self {
            config,
            tx_buffer: Buffer::new(tx_buffer_size),
            database,
            state: SessionState::new(),
            application,
            control_handler,
        }
    }

    pub(crate) async fn run<T>(
        &mut self,
        io: &mut T,
        reader: &mut TransportReader,
        writer: &mut TransportWriter,
    ) -> Result<(), LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        loop {
            self.run_idle_state(io, reader, writer).await?;
        }
    }

    async fn respond<T>(
        &self,
        io: &mut T,
        writer: &mut TransportWriter,
        num_bytes: usize,
        address: EndpointAddress,
    ) -> Result<(), LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        writer
            .write(
                io,
                self.config.level,
                address.wrap(),
                self.tx_buffer.get(num_bytes).unwrap(),
            )
            .await
    }

    async fn run_idle_state<T>(
        &mut self,
        io: &mut T,
        reader: &mut TransportReader,
        writer: &mut TransportWriter,
    ) -> Result<(), LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        let series: Option<ResponseSeries> = self
            .handle_one_request_from_idle(io, reader, writer)
            .await?;

        // we've handled the current fragment
        reader.pop();

        if let Some(series) = series {
            self.sol_confirm_wait(io, reader, writer, series).await?;
        }

        crate::tokio::select! {
            frame_read = reader.read(io) => {
                frame_read?;
            }
            _ = self.database.wait_for_change() => {
                // wake for unsolicited here
            }
        }

        Ok(())
    }

    async fn handle_one_request_from_idle<T>(
        &mut self,
        io: &mut T,
        reader: &mut TransportReader,
        writer: &mut TransportWriter,
    ) -> Result<Option<ResponseSeries>, LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        if let Some((info, request)) = reader.peek_request(self.config.level) {
            if let Some(result) = self.process_request_from_idle(info, request) {
                self.state.last_valid_request = Some(result);
                self.respond(io, writer, result.response_length, info.source)
                    .await?;
                if let NextState::SolConfirmWait(series) = result.next {
                    return Ok(Some(series));
                }
            }
        }

        Ok(None)
    }

    fn process_request_from_idle(
        &mut self,
        info: FragmentInfo,
        request: Request,
    ) -> Option<LastValidRequest> {
        let seq = request.header.control.seq;

        match self.classify(info, request) {
            FragmentType::MalformedRequest(hash, err) => {
                let length = self.write_empty_response(seq, err.into());
                Some(LastValidRequest::new(seq, hash, length, NextState::Idle))
            }
            FragmentType::NewRead(hash, objects) => {
                let (length, next) = self.write_first_read_response(info.source, seq, objects);
                Some(LastValidRequest::new(seq, hash, length, next))
            }
            FragmentType::RepeatRead(hash, _, objects) => {
                // this deviates a bit from the spec, the specification says to
                // also reply to duplicate READ requests from idle, but this
                // is plainly wrong since it can't possibly handle a multi-fragmented
                // response correctly. Answering a repeat READ with a fresh response is harmless
                let (length, next) = self.write_first_read_response(info.source, seq, objects);
                Some(LastValidRequest::new(seq, hash, length, next))
            }
            FragmentType::NewNonRead(hash, objects) => {
                let length = self.handle_non_read(request.header.function, seq, info.id, objects);
                Some(LastValidRequest::new(seq, hash, length, NextState::Idle))
            }
            FragmentType::RepeatNonRead(hash, last_response_length) => {
                // per the spec, we just echo the last response
                Some(LastValidRequest::new(
                    seq,
                    hash,
                    last_response_length,
                    NextState::Idle,
                ))
            }
            FragmentType::Broadcast(mode) => {
                self.process_broadcast(mode, request);
                None
            }
            FragmentType::SolicitedConfirm => {
                log::warn!(
                    "ignoring solicited CONFIRM from idle state with seq: {}",
                    seq.value()
                );
                None
            }
            FragmentType::UnsolicitedConfirm => {
                log::warn!(
                    "ignoring unsolicited CONFIRM from idle state with seq: {}",
                    seq.value()
                );
                None
            }
        }
    }

    fn write_empty_response(&mut self, seq: Sequence, iin2: IIN2) -> usize {
        let iin = IIN::new(self.get_response_iin(), iin2);
        let header = ResponseHeader::new(
            Control::response(seq, true, true, false),
            ResponseFunction::Response,
            iin,
        );
        let mut cursor = self.tx_buffer.write_cursor();
        header.write(&mut cursor).unwrap();
        cursor.written().len()
    }

    fn write_first_read_response(
        &mut self,
        master: EndpointAddress,
        seq: Sequence,
        object_headers: HeaderCollection,
    ) -> (usize, NextState) {
        let iin2 = self.database.select(&object_headers);
        self.write_read_response(master, true, seq, iin2)
    }

    fn write_read_response(
        &mut self,
        master: EndpointAddress,
        fir: bool,
        seq: Sequence,
        iin2: IIN2,
    ) -> (usize, NextState) {
        let iin1 = self.get_response_iin();
        let mut cursor = self.tx_buffer.write_cursor();
        cursor.skip(ResponseHeader::LENGTH).unwrap();
        let info = self.database.write_response_headers(&mut cursor);
        let header = ResponseHeader::new(
            Control::response(seq, fir, info.complete, info.need_confirm()),
            ResponseFunction::Response,
            iin1 + iin2,
        );
        cursor.at_start(|cur| header.write(cur)).unwrap();
        (cursor.written().len(), info.to_next_state(seq, master))
    }

    fn handle_non_read(
        &mut self,
        function: FunctionCode,
        seq: Sequence,
        frame_id: u32,
        object_headers: HeaderCollection,
    ) -> usize {
        let iin2 = Self::get_iin2(function, object_headers);

        match function {
            FunctionCode::Write => self.handle_write(seq, object_headers),
            // these function don't process objects
            FunctionCode::DelayMeasure => self.handle_delay_measure(seq, iin2),
            FunctionCode::ColdRestart => {
                let delay = self.application.cold_restart();
                self.handle_restart(seq, delay, iin2)
            }
            FunctionCode::WarmRestart => {
                let delay = self.application.warm_restart();
                self.handle_restart(seq, delay, iin2)
            }
            // controls
            FunctionCode::Select => self.handle_select(seq, frame_id, object_headers),
            FunctionCode::Operate => self.handle_operate(seq, frame_id, object_headers),
            FunctionCode::DirectOperate => self.handle_direct_operate(seq, object_headers),

            _ => {
                log::warn!("unsupported function code: {:?}", function);
                self.write_empty_response(seq, IIN2::NO_FUNC_CODE_SUPPORT)
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

    fn expect_empty(function: FunctionCode, object_headers: HeaderCollection) -> IIN2 {
        if object_headers.is_empty() {
            IIN2::default()
        } else {
            log::warn!("ignoring object headers in {:?} request", function);
            IIN2::NO_FUNC_CODE_SUPPORT
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

        self.write_empty_response(seq, iin2)
    }

    fn handle_delay_measure(&mut self, seq: Sequence, iin2: IIN2) -> usize {
        let iin = self.get_response_iin() + iin2;

        let g52v2 = Group52Var2 {
            time: self.application.get_processing_delay_ms(),
        };

        let mut cursor = self.tx_buffer.write_cursor();
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
            None => return self.write_empty_response(seq, iin2 | IIN2::NO_FUNC_CODE_SUPPORT),
            Some(x) => x,
        };

        let iin = self.get_response_iin() + iin2;

        // respond with the delay
        let mut cursor = self.tx_buffer.write_cursor();
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

    fn handle_direct_operate(&mut self, seq: Sequence, object_headers: HeaderCollection) -> usize {
        let controls = match ControlCollection::from(object_headers) {
            Err(err) => {
                log::warn!(
                    "ignoring control request containing non-control object header {} - {}",
                    err.variation,
                    err.qualifier
                );
                return self.write_empty_response(seq, IIN2::PARAMETER_ERROR);
            }
            Ok(controls) => controls,
        };

        let iin = self.get_response_iin() + IIN2::default();
        let mut cursor = self.tx_buffer.write_cursor();
        ResponseHeader::new(
            Control::single_response(seq),
            ResponseFunction::Response,
            iin,
        )
        .write(&mut cursor)
        .unwrap();

        let handler = self.control_handler.borrow_mut();

        let _ = self.database.transaction(|database| {
            controls.operate_with_response(
                &mut cursor,
                OperateType::DirectOperate,
                handler,
                database,
            )
        });

        cursor.written().len()
    }

    fn handle_select(
        &mut self,
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
                return self.write_empty_response(seq, IIN2::PARAMETER_ERROR);
            }
            Ok(controls) => controls,
        };

        let iin = self.get_response_iin() + IIN2::default();
        let mut cursor = self.tx_buffer.write_cursor();
        ResponseHeader::new(
            Control::single_response(seq),
            ResponseFunction::Response,
            iin,
        )
        .write(&mut cursor)
        .unwrap();

        let handler = self.control_handler.borrow_mut();

        let result: Result<CommandStatus, WriteError> = self
            .database
            .transaction(|database| controls.select_with_response(&mut cursor, handler, database));

        if let Ok(CommandStatus::Success) = result {
            self.state.select_state = Some(SelectState::new(
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
                return self.write_empty_response(seq, IIN2::PARAMETER_ERROR);
            }
            Ok(controls) => controls,
        };

        let iin = self.get_response_iin() + IIN2::default();
        let mut cursor = self.tx_buffer.write_cursor();
        ResponseHeader::new(
            Control::single_response(seq),
            ResponseFunction::Response,
            iin,
        )
        .write(&mut cursor)
        .unwrap();

        // determine if we have a matching SELECT
        match self.state.select_state {
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
                        let handler = self.control_handler.borrow_mut();
                        let _ = self.database.transaction(|db| {
                            controls.operate_with_response(
                                &mut cursor,
                                OperateType::SelectBeforeOperate,
                                handler,
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

    fn process_broadcast(&mut self, _: BroadcastConfirmMode, request: Request) {
        // TODO - implement broadcast request handling
        log::warn!(
            "ignoring broadcast frame with function: {:?}",
            request.header.function
        )
    }

    fn new_confirm_deadline(&self) -> crate::tokio::time::Instant {
        crate::tokio::time::Instant::now() + self.config.confirm_timeout
    }

    async fn sol_confirm_wait<T>(
        &mut self,
        io: &mut T,
        reader: &mut TransportReader,
        writer: &mut TransportWriter,
        mut series: ResponseSeries,
    ) -> Result<(), LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        loop {
            match self
                .wait_for_sol_confirm(io, reader, writer, series.ecsn)
                .await?
            {
                Confirm::Yes => {
                    reader.pop();
                    self.database.clear_written_events();
                    if series.last_response {
                        // done with response series
                        return Ok(());
                    }
                    // format the next response in the series
                    series.ecsn.increment();
                    let (length, next_state) = self.write_read_response(
                        series.master,
                        false,
                        series.ecsn,
                        IIN2::default(),
                    );
                    self.respond(io, writer, length, series.master).await?;
                    match next_state {
                        NextState::Idle => return Ok(()),
                        NextState::SolConfirmWait(next) => {
                            series = next;
                        }
                    }
                }
                Confirm::Timeout => {
                    self.database.reset();
                    return Ok(());
                }
                Confirm::NewRequest => {
                    self.database.reset();
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
    ) -> Result<Confirm, LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        let mut deadline = self.new_confirm_deadline();
        loop {
            match reader.read_with_timeout(io, deadline).await? {
                Timeout::Yes => return Ok(Confirm::Timeout),
                // process data
                Timeout::No => {
                    if let Some((info, request)) = reader.peek_request(self.config.level) {
                        match self.expect_sol_confirm(ecsn, info, request) {
                            ConfirmAction::ContinueWait => {}
                            ConfirmAction::Confirmed => return Ok(Confirm::Yes),
                            ConfirmAction::NewRequest => return Ok(Confirm::NewRequest),
                            ConfirmAction::EchoLastResponse(length) => {
                                self.respond(io, writer, length, info.source).await?;
                                // per the spec, we restart the confirm timer
                                deadline = self.new_confirm_deadline();
                            }
                        }
                    }
                }
            }
        }
    }

    fn expect_sol_confirm(
        &self,
        ecsn: Sequence,
        info: FragmentInfo,
        request: Request,
    ) -> ConfirmAction {
        match self.classify(info, request) {
            FragmentType::MalformedRequest(_, _) => ConfirmAction::NewRequest,
            FragmentType::NewRead(_, _) => ConfirmAction::NewRequest,
            FragmentType::RepeatRead(_, response_length, _) => {
                ConfirmAction::EchoLastResponse(response_length)
            }
            FragmentType::NewNonRead(_, _) => ConfirmAction::NewRequest,
            // this should never happen, but if it does, new request is probably best course of action
            FragmentType::RepeatNonRead(_, _) => ConfirmAction::NewRequest,
            FragmentType::Broadcast(_) => ConfirmAction::NewRequest,
            FragmentType::SolicitedConfirm => {
                if request.header.control.seq == ecsn {
                    ConfirmAction::Confirmed
                } else {
                    log::warn!("received solicited confirm with wrong sequence number, expected: {} received: {}", ecsn.value(), request.header.control.seq.value());
                    ConfirmAction::ContinueWait
                }
            }
            FragmentType::UnsolicitedConfirm => {
                log::warn!(
                    "ignoring unsolicited CONFIRM while waiting for solicited confirm, seq: {}",
                    request.header.control.seq.value()
                );
                ConfirmAction::ContinueWait
            }
        }
    }

    fn classify<'a>(&self, info: FragmentInfo, request: Request<'a>) -> FragmentType<'a> {
        if request.header.function == FunctionCode::Confirm {
            return if request.header.control.uns {
                FragmentType::UnsolicitedConfirm
            } else {
                FragmentType::SolicitedConfirm
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::link::header::FrameInfo;
    use crate::outstation::traits::{DefaultControlHandler, DefaultOutstationApplication};
    use crate::tokio::test::*;

    struct OutstationTestHarness<T>
    where
        T: std::future::Future<Output = Result<(), LinkError>>,
    {
        database: DatabaseHandle,
        io: io::Handle,
        task: Spawn<T>,
    }

    fn new_harness(
        config: OutstationConfig,
    ) -> OutstationTestHarness<impl std::future::Future<Output = Result<(), LinkError>>> {
        let (task, database) = OutstationTask::create(
            config,
            DatabaseConfig::default(),
            DefaultOutstationApplication::create(),
            DefaultControlHandler::with_status(CommandStatus::Success),
        );

        let mut task = Box::new(task);

        task.reader
            .get_inner()
            .set_rx_frame_info(FrameInfo::new(EndpointAddress::from(10).unwrap(), None));

        let (mut io, io_handle) = io::mock();

        OutstationTestHarness {
            database,
            io: io_handle,
            task: spawn(async move { task.run(&mut io).await }),
        }
    }

    fn get_config() -> OutstationConfig {
        OutstationConfig::new(
            2048,
            2048,
            EndpointAddress::from(10).unwrap(),
            EndpointAddress::from(1).unwrap(),
            SelfAddressSupport::Disabled,
            DecodeLogLevel::ObjectValues,
            Duration::from_secs(2),
            Duration::from_secs(5),
        )
    }

    #[test]
    fn performs_direct_operate() {
        colog::init();

        let mut harness = new_harness(get_config());

        harness.io.read(&[
            // direct operate, seq == 0, g41v2 - count == 1, index == 7, value = 513, status == SUCCESS
            0xC0, 0x05, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00,
        ]);

        assert_pending!(harness.task.poll());
        harness.io.write(&[
            // response, seq == 0, restart IIN + echo of request headers
            0xC0, 0x81, 0x80, 0x00, 41, 2, 0x17, 0x1, 0x07, 0x01, 0x02, 0x00,
        ]);
        assert_pending!(harness.task.poll());
        assert!(harness.io.all_done())
    }
}
