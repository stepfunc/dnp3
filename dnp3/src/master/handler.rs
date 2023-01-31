use std::time::{Duration, SystemTime};

use crate::app::attr::*;
use crate::app::measurement::*;
use crate::app::variations::Variation;
use crate::app::*;

use crate::decode::DecodeLevel;
use crate::link::EndpointAddress;
use crate::master::association::AssociationConfig;
use crate::master::error::{AssociationError, CommandError, PollError, TaskError, TimeSyncError};
use crate::master::messages::{AssociationMsg, AssociationMsgType, MasterMsg, Message};
use crate::master::poll::{PollHandle, PollMsg};
use crate::master::request::{CommandHeaders, CommandMode, ReadRequest, TimeSyncProcedure};
use crate::master::tasks::command::CommandTask;
use crate::master::tasks::deadbands::WriteDeadBandsTask;
use crate::master::tasks::empty_response::EmptyResponseTask;
use crate::master::tasks::file_read::FileReadTask;
use crate::master::tasks::read::SingleReadTask;
use crate::master::tasks::restart::{RestartTask, RestartType};
use crate::master::tasks::time::TimeSyncTask;
use crate::master::tasks::{NonReadTask, Task};
use crate::master::{
    DeadBandHeader, DirectoryReader, FileCredentials, FileReadConfig, FileReader, Headers,
    WriteError,
};
use crate::util::channel::Sender;
use crate::util::session::Enabled;

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
    /// TX buffer size
    ///
    /// Must be at least 249.
    pub tx_buffer_size: BufferSize<249, 2048>,
    /// RX buffer size
    ///
    /// Must be at least 2048.
    pub rx_buffer_size: BufferSize<2048, 2048>,
}

impl MasterChannelConfig {
    /// Create a configuration with default buffer sizes, no decoding, and a default timeout of 5 seconds
    pub fn new(master_address: EndpointAddress) -> Self {
        Self {
            master_address,
            decode_level: DecodeLevel::nothing(),
            tx_buffer_size: BufferSize::default(),
            rx_buffer_size: BufferSize::default(),
        }
    }
}

impl MasterChannel {
    pub(crate) fn new(sender: Sender<Message>) -> Self {
        Self { sender }
    }

    /// enable communications
    pub async fn enable(&mut self) -> Result<(), Shutdown> {
        self.send_master_message(MasterMsg::EnableCommunication(Enabled::Yes))
            .await?;
        Ok(())
    }

