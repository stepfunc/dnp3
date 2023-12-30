use crate::app::format::write::HeaderWriter;
use crate::app::format::WriteError;
use crate::app::parse::parser::{ObjectHeader, Response};
use crate::app::{FunctionCode, Group70Var3, Timestamp};
use crate::master::tasks::file::*;
use crate::master::tasks::NonReadTask;
use crate::master::{
    FileAction, FileCredentials, FileError, FileWriteConfig, FileWriter, TaskError,
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

    fn opened(&mut self, size: u32) -> FileAction {
        if let Some(x) = self.inner.as_mut() {
            x.opened(size)
        } else {
            FileAction::Abort
        }
    }

    async fn next_block(&mut self, total_tx: usize, dest: &mut [u8]) -> Result<usize, FileAction> {
        if let Some(x) = self.inner.as_mut() {
            x.next_block(total_tx, dest).get().await
        } else {
            Err(FileAction::Abort)
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

#[derive(Copy, Clone)]
struct WriteState {
    handle: FileHandle,
    block: BlockNumber,
    total_tx: usize,
}

impl WriteState {
    fn new(handle: FileHandle) -> Self {
        Self {
            handle,
            block: Default::default(),
            total_tx: 0,
        }
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
            None => State::Open(AuthKey::default()),
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
            State::Write(_state) => todo!(),
            State::Close(handle) => write_close(*handle, writer),
        }
    }

    pub(crate) fn on_task_error(mut self, err: TaskError) {
        self.settings.writer.aborted(FileError::TaskError(err));
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
            State::GetAuth(_) => Self::handle_auth_response(self.settings, header),
            State::Open(_) => Self::handle_open_response(self.settings, header),
            State::Write(state) => Self::handle_write_response(self.settings, state, header).await,
            State::Close(_) => Self::handle_close_response(header),
        };

        Ok(next.map(NonReadTask::FileWrite))
    }

    fn handle_auth_response(mut settings: Settings, header: ObjectHeader) -> Option<FileWriteTask> {
        match handle_auth_response(header) {
            Ok(key) => Some(FileWriteTask::new(settings, State::Open(key))),
            Err(err) => {
                settings.writer.aborted(err);
                None
            }
        }
    }

    fn handle_open_response(mut settings: Settings, header: ObjectHeader) -> Option<FileWriteTask> {
        match handle_open_response(header) {
            Ok((file_size, handle)) => {
                if settings.writer.opened(file_size).is_abort() {
                    tracing::warn!("File transfer aborted by user");
                    Some(FileWriteTask::new(settings, State::Close(handle)))
                } else {
                    Some(FileWriteTask::new(
                        settings,
                        State::Write(WriteState::new(handle)),
                    ))
                }
            }
            Err(err) => {
                settings.writer.aborted(err);
                None
            }
        }
    }

    async fn handle_write_response(
        _settings: Settings,
        _state: WriteState,
        _header: ObjectHeader<'_>,
    ) -> Option<FileWriteTask> {
        todo!()
    }

    fn handle_close_response(header: ObjectHeader) -> Option<FileWriteTask> {
        process_close_response(header);
        None
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
        auth_key: key.0,
        file_size: 0,
        mode: settings.config.mode.into(),
        max_block_size: settings.config.max_block_size,
        request_id: REQUEST_ID,
        file_name: &settings.name.0,
    };
    writer.write_free_format(&obj)
}
