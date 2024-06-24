use std::net::SocketAddr;
use std::time::{Duration, SystemTime};

use crate::app::*;

use crate::decode::DecodeLevel;
use crate::link::EndpointAddress;
use crate::master::association::AssociationConfig;
use crate::master::error::{AssociationError, CommandError, PollError, TaskError, TimeSyncError};
use crate::master::messages::{AssociationMsg, AssociationMsgType, MasterMsg, Message};
use crate::master::poll::{PollHandle, PollMsg};
use crate::master::promise::Promise;
use crate::master::request::{CommandHeaders, CommandMode, ReadRequest, TimeSyncProcedure};
use crate::master::tasks::command::CommandTask;
use crate::master::tasks::deadbands::WriteDeadBandsTask;
use crate::master::tasks::empty_response::EmptyResponseTask;
use crate::master::tasks::file::authenticate::AuthFileTask;
use crate::master::tasks::file::close::CloseFileTask;
use crate::master::tasks::file::directory::DirectoryReader;
use crate::master::tasks::file::get_info::GetFileInfoTask;
use crate::master::tasks::file::open::{OpenFileRequest, OpenFileTask};
use crate::master::tasks::file::read::{FileReadTask, FileReaderType};
use crate::master::tasks::file::write_block::{WriteBlockRequest, WriteBlockTask};
use crate::master::tasks::read::SingleReadTask;
use crate::master::tasks::restart::{RestartTask, RestartType};
use crate::master::tasks::time::TimeSyncTask;
use crate::master::tasks::Task;
use crate::master::{
    AuthKey, BlockNumber, DeadBandHeader, DirReadConfig, FileCredentials, FileError, FileHandle,
    FileInfo, FileMode, FileReadConfig, FileReader, Headers, OpenFile, ReadHandler, WriteError,
};
use crate::transport::FragmentAddr;
use crate::util::channel::Sender;
use crate::util::phys::PhysAddr;
use crate::util::session::Enabled;

/// Master channels may be Udp or of a "stream" type such as TCP
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MasterChannelType {
    /// UDP aka datagram based
    Udp,
    /// Stream-oriented e.g. TCP
    Stream,
}

/// Handle to a master communication channel. This handle controls
/// a task running on the Tokio Runtime.
///
/// It provides a uniform API for all the various types of communication channels supported
/// by the library.
#[derive(Debug, Clone)]
pub struct MasterChannel {
    channel_type: MasterChannelType,
    sender: Sender<Message>,
}

/// Handle used to make requests against a particular outstation associated with the master channel
#[derive(Clone, Debug)]
pub struct AssociationHandle {
    address: EndpointAddress,
    master: MasterChannel,
}

