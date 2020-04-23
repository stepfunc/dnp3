use crate::app::format::write::HeaderWriter;
use crate::app::header::ResponseHeader;
use crate::app::parse::parser::HeaderCollection;
use crate::master::handlers::ReadTaskHandler;
use crate::master::request::{RequestDetails, RequestStatus};
use crate::master::runner::TaskError;
use crate::master::types::ReadRequest;
use crate::util::cursor::WriteError;

pub(crate) struct ReadRequestDetails {
    request: ReadRequest,
    handler: Box<dyn ReadTaskHandler>,
}

impl ReadRequestDetails {
    pub(crate) fn create(
        request: ReadRequest,
        handler: Box<dyn ReadTaskHandler>,
    ) -> RequestDetails {
        RequestDetails::Read(Self { request, handler })
    }

    pub(crate) fn format(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        self.request.format(writer)
    }

    pub(crate) fn handle(
        &mut self,
        source: u16,
        response: ResponseHeader,
        headers: HeaderCollection,
    ) -> RequestStatus {
        self.handler.handle(source, response, headers);
        if response.control.fin {
            RequestStatus::Complete
        } else {
            RequestStatus::ReadNextResponse
        }
    }

    pub(crate) fn on_complete(&mut self, result: Result<(), TaskError>) {
        self.handler.on_complete(result)
    }
}
