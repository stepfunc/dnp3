use crate::app::enums::QualifierCode;
use crate::app::header::ResponseHeader;
use crate::app::measurement::*;
use crate::app::parse::bytes::Bytes;
use crate::app::parse::DecodeLogLevel;
use crate::app::retry::RetryStrategy;
use crate::app::timeout::Timeout;
use crate::app::types::LinkStatusResult;
use crate::app::types::Timestamp;
use crate::app::variations::Variation;
use crate::entry::EndpointAddress;
use crate::master::association::{Association, Configuration};
use crate::master::error::{AssociationError, CommandError, PollError, TaskError, TimeSyncError};
use crate::master::messages::{AssociationMsg, AssociationMsgType, MasterMsg, Message};
use crate::master::poll::{PollHandle, PollMsg};
use crate::master::request::{CommandHeaders, CommandMode, ReadRequest, TimeSyncProcedure};
use crate::master::session::MasterSession;
use crate::master::tasks::command::CommandTask;
use crate::master::tasks::read::SingleReadTask;
use crate::master::tasks::restart::{RestartTask, RestartType};
use crate::master::tasks::time::TimeSyncTask;
use crate::master::tasks::Task;
use crate::tokio::sync::mpsc::error::SendError;
use crate::util::task::Shutdown;
use std::time::{Duration, SystemTime};

#[derive(Clone, Debug)]
pub struct MasterHandle {
    sender: crate::tokio::sync::mpsc::Sender<Message>,
}

/// Handle used to make requests against
#[derive(Clone, Debug)]
pub struct AssociationHandle {
    address: EndpointAddress,
    master: MasterHandle,
}

/// Master configuration
#[derive(Copy, Clone, Debug)]
pub struct MasterConfiguration {
    /// Local DNP3 master address
    pub address: EndpointAddress,
    /// Decode-level for DNP3 objects
    pub level: DecodeLogLevel,
    /// Reconnection strategy
    pub reconnection_strategy: RetryStrategy,
    /// Response timeout
    pub response_timeout: Timeout,
    /// TX buffer size
    ///
    /// Should be at least 249.
    pub tx_buffer_size: usize,
    /// RX buffer size
    ///
    /// Should be at least 2048.
    pub rx_buffer_size: usize,
    /// Close the connection when a framing error occurs
    pub bubble_framing_errors: bool,
}

impl MasterConfiguration {
    /// Create a configuration with default buffer sizes
    pub fn new(
        address: EndpointAddress,
        level: DecodeLogLevel,
        reconnection_strategy: RetryStrategy,
        response_timeout: Timeout,
    ) -> Self {
        Self {
            address,
            level,
            reconnection_strategy,
            response_timeout,
            tx_buffer_size: MasterSession::DEFAULT_TX_BUFFER_SIZE,
            rx_buffer_size: MasterSession::DEFAULT_RX_BUFFER_SIZE,
            bubble_framing_errors: false,
        }
    }
}

impl MasterHandle {
    pub(crate) fn new(sender: crate::tokio::sync::mpsc::Sender<Message>) -> Self {
        Self { sender }
    }

    /// Set the decoding level used by this master
    pub async fn set_decode_log_level(&mut self, level: DecodeLogLevel) -> Result<(), Shutdown> {
        self.send_master_message(MasterMsg::SetDecodeLogLevel(level))
            .await?;
        Ok(())
    }

    /// Get the current decoding level used by this master
    pub async fn get_decode_log_level(&mut self) -> Result<DecodeLogLevel, Shutdown> {
        let (tx, rx) = crate::tokio::sync::oneshot::channel::<Result<DecodeLogLevel, Shutdown>>();
        self.send_master_message(MasterMsg::GetDecodeLogLevel(Promise::OneShot(tx)))
            .await?;
        rx.await?
    }

    /// Create a new association:
    /// * `address` is the DNP3 link-layer address of the outstation
    /// * `config` controls the behavior of the master for this outstation
    /// * `handler` is a callback trait invoked when events occur for this outstation
    pub async fn add_association(
        &mut self,
        address: EndpointAddress,
        config: Configuration,
        handler: Box<dyn AssociationHandler>,
    ) -> Result<AssociationHandle, AssociationError> {
        let association = Association::new(address, config, handler);
        let (tx, rx) = crate::tokio::sync::oneshot::channel::<Result<(), AssociationError>>();
        self.send_master_message(MasterMsg::AddAssociation(association, Promise::OneShot(tx)))
            .await?;
        rx.await?
            .map(|_| (AssociationHandle::new(address, self.clone())))
    }

    async fn send_master_message(&mut self, msg: MasterMsg) -> Result<(), SendError<Message>> {
        self.sender.send(Message::Master(msg)).await
    }

