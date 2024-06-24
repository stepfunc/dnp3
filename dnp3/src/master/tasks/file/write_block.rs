use crate::app::format::write::HeaderWriter;
use crate::app::format::WriteError;
use crate::app::parse::free_format::FreeFormatVariation;
use crate::app::parse::parser::{HeaderDetails, Response};
use crate::app::{FileStatus, FunctionCode, Group70Var5};
use crate::master::promise::Promise;
use crate::master::tasks::{AppTask, NonReadTask, Task};
use crate::master::{BlockNumber, FileError, FileHandle, TaskError};

pub(crate) struct WriteBlockRequest {
    pub(crate) handle: FileHandle,
    pub(crate) block_number: BlockNumber,
    pub(crate) block_data: Vec<u8>,
}

pub(crate) struct WriteBlockTask {
    pub(crate) request: WriteBlockRequest,
    pub(crate) promise: Promise<Result<(), FileError>>,
}

impl From<WriteBlockTask> for Task {
    fn from(value: WriteBlockTask) -> Self {
        Task::App(AppTask::NonRead(NonReadTask::WriteFileBlock(value)))
    }
}

impl WriteBlockTask {
    pub(crate) fn function(&self) -> FunctionCode {
        FunctionCode::Write
    }

    pub(crate) fn write(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        let obj = Group70Var5 {
            file_handle: self.request.handle.into(),
            block_number: self.request.block_number.wire_value(),
            file_data: &self.request.block_data,
        };

        writer.write_free_format(&obj)
    }

    pub(crate) fn on_task_error(self, err: TaskError) {
        self.promise.complete(Err(err.into()))
    }

    pub(crate) fn handle(self, response: Response<'_>) -> Result<Option<NonReadTask>, TaskError> {
        fn process(response: Response<'_>) -> Result<(), FileError> {
            let header = response.objects?.get_only_header()?;

            let obj = match header.details {
                HeaderDetails::TwoByteFreeFormat(_, FreeFormatVariation::Group70Var6(obj)) => obj,
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

        let result = process(response);
        self.promise.complete(result);

        match result {
            Ok(_) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }
}
