use crate::app::format::write::HeaderWriter;
use crate::app::parse::parser::Response;
use crate::app::FunctionCode;
use crate::master::tasks::NonReadTask;
use crate::master::{DeadBandHeader, DeadBandHeaderVariants, Promise, TaskError, WriteError};

pub(crate) struct WriteDeadBandsTask {
    headers: Vec<DeadBandHeader>,
    promise: Promise<Result<(), WriteError>>,
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

    pub(crate) fn wrap(self) -> NonReadTask {
        NonReadTask::DeadBands(self)
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

    pub(crate) fn handle(self, response: Response) -> Option<NonReadTask> {
        if !response.raw_objects.is_empty() {
            self.promise
                .complete(Err(WriteError::Task(TaskError::UnexpectedResponseHeaders)));
            return None;
        }

        if response.header.iin.has_request_error() {
            self.promise
                .complete(Err(WriteError::IinError(response.header.iin.iin2)));
            return None;
        }

        self.promise.complete(Ok(()));

        None
    }
}
