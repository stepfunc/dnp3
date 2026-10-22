use crate::app::format::write::HeaderWriter;
use crate::app::parse::parser::Response;
use crate::app::parse::traits::Index;
use crate::app::FunctionCode;
use crate::master::tasks::empty_response::expect_empty;
use crate::master::tasks::{NonReadTask, OneShot, OneShotRequest};
use crate::master::{
    DeadBandHeader, DeadBandHeaderVariants, DeadBands, TaskError, TaskType, WriteError,
};

pub(crate) struct DeadBandsRequest {
    headers: Vec<DeadBandHeader>,
}

impl DeadBandHeaderVariants {
    fn write(&self, writer: &mut HeaderWriter) -> Result<(), scursor::WriteError> {
        match self {
            Self::U8(x) => x.write(writer),
            Self::U16(x) => x.write(writer),
        }
    }
}

impl<I: Index> DeadBands<I> {
    fn write(&self, writer: &mut HeaderWriter) -> Result<(), scursor::WriteError> {
        match self {
            Self::V1(x) => writer.write_prefixed_items(x.iter()),
            Self::V2(x) => writer.write_prefixed_items(x.iter()),
            Self::V3(x) => writer.write_prefixed_items(x.iter()),
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

    fn wrap(task: OneShot<Self>) -> NonReadTask {
        NonReadTask::DeadBands(task)
    }
}
