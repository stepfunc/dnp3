use crate::app::format::write::HeaderWriter;
use crate::app::parse::parser::Response;
use crate::app::FunctionCode;
use crate::master::tasks::empty_response::{expect_empty, write_error};
use crate::master::tasks::{NonReadTask, OneShot, OneShotRequest};
use crate::master::{DeadBandHeader, DeadBandHeaderVariants, TaskError, TaskType, WriteError};

pub(crate) struct DeadBandsRequest {
    headers: Vec<DeadBandHeader>,
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

impl DeadBandsRequest {
    pub(crate) fn new(headers: Vec<DeadBandHeader>) -> Self {
        Self { headers }
    }
}

impl OneShotRequest for DeadBandsRequest {
    type Output = ();
    type Error = WriteError;

    fn function(&self) -> FunctionCode {
        FunctionCode::Write
    }

    fn task_type(&self) -> TaskType {
        TaskType::WriteDeadBands
    }

    fn write(&self, writer: &mut HeaderWriter) -> Result<(), TaskError> {
        for header in self.headers.iter() {
            header.inner.write(writer)?;
        }

        Ok(())
    }

    fn parse(&self, response: Response<'_>) -> Result<(), WriteError> {
        expect_empty(response)
    }

    fn task_error(err: WriteError) -> TaskError {
        write_error(err)
    }

    fn wrap(task: OneShot<Self>) -> NonReadTask {
        NonReadTask::DeadBands(task)
    }
}
