use crate::app::gen::enums::QualifierCode;
use crate::app::gen::variations::variation::Variation;
use crate::app::header::ResponseHeader;
use crate::app::measurement::*;
use crate::app::parse::bytes::Bytes;
use crate::app::parse::DecodeLogLevel;
use crate::master::association::Association;
use crate::master::error::{AssociationError, CommandError, TaskError, TimeSyncError};
use crate::master::request::{CommandHeaders, CommandMode, TimeSyncProcedure};
use crate::master::task::{Task, TaskType};
use crate::master::tasks::command::CommandTask;
use crate::master::tasks::time::TimeSyncTask;
use std::time::SystemTime;

// messages sent from the handles to the master task via an mpsc
pub(crate) enum Message {
    QueueTask(Task),
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

    pub async fn add_association(
        &mut self,
        association: Association,
    ) -> Result<AssociationHandle, AssociationError> {
        let address = association.get_address();
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

    pub async fn remove(mut self) {
        self.sender
            .send(Message::RemoveAssociation(self.address))
            .await
            .ok();
    }

    pub async fn operate(&mut self, mode: CommandMode, headers: CommandHeaders) -> CommandResult {
        let (tx, rx) = tokio::sync::oneshot::channel::<CommandResult>();
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
        F: FnOnce(CommandResult) -> () + Send + Sync + 'static,
    {
        let task = CommandTask::from_mode(mode, headers, Promise::BoxedFn(Box::new(callback)));
        self.inner.send_task(task.wrap().wrap()).await;
    }

    pub async fn perform_time_sync<F>(&mut self, procedure: TimeSyncProcedure, callback: F)
    where
        F: FnOnce(Result<(), TimeSyncError>) -> () + Send + Sync + 'static,
    {
        let task =
            TimeSyncTask::get_procedure(procedure, false, Promise::BoxedFn(Box::new(callback)));
        self.inner.send_task(task.wrap().wrap()).await;
    }
}

/// A generic callback type that must be invoked once and only once.
/// The user can select to implement it using FnOnce or a
/// one-shot reply channel
pub enum Promise<T> {
    Empty,
    BoxedFn(Box<dyn FnOnce(T) -> () + Send + Sync>),
    OneShot(tokio::sync::oneshot::Sender<T>),
}

impl<T> Promise<T> {
    pub(crate) fn complete(self, value: T) {
        match self {
            Promise::Empty => {}
            Promise::BoxedFn(func) => func(value),
            Promise::OneShot(s) => {
                s.send(value).ok();
            }
        }
    }
}

pub type CommandResult = Result<(), CommandError>;

pub trait AssociationHandler: Send {
    fn get_read_handler(&mut self) -> &mut dyn ReadHandler;
    fn get_system_time(&self) -> SystemTime {
        SystemTime::now()
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
        iter: &mut dyn Iterator<Item = (Bytes<'a>, u16)>,
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
    fn get_read_handler(&mut self) -> &mut dyn ReadHandler {
        self
    }
}
