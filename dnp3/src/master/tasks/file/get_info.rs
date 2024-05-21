use crate::app::file::*;
use crate::app::format::write::HeaderWriter;
use crate::app::format::WriteError;
use crate::app::parse::free_format::FreeFormatVariation;
use crate::app::parse::parser::{HeaderDetails, Response};
use crate::app::{FunctionCode, Timestamp};
use crate::master::promise::Promise;
use crate::master::tasks::{AppTask, NonReadTask, Task};
use crate::master::{FileError, FileInfo, TaskError};

pub(crate) struct GetFileInfoTask {
    file_name: String,
    promise: Promise<Result<FileInfo, FileError>>,
}

impl From<GetFileInfoTask> for Task {
    fn from(value: GetFileInfoTask) -> Self {
        Task::App(AppTask::NonRead(NonReadTask::GetFileInfo(value)))
    }
}

impl GetFileInfoTask {
    pub(crate) fn new(file_name: String, promise: Promise<Result<FileInfo, FileError>>) -> Self {
        Self { file_name, promise }
    }

    pub(crate) fn function(&self) -> FunctionCode {
        FunctionCode::GetFileInfo
    }

    pub(crate) fn write(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        let obj = Group70Var7 {
            file_type: FileType::Other(0),
            file_size: 0,
            time_of_creation: Timestamp::zero(),
            permissions: Default::default(),
            request_id: 0xCAFE,
            file_name: self.file_name.as_str(),
        };
        writer.write_free_format(&obj)
    }

    pub(crate) fn on_task_error(self, err: TaskError) {
        self.promise.complete(Err(FileError::TaskError(err)));
    }

    pub(crate) fn handle(self, response: Response) -> Result<Option<NonReadTask>, TaskError> {
        fn inner(response: Response) -> Result<FileInfo, FileError> {
            let header = response.objects?.get_only_header()?;

            match header.details {
                HeaderDetails::TwoByteFreeFormat(_, FreeFormatVariation::Group70Var7(obj)) => {
                    Ok(obj.into())
                }
                HeaderDetails::TwoByteFreeFormat(_, FreeFormatVariation::Group70Var4(obj)) => {
                    if obj.status_code != FileStatus::Success {
                        tracing::warn!("Unable to get file info: {}", obj.text);
                    }
                    Err(FileError::BadStatus(obj.status_code))
                }
                _ => Err(FileError::TaskError(TaskError::UnexpectedResponseHeaders)),
            }
        }

        let result = inner(response);
        let ret = match &result {
            Ok(_) => Ok(None),
            Err(err) => match err {
                FileError::TaskError(x) => Err(*x),
                _ => Err(TaskError::UnexpectedResponseHeaders),
            },
        };
        self.promise.complete(result);
        ret
    }
}
