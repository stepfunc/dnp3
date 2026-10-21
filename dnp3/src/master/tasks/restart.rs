use std::time::Duration;

use crate::app::gen::count::CountVariation;
use crate::app::parse::parser::Response;
use crate::app::FunctionCode;
use crate::master::error::TaskError;
use crate::master::promise::Promise;
use crate::master::tasks::{AppTask, NonReadTask, Task};

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

pub(crate) struct RestartTask {
    restart_type: RestartType,
    promise: Promise<Result<Duration, TaskError>>,
}

impl From<RestartTask> for Task {
    fn from(value: RestartTask) -> Self {
        Task::App(AppTask::NonRead(NonReadTask::Restart(value)))
    }
}

impl RestartType {
    fn function(&self) -> FunctionCode {
        match self {
            Self::ColdRestart => FunctionCode::ColdRestart,
            Self::WarmRestart => FunctionCode::WarmRestart,
        }
    }
}

impl RestartTask {
    pub(crate) fn new(
        restart_type: RestartType,
        promise: Promise<Result<Duration, TaskError>>,
    ) -> Self {
        Self {
            restart_type,
            promise,
        }
    }

    pub(crate) fn function(&self) -> FunctionCode {
        self.restart_type.function()
    }

    pub(crate) fn on_task_error(self, err: TaskError) {
        self.promise.complete(Err(err))
    }

    pub(crate) fn handle(self, response: Response) -> Result<Option<NonReadTask>, TaskError> {
        let header = match response.get_only_object_header() {
            Ok(x) => x,
            Err(err) => {
                let err: TaskError = err.into();
                self.promise.complete(Err(err));
                return Err(err);
            }
        };

        let count = match header.details.count() {
            Some(x) => x,
            None => {
                self.promise
                    .complete(Err(TaskError::UnexpectedResponseHeaders));
                return Err(TaskError::UnexpectedResponseHeaders);
            }
        };

        let result: Result<Duration, TaskError> = match count {
            CountVariation::Group52Var1(val) => match val.single() {
                Some(val) => Ok(Duration::from_secs(val.time.into())),
                None => Err(TaskError::UnexpectedResponseHeaders),
            },
            CountVariation::Group52Var2(val) => match val.single() {
                Some(val) => Ok(Duration::from_millis(val.time.into())),
                None => Err(TaskError::UnexpectedResponseHeaders),
            },
            _ => Err(TaskError::UnexpectedResponseHeaders),
        };

        self.promise.complete(result);

        result.map(|_| None)
    }
}