    async fn send_association_message(
        &mut self,
        address: EndpointAddress,
        msg: AssociationMsgType,
    ) -> Result<(), SendError<Message>> {
        self.sender
            .send(Message::Association(AssociationMsg {
                address,
                details: msg,
            }))
            .await
    }
}

impl AssociationHandle {
    pub(crate) fn new(address: EndpointAddress, master: MasterHandle) -> Self {
        Self { address, master }
    }

    pub fn address(&self) -> EndpointAddress {
        self.address
    }

    /// Add a poll to the association
    /// * `request` defines what data is being requested
    /// * `period` defines how often the READ operation is performed
    pub async fn add_poll(
        &mut self,
        request: ReadRequest,
        period: Duration,
    ) -> Result<PollHandle, PollError> {
        let (tx, rx) = crate::tokio::sync::oneshot::channel::<Result<PollHandle, PollError>>();
        self.send_poll_message(PollMsg::AddPoll(
            self.clone(),
            request,
            period,
            Promise::OneShot(tx),
        ))
        .await?;
        rx.await?
    }

    pub async fn remove(mut self) -> Result<(), Shutdown> {
        self.master
            .send_master_message(MasterMsg::RemoveAssociation(self.address))
            .await?;
        Ok(())
    }

    pub async fn read(&mut self, request: ReadRequest) -> Result<(), TaskError> {
        let (tx, rx) = crate::tokio::sync::oneshot::channel::<Result<(), TaskError>>();
        let task = SingleReadTask::new(request, Promise::OneShot(tx));
        self.send_task(task.wrap().wrap()).await?;
        rx.await?
    }

    pub async fn operate(
        &mut self,
        mode: CommandMode,
        headers: CommandHeaders,
    ) -> Result<(), CommandError> {
        let (tx, rx) = crate::tokio::sync::oneshot::channel::<Result<(), CommandError>>();
        let task = CommandTask::from_mode(mode, headers, Promise::OneShot(tx));
        self.send_task(task.wrap().wrap()).await?;
        rx.await?
    }

    pub async fn warm_restart(&mut self) -> Result<Duration, TaskError> {
        let (tx, rx) = crate::tokio::sync::oneshot::channel::<Result<Duration, TaskError>>();
        let task = RestartTask::new(RestartType::WarmRestart, Promise::OneShot(tx));
        self.send_task(task.wrap().wrap()).await?;
        rx.await?
    }

    pub async fn cold_restart(&mut self) -> Result<Duration, TaskError> {
        let (tx, rx) = crate::tokio::sync::oneshot::channel::<Result<Duration, TaskError>>();
        let task = RestartTask::new(RestartType::ColdRestart, Promise::OneShot(tx));
        self.send_task(task.wrap().wrap()).await?;
        rx.await?
    }

    pub async fn perform_time_sync(
        &mut self,
        procedure: TimeSyncProcedure,
    ) -> Result<(), TimeSyncError> {
        let (tx, rx) = crate::tokio::sync::oneshot::channel::<Result<(), TimeSyncError>>();
        let task = TimeSyncTask::get_procedure(procedure, Promise::OneShot(tx));
        self.send_task(task.wrap().wrap()).await?;
        rx.await?
    }

    pub async fn check_link_status(&mut self) -> Result<LinkStatusResult, TaskError> {
        let (tx, rx) =
            crate::tokio::sync::oneshot::channel::<Result<LinkStatusResult, TaskError>>();
        self.send_task(Task::LinkStatus(Promise::OneShot(tx)))
            .await?;
        rx.await?
    }

    async fn send_task(&mut self, task: Task) -> Result<(), SendError<Message>> {
        self.master
            .send_association_message(self.address, AssociationMsgType::QueueTask(task))
            .await
    }

    pub(crate) async fn send_poll_message(
        &mut self,
        msg: PollMsg,
    ) -> Result<(), SendError<Message>> {
        self.master
            .send_association_message(self.address, AssociationMsgType::Poll(msg))
            .await
    }
}

/// A generic listener type that can be invoked multiple times.
/// The user can select to implement it using FnMut, Watch, or not at all.
pub enum Listener<T> {
    /// nothing is listening
    None,
    /// listener is a boxed FnMut
    BoxedFn(Box<dyn FnMut(T) + Send + Sync>),
    /// listener is a broadcast channel
    Watch(crate::tokio::sync::broadcast::Sender<T>),
}

/// A generic callback type that must be invoked once and only once.
/// The user can select to implement it using FnOnce or a
/// one-shot reply channel
pub enum Promise<T> {
    /// nothing happens when the promise is completed
    None,
    /// Box<FnOnce> is consumed when the promise is completed
    BoxedFn(Box<dyn FnOnce(T) + Send + Sync>),
    /// one-shot reply channel is consumed when the promise is completed
    OneShot(crate::tokio::sync::oneshot::Sender<T>),
}

