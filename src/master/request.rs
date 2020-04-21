use crate::app::format::write::start_request;
use crate::app::gen::enums::FunctionCode;
use crate::app::header::{Control, ResponseHeader};
use crate::app::parse::parser::HeaderCollection;
use crate::app::sequence::Sequence;
use crate::master::requests::auto::AutoRequestDetails;
use crate::master::requests::command::CommandRequestDetails;
use crate::master::requests::read::ReadRequestDetails;
use crate::master::runner::RequestError;
use crate::master::session::SessionHandle;
use crate::util::cursor::{WriteCursor, WriteError};

#[derive(Copy, Clone)]
pub(crate) enum RequestStatus {
    /// go through the whole cycle of formatting and waiting for a reply again
    ExecuteNextStep,
    /// The response was not for the task, so keep waiting on the current timeout
    ContinueWaiting,
    /// read another response with a new timeout
    ReadNextResponse,
    /// The task is complete
    Complete,
}

pub(crate) enum RequestDetails {
    Read(ReadRequestDetails),
    Command(CommandRequestDetails),
    Auto(AutoRequestDetails),
}

impl RequestDetails {
    pub(crate) fn function(&self) -> FunctionCode {
        match self {
            RequestDetails::Read(_) => FunctionCode::Read,
            RequestDetails::Command(x) => x.function(),
            RequestDetails::Auto(x) => x.function(),
        }
    }

    pub(crate) fn format(&self, seq: Sequence, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        let mut writer = start_request(Control::request(seq), self.function(), cursor)?;
        match self {
            RequestDetails::Read(task) => task.format(&mut writer),
            RequestDetails::Command(task) => task.format(&mut writer),
            RequestDetails::Auto(task) => task.format(&mut writer),
        }
    }

    pub(crate) fn handle(
        &mut self,
        session: &SessionHandle,
        source: u16,
        response: ResponseHeader,
        headers: HeaderCollection,
    ) -> RequestStatus {
        match self {
            RequestDetails::Read(task) => task.handle(source, response, headers),
            RequestDetails::Command(task) => task.handle(headers),
            RequestDetails::Auto(task) => task.handle(session, response, headers),
        }
    }

    pub(crate) fn on_complete(&mut self, result: Result<(), RequestError>) {
        match self {
            RequestDetails::Auto(_) => {}
            RequestDetails::Read(task) => task.on_complete(result),
            RequestDetails::Command(task) => task.on_complete(result),
        }
    }
}

pub struct MasterRequest {
    pub(crate) session: SessionHandle,
    pub(crate) details: RequestDetails,
}

impl MasterRequest {
    pub(crate) fn new(session: SessionHandle, details: RequestDetails) -> Self {
        Self { session, details }
    }
}
