use crate::app::format::write::HeaderWriter;
use crate::app::gen::enums::FunctionCode;
use crate::app::header::ResponseHeader;
use crate::app::parse::parser::HeaderCollection;
use crate::master::handlers::RequestCompletionHandler;
use crate::master::request::{RequestDetails, RequestStatus};
use crate::master::runner::RequestError;
use crate::master::session::Session;
use crate::master::types::AutoRequest;
use crate::util::cursor::WriteError;

pub(crate) struct AutoRequestDetails {
    pub(crate) request: AutoRequest,
    pub(crate) handler: Box<dyn RequestCompletionHandler>,
}

impl AutoRequestDetails {
    pub(crate) fn new(
        request: AutoRequest,
        handler: Box<dyn RequestCompletionHandler>,
    ) -> RequestDetails {
        RequestDetails::Auto(Self { request, handler })
    }

    pub(crate) fn format(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        self.request.format(writer)
    }

    pub(crate) fn function(&self) -> FunctionCode {
        self.request.function()
    }

    pub(crate) fn handle(
        &mut self,
        session: &mut Session,
        response: ResponseHeader,
        headers: HeaderCollection,
    ) -> RequestStatus {
        if !headers.is_empty() {
            log::warn!("ignoring non-empty reply to {}", self.request.description());
        }

        match self.request {
            AutoRequest::DisableUnsolicited(_) => {}
            AutoRequest::EnableUnsolicited(_) => {}
            AutoRequest::ClearRestartBit => self.handle_clear_restart_response(session, &response),
        }

        RequestStatus::Complete
    }

    pub(crate) fn handle_clear_restart_response(
        &mut self,
        session: &mut Session,
        response: &ResponseHeader,
    ) {
        if response.iin.iin1.get_device_restart() {
            // prevents all future retries b/c the device doesn't handle this properly
            session.on_clear_restart_iin_failed()
        } else {
            session.on_clear_restart_iin_success()
        }
    }

    pub(crate) fn on_complete(&mut self, result: Result<(), RequestError>) {
        self.handler.on_complete(result)
    }
}