    /// disable communications
    pub async fn disable(&mut self) -> Result<(), Shutdown> {
        self.send_master_message(MasterMsg::EnableCommunication(Enabled::No))
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
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<DecodeLevel, Shutdown>>();
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
        assoc_information: Box<dyn AssociationInformation>,
    ) -> Result<AssociationHandle, AssociationError> {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), AssociationError>>();
        self.send_master_message(MasterMsg::AddAssociation(
            address,
            config,
            read_handler,
            assoc_handler,
            assoc_information,
            Promise::OneShot(tx),
        ))
        .await?;
        rx.await?
            .map(|_| (AssociationHandle::new(address, self.clone())))
    }

    /// Remove an association
    /// * `address` is the DNP3 link-layer address of the outstation
    pub async fn remove_association(&mut self, address: EndpointAddress) -> Result<(), Shutdown> {
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
    #[doc(hidden)]
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
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<PollHandle, PollError>>();
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
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), TaskError>>();
        let task = SingleReadTask::new(request, Promise::OneShot(tx));
        self.send_task(task.wrap().wrap()).await?;
        rx.await?
    }

    /// Perform an asynchronous request with the specified function code and object headers
    ///
    /// This is useful for constructing various types of WRITE and FREEZE operations where
    /// an empty response is expected from the outstation, and the only indication of success
    /// are bits in IIN.2
    pub async fn send_and_expect_empty_response(
        &mut self,
        function: FunctionCode,
        headers: Headers,
    ) -> Result<(), WriteError> {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), WriteError>>();
        let task = EmptyResponseTask::new(function, headers, Promise::OneShot(tx));
        self.send_task(task.wrap().wrap()).await?;
        rx.await?
    }

    /// Perform an asynchronous READ request with a custom read handler
    ///
    /// If successful, the custom [ReadHandler](crate::master::ReadHandler) will process the received measurement data
    pub async fn read_with_handler(
        &mut self,
        request: ReadRequest,
        handler: Box<dyn ReadHandler>,
    ) -> Result<(), TaskError> {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), TaskError>>();
        let task = SingleReadTask::new_with_custom_handler(request, handler, Promise::OneShot(tx));
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
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), CommandError>>();
        let task = CommandTask::from_mode(mode, headers, Promise::OneShot(tx));
        self.send_task(task.wrap().wrap()).await?;
        rx.await?
    }

    /// Perform a WARM_RESTART operation
    ///
    /// Returns the delay from the outstation's response as a [Duration](std::time::Duration)
    pub async fn warm_restart(&mut self) -> Result<Duration, TaskError> {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<Duration, TaskError>>();
        let task = RestartTask::new(RestartType::WarmRestart, Promise::OneShot(tx));
        self.send_task(task.wrap().wrap()).await?;
        rx.await?
    }

    /// Perform a COLD_RESTART operation
    ///
    /// Returns the delay from the outstation's response as a [Duration](std::time::Duration)
    pub async fn cold_restart(&mut self) -> Result<Duration, TaskError> {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<Duration, TaskError>>();
        let task = RestartTask::new(RestartType::ColdRestart, Promise::OneShot(tx));
        self.send_task(task.wrap().wrap()).await?;
        rx.await?
    }

    /// Perform the specified time synchronization operation
    pub async fn synchronize_time(
        &mut self,
        procedure: TimeSyncProcedure,
    ) -> Result<(), TimeSyncError> {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), TimeSyncError>>();
        let task = TimeSyncTask::get_procedure(procedure, Promise::OneShot(tx));
        self.send_task(task.wrap().wrap()).await?;
        rx.await?
    }

    /// Perform write one or more headers of analog input dead-bands to the outstation
    pub async fn write_dead_bands(
        &mut self,
        headers: Vec<DeadBandHeader>,
    ) -> Result<(), WriteError> {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), WriteError>>();
        let task = WriteDeadBandsTask::new(headers, Promise::OneShot(tx));
        self.send_task(task.wrap().wrap()).await?;
        rx.await?
    }

    /// Trigger the master to issue a REQUEST_LINK_STATUS function in advance of the link status timeout
    ///
    /// This function is provided for testing purposes. Using the configured link status timeout
    /// is the preferred so that the master automatically issues these requests.
    ///
    /// If a [`TaskError::UnexpectedResponseHeaders`] is returned, the link might be alive
    /// but it didn't answer with the expected `LINK_STATUS`.
    pub async fn check_link_status(&mut self) -> Result<(), TaskError> {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), TaskError>>();
        self.send_task(Task::LinkStatus(Promise::OneShot(tx)))
            .await?;
        rx.await?
    }

    /// Read a file
    pub async fn read_file<T: ToString>(
        &mut self,
        file_path: T,
        config: FileReadConfig,
        reader: Box<dyn FileReader>,
        credentials: Option<FileCredentials>,
    ) -> Result<(), Shutdown> {
        let task = FileReadTask::start(file_path.to_string(), config, reader, credentials);
        self.send_task(Task::NonRead(NonReadTask::FileRead(task)))
            .await
    }

    /// Read a file directory
    pub async fn read_directory<T: ToString>(
        &mut self,
        dir_path: T,
        config: FileReadConfig,
        credentials: Option<FileCredentials>,
    ) -> Result<(), Shutdown> {
        let reader = Box::new(DirectoryReader::new());
        self.read_file(dir_path, config, reader, credentials).await
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
pub(crate) enum Promise<T> {
    /// nothing happens when the promise is completed
    None,
    /// one-shot reply channel is consumed when the promise is completed
    OneShot(tokio::sync::oneshot::Sender<T>),
}

impl<T> Promise<T> {
    pub(crate) fn complete(self, value: T) {
        match self {
            Promise::None => {}
            Promise::OneShot(s) => {
                s.send(value).ok();
            }
        }
    }
}

/// Task types used in [`AssociationInformation`]
#[cfg_attr(not(feature = "ffi"), non_exhaustive)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskType {
    /// User-defined read request
    UserRead,
    /// Periodic poll task
    PeriodicPoll,
    /// Startup integrity scan
    StartupIntegrity,
    /// Automatic event scan caused by `RESTART` IIN bit detection
    AutoEventScan,
    /// Command request
    Command,
    /// Clear `RESTART` IIN bit
    ClearRestartBit,
    /// Enable unsolicited startup request
    EnableUnsolicited,
    /// Disable unsolicited startup request
    DisableUnsolicited,
    /// Time synchronisation task
    TimeSync,
    /// Cold or warm restart task
    Restart,
    /// Write dead-bands
    WriteDeadBands,
    /// Generic task which
    GenericEmptyResponse(FunctionCode),
    /// Read a file from the outstation
    FileRead,
}

/// callbacks associated with a single master to outstation association
pub trait AssociationHandler: Send + Sync {
    /// Retrieve the system time used for time synchronization
    fn get_current_time(&self) -> Option<Timestamp> {
        Timestamp::try_from_system_time(SystemTime::now())
    }
}

/// Informational callbacks that can be used to monitor master communication
/// to a particular outstation. Useful for assessing communication health.
pub trait AssociationInformation: Send + Sync {
    /// Called when a new task is started
    fn task_start(&mut self, _task_type: TaskType, _fc: FunctionCode, _seq: Sequence) {}
    /// Called when a task successfully completes
    fn task_success(&mut self, _task_type: TaskType, _fc: FunctionCode, _seq: Sequence) {}
    /// Called when a task fails
    fn task_fail(&mut self, _task_type: TaskType, _error: TaskError) {}