/// Configuration for a MasterChannel that is independent of the physical layer
#[derive(Copy, Clone, Debug)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct MasterChannelConfig {
    /// Local DNP3 master address
    pub master_address: EndpointAddress,
    /// Decode-level for DNP3 objects
    #[cfg_attr(feature = "serialization", serde(default))]
    pub decode_level: DecodeLevel,
    /// TX buffer size
    ///
    /// Must be at least 249.
    #[cfg_attr(feature = "serialization", serde(default))]
    pub tx_buffer_size: BufferSize<249, 2048>,
    /// RX buffer size
    ///
    /// Must be at least 2048.
    #[cfg_attr(feature = "serialization", serde(default))]
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
    pub(crate) fn new(sender: Sender<Message>, channel_type: MasterChannelType) -> Self {
        Self {
            sender,
            channel_type,
        }
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
        let (promise, rx) = Promise::one_shot();
        self.send_master_message(MasterMsg::GetDecodeLevel(promise))
            .await?;
        rx.await?
    }

    fn assert_channel_type(&self, required: MasterChannelType) -> Result<(), AssociationError> {
        if self.channel_type == required {
            Ok(())
        } else {
            Err(AssociationError::WrongChannelType {
                actual: self.channel_type,
                required,
            })
        }
    }

    /// Create a new association on any stream-based channel (TCP, TLS, serial)
    ///
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
        self.assert_channel_type(MasterChannelType::Stream)?;

        let (promise, rx) = Promise::one_shot();
        let addr = FragmentAddr {
            link: address,
            phys: PhysAddr::None,
        };
        self.send_master_message(MasterMsg::AddAssociation(
            addr,
            config,
            read_handler,
            assoc_handler,
            assoc_information,
            promise,
        ))
        .await?;
        rx.await?
            .map(|_| (AssociationHandle::new(address, self.clone())))
    }

    /// Create a new association on a UDP-based channel.
    ///
    /// * `address` is the DNP3 link-layer address of the outstation
    /// * `destination` is IP address and port of the outstation
    /// * `config` controls the behavior of the master for this outstation
    /// * `handler` is a callback trait invoked when events occur for this outstation
    pub async fn add_udp_association(
        &mut self,
        address: EndpointAddress,
        destination: SocketAddr,
        config: AssociationConfig,
        read_handler: Box<dyn ReadHandler>,
        assoc_handler: Box<dyn AssociationHandler>,
        assoc_information: Box<dyn AssociationInformation>,
    ) -> Result<AssociationHandle, AssociationError> {
        self.assert_channel_type(MasterChannelType::Udp)?;

        let (promise, rx) = Promise::one_shot();
        let addr = FragmentAddr {
            link: address,
            phys: PhysAddr::Udp(destination),
        };
        self.send_master_message(MasterMsg::AddAssociation(
            addr,
            config,
            read_handler,
            assoc_handler,
            assoc_information,
            promise,
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
        let (promise, rx) = Promise::one_shot();
        self.send_poll_message(PollMsg::AddPoll(self.clone(), request, period, promise))
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
    /// If successful, the [ReadHandler](ReadHandler) will process the received measurement data
    pub async fn read(&mut self, request: ReadRequest) -> Result<(), TaskError> {
        let (promise, rx) = Promise::one_shot();
        let task = SingleReadTask::new(request, promise);
        self.send_task(task).await?;
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
        let (promise, rx) = Promise::one_shot();
        let task = EmptyResponseTask::new(function, headers, promise);
        self.send_task(task).await?;
        rx.await?
    }

    /// Perform an asynchronous READ request with a custom read handler
    ///
    /// If successful, the custom [ReadHandler](ReadHandler) will process the received measurement data
    pub async fn read_with_handler(
        &mut self,
        request: ReadRequest,
        handler: Box<dyn ReadHandler>,
    ) -> Result<(), TaskError> {
        let (promise, rx) = Promise::one_shot();
        let task = SingleReadTask::new_with_custom_handler(request, handler, promise);
        self.send_task(task).await?;
        rx.await?
    }

    /// Perform an asynchronous operate request
    ///
    /// The actual function code used depends on the value of the [CommandMode](CommandMode).
    pub async fn operate(
        &mut self,
        mode: CommandMode,
        headers: CommandHeaders,
    ) -> Result<(), CommandError> {
        let (promise, rx) = Promise::one_shot();
        let task = CommandTask::from_mode(mode, headers, promise);
        self.send_task(task).await?;
        rx.await?
    }

    /// Perform a WARM_RESTART operation
    ///
    /// Returns the delay from the outstation's response as a [Duration](Duration)
    pub async fn warm_restart(&mut self) -> Result<Duration, TaskError> {
        self.restart(RestartType::WarmRestart).await
    }

    /// Perform a COLD_RESTART operation
    ///
    /// Returns the delay from the outstation's response as a [Duration](Duration)
    pub async fn cold_restart(&mut self) -> Result<Duration, TaskError> {
        self.restart(RestartType::ColdRestart).await
    }

    async fn restart(&mut self, restart_type: RestartType) -> Result<Duration, TaskError> {
        let (promise, rx) = Promise::one_shot();
        let task = RestartTask::new(restart_type, promise);
        self.send_task(task).await?;
        rx.await?
    }

    /// Perform the specified time synchronization operation
    pub async fn synchronize_time(
        &mut self,
        procedure: TimeSyncProcedure,
    ) -> Result<(), TimeSyncError> {
        let (promise, rx) = Promise::one_shot();
        let task = TimeSyncTask::get_procedure(procedure, Some(promise));
        self.send_task(task).await?;
        rx.await?
    }

    /// Perform write one or more headers of analog input dead-bands to the outstation
    pub async fn write_dead_bands(
        &mut self,
        headers: Vec<DeadBandHeader>,
    ) -> Result<(), WriteError> {
        let (promise, rx) = Promise::one_shot();
        let task = WriteDeadBandsTask::new(headers, promise);
        self.send_task(task).await?;
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
        let (promise, rx) = Promise::one_shot();
        self.send_task(Task::LinkStatus(promise)).await?;
        rx.await?
    }

    /// Obtain an [`AuthKey`] from the outstation which may then be used to open the file.
    pub async fn get_file_auth_key(
        &mut self,
        credentials: FileCredentials,
    ) -> Result<AuthKey, FileError> {
        let (promise, rx) = Promise::one_shot();

        let task = AuthFileTask {
            credentials,
            promise,
        };

        self.send_task(task).await?;
        rx.await?
    }

    /// Open a file on the outstation with the requested parameters
    pub async fn open_file<T: ToString>(
        &mut self,
        file_name: T,
        auth_key: AuthKey,
        permissions: Permissions,
        file_size: u32,
        file_mode: FileMode,
        max_block_size: u16,
    ) -> Result<OpenFile, FileError> {
        let (promise, rx) = Promise::one_shot();

        let task = OpenFileTask {
            request: OpenFileRequest {
                file_name: file_name.to_string(),
                auth_key,
                file_size,
                file_mode,
                permissions,
                max_block_size,
            },
            promise,
        };

        self.send_task(task).await?;
        rx.await?
    }

    /// Write a file block to the outstation
    pub async fn write_file_block(
        &mut self,
        handle: FileHandle,
        block_number: BlockNumber,
        block_data: Vec<u8>,
    ) -> Result<(), FileError> {
        let (promise, rx) = Promise::one_shot();

        let task = WriteBlockTask {
            request: WriteBlockRequest {
                handle,
                block_number,
                block_data,
            },
            promise,
        };

        self.send_task(task).await?;
        rx.await?
    }

    /// Close a file on the outstation
    pub async fn close_file(&mut self, handle: FileHandle) -> Result<(), FileError> {
        let (promise, rx) = Promise::one_shot();

        let task = CloseFileTask { handle, promise };

        self.send_task(task).await?;
        rx.await?
    }

    /// Start an operation to READ a file from the outstation using a [`FileReader`] to receive data
    pub async fn read_file<T: ToString>(
        &mut self,
        remote_file_path: T,
        config: FileReadConfig,
        reader: Box<dyn FileReader>,
        credentials: Option<FileCredentials>,
    ) -> Result<(), Shutdown> {
        let task = FileReadTask::start(
            remote_file_path.to_string(),
            config,
            FileReaderType::from_reader(reader),
            credentials,
        );
        self.send_task(task).await
    }

    /// Read a file directory
    pub async fn read_directory<T: ToString>(
        &mut self,
        dir_path: T,
        config: DirReadConfig,
        credentials: Option<FileCredentials>,
    ) -> Result<Vec<FileInfo>, FileError> {
        let (promise, rx) = Promise::one_shot();
        let reader = Box::new(DirectoryReader::new(promise));
        self.read_file(dir_path, config.into(), reader, credentials)
            .await?;
        rx.await?
    }

    /// Get information about a file
    pub async fn get_file_info<T: ToString>(
        &mut self,
        file_path: T,
    ) -> Result<FileInfo, FileError> {
        let (promise, reply) = Promise::one_shot();
        let task = GetFileInfoTask::new(file_path.to_string(), promise);
        self.send_task(task).await?;
        reply.await?
    }

    async fn send_task<T: Into<Task>>(&mut self, task: T) -> Result<(), Shutdown> {
        self.master
            .send_association_message(self.address, AssociationMsgType::QueueTask(task.into()))
            .await
    }

    pub(crate) async fn send_poll_message(&mut self, msg: PollMsg) -> Result<(), Shutdown> {
        self.master
            .send_association_message(self.address, AssociationMsgType::Poll(msg))
            .await
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
    /// Send username/password and get back an auth key
    FileAuth,
    /// Open a file on the outstation
    FileOpen,
    /// Write a file block
    FileWriteBlock,
    /// Close a file on the outstation
    FileClose,
    /// Get information about a file
    GetFileInfo,
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
