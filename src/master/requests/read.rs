use crate::app::format::write::HeaderWriter;
use crate::app::header::ResponseHeader;
use crate::app::parse::parser::HeaderCollection;
use crate::master::handlers::ReadTaskHandler;
use crate::master::request::RequestStatus;
use crate::master::runner::RequestError;
use crate::master::types::ReadRequest;
use crate::util::cursor::WriteError;

pub(crate) struct ReadRequestImpl {
    pub(crate) request: ReadRequest,
    pub(crate) handler: Box<dyn ReadTaskHandler>,
}

impl ReadRequestImpl {
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

    pub(crate) fn on_complete(&mut self, result: Result<(), RequestError>) {
        self.handler.on_complete(result)
    }
}
