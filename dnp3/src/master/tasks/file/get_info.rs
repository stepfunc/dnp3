use crate::app::file::*;
use crate::app::format::write::HeaderWriter;
use crate::app::parse::free_format::FreeFormatVariation;
use crate::app::parse::parser::{ObjectHeader, Response};
use crate::app::{FunctionCode, Timestamp};
use crate::master::tasks::{NonReadTask, OneShot, OneShotRequest};
use crate::master::{FileError, FileInfo, TaskError, TaskType};

pub(crate) struct GetFileInfoRequest {
    file_name: String,
}

impl GetFileInfoRequest {
    pub(crate) fn new(file_name: String) -> Self {
        Self { file_name }
    }
}

impl OneShotRequest for GetFileInfoRequest {
    type Output = FileInfo;
    type Error = FileError;

    fn function(&self) -> FunctionCode {
        FunctionCode::GetFileInfo
    }

    fn task_type(&self) -> TaskType {
        TaskType::GetFileInfo
    }

    fn write(&self, writer: &mut HeaderWriter) -> Result<(), TaskError> {
        let obj = Group70Var7 {
            file_type: FileType::Other(0),
            file_size: 0,
            time_of_creation: Timestamp::zero(),
            permissions: Default::default(),
            request_id: 0xCAFE,
            file_name: self.file_name.as_str(),
        };
        Ok(writer.write_free_format(&obj)?)
    }

    fn parse(&self, response: Response<'_>) -> Result<FileInfo, FileError> {
        let header = response.objects?.get_only_header()?;

        match header {
            ObjectHeader::FreeFormat(FreeFormatVariation::Group70Var7(obj)) => Ok(obj.into()),
            ObjectHeader::FreeFormat(FreeFormatVariation::Group70Var4(obj)) => {
                if obj.status_code != FileStatus::Success {
                    tracing::warn!("Unable to get file info: {}", obj.text);
                }
                Err(FileError::BadStatus(obj.status_code))
            }
            _ => {
                tracing::warn!(
                    "File GET_FILE_INFO response contains unexpected variation: {}",
                    header.variation()
                );
                Err(FileError::BadResponse)
            }
        }
    }

    fn wrap(task: OneShot<Self>) -> NonReadTask {
        NonReadTask::GetFileInfo(task)
    }
}
