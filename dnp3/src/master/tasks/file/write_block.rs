use crate::app::format::write::HeaderWriter;
use crate::app::parse::free_format::FreeFormatVariation;
use crate::app::parse::parser::{HeaderDetails, Response};
use crate::app::{FileStatus, FunctionCode, Group70Var5};
use crate::master::tasks::{NonReadTask, OneShot, OneShotRequest};
use crate::master::{BlockNumber, FileError, FileHandle, TaskError, TaskType};

pub(crate) struct WriteBlockRequest {
    pub(crate) handle: FileHandle,
    pub(crate) block_number: BlockNumber,
    pub(crate) block_data: Vec<u8>,
}

impl OneShotRequest for WriteBlockRequest {
    type Output = ();
    type Error = FileError;

    fn function(&self) -> FunctionCode {
        FunctionCode::Write
    }

    fn task_type(&self) -> TaskType {
        TaskType::FileWriteBlock
    }

    fn write(&self, writer: &mut HeaderWriter) -> Result<(), TaskError> {
        let obj = Group70Var5 {
            file_handle: self.handle.into(),
            block_number: self.block_number.wire_value(),
            file_data: &self.block_data,
        };

        Ok(writer.write_free_format(&obj)?)
    }

    fn parse(&self, response: Response<'_>) -> Result<(), FileError> {
        let header = response.objects?.get_only_header()?;

        let obj = match header.details {
            HeaderDetails::FreeFormat(FreeFormatVariation::Group70Var6(obj)) => obj,
            _ => {
                tracing::warn!(
                    "File WRITE response contains unexpected variation: {}",
                    header.variation
                );
                return Err(FileError::BadResponse);
            }
        };

        if obj.status_code != FileStatus::Success {
            tracing::warn!(
                "Unable to write file block (status code == {:?})",
                obj.status_code
            );
            return Err(FileError::BadStatus(obj.status_code));
        }

        Ok(())
    }

    fn task_error(err: FileError) -> TaskError {
        err.into()
    }

    fn wrap(task: OneShot<Self>) -> NonReadTask {
        NonReadTask::WriteFileBlock(task)
    }
}
