// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::Shutdown;
use crate::decode::DecodeLevel;
use crate::link::EndpointAddress;
use crate::master::error::PollError;
use crate::master::error::{AssociationError, TaskError};
use crate::master::poll::PollMsg;
use crate::master::promise::Promise;
use crate::master::tasks::QueuedTask;
use crate::master::{AssociationConfig, AssociationHandler, AssociationInformation, ReadHandler};
use crate::transport::FragmentAddr;
use crate::util::lifecycle::Enabled;

/// Messages sent from the handles to the master task via an mpsc.
pub(crate) enum Message {
    /// Message to send to the master
    Master(MasterMsg),
    /// Message to send to an association
    Association(AssociationMsg),
}

pub(crate) enum MasterMsg {
    /// enable or disable communication
    EnableCommunication(Enabled),
    /// Add an association to the master
    AddAssociation(
        FragmentAddr,
        AssociationConfig,
        Box<dyn ReadHandler>,
        Box<dyn AssociationHandler>,
        Box<dyn AssociationInformation>,
        Promise<Result<(), AssociationError>>,
    ),
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
    QueueTask(QueuedTask),
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
