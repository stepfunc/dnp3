use crate::app::header::ResponseHeader;
use crate::app::parse::parser::HeaderCollection;
use crate::app::sequence::Sequence;
use crate::master::handlers::ResponseHandler;
use crate::master::tasks::class_scan::ClassScanTask;
use crate::master::tasks::command::CommandTask;
use crate::master::types::{ClassScan, CommandRequest};
use crate::util::cursor::{WriteCursor, WriteError};

#[derive(Copy, Clone, Debug)]
pub enum ResponseError {
    Todo,
}

pub(crate) enum ResponseResult {
    /// the response completed the task
    Success,
    ///// run a new task - e.g. select then operate
    //Transition(MasterTask),
}

pub(crate) enum TaskDetails {
    ClassScan(ClassScanTask),
    Command(CommandTask),
}

impl TaskDetails {
    pub(crate) fn is_read_request(&self) -> bool {
        match self {
            TaskDetails::ClassScan(_) => true,
            TaskDetails::Command(_) => false,
        }
    }

    pub(crate) fn format(&self, seq: Sequence, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        match self {
            TaskDetails::ClassScan(task) => task.format(seq, cursor),
            TaskDetails::Command(task) => task.format(seq, cursor),
        }
    }

    pub(crate) fn handle(
        &mut self,
        response: ResponseHeader,
        headers: HeaderCollection,
    ) -> Result<ResponseResult, ResponseError> {
        match self {
            TaskDetails::ClassScan(task) => task.handle(response, headers),
            TaskDetails::Command(task) => task.handle(response, headers),
        }
    }
}

pub struct MasterTask {
    pub(crate) destination: u16,
    pub(crate) details: TaskDetails,
}

impl MasterTask {
    pub fn class_scan(
        destination: u16,
        scan: ClassScan,
        handler: Box<dyn ResponseHandler>,
    ) -> Self {
        Self {
            destination,
            details: TaskDetails::ClassScan(ClassScanTask { scan, handler }),
        }
    }

    pub fn command(destination: u16, request: CommandRequest) -> Self {
        Self {
            destination,
            details: TaskDetails::Command(CommandTask { request }),
        }
    }
}
