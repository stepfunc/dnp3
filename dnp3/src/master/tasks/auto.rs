// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::format::write::HeaderWriter;
use crate::app::parse::parser::Response;
use crate::app::FunctionCode;
use crate::master::association::Association;
use crate::master::error::TaskErrorKind;
use crate::master::request::EventClasses;
use crate::master::tasks::{AppTask, NonReadTask, Task};
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

    pub(crate) fn function(&self) -> FunctionCode {
        match self {
            AutoTask::ClearRestartBit => FunctionCode::Write,
            AutoTask::EnableUnsolicited(_) => FunctionCode::EnableUnsolicited,
            AutoTask::DisableUnsolicited(_) => FunctionCode::DisableUnsolicited,
        }
    }

    pub(crate) fn task_type(&self) -> TaskType {
        match self {
            AutoTask::ClearRestartBit => TaskType::ClearRestartBit,
            AutoTask::EnableUnsolicited(_) => TaskType::EnableUnsolicited,
            AutoTask::DisableUnsolicited(_) => TaskType::DisableUnsolicited,
        }
    }

    pub(crate) fn write(&self, writer: &mut HeaderWriter) -> Result<(), TaskErrorKind> {
        match self {
            AutoTask::ClearRestartBit => writer.write_clear_restart()?,
            AutoTask::EnableUnsolicited(classes) => classes.write(writer)?,
            AutoTask::DisableUnsolicited(classes) => classes.write(writer)?,
        }
        Ok(())
    }

    /// Check the response, which completes the task
    fn check_response(&self, response: Response<'_>) -> Result<(), TaskErrorKind> {
        if !response.raw_objects.is_empty() {
            tracing::warn!("ignoring object headers in reply to {}", self.description());
        }

        if matches!(self, AutoTask::ClearRestartBit)
            && response.header.iin.iin1.get_device_restart()
        {
            tracing::warn!("device failed to clear restart IIN bit");
            return Err(TaskErrorKind::UnexpectedResponseHeaders);
        }

        Ok(())
    }
}

impl Association {
    /// Handle the response to an automatic task, which completes it
    pub(crate) fn respond_auto_task(
        &mut self,
        task: AutoTask,
        response: Response<'_>,
    ) -> Result<(), TaskErrorKind> {
        let result = task.check_response(response);
        self.complete_auto_task(&task, result);
        result
    }

    /// Update the schedule with the outcome of an automatic task
    pub(crate) fn complete_auto_task(
        &mut self,
        task: &AutoTask,
        result: Result<(), TaskErrorKind>,
    ) {
        match result {
            Ok(()) => self.on_auto_task_success(task),
            Err(TaskErrorKind::RejectedByIin2(iin)) => {
                tracing::warn!("Auto task '{}' rejected by outstation IIN.2: {}. Check your outstation configuration?", task.description(), iin.iin2);
                // a rejection still answers the task, unless the restart bit is still set
                if matches!(task, AutoTask::ClearRestartBit) && iin.iin1.get_device_restart() {
                    tracing::warn!("device failed to clear restart IIN bit");
                    self.on_auto_task_failure(task);
                } else {
                    self.on_auto_task_success(task);
                }
            }
            Err(_) => {
                tracing::warn!("failed to {}", task.description());
                self.on_auto_task_failure(task);
            }
        }
    }
}
