use crate::app::header::ResponseHeader;
use crate::app::measurement::*;
use crate::app::parse::bytes::Bytes;
use crate::app::parse::parser::HeaderCollection;
use crate::master::association::Association;
use crate::master::error::{AssociationError, CommandError, TaskError};
use crate::master::types::{CommandHeaders, CommandMode};

/// messages sent from the handles to the master task via an mpsc
pub(crate) enum Message {
    Command(u16, CommandMode, CommandHeaders, CommandCallback),
    AddAssociation(Association, CallbackOnce<Result<(), AssociationError>>),
    RemoveAssociation(u16),
}

impl Message {
    pub(crate) fn on_send_failure(self) {
        match self {
            Message::Command(_, _, _, callback) => {
                callback.complete(Err(TaskError::Shutdown.into()))
            }
            Message::AddAssociation(_, callback) => {
                callback.complete(Err(AssociationError::Shutdown))
            }
            Message::RemoveAssociation(_) => {}
        }
    }
}

#[derive(Clone)]
pub struct MasterHandle {
    sender: tokio::sync::mpsc::Sender<Message>,
}

#[derive(Debug)]
pub struct AssociationHandle {
    address: u16,
    sender: tokio::sync::mpsc::Sender<Message>,
}

impl MasterHandle {
    pub(crate) fn new(sender: tokio::sync::mpsc::Sender<Message>) -> Self {
        Self { sender }
    }

    pub async fn add_association(
        &mut self,
        association: Association,
    ) -> Result<AssociationHandle, AssociationError> {
        let address = association.get_address();
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), AssociationError>>();
        if self
            .sender
            .send(Message::AddAssociation(
                association,
                CallbackOnce::OneShot(tx),
            ))
            .await
            .is_err()
        {
            return Err(AssociationError::Shutdown);
        }
        rx.await?
            .map(|_| (AssociationHandle::new(address, self.sender.clone())))
    }

    pub async fn remove_association(
        &mut self,
        handle: AssociationHandle,
    ) -> Result<(), AssociationError> {
        if self
            .sender
            .send(Message::RemoveAssociation(handle.address))
            .await
            .is_err()
        {
            return Err(AssociationError::Shutdown);
        }
        Ok(())
    }
}

impl AssociationHandle {
    pub(crate) fn new(address: u16, sender: tokio::sync::mpsc::Sender<Message>) -> Self {
        Self { address, sender }
    }

    pub fn address(&self) -> u16 {
        self.address
    }

    pub async fn operate(&mut self, mode: CommandMode, headers: CommandHeaders) -> CommandResult {
        let (tx, rx) = tokio::sync::oneshot::channel::<CommandResult>();
        self.send_operate_message(mode, headers, CallbackOnce::OneShot(tx))
            .await;
        rx.await?
    }

    pub async fn operate_cb<F>(&mut self, mode: CommandMode, headers: CommandHeaders, callback: F)
    where
        F: FnOnce(CommandResult) -> () + Send + Sync + 'static,
    {
        self.send_operate_message(mode, headers, CallbackOnce::BoxedFn(Box::new(callback)))
            .await;
    }

    async fn send_operate_message(
        &mut self,
        mode: CommandMode,
        headers: CommandHeaders,
        callback: CommandCallback,
    ) {
        if let Err(tokio::sync::mpsc::error::SendError(msg)) = self
            .sender
            .send(Message::Command(self.address, mode, headers, callback))
            .await
        {
            msg.on_send_failure();
        }
    }
}

pub trait ResponseHandler: Send {
    fn handle(&mut self, source: u16, header: ResponseHeader, headers: HeaderCollection);
}

/// A generic callback type that can only be invoked once.
/// The user can select to implement it using FnOnce or a
/// one-shot reply channel
pub enum CallbackOnce<T> {
    BoxedFn(Box<dyn FnOnce(T) -> () + Send + Sync>),
    OneShot(tokio::sync::oneshot::Sender<T>),
}

impl<T> CallbackOnce<T> {
    pub(crate) fn complete(self, value: T) {
        match self {
            CallbackOnce::BoxedFn(func) => func(value),
            CallbackOnce::OneShot(s) => {
                s.send(value).ok();
            }
        }
    }
}

pub type CommandResult = Result<(), CommandError>;
pub type CommandCallback = CallbackOnce<CommandResult>;

pub trait AssociationHandler: ResponseHandler {
    // TODO - add additional methods
}

pub trait MeasurementHandler {
    fn handle_binary(&mut self, x: impl Iterator<Item = (Binary, u16)>);
    fn handle_double_bit_binary(&mut self, x: impl Iterator<Item = (DoubleBitBinary, u16)>);
    fn handle_binary_output_status(&mut self, x: impl Iterator<Item = (BinaryOutputStatus, u16)>);
    fn handle_counter(&mut self, x: impl Iterator<Item = (Counter, u16)>);
    fn handle_frozen_counter(&mut self, x: impl Iterator<Item = (FrozenCounter, u16)>);
    fn handle_analog(&mut self, x: impl Iterator<Item = (Analog, u16)>);
    fn handle_analog_output_status(&mut self, x: impl Iterator<Item = (AnalogOutputStatus, u16)>);
    fn handle_octet_string<'a>(&mut self, x: impl Iterator<Item = (Bytes<'a>, u16)>);
}
