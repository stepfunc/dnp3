use crate::app::format::write::HeaderWriter;
use crate::app::gen::enums::FunctionCode;
use crate::app::header::ResponseHeader;
use crate::app::parse::parser::HeaderCollection;
use crate::master::association::Association;
use crate::master::task::{NonReadTask, NonReadTaskStatus, TaskType};
use crate::master::types::EventClasses;
use crate::util::cursor::WriteError;

#[derive(Clone)]
pub(crate) enum AutoTask {
    ClearRestartBit,
    EnableUnsolicited(EventClasses),
    DisableUnsolicited(EventClasses),
}

impl AutoTask {
    pub(crate) fn wrap(self) -> TaskType {
        TaskType::NonRead(NonReadTask::Auto(self))
    }

    pub(crate) fn format(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        match self {
            AutoTask::ClearRestartBit => writer.write_clear_restart(),
            AutoTask::EnableUnsolicited(classes) => classes.write(writer),
            AutoTask::DisableUnsolicited(classes) => classes.write(writer),
        }
    }

    pub(crate) fn function(&self) -> FunctionCode {
        match self {
            AutoTask::ClearRestartBit => FunctionCode::Write,
            AutoTask::EnableUnsolicited(_) => FunctionCode::EnabledUnsolicited,
            AutoTask::DisableUnsolicited(_) => FunctionCode::DisableUnsolicited,
        }
    }

    pub(crate) fn description(&self) -> &'static str {
        match self {
            AutoTask::ClearRestartBit => "clear restart IIN bit",
            AutoTask::EnableUnsolicited(_) => "enable unsolicited reporting",
            AutoTask::DisableUnsolicited(_) => "disable unsolicited reporting",
        }
    }

    pub(crate) fn handle(
        self,
        association: &mut Association,
        header: ResponseHeader,
        objects: HeaderCollection,
    ) -> NonReadTaskStatus {
        if !objects.is_empty() {
            log::warn!("ignoring object headers in reply to {}", self.description());
        }

        match &self {
            AutoTask::DisableUnsolicited(_) => {
                association.on_disable_unsolicited_response(header.iin);
                NonReadTaskStatus::Complete
            }
            AutoTask::EnableUnsolicited(_) => {
                association.on_enable_unsolicited_response(header.iin);
                NonReadTaskStatus::Complete
            }
            AutoTask::ClearRestartBit => {
                association.on_clear_restart_iin_response(header.iin);
                NonReadTaskStatus::Complete
            }
        }
    }
}
