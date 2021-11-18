use std::time::{Duration, SystemTime};

use crate::app::measurement::*;
use crate::app::variations::Variation;
use crate::app::*;
use crate::decode::DecodeLevel;
use crate::link::{EndpointAddress, LinkStatusResult};
use crate::master::association::AssociationConfig;
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
use crate::util::channel::Sender;

/// Handle to a master communication channel. This handle controls
/// a task running on the Tokio Runtime.
///
/// It provides a uniform API for all of the various types of communication channels supported
/// by the library.
#[derive(Debug, Clone)]
pub struct MasterChannel {
    sender: Sender<Message>,
}

/// Handle used to make requests against a particular outstation associated with the master channel
#[derive(Clone, Debug)]
pub struct AssociationHandle {
    address: EndpointAddress,
    master: MasterChannel,
}

/// Configuration for a MasterChannel
#[derive(Copy, Clone, Debug)]
pub struct MasterChannelConfig {
    /// Local DNP3 master address
    pub master_address: EndpointAddress,
    /// Decode-level for DNP3 objects
    pub decode_level: DecodeLevel,
    /// Response timeout
    pub response_timeout: Timeout,
    /// TX buffer size
    ///
    /// Must be at least 249.
    pub tx_buffer_size: usize,
    /// RX buffer size
    ///
    /// Must be at least 2048.
    pub rx_buffer_size: usize,
}

impl MasterChannelConfig {
    /// Create a configuration with default buffer sizes, no decoding, and a default timeout of 5 seconds
    pub fn new(master_address: EndpointAddress) -> Self {
        Self {
            master_address,
            decode_level: DecodeLevel::nothing(),
            response_timeout: Timeout::default(),
            tx_buffer_size: MasterSession::DEFAULT_TX_BUFFER_SIZE,
            rx_buffer_size: MasterSession::DEFAULT_RX_BUFFER_SIZE,
        }
    }
}

impl MasterChannel {
    pub(crate) fn new(sender: Sender<Message>) -> Self {
        Self { sender }
    }

    /// enable communications
    pub async fn enable(&mut self) -> Result<(), Shutdown> {
        self.send_master_message(MasterMsg::EnableCommunication(true))
            .await?;
        Ok(())
    }

    /// disable communications
    pub async fn disable(&mut self) -> Result<(), Shutdown> {
        self.send_master_message(MasterMsg::EnableCommunication(false))
            .await?;
        Ok(())
    }

    /// Set the decoding level used by this master
    pub async fn set_decode_level(&mut self, decode_level: DecodeLevel) -> Result<(), Shutdown> {
        self.send_master_message(MasterMsg::SetDecodeLevel(decode_level))
            .await?;
        Ok(())
    }

    /// Get the current decoding level used by this master
    pub async fn get_decode_level(&mut self) -> Result<DecodeLevel, Shutdown> {
        let (tx, rx) = crate::tokio::sync::oneshot::channel::<Result<DecodeLevel, Shutdown>>();
        self.send_master_message(MasterMsg::GetDecodeLevel(Promise::OneShot(tx)))
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
        config: AssociationConfig,
        read_handler: Box<dyn ReadHandler>,
        assoc_handler: Box<dyn AssociationHandler>,
    ) -> Result<AssociationHandle, AssociationError> {
        let (tx, rx) = crate::tokio::sync::oneshot::channel::<Result<(), AssociationError>>();
        self.send_master_message(MasterMsg::AddAssociation(
            address,
            config,
            read_handler,
            assoc_handler,
            Promise::OneShot(tx),
        ))
        .await?;
        rx.await?
            .map(|_| (AssociationHandle::new(address, self.clone())))
    }

    /// Remove an association
    /// * `address` is the DNP3 link-layer address of the outstation
    pub async fn remove_association(
        &mut self,
        address: EndpointAddress,
    ) -> Result<(), AssociationError> {
        self.send_master_message(MasterMsg::RemoveAssociation(address))
            .await?;
        Ok(())
    }

    async fn send_master_message(&mut self, msg: MasterMsg) -> Result<(), Shutdown> {
        self.sender.send(Message::Master(msg)).await?;
        Ok(())
    }

    async fn send_association_message(
        &mut self,
        address: EndpointAddress,
        msg: AssociationMsgType,
    ) -> Result<(), Shutdown> {
        self.sender
            .send(Message::Association(AssociationMsg {
                address,
                details: msg,
            }))
            .await
    }
}

impl AssociationHandle {
    /// constructor only used in the FFI
    #[cfg(feature = "ffi")]
    pub fn create(address: EndpointAddress, master: MasterChannel) -> Self {
        Self::new(address, master)
    }

    pub(crate) fn new(address: EndpointAddress, master: MasterChannel) -> Self {
        Self { address, master }
    }

    /// retrieve the outstation address of the association
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

    /// Remove the association from the master
    pub async fn remove(mut self) -> Result<(), Shutdown> {
        self.master
            .send_master_message(MasterMsg::RemoveAssociation(self.address))
            .await?;
        Ok(())
    }

    /// Perform an asynchronous READ request
    ///
    /// If successful, the [ReadHandler](crate::master::ReadHandler) will process the received measurement data
    pub async fn read(&mut self, request: ReadRequest) -> Result<(), TaskError> {
        let (tx, rx) = crate::tokio::sync::oneshot::channel::<Result<(), TaskError>>();
        let task = SingleReadTask::new(request, Promise::OneShot(tx));
        self.send_task(task.wrap().wrap()).await?;
        rx.await?
    }

    /// Perform an asynchronous operate request
    ///
    /// The actual function code used depends on the value of the [CommandMode](crate::master::CommandMode).
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

