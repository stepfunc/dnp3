use crate::app::file::*;
use crate::app::format::write::HeaderWriter;
use crate::app::format::WriteError;
use crate::app::parse::free_format::FreeFormatVariation;
use crate::app::parse::parser::{HeaderDetails, Response};
use crate::app::{FunctionCode, Timestamp};
use crate::master::tasks::NonReadTask;
use crate::master::{FileError, FileInfo, Promise, TaskError};

pub(crate) struct GetFileInfoTask {
    file_name: String,
    promise: Promise<Result<FileInfo, FileError>>,
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

    pub(crate) fn handle(self, response: Response) -> Option<NonReadTask> {
        fn inner(response: Response) -> Result<FileInfo, FileError> {
            let headers = match response.objects {
                Ok(x) => x,
                Err(err) => {
                    tracing::warn!("File operation received malformed response: {err}");
                    return Err(FileError::TaskError(TaskError::MalformedResponse(err)));
                }
            };

            let header = match headers.get_only_header() {
                None => {
                    tracing::warn!("File operation response contains unexpected number of headers");
                    return Err(FileError::TaskError(TaskError::UnexpectedResponseHeaders));
                }
                Some(x) => x,
            };

            match header.details {
                HeaderDetails::TwoByteFreeFormat(_, FreeFormatVariation::Group70Var7(obj)) => {
                    Ok(obj.into())
                }
                HeaderDetails::TwoByteFreeFormat(_, FreeFormatVariation::Group70Var4(obj)) => {
                    if !obj.text.is_empty() {
                        tracing::warn!("Unable to get file info: {}", obj.text);
                    }
                    Err(FileError::BadStatus(obj.status_code))
                }
                _ => Err(FileError::TaskError(TaskError::UnexpectedResponseHeaders)),
            }
        }

        self.promise.complete(inner(response));
        None
    }
}
