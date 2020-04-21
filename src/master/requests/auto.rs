use crate::app::format::write::HeaderWriter;
use crate::app::gen::enums::FunctionCode;
use crate::app::header::ResponseHeader;
use crate::app::parse::parser::HeaderCollection;
use crate::master::request::{RequestDetails, RequestStatus};
use crate::master::session::SessionHandle;
use crate::master::types::AutoRequest;
use crate::util::cursor::WriteError;

pub(crate) struct AutoRequestDetails {
    pub(crate) request: AutoRequest,
}

impl AutoRequestDetails {
    pub(crate) fn create(request: AutoRequest) -> RequestDetails {
        RequestDetails::Auto(Self { request })
    }

    pub(crate) fn format(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        self.request.format(writer)
    }

    pub(crate) fn function(&self) -> FunctionCode {
        self.request.function()
    }

    pub(crate) fn handle(
        &mut self,
        session: &SessionHandle,
        header: ResponseHeader,
        objects: HeaderCollection,
    ) -> RequestStatus {
        if self.request.expects_empty_response() && !objects.is_empty() {
            log::warn!(
                "ignoring objects headers reply to {}",
                self.request.description()
            );
        }

        match &mut self.request {
            AutoRequest::PeriodicPoll(poll) => {
                session.handle_response(header, objects);
                if header.control.fin {
                    poll.success();
                    RequestStatus::Complete
                } else {
                    RequestStatus::ReadNextResponse
                }
            }
            AutoRequest::IntegrityScan => {
                session.handle_response(header, objects);
                if header.control.fin {
                    session.on_integrity_scan_complete();
                    RequestStatus::Complete
                } else {
                    RequestStatus::ReadNextResponse
                }
            }
            AutoRequest::DisableUnsolicited(_) => {
                session.on_disable_unsolicited_response(header.iin);
                RequestStatus::Complete
            }
            AutoRequest::EnableUnsolicited(_) => {
                session.on_enable_unsolicited_response(header.iin);
                RequestStatus::Complete
            }
            AutoRequest::ClearRestartBit => {
                session.on_clear_restart_iin_response(header.iin);
                RequestStatus::Complete
            }
        }
    }
}
