// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use std::time::Duration;

use crate::app::format::write::HeaderWriter;
use crate::app::gen::count::CountVariation;
use crate::app::parse::parser::Response;
use crate::app::FunctionCode;
use crate::master::error::TaskError;
use crate::master::tasks::{NonReadTask, OneShot, OneShotRequest};
use crate::master::TaskType;

/// Type of restart to request
pub(crate) enum RestartType {
    /// Cold restart
    ///
    /// Forces the outstation to perform a complete restart similar to what the device
    /// would do upon powering up after a long-term power loss.
    ColdRestart,
    /// Warm restart
    ///
    /// Forces the outstation to perform a partial reset.
    WarmRestart,
}

impl OneShotRequest for RestartType {
    type Output = Duration;
    type Error = TaskError;

    fn function(&self) -> FunctionCode {
        match self {
            Self::ColdRestart => FunctionCode::ColdRestart,
            Self::WarmRestart => FunctionCode::WarmRestart,
        }
    }

    fn task_type(&self) -> TaskType {
        TaskType::Restart
    }

    fn write(&self, _writer: &mut HeaderWriter) -> Result<(), TaskError> {
        Ok(())
    }

    fn parse(&self, response: Response<'_>) -> Result<Duration, TaskError> {
        let header = response.get_only_object_header()?;

        let count = header.count().ok_or(TaskError::UnexpectedResponseHeaders)?;

        match count {
            CountVariation::Group52Var1(val) => match val.single() {
                Some(val) => Ok(Duration::from_secs(val.time.into())),
                None => Err(TaskError::UnexpectedResponseHeaders),
            },
            CountVariation::Group52Var2(val) => match val.single() {
                Some(val) => Ok(Duration::from_millis(val.time.into())),
                None => Err(TaskError::UnexpectedResponseHeaders),
            },
            _ => Err(TaskError::UnexpectedResponseHeaders),
        }
    }

    fn wrap(task: OneShot<Self>) -> NonReadTask {
        NonReadTask::Restart(task)
    }
}
