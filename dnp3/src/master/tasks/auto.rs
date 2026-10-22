use crate::app::format::write::HeaderWriter;
use crate::app::parse::parser::Response;
use crate::app::{FunctionCode, Iin};
use crate::master::association::Association;
use crate::master::error::TaskError;
use crate::master::request::EventClasses;
use crate::master::tasks::{AppTask, NonReadTask, Task};

#[derive(Clone)]
pub(crate) enum AutoTask {
    ClearRestartBit,
    EnableUnsolicited(EventClasses),
    DisableUnsolicited(EventClasses),
}

impl AutoTask {
    pub(crate) fn wrap(self) -> Task {
        Task::App(AppTask::NonRead(NonReadTask::Auto(self)))
    }

    pub(crate) fn write(&self, writer: &mut HeaderWriter) -> Result<(), scursor::WriteError> {
        match self {
            AutoTask::ClearRestartBit => writer.write_clear_restart(),
            AutoTask::EnableUnsolicited(classes) => classes.write(writer),
            AutoTask::DisableUnsolicited(classes) => classes.write(writer),
        }
    }

    pub(crate) fn function(&self) -> FunctionCode {
        match self {
            AutoTask::ClearRestartBit => FunctionCode::Write,
            AutoTask::EnableUnsolicited(_) => FunctionCode::EnableUnsolicited,
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
        response: Response<'_>,
    ) -> Result<Option<NonReadTask>, TaskError> {
        if !response.raw_objects.is_empty() {
            tracing::warn!("ignoring object headers in reply to {}", self.description());
        }

        self.on_response(association, response.header.iin);

        Ok(None)
    }

    pub(crate) fn on_task_error(self, association: Option<&mut Association>, err: TaskError) {
        let association = match association {
            None => return,
            Some(x) => x,
        };

        if let TaskError::RejectedByIin2(iin) = err {
            tracing::warn!("Auto task '{}' rejected by outstation IIN.2: {}. Check your outstation configuration?", self.description(), iin.iin2);
            self.on_response(association, iin);
            return;
        }

        tracing::warn!("failed to {}", self.description());
        association.on_auto_task_failure(&self);
    }

    /// The outstation answered, so the task is done, unless it didn't clear its restart bit
    fn on_response(&self, association: &mut Association, iin: Iin) {
        if matches!(self, AutoTask::ClearRestartBit) && iin.iin1.get_device_restart() {
            tracing::warn!("device failed to clear restart IIN bit");
            association.on_auto_task_failure(self);
        } else {
            association.on_auto_task_success(self);
        }
    }
}
