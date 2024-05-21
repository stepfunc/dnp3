use crate::app::format::write::HeaderWriter;
use crate::app::format::WriteError;
use crate::app::parse::free_format::FreeFormatVariation;
use crate::app::parse::parser::{HeaderDetails, Response};
use crate::app::{FileStatus, FunctionCode, Group70Var3, Permissions, Timestamp};
use crate::master::promise::Promise;
use crate::master::tasks::file::REQUEST_ID;
use crate::master::tasks::{AppTask, NonReadTask, Task};
use crate::master::{AuthKey, FileError, FileHandle, FileMode, OpenFile, TaskError};
pub(crate) struct OpenFileRequest {
    pub(crate) file_name: String,
    pub(crate) auth_key: AuthKey,
    pub(crate) file_size: u32,
    pub(crate) file_mode: FileMode,
    pub(crate) permissions: Permissions,
    pub(crate) max_block_size: u16,
}

pub(crate) struct OpenFileTask {
    pub(crate) request: OpenFileRequest,
    pub(crate) promise: Promise<Result<OpenFile, FileError>>,
}

impl From<OpenFileTask> for Task {
    fn from(value: OpenFileTask) -> Self {
        Task::App(AppTask::NonRead(NonReadTask::OpenFile(value)))
    }
}

impl OpenFileTask {
    pub(crate) fn function(&self) -> FunctionCode {
        FunctionCode::OpenFile
    }

    pub(crate) fn write(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        let obj = Group70Var3 {
            time_of_creation: Timestamp::zero(),
            permissions: self.request.permissions,
            auth_key: self.request.auth_key.into(),
            file_size: self.request.file_size,
            mode: self.request.file_mode,
            max_block_size: self.request.max_block_size,
            request_id: REQUEST_ID,
            file_name: &self.request.file_name,
        };

        writer.write_free_format(&obj)
    }

    pub(crate) fn on_task_error(self, err: TaskError) {
        self.promise.complete(Err(err.into()))
    }

    pub(crate) fn handle(self, response: Response<'_>) -> Result<Option<NonReadTask>, TaskError> {
        fn process(response: Response<'_>) -> Result<OpenFile, FileError> {
            let header = response.objects?.get_only_header()?;

            let obj = match header.details {
                HeaderDetails::TwoByteFreeFormat(_, FreeFormatVariation::Group70Var4(obj)) => obj,
                _ => {
                    tracing::warn!(
                        "File OPEN response contains unexpected variation: {}",
                        header.variation
                    );
                    return Err(FileError::BadResponse);
                }
            };

            if obj.status_code != FileStatus::Success {
                tracing::warn!("Unable to open file (status code == {:?})", obj.status_code);
                return Err(FileError::BadStatus(obj.status_code));
            }

            Ok(OpenFile {
                max_block_size: obj.max_block_size,
                file_handle: FileHandle::new(obj.file_handle),
                file_size: obj.file_size,
            })
        }

        let result = process(response);
        self.promise.complete(result);

        match result {
            Ok(_) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }
}
