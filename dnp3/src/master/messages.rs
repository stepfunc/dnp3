use crate::decode::DecodeLevel;
use crate::link::EndpointAddress;
use crate::master::association::Association;
use crate::master::error::PollError;
use crate::master::error::{AssociationError, TaskError};
use crate::master::handle::Promise;
use crate::master::poll::PollMsg;
use crate::master::tasks::Task;
use crate::util::task::Shutdown;

/// Messages sent from the handles to the master task via an mpsc.
pub(crate) enum Message {
    /// Message to send to the master
    Master(MasterMsg),
    /// Message to send to an association
    Association(AssociationMsg),
}

pub(crate) enum MasterMsg {
    /// enable or disable communication
    EnableCommunication(bool),
    /// Add an association to the master
    AddAssociation(Association, Promise<Result<(), AssociationError>>),
    /// Remove an association from the master
    RemoveAssociation(EndpointAddress),
    /// Set the decoding level
    SetDecodeLevel(DecodeLevel),
    /// Get the decoding level
    GetDecodeLevel(Promise<Result<DecodeLevel, Shutdown>>),
}

pub(crate) struct AssociationMsg {
    pub(crate) address: EndpointAddress,
    pub(crate) details: AssociationMsgType,
}

pub(crate) enum AssociationMsgType {
    /// Queue an I/O task for execution later
    QueueTask(Task),
    /// Modify polls
    Poll(PollMsg),
}

impl AssociationMsg {
    pub(crate) fn on_association_failure(self) {
        self.details.on_association_failure(self.address);
    }
}

impl AssociationMsgType {
    pub(crate) fn on_association_failure(self, address: EndpointAddress) {
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