impl<T> Promise<T> {
    pub(crate) fn complete(self, value: T) {
        match self {
            Promise::None => {}
            Promise::BoxedFn(func) => func(value),
            Promise::OneShot(s) => {
                s.send(value).ok();
            }
        }
    }
}

impl<T> Listener<T> {
    pub(crate) fn update(&mut self, value: T) {
        match self {
            Listener::None => {}
            Listener::BoxedFn(func) => func(value),
            Listener::Watch(s) => {
                s.send(value).ok();
            }
        }
    }
}

/// callbacks associated with a single master to outstation association
pub trait AssociationHandler: Send {
    /// Retrieve the system time used for time synchronization
    fn get_system_time(&self) -> Option<Timestamp> {
        Timestamp::try_from_system_time(SystemTime::now())
    }

    /// retrieve a handler used to process integrity polls
    fn get_integrity_handler(&mut self) -> &mut dyn ReadHandler;
    /// retrieve a handler used to process unsolicited responses
    fn get_unsolicited_handler(&mut self) -> &mut dyn ReadHandler;
    /// retrieve a default handler used to process user-defined polls
    fn get_default_poll_handler(&mut self) -> &mut dyn ReadHandler;
}

/// Information about the object header from which the measurement values were mapped
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct HeaderInfo {
    /// underlying variation
    pub variation: Variation,
    /// qualifier code used in the header
    pub qualifier: QualifierCode,
}

impl HeaderInfo {
    pub(crate) fn new(variation: Variation, qualifier: QualifierCode) -> Self {
        Self {
            variation,
            qualifier,
        }
    }
}

pub trait ReadHandler {
    fn begin_fragment(&mut self, header: ResponseHeader);
    fn end_fragment(&mut self, header: ResponseHeader);

    fn handle_binary(&mut self, info: HeaderInfo, iter: &mut dyn Iterator<Item = (Binary, u16)>);
    fn handle_double_bit_binary(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (DoubleBitBinary, u16)>,
    );
    fn handle_binary_output_status(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (BinaryOutputStatus, u16)>,
    );
    fn handle_counter(&mut self, info: HeaderInfo, iter: &mut dyn Iterator<Item = (Counter, u16)>);
    fn handle_frozen_counter(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (FrozenCounter, u16)>,
    );
    fn handle_analog(&mut self, info: HeaderInfo, iter: &mut dyn Iterator<Item = (Analog, u16)>);
    fn handle_analog_output_status(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (AnalogOutputStatus, u16)>,
    );
    fn handle_octet_string<'a>(
        &mut self,
        info: HeaderInfo,
        iter: &'a mut dyn Iterator<Item = (Bytes<'a>, u16)>,
    );
}

/// no-op default association handler type
#[derive(Copy, Clone)]
pub struct NullHandler;

impl NullHandler {
    /// create a default boxed instance of the NullHandler
    pub fn boxed() -> Box<NullHandler> {
        Box::new(Self {})
    }
}

impl ReadHandler for NullHandler {
    fn begin_fragment(&mut self, _header: ResponseHeader) {}

    fn end_fragment(&mut self, _header: ResponseHeader) {}

    fn handle_binary(&mut self, _info: HeaderInfo, _iter: &mut dyn Iterator<Item = (Binary, u16)>) {
    }

    fn handle_double_bit_binary(
        &mut self,
        _info: HeaderInfo,
        _iter: &mut dyn Iterator<Item = (DoubleBitBinary, u16)>,
    ) {
    }

    fn handle_binary_output_status(
        &mut self,
        _info: HeaderInfo,
        _iter: &mut dyn Iterator<Item = (BinaryOutputStatus, u16)>,
    ) {
    }

    fn handle_counter(
        &mut self,
        _info: HeaderInfo,
        _iter: &mut dyn Iterator<Item = (Counter, u16)>,
    ) {
    }

    fn handle_frozen_counter(
        &mut self,
        _info: HeaderInfo,
        _iter: &mut dyn Iterator<Item = (FrozenCounter, u16)>,
    ) {
    }

    fn handle_analog(&mut self, _info: HeaderInfo, _iter: &mut dyn Iterator<Item = (Analog, u16)>) {
    }

    fn handle_analog_output_status(
        &mut self,
        _info: HeaderInfo,
        _iter: &mut dyn Iterator<Item = (AnalogOutputStatus, u16)>,
    ) {
    }

    fn handle_octet_string<'a>(
        &mut self,
        _info: HeaderInfo,
        _iter: &mut dyn Iterator<Item = (Bytes<'a>, u16)>,
    ) {
    }
}

impl AssociationHandler for NullHandler {
    fn get_integrity_handler(&mut self) -> &mut dyn ReadHandler {
        self
    }

    fn get_unsolicited_handler(&mut self) -> &mut dyn ReadHandler {
        self
    }

    fn get_default_poll_handler(&mut self) -> &mut dyn ReadHandler {
        self
    }
}
