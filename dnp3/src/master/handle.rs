use crate::app::enums::QualifierCode;
use crate::app::header::ResponseHeader;
use crate::app::measurement::*;
use crate::app::parse::bytes::Bytes;
use crate::app::parse::DecodeLogLevel;
use crate::app::variations::Variation;
use crate::master::association::{Association, Configuration};
use crate::master::error::{AssociationError, CommandError, PollError, TaskError, TimeSyncError};
use crate::master::poll::{PollHandle, PollTask, PollTaskMsg};
use crate::master::request::{CommandHeaders, CommandMode, ReadRequest, TimeSyncProcedure};
use crate::master::task::{Task, TaskType};
use crate::master::tasks::command::CommandTask;
use crate::master::tasks::read::SingleReadTask;
use crate::master::tasks::time::TimeSyncTask;
use std::time::{Duration, SystemTime};

// messages sent from the handles to the master task via an mpsc
pub(crate) enum Message {
    QueueTask(Task),
    PollTask(PollTaskMsg),
    AddAssociation(Association, Promise<Result<(), AssociationError>>),
    RemoveAssociation(u16),
    SetDecodeLogLevel(DecodeLogLevel),
}

impl Message {
    pub(crate) fn on_send_failure(self) {
        match self {
            Message::QueueTask(task) => {
                task.details.on_task_error(TaskError::Shutdown);
            }
            Message::PollTask(msg) => {
                msg.task.on_error(PollError::Shutdown);
            }
            Message::AddAssociation(_, promise) => {
                promise.complete(Err(AssociationError::Shutdown))
            }
            Message::RemoveAssociation(_) => {}
            Message::SetDecodeLogLevel(_) => {}
        }
    }
}

#[derive(Clone)]
pub struct MasterHandle {
    sender: tokio::sync::mpsc::Sender<Message>,
}

/// handle used to make requests against
#[derive(Debug)]
pub struct AssociationHandle {
    address: u16,
    sender: tokio::sync::mpsc::Sender<Message>,
}

impl MasterHandle {
    pub(crate) fn new(sender: tokio::sync::mpsc::Sender<Message>) -> Self {
        Self { sender }
    }

    pub async fn set_decode_log_level(&mut self, level: DecodeLogLevel) {
        self.sender
            .send(Message::SetDecodeLogLevel(level))
            .await
            .ok();
    }

    /// Create a new association:
    /// * `address` is the DNP3 link-layer address of the outstation
    /// * `config` controls the behavior of the master for this outstation
    /// * `handler` is a callback trait invoked when events occur for this outstation
    pub async fn add_association(
        &mut self,
        address: u16,
        config: Configuration,
        handler: Box<dyn AssociationHandler>,
    ) -> Result<AssociationHandle, AssociationError> {
        let association = Association::new(address, config, handler);
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), AssociationError>>();
        if self
            .sender
            .send(Message::AddAssociation(association, Promise::OneShot(tx)))
            .await
            .is_err()
        {
            return Err(AssociationError::Shutdown);
        }
        rx.await?
            .map(|_| (AssociationHandle::new(address, self.sender.clone())))
    }
}

impl AssociationHandle {
    pub(crate) fn new(address: u16, sender: tokio::sync::mpsc::Sender<Message>) -> Self {
        Self { address, sender }
    }

    pub fn address(&self) -> u16 {
        self.address
    }

    pub fn callbacks(self) -> CallbackAssociationHandle {
        CallbackAssociationHandle { inner: self }
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
        self.sender
            .send(
                PollTaskMsg::new(
                    self.address,
                    PollTask::AddPoll(request, period, self.sender.clone(), Promise::OneShot(tx)),
                )
                .into(),
            )
            .await
            .ok();
        rx.await?
    }

    pub async fn remove(mut self) {
        self.sender
            .send(Message::RemoveAssociation(self.address))
            .await
            .ok();
    }

    pub async fn read(&mut self, request: ReadRequest) -> Result<(), TaskError> {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), TaskError>>();
        let task = SingleReadTask::new(request, Promise::OneShot(tx));
        self.send_task(task.wrap().wrap()).await;
        rx.await?
    }

    pub async fn operate(
        &mut self,
        mode: CommandMode,
        headers: CommandHeaders,
    ) -> Result<(), CommandError> {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), CommandError>>();
        let task = CommandTask::from_mode(mode, headers, Promise::OneShot(tx));
        self.send_task(task.wrap().wrap()).await;
        rx.await?
    }

    pub async fn perform_time_sync(
        &mut self,
        procedure: TimeSyncProcedure,
    ) -> Result<(), TimeSyncError> {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), TimeSyncError>>();
        let task = TimeSyncTask::get_procedure(procedure, false, Promise::OneShot(tx));
        self.send_task(task.wrap().wrap()).await;
        rx.await?
    }

    async fn send_task(&mut self, task: TaskType) {
        if let Err(tokio::sync::mpsc::error::SendError(msg)) = self
            .sender
            .send(Message::QueueTask(Task::new(self.address, task)))
            .await
        {
            msg.on_send_failure();
        }
    }
}

pub struct CallbackAssociationHandle {
    inner: AssociationHandle,
}

impl CallbackAssociationHandle {
    pub fn address(&self) -> u16 {
        self.inner.address
    }

    pub async fn remove(self) {
        self.inner.remove().await
    }

    pub async fn operate<F>(&mut self, mode: CommandMode, headers: CommandHeaders, callback: F)
    where
        F: FnOnce(Result<(), CommandError>) -> () + Send + Sync + 'static,
    {
        let task = CommandTask::from_mode(mode, headers, Promise::BoxedFn(Box::new(callback)));
        self.inner.send_task(task.wrap().wrap()).await;
    }

    pub async fn perform_time_sync<F>(&mut self, procedure: TimeSyncProcedure, callback: F)
    where
        F: FnOnce(Result<(), TimeSyncError>) + Send + Sync + 'static,
    {
        let task =
            TimeSyncTask::get_procedure(procedure, false, Promise::BoxedFn(Box::new(callback)));
        self.inner.send_task(task.wrap().wrap()).await;
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
    Watch(tokio::sync::broadcast::Sender<T>),
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
    OneShot(tokio::sync::oneshot::Sender<T>),
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
    fn get_system_time(&self) -> SystemTime {
        SystemTime::now()
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
