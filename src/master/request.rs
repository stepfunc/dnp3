use crate::app::format::write::start_request;
use crate::app::gen::enums::FunctionCode;
use crate::app::header::{Control, ResponseHeader};
use crate::app::parse::parser::HeaderCollection;
use crate::app::sequence::Sequence;
use crate::master::handlers::{ReadTaskHandler, RequestCompletionHandler};
use crate::master::requests::basic::BasicRequestImpl;
use crate::master::requests::command::CommandRequestImpl;
use crate::master::requests::read::ReadRequestImpl;
use crate::master::runner::RequestError;
use crate::master::types::{
    BasicRequest, CommandHeader, CommandTaskHandler, EventClasses, ReadRequest,
};
use crate::util::cursor::{WriteCursor, WriteError};

#[derive(Copy, Clone, Debug)]
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
    Read(ReadRequestImpl),
    Command(CommandRequestImpl),
    EmptyResponse(BasicRequestImpl),
}

impl RequestDetails {
    pub(crate) fn function(&self) -> FunctionCode {
        match self {
            RequestDetails::Read(_) => FunctionCode::Read,
            RequestDetails::Command(x) => x.function(),
            RequestDetails::EmptyResponse(x) => x.function(),
        }
    }

    pub(crate) fn format(&self, seq: Sequence, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        let mut writer = start_request(Control::request(seq), self.function(), cursor)?;
        match self {
            RequestDetails::Read(task) => task.format(&mut writer),
            RequestDetails::Command(task) => task.format(&mut writer),
            RequestDetails::EmptyResponse(task) => task.format(&mut writer),
        }
    }

    pub(crate) fn handle(
        &mut self,
        source: u16,
        response: ResponseHeader,
        headers: HeaderCollection,
    ) -> RequestStatus {
        match self {
            RequestDetails::Read(task) => task.handle(source, response, headers),
            RequestDetails::Command(task) => task.handle(source, response, headers),
            RequestDetails::EmptyResponse(task) => task.handle(source, response, headers),
        }
    }

    pub(crate) fn on_complete(&mut self, result: Result<(), RequestError>) {
        match self {
            RequestDetails::Read(task) => task.on_complete(result),
            RequestDetails::Command(task) => task.on_complete(result),
            RequestDetails::EmptyResponse(task) => task.on_complete(result),
        }
    }
}

pub struct MasterRequest {
    pub(crate) destination: u16,
    pub(crate) details: RequestDetails,
}

impl MasterRequest {
    pub fn read(destination: u16, request: ReadRequest, handler: Box<dyn ReadTaskHandler>) -> Self {
        Self {
            destination,
            details: RequestDetails::Read(ReadRequestImpl { request, handler }),
        }
    }

    pub fn disable_unsolicited(
        destination: u16,
        classes: EventClasses,
        handler: Box<dyn RequestCompletionHandler>,
    ) -> Self {
        Self {
            destination,
            details: RequestDetails::EmptyResponse(BasicRequestImpl {
                request: BasicRequest::DisableUnsolicited(classes),
                handler,
            }),
        }
    }

    pub fn enable_unsolicited(
        destination: u16,
        classes: EventClasses,
        handler: Box<dyn RequestCompletionHandler>,
    ) -> Self {
        Self {
            destination,
            details: RequestDetails::EmptyResponse(BasicRequestImpl {
                request: BasicRequest::EnableUnsolicited(classes),
                handler,
            }),
        }
    }

    pub fn select_before_operate(
        destination: u16,
        headers: Vec<CommandHeader>,
        handler: Box<dyn CommandTaskHandler>,
    ) -> Self {
        Self {
            destination,
            details: RequestDetails::Command(CommandRequestImpl::select_before_operate(
                headers, handler,
            )),
        }
    }

    pub fn direct_operate(
        destination: u16,
        headers: Vec<CommandHeader>,
        handler: Box<dyn CommandTaskHandler>,
    ) -> Self {
        Self {
            destination,
            details: RequestDetails::Command(CommandRequestImpl::direct_operate(headers, handler)),
        }
    }
}
