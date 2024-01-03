use crate::app::format::write::HeaderWriter;
use crate::app::format::WriteError;
use crate::app::parse::parser::{ObjectHeader, Response};
use crate::app::{FunctionCode, Group70Var3, Group70Var5, Timestamp};
use crate::master::file::BlockNumber;
use crate::master::tasks::file::*;
use crate::master::tasks::NonReadTask;
use crate::master::{
    AuthKey, Block, FileAction, FileCredentials, FileError, FileHandle, FileWriteConfig,
    FileWriter, TaskError,
};

pub(crate) struct FileWriterType {
    inner: Option<Box<dyn FileWriter>>,
}

impl FileWriterType {
    pub(crate) fn new(writer: Box<dyn FileWriter>) -> Self {
        Self {
            inner: Some(writer),
        }
    }

    fn opened(&mut self, handle: FileHandle, size: u32) -> FileAction {
        if let Some(x) = self.inner.as_mut() {
            x.opened(handle, size)
        } else {
            FileAction::Abort
        }
    }

    async fn next_block(&mut self, max_block_size: u16) -> Option<Block> {
        if let Some(x) = self.inner.as_mut() {
            x.next_block(max_block_size).get().await
        } else {
            None
        }
    }

    fn aborted(&mut self, err: FileError) {
        if let Some(mut x) = self.inner.take() {
            x.aborted(err)
        }
    }

    fn completed(&mut self) {
        if let Some(mut x) = self.inner.take() {
            x.completed();
        }
    }
}

impl Drop for FileWriterType {
    fn drop(&mut self) {
        if let Some(mut x) = self.inner.take() {
            x.aborted(FileError::TaskError(TaskError::Shutdown));
        }
    }
}

struct WriteState {
    handle: FileHandle,
    block_num: BlockNumber,
    next_block: Vec<u8>,
}

impl WriteState {
    fn write_block(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        let obj = Group70Var5 {
            file_handle: self.handle.into(),
            block_number: self.block_num.wire_value(),
            file_data: &self.next_block,
        };

        writer.write_free_format(&obj)
    }
}

/// States of the file transfer
enum State {
    /// Obtain and authentication key using file name and authentication data
    GetAuth(FileCredentials),
    /// Open the file - We might state in this state w/ the default AuthKey if auth not required
    Open(AuthKey),
    /// Read the next block
    Write(WriteState),
    /// Close the file
    Close(FileHandle),
}

pub(crate) struct FileWriteTask {
    /// settings that don't change
    settings: Settings,
    /// state of the read operation determines the next action
    state: State,
}

pub(crate) struct Settings {
    pub(crate) name: Filename,
    pub(crate) config: FileWriteConfig,
    pub(crate) writer: FileWriterType,
}

impl FileWriteTask {
    fn new(settings: Settings, state: State) -> Self {
        Self { settings, state }
    }

    pub(crate) fn on_task_error(mut self, err: TaskError) {
        self.settings.writer.aborted(err.into());
    }

    pub(crate) fn start(
        file_name: String,
        config: FileWriteConfig,
        writer: FileWriterType,
        credentials: Option<FileCredentials>,
    ) -> Self {
        let settings = Settings {
            name: Filename(file_name),
            config,
            writer,
        };
        let state = match credentials {
            None => State::Open(AuthKey::none()),
            Some(x) => State::GetAuth(x),
        };
        Self::new(settings, state)
    }

    pub(crate) fn function(&self) -> FunctionCode {
        match self.state {
            State::GetAuth(_) => FunctionCode::AuthenticateFile,
            State::Open(_) => FunctionCode::OpenFile,
            State::Write(_) => FunctionCode::Write,
            State::Close(_) => FunctionCode::CloseFile,
        }
    }

    pub(crate) fn write(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        match &self.state {
            State::GetAuth(auth) => write_auth(auth, writer),
            State::Open(auth) => write_open(&self.settings, *auth, writer),
            State::Write(state) => state.write_block(writer),
            State::Close(handle) => write_close(*handle, writer),
        }
    }

    pub(crate) async fn handle(
        mut self,
        response: Response<'_>,
    ) -> Result<Option<NonReadTask>, TaskError> {
        let header = match get_only_header(response) {
            Ok(x) => x,
            Err(err) => {
                self.settings.writer.aborted(err.into());
                return Err(err);
            }
        };

        let next = match self.state {
            State::GetAuth(_) => Self::handle_auth_response(&mut self.settings, header),
            State::Open(_) => Self::handle_open_response(&mut self.settings, header).await,
            State::Write(state) => {
                Self::handle_write_response(&mut self.settings, state, header).await
            }
            State::Close(_) => {
                Self::handle_close_response(&mut self.settings, header);
                None
            }
        };

        Ok(next.map(|s| NonReadTask::FileWrite(FileWriteTask::new(self.settings, s))))
    }

