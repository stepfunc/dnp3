use crate::app::format::write::HeaderWriter;
use crate::app::parse::parser::Response;
use crate::app::FunctionCode;
use crate::master::promise::Promise;
use crate::master::tasks::{AppTask, NonReadTask, Task};
use crate::master::{DeadBandHeader, DeadBandHeaderVariants, TaskError, WriteError};

pub(crate) struct WriteDeadBandsTask {
    headers: Vec<DeadBandHeader>,
    promise: Promise<Result<(), WriteError>>,
}

impl From<WriteDeadBandsTask> for Task {
    fn from(value: WriteDeadBandsTask) -> Self {
        Task::App(AppTask::NonRead(NonReadTask::DeadBands(value)))
    }
}

impl DeadBandHeaderVariants {
    fn write(&self, writer: &mut HeaderWriter) -> Result<(), scursor::WriteError> {
        match self {
            Self::G34V1U8(x) => writer.write_prefixed_items(x.iter()),
            Self::G34V1U16(x) => writer.write_prefixed_items(x.iter()),
            Self::G34V2U8(x) => writer.write_prefixed_items(x.iter()),
            Self::G34V2U16(x) => writer.write_prefixed_items(x.iter()),
            Self::G34V3U8(x) => writer.write_prefixed_items(x.iter()),
            Self::G34V3U16(x) => writer.write_prefixed_items(x.iter()),
        }
    }
}

impl WriteDeadBandsTask {
    pub(crate) fn new(
        headers: Vec<DeadBandHeader>,
        promise: Promise<Result<(), WriteError>>,
    ) -> Self {
        Self { headers, promise }
    }

    pub(crate) const fn function(&self) -> FunctionCode {
        FunctionCode::Write
    }

    pub(crate) fn write(&self, writer: &mut HeaderWriter) -> Result<(), scursor::WriteError> {
        for header in self.headers.iter() {
            header.inner.write(writer)?;
        }

        Ok(())
    }

    pub(crate) fn on_task_error(self, err: TaskError) {
        self.promise.complete(Err(err.into()))
    }

    pub(crate) fn handle(self, response: Response) -> Result<Option<NonReadTask>, TaskError> {
        if response.raw_objects.is_empty() {
            self.promise.complete(Ok(()));
            Ok(None)
        } else {
            self.promise
                .complete(Err(WriteError::Task(TaskError::UnexpectedResponseHeaders)));
            Err(TaskError::UnexpectedResponseHeaders)
        }
    }
}