    /// Called when an unsolicited response is received
    fn unsolicited_response(&mut self, _is_duplicate: bool, _seq: Sequence) {}
}

pub(crate) struct NullAssociationInformation;

impl AssociationInformation for NullAssociationInformation {}

/// Information about the object header and specific variation
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct HeaderInfo {
    /// underlying variation in the response
    pub variation: Variation,
    /// qualifier code used in the response
    pub qualifier: QualifierCode,
    /// true if the received variation is an event type, false otherwise
    pub is_event: bool,
    /// true if a flags byte is present on the underlying variation, false otherwise
    pub has_flags: bool,
}

impl HeaderInfo {
    pub(crate) fn new(
        variation: Variation,
        qualifier: QualifierCode,
        is_event: bool,
        has_flags: bool,
    ) -> Self {
        Self {
            variation,
            qualifier,
            is_event,
            has_flags,
        }
    }
}

/// Describes the source of a read event
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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
pub trait ReadHandler: Send + Sync {
    /// Called as the first action before any of the type-specific handle methods are invoked
    ///
    /// `read_type` provides information about what triggered the call, e.g. response vs unsolicited
    /// `header` provides the full response header
    ///
    /// Note: The operation may or may not be async depending
    #[allow(unused_variables)]
    fn begin_fragment(&mut self, read_type: ReadType, header: ResponseHeader) -> MaybeAsync<()> {
        MaybeAsync::ready(())
    }

    /// Called as the last action after all of the type-specific handle methods have been invoked
    ///
    /// `read_type` provides information about what triggered the call, e.g. response vs unsolicited
    /// `header` provides the full response header
    ///
    /// Note: The operation may or may not be async depending. A typical use case for using async
    /// here would be to publish a message to an async MPSC.
    #[allow(unused_variables)]
    fn end_fragment(&mut self, read_type: ReadType, header: ResponseHeader) -> MaybeAsync<()> {
        MaybeAsync::ready(())
    }

    /// Process an object header of `BinaryInput` values
    #[allow(unused_variables)]
    fn handle_binary_input(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (BinaryInput, u16)>,
    ) {
    }

    /// Process an object header of `DoubleBitBinaryInput` values
    #[allow(unused_variables)]
    fn handle_double_bit_binary_input(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (DoubleBitBinaryInput, u16)>,
    ) {
    }

    /// Process an object header of `BinaryOutputStatus` values
    #[allow(unused_variables)]
    fn handle_binary_output_status(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (BinaryOutputStatus, u16)>,
    ) {
    }

    /// Process an object header of `Counter` values
    #[allow(unused_variables)]
    fn handle_counter(&mut self, info: HeaderInfo, iter: &mut dyn Iterator<Item = (Counter, u16)>) {
    }

    /// Process an object header of `FrozenCounter` values
    #[allow(unused_variables)]
    fn handle_frozen_counter(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (FrozenCounter, u16)>,
    ) {
    }

    /// Process an object header of `AnalogInput` values
    #[allow(unused_variables)]
    fn handle_analog_input(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (AnalogInput, u16)>,
    ) {
    }

    /// Process an object header of `FrozenAnalogInput` values
    #[allow(unused_variables)]
    fn handle_frozen_analog_input(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (FrozenAnalogInput, u16)>,
    ) {
    }

    /// Process an object header of `AnalogInputDeadBand` values
    #[allow(unused_variables)]
    fn handle_analog_input_dead_band(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (AnalogInputDeadBand, u16)>,
    ) {
    }

    /// Process an object header of `AnalogOutputStatus` values
    #[allow(unused_variables)]
    fn handle_analog_output_status(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (AnalogOutputStatus, u16)>,
    ) {
    }

    /// Process an object header of octet string values
    #[allow(unused_variables)]
    fn handle_octet_string<'a>(
        &mut self,
        info: HeaderInfo,
        iter: &'a mut dyn Iterator<Item = (&'a [u8], u16)>,
    ) {
    }

    /// Process a device attribute
    #[allow(unused_variables)]
    fn handle_device_attribute(&mut self, info: HeaderInfo, attr: AnyAttribute) {}
}

pub(crate) fn handle_attribute(
    var: Variation,
    qualifier: QualifierCode,
    attr: &Option<Attribute>,
    handler: &mut dyn ReadHandler,
) {
    if let Some(attr) = attr {
        match AnyAttribute::try_from(attr) {
            Ok(attr) => {
                handler
                    .handle_device_attribute(HeaderInfo::new(var, qualifier, false, false), attr);
            }
            Err(err) => {
                tracing::warn!(
                    "Expected attribute type {:?} but received {:?}",
                    err.expected,
                    err.actual
                );
            }
        }
    }
}

/// read handler that does nothing
#[derive(Copy, Clone)]
pub(crate) struct NullReadHandler;
impl ReadHandler for NullReadHandler {}