    fn handle_auth_response(settings: &mut Settings, header: ObjectHeader) -> Option<State> {
        match handle_auth_response(header) {
            Ok(key) => Some(State::Open(key)),
            Err(err) => {
                settings.writer.aborted(err);
                None
            }
        }
    }

    async fn handle_open_response(
        settings: &mut Settings,
        header: ObjectHeader<'_>,
    ) -> Option<State> {
        let (file_size, handle) = match handle_open_response(header) {
            Ok(x) => x,
            Err(err) => {
                settings.writer.aborted(err);
                return None;
            }
        };

        if settings.writer.opened(handle, file_size).is_abort() {
            tracing::warn!("File transfer aborted by user");
            return Some(State::Close(handle));
        }

        // load the first block
        let next_state = match settings
            .writer
            .next_block(settings.config.max_block_size)
            .await
        {
            None => State::Close(handle),
            Some(block) => {
                let mut block_num = BlockNumber::default();
                if block.last {
                    block_num.set_last()
                }

                State::Write(WriteState {
                    handle,
                    block_num,
                    next_block: block.data,
                })
            }
        };

        Some(next_state)
    }

    async fn handle_write_response(
        settings: &mut Settings,
        state: WriteState,
        header: ObjectHeader<'_>,
    ) -> Option<State> {
        let handle = state.handle;
        // If a write fails, we still want to try and close the file
        let next = Self::handle_write_response_inner(settings, state, header)
            .await
            .unwrap_or_else(|err| {
                settings.writer.aborted(err);
                State::Close(handle)
            });
        Some(next)
    }

    async fn handle_write_response_inner(
        settings: &mut Settings,
        mut state: WriteState,
        header: ObjectHeader<'_>,
    ) -> Result<State, FileError> {
        let obj = match header.details {
            HeaderDetails::TwoByteFreeFormat(_, FreeFormatVariation::Group70Var6(obj)) => obj,
            _ => {
                tracing::warn!(
                    "Unexpected response to file write: {} - {}",
                    header.details.qualifier().description(),
                    header.variation.description()
                );
                return Err(FileError::BadResponse);
            }
        };

        if obj.status_code != FileStatus::Success {
            tracing::warn!(
                "Outstation returned file status error in response to WRITE: {:?}",
                obj.status_code
            );
            return Err(FileError::BadStatus(obj.status_code));
        }

        if obj.block_number != state.block_num.wire_value() {
            tracing::warn!(
                "Expected block number {} but outstation returned {} in response",
                state.block_num.wire_value(),
                obj.block_number
            );
            return Err(FileError::BadBlockNum);
        }

        if state.block_num.is_last() {
            return Ok(State::Close(state.handle));
        }

        let block = match settings
            .writer
            .next_block(settings.config.max_block_size)
            .await
        {
            None => {
                settings.writer.aborted(FileError::AbortByUser);
                return Err(FileError::AbortByUser);
            }
            Some(x) => x,
        };

        if block.data.is_empty() {
            return Ok(State::Close(state.handle));
        }

        if let Err(max_value) = state.block_num.increment() {
            tracing::warn!("File block number overflowed max value of {max_value}");
            return Err(FileError::BadBlockNum);
        }

        if block.last {
            state.block_num.set_last();
        }

        Ok(State::Write(WriteState {
            handle: state.handle,
            block_num: state.block_num,
            next_block: block.data,
        }))
    }

    fn handle_close_response(settings: &mut Settings, header: ObjectHeader) {
        match process_close_response(header) {
            Ok(()) => {
                settings.writer.completed();
            }
            Err(err) => {
                settings.writer.aborted(err);
            }
        }
    }
}

fn write_open(
    settings: &Settings,
    key: AuthKey,
    writer: &mut HeaderWriter,
) -> Result<(), WriteError> {
    let obj = Group70Var3 {
        time_of_creation: Timestamp::zero(),
        permissions: settings.config.permissions,
        auth_key: key.into(),
        file_size: settings.config.file_size,
        mode: settings.config.mode.into(),
        max_block_size: settings.config.max_block_size,
        request_id: REQUEST_ID,
        file_name: &settings.name.0,
    };
    writer.write_free_format(&obj)
}
