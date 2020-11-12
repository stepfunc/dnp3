use crate::app::parse::DecodeLogLevel;
use crate::master::association::Association;
use crate::master::error::AssociationError;
use crate::master::error::{PollError, TaskError};
use crate::master::handle::Promise;
use crate::master::poll::PollMsg;
use crate::master::tasks::Task;

/// Messages sent from the handles to the master task via an mpsc.
pub(crate) enum Message {
    /// Message to send to the master
    Master(MasterMsg),
    /// Message to send to an association
    Association(AssociationMsg),
}

pub(crate) enum MasterMsg {
    /// Add an association to the master
    AddAssociation(Association, Promise<Result<(), AssociationError>>),
    /// Remove an association from the master
    RemoveAssociation(u16),
    /// Set the decoding level
    SetDecodeLogLevel(DecodeLogLevel),
}

pub(crate) struct AssociationMsg {
    pub(crate) address: u16,
    pub(crate) details: AssociationMsgType,
}

pub(crate) enum AssociationMsgType {
    /// Queue an I/O task for execution later
    QueueTask(Task),
    /// Modify polls
    Poll(PollMsg),
}

impl Message {
    pub(crate) fn on_send_failure(self) {
        match self {
            Message::Master(msg) => {
                msg.on_send_failure();
            }
            Message::Association(msg) => {
                msg.on_send_failure();
            }
        }
    }
}

impl MasterMsg {
    pub(crate) fn on_send_failure(self) {
        match self {
            MasterMsg::AddAssociation(_, promise) => {
                promise.complete(Err(AssociationError::Shutdown));
            }
            MasterMsg::RemoveAssociation(_) => {}
            MasterMsg::SetDecodeLogLevel(_) => {}
        }
    }
}

impl AssociationMsg {
    pub(crate) fn on_send_failure(self) {
        self.details.on_send_failure();
    }

    pub(crate) fn on_association_failure(self) {
        self.details.on_association_failure(self.address);
    }
}

impl AssociationMsgType {
    pub(crate) fn on_send_failure(self) {
        match self {
            AssociationMsgType::QueueTask(task) => {
                task.on_task_error(None, TaskError::Shutdown);
            }
            AssociationMsgType::Poll(msg) => {
                msg.on_error(PollError::Shutdown);
            }
        }
    }

    pub(crate) fn on_association_failure(self, address: u16) {
        match self {
            AssociationMsgType::QueueTask(task) => {
                task.on_task_error(None, TaskError::NoSuchAssociation(address));
            }
            AssociationMsgType::Poll(msg) => {
                msg.on_error(PollError::NoSuchAssociation(address));
            }
        }
    }
}
