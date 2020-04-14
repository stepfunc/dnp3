use crate::app::format::write::HeaderWriter;
use crate::app::header::ResponseHeader;
use crate::app::parse::parser::HeaderCollection;
use crate::master::handlers::ReadTaskHandler;
use crate::master::runner::TaskError;
use crate::master::task::TaskStatus;
use crate::master::types::ReadRequest;
use crate::util::cursor::WriteError;

pub(crate) struct ReadTask {
    pub(crate) request: ReadRequest,
    pub(crate) handler: Box<dyn ReadTaskHandler>,
}

impl ReadTask {
    pub(crate) fn format(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        self.request.format(writer)
    }

    pub(crate) fn handle(
        &mut self,
        source: u16,
        response: ResponseHeader,
        headers: HeaderCollection,
    ) -> TaskStatus {
        self.handler.handle(source, response, headers);
        if response.control.fin {
            TaskStatus::Complete
        } else {
            TaskStatus::ReadNextResponse
        }
    }

    pub(crate) fn on_error(&mut self, error: TaskError) {
        self.handler.on_error(error)
    }
}
