use crate::app::format::write::HeaderWriter;
use crate::app::gen::enums::FunctionCode;
use crate::app::header::ResponseHeader;
use crate::app::parse::parser::HeaderCollection;
use crate::master::handlers::TaskCompletionHandler;
use crate::master::runner::TaskError;
use crate::master::task::TaskStatus;
use crate::master::types::BasicRequest;
use crate::util::cursor::WriteError;

pub(crate) struct BasicResponseTask {
    pub(crate) request: BasicRequest,
    pub(crate) handler: Box<dyn TaskCompletionHandler>,
}

impl BasicResponseTask {
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
    ) -> TaskStatus {
        TaskStatus::Complete
    }

    pub(crate) fn on_complete(&mut self, result: Result<(), TaskError>) {
        self.handler.on_complete(result)
    }
}