    /// Perform a WARM_RESTART operation
    ///
    /// Returns the delay from the outstation's response as a [Duration](std::time::Duration)
    pub async fn warm_restart(&mut self) -> Result<Duration, TaskError> {
        let (tx, rx) = crate::tokio::sync::oneshot::channel::<Result<Duration, TaskError>>();
        let task = RestartTask::new(RestartType::WarmRestart, Promise::OneShot(tx));
        self.send_task(task.wrap().wrap()).await?;
        rx.await?
    }

    /// Perform a COLD_RESTART operation
    ///
    /// Returns the delay from the outstation's response as a [Duration](std::time::Duration)
    pub async fn cold_restart(&mut self) -> Result<Duration, TaskError> {
        let (tx, rx) = crate::tokio::sync::oneshot::channel::<Result<Duration, TaskError>>();
        let task = RestartTask::new(RestartType::ColdRestart, Promise::OneShot(tx));
        self.send_task(task.wrap().wrap()).await?;
        rx.await?
    }

    /// Perform the specified time synchronization operation
    pub async fn synchronize_time(
        &mut self,
        procedure: TimeSyncProcedure,
    ) -> Result<(), TimeSyncError> {
        let (tx, rx) = crate::tokio::sync::oneshot::channel::<Result<(), TimeSyncError>>();
        let task = TimeSyncTask::get_procedure(procedure, Promise::OneShot(tx));
        self.send_task(task.wrap().wrap()).await?;
        rx.await?
    }

    /// Trigger the master to issue a REQUEST_LINK_STATUS function in advance of the link status timeout
    ///
    /// This function is provided for testing purposes. Using the configured link status timeout
    /// is the preferred so that the master automatically issues these requests.
    pub async fn check_link_status(&mut self) -> Result<LinkStatusResult, TaskError> {
        let (tx, rx) =
            crate::tokio::sync::oneshot::channel::<Result<LinkStatusResult, TaskError>>();
        self.send_task(Task::LinkStatus(Promise::OneShot(tx)))
            .await?;
        rx.await?
    }

    async fn send_task(&mut self, task: Task) -> Result<(), Shutdown> {
        self.master
            .send_association_message(self.address, AssociationMsgType::QueueTask(task))
            .await
    }

    pub(crate) async fn send_poll_message(&mut self, msg: PollMsg) -> Result<(), Shutdown> {
        self.master
            .send_association_message(self.address, AssociationMsgType::Poll(msg))
            .await
    }
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

/// callbacks associated with a single master to outstation association
pub trait AssociationHandler: Send {
    /// Retrieve the system time used for time synchronization
    fn get_system_time(&self) -> Option<Timestamp> {
        Timestamp::try_from_system_time(SystemTime::now())
    }
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

/// Describes the source of a read event
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ReadType {
    /// Startup integrity poll
    StartupIntegrity,
    /// Unsolicited message
    Unsolicited,
    /// Single poll requested by the user
    SinglePoll,
    /// Periodic poll configured by the user
    PeriodicPoll,
}

/// Trait used to process measurement data received from an outstation
pub trait ReadHandler: Send {
    /// Called as the first action before any of the type-specific handle methods are invoked
    ///
    /// `read_type` provides information about what triggered the call, e.g. response vs unsolicited
    /// `header` provides the full response header
    fn begin_fragment(&mut self, read_type: ReadType, header: ResponseHeader);

    /// Called as the last action after all of the type-specific handle methods have been invoked
    ///
    /// `read_type` provides information about what triggered the call, e.g. response vs unsolicited
    /// `header` provides the full response header
    fn end_fragment(&mut self, read_type: ReadType, header: ResponseHeader);

    /// Process an object header of `Binary` values
    fn handle_binary(&mut self, info: HeaderInfo, iter: &mut dyn Iterator<Item = (Binary, u16)>);

    /// Process an object header of `DoubleBitBinary` values
    fn handle_double_bit_binary(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (DoubleBitBinary, u16)>,
    );

    /// Process an object header of `BinaryOutputStatus` values
    fn handle_binary_output_status(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (BinaryOutputStatus, u16)>,
    );

    /// Process an object header of `Counter` values
    fn handle_counter(&mut self, info: HeaderInfo, iter: &mut dyn Iterator<Item = (Counter, u16)>);

    /// Process an object header of `FrozenCounter` values
    fn handle_frozen_counter(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (FrozenCounter, u16)>,
    );

    /// Process an object header of `Analog` values
    fn handle_analog(&mut self, info: HeaderInfo, iter: &mut dyn Iterator<Item = (Analog, u16)>);

    /// Process an object header of `AnalogOutputStatus` values
    fn handle_analog_output_status(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (AnalogOutputStatus, u16)>,
    );

    /// Process an object header of octet string values
    fn handle_octet_string<'a>(
        &mut self,
        info: HeaderInfo,
        iter: &'a mut dyn Iterator<Item = (Bytes<'a>, u16)>,
    );
}

/// no-op default association handler type
#[derive(Copy, Clone)]
pub struct DefaultAssociationHandler;

impl AssociationHandler for DefaultAssociationHandler {}

impl DefaultAssociationHandler {
    /// create a boxed instance of the DefaultAssociationHandler
    pub fn boxed() -> Box<dyn AssociationHandler> {
        Box::new(Self {})
    }
}

/// read handler that does nothing
#[derive(Copy, Clone)]
pub struct NullReadHandler;

impl NullReadHandler {
    /// create a boxed instance of the NullReadHandler
    pub fn boxed() -> Box<dyn ReadHandler> {
        Box::new(Self {})
    }
}

impl ReadHandler for NullReadHandler {
    fn begin_fragment(&mut self, _read_type: ReadType, _header: ResponseHeader) {}

    fn end_fragment(&mut self, _read_type: ReadType, _header: ResponseHeader) {}

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
