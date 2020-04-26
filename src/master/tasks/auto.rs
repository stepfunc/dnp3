use crate::app::format::write::HeaderWriter;
use crate::app::gen::enums::FunctionCode;
use crate::app::header::ResponseHeader;
use crate::app::parse::parser::HeaderCollection;
use crate::master::association::Association;
use crate::master::task::{NonReadTask, NonReadTaskStatus, TaskType};
use crate::master::types::AutoRequest;
use crate::util::cursor::WriteError;

pub(crate) struct AutoTask {
    pub(crate) request: AutoRequest,
}

impl AutoTask {
    pub(crate) fn create(request: AutoRequest) -> TaskType {
        TaskType::NonRead(NonReadTask::Auto(Self { request }))
    }

    pub(crate) fn format(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        self.request.format(writer)
    }

    pub(crate) fn function(&self) -> FunctionCode {
        self.request.function()
    }

    pub(crate) fn handle(
        self,
        association: &mut Association,
        header: ResponseHeader,
        objects: HeaderCollection,
    ) -> NonReadTaskStatus {
        if !objects.is_empty() {
            log::warn!(
                "ignoring objects headers in reply to {}",
                self.request.description()
            );
        }

        match &self.request {
            AutoRequest::DisableUnsolicited(_) => {
                association.on_disable_unsolicited_response(header.iin);
                NonReadTaskStatus::Complete
            }
            AutoRequest::EnableUnsolicited(_) => {
                association.on_enable_unsolicited_response(header.iin);
                NonReadTaskStatus::Complete
            }
            AutoRequest::ClearRestartBit => {
                association.on_clear_restart_iin_response(header.iin);
                NonReadTaskStatus::Complete
            }
        }
    }
}
