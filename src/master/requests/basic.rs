use crate::app::format::write::HeaderWriter;
use crate::app::gen::enums::FunctionCode;
use crate::app::header::ResponseHeader;
use crate::app::parse::parser::HeaderCollection;
use crate::master::handlers::RequestCompletionHandler;
use crate::master::request::RequestStatus;
use crate::master::runner::RequestError;
use crate::master::types::BasicRequest;
use crate::util::cursor::WriteError;

pub(crate) struct BasicRequestImpl {
    pub(crate) request: BasicRequest,
    pub(crate) handler: Box<dyn RequestCompletionHandler>,
}

impl BasicRequestImpl {
    pub(crate) fn format(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        self.request.format(writer)
    }

    pub(crate) fn function(&self) -> FunctionCode {
        self.request.function()
    }

    pub(crate) fn handle(
        &mut self,
        _source: u16,
        _response: ResponseHeader,
        _headers: HeaderCollection,
    ) -> RequestStatus {
        RequestStatus::Complete
    }

    pub(crate) fn on_complete(&mut self, result: Result<(), RequestError>) {
        self.handler.on_complete(result)
    }
}
