// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::format::write::HeaderWriter;
use crate::app::parse::parser::Response;
use crate::app::{FunctionCode, Iin};
use crate::master::association::Association;
use crate::master::error::TaskError;
use crate::master::request::EventClasses;
use crate::master::tasks::{AppTask, NonReadTask, ResponseTask, Step, Task};
use crate::master::TaskType;

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

    pub(crate) fn description(&self) -> &'static str {
        match self {
            AutoTask::ClearRestartBit => "clear restart IIN bit",
            AutoTask::EnableUnsolicited(_) => "enable unsolicited reporting",
            AutoTask::DisableUnsolicited(_) => "disable unsolicited reporting",
        }
    }

    /// The outstation answered, so the task is done, unless it didn't clear its restart bit
    fn answered(&self, association: &mut Association, iin: Iin) {
        if matches!(self, AutoTask::ClearRestartBit) && iin.iin1.get_device_restart() {
            tracing::warn!("device failed to clear restart IIN bit");
            association.on_auto_task_failure(self);
        } else {
            association.on_auto_task_success(self);
        }
    }
}

impl ResponseTask for AutoTask {
    /// the IIN of the answer
    type Output = Iin;
    type Error = TaskError;

    fn function(&self) -> FunctionCode {
        match self {
            AutoTask::ClearRestartBit => FunctionCode::Write,
            AutoTask::EnableUnsolicited(_) => FunctionCode::EnableUnsolicited,
            AutoTask::DisableUnsolicited(_) => FunctionCode::DisableUnsolicited,
        }
    }

    fn task_type(&self) -> TaskType {
        match self {
            AutoTask::ClearRestartBit => TaskType::ClearRestartBit,
            AutoTask::EnableUnsolicited(_) => TaskType::EnableUnsolicited,
            AutoTask::DisableUnsolicited(_) => TaskType::DisableUnsolicited,
        }
    }

    fn write(&self, writer: &mut HeaderWriter) -> Result<(), TaskError> {
        match self {
            AutoTask::ClearRestartBit => writer.write_clear_restart()?,
            AutoTask::EnableUnsolicited(classes) => classes.write(writer)?,
            AutoTask::DisableUnsolicited(classes) => classes.write(writer)?,
        }
        Ok(())
    }

    async fn on_response(
        &mut self,
        _association: &mut Association,
        response: Response<'_>,
    ) -> Result<Step<Iin>, TaskError> {
        if !response.raw_objects.is_empty() {
            tracing::warn!("ignoring object headers in reply to {}", self.description());
        }

        if matches!(self, AutoTask::ClearRestartBit)
            && response.header.iin.iin1.get_device_restart()
        {
            tracing::warn!("device failed to clear restart IIN bit");
            return Err(TaskError::UnexpectedResponseHeaders);
        }

        Ok(Step::Done(response.header.iin))
    }

    fn complete(self, association: Option<&mut Association>, result: Result<Iin, TaskError>) {
        let association = match association {
            None => return,
            Some(x) => x,
        };

        match result {
            Ok(iin) => self.answered(association, iin),
            Err(TaskError::RejectedByIin2(iin)) => {
                tracing::warn!("Auto task '{}' rejected by outstation IIN.2: {}. Check your outstation configuration?", self.description(), iin.iin2);
                self.answered(association, iin);
            }
            Err(_) => {
                tracing::warn!("failed to {}", self.description());
                association.on_auto_task_failure(&self);
            }
        }
    }
}
