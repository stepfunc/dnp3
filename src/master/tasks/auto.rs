use crate::app::format::write::HeaderWriter;
use crate::app::gen::enums::FunctionCode;
use crate::app::header::ResponseHeader;
use crate::app::parse::parser::HeaderCollection;
use crate::master::association::Association;
use crate::master::task::{NonReadTask, TaskStatus, TaskType};
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
        &mut self,
        session: &mut Association,
        header: ResponseHeader,
        objects: HeaderCollection,
    ) -> TaskStatus {
        if self.request.expects_empty_response() && !objects.is_empty() {
            log::warn!(
                "ignoring objects headers reply to {}",
                self.request.description()
            );
        }

        match &self.request {
            AutoRequest::PeriodicPoll(_, id) => {
                session.handle_response(header, objects);
                if header.control.fin {
                    session.complete_poll(*id);
                    TaskStatus::Complete
                } else {
                    TaskStatus::ReadNextResponse
                }
            }
            AutoRequest::IntegrityScan => {
                session.handle_response(header, objects);
                if header.control.fin {
                    session.on_integrity_scan_complete();
                    TaskStatus::Complete
                } else {
                    TaskStatus::ReadNextResponse
                }
            }
            AutoRequest::DisableUnsolicited(_) => {
                session.on_disable_unsolicited_response(header.iin);
                TaskStatus::Complete
            }
            AutoRequest::EnableUnsolicited(_) => {
                session.on_enable_unsolicited_response(header.iin);
                TaskStatus::Complete
            }
            AutoRequest::ClearRestartBit => {
                session.on_clear_restart_iin_response(header.iin);
                TaskStatus::Complete
            }
        }
    }
}
