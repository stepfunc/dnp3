use std::time::Duration;

use crate::app::enums::FunctionCode;
use crate::app::format::write::HeaderWriter;
use crate::app::gen::count::CountVariation;
use crate::app::parse::parser::Response;
use crate::master::error::TaskError;
use crate::master::handle::Promise;
use crate::master::tasks::NonReadTask;
use crate::util::cursor::WriteError;

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

    pub(crate) fn wrap(self) -> NonReadTask {
        NonReadTask::Restart(self)
    }

    pub(crate) fn function(&self) -> FunctionCode {
        self.restart_type.function()
    }

    pub(crate) fn write(&self, _writer: &mut HeaderWriter) -> Result<(), WriteError> {
        // empty body
        Ok(())
    }

    pub(crate) fn on_task_error(self, err: TaskError) {
        self.promise.complete(Err(err))
    }

    pub(crate) fn handle(self, response: Response) -> Option<NonReadTask> {
        let headers = match response.objects {
            Ok(x) => x,
            Err(err) => {
                self.promise
                    .complete(Err(TaskError::MalformedResponse(err)));
                return None;
            }
        };

        let header = match headers.get_only_header() {
            Some(x) => x,
            None => {
                self.promise
                    .complete(Err(TaskError::UnexpectedResponseHeaders));
                return None;
            }
        };

        let count = match header.details.count() {
            Some(x) => x,
            None => {
                self.promise
                    .complete(Err(TaskError::UnexpectedResponseHeaders));
                return None;
            }
        };

        match count {
            CountVariation::Group52Var1(val) => match val.single() {
                Some(val) => self
                    .promise
                    .complete(Ok(Duration::from_secs(val.time as u64))),
                None => self
                    .promise
                    .complete(Err(TaskError::UnexpectedResponseHeaders)),
            },
            CountVariation::Group52Var2(val) => match val.single() {
                Some(val) => self
                    .promise
                    .complete(Ok(Duration::from_millis(val.time as u64))),
                None => self
                    .promise
                    .complete(Err(TaskError::UnexpectedResponseHeaders)),
            },
            _ => self
                .promise
                .complete(Err(TaskError::UnexpectedResponseHeaders)),
        }

        None
    }
}
