use super::*;
use crate::app::file::*;
use crate::app::parse::parser::Response;
use crate::app::FunctionCode;
use crate::master::association::Association;
use crate::master::file::BlockNumber;
use crate::master::tasks::{AppTask, NonReadTask, ResponseTask, Step, Task};
use crate::master::{FileAction, FileMode, FileReadConfig, FileReader, TaskError, TaskType};

/// The user's reader, until it is told how the read ended
pub(crate) struct FileReaderType {
    inner: Option<Box<dyn FileReader>>,
}

impl FileReaderType {
    pub(crate) fn from_reader(reader: Box<dyn FileReader>) -> Self {
        Self {
            inner: Some(reader),
        }
    }

    fn opened(&mut self, size: u32) -> FileAction {
        match self.inner.as_mut() {
            Some(x) => x.opened(size),
            None => FileAction::Abort,
        }
    }

    async fn block_received(&mut self, block_num: u32, data: &[u8]) -> FileAction {
        match self.inner.as_mut() {
            Some(x) => x.block_received(block_num, data).get().await,
            None => FileAction::Abort,
        }
    }

    fn aborted(&mut self, err: FileError) {
        if let Some(mut x) = self.inner.take() {
            x.aborted(err);
        }
    }

    fn completed(&mut self) {
        if let Some(mut x) = self.inner.take() {
            x.completed();
        }
    }
}

impl Drop for FileReaderType {
    fn drop(&mut self) {
        self.aborted(FileError::TaskError(TaskError::Shutdown));
    }
}

pub(crate) struct Settings {
    pub(crate) name: Filename,
    pub(crate) config: FileReadConfig,
    pub(crate) reader: FileReaderType,
}

#[derive(Copy, Clone)]
struct ReadState {
    handle: FileHandle,
    block: BlockNumber,
    total_rx: usize,
}

impl ReadState {
    fn new(handle: FileHandle) -> Self {
        Self {
            handle,
            block: Default::default(),
            total_rx: 0,
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
    Read(ReadState),
    /// Close the file
    ///
    /// If the transfer ended early, this holds the error that ended it, which the reader was
    /// already given, until the CLOSE response arrives
    Close(FileHandle, Option<FileError>),
}

pub(crate) struct FileReadTask {
    /// settings that don't change
    settings: Settings,
    /// state of the read operation determines the next action
    state: State,
}

impl From<FileReadTask> for Task {
    fn from(value: FileReadTask) -> Self {
        Task::App(AppTask::NonRead(NonReadTask::FileRead(value)))
    }
}

impl FileReadTask {
    fn new(settings: Settings, state: State) -> Self {
        Self { settings, state }
    }

    pub(crate) fn start(
        file_name: String,
        config: FileReadConfig,
        reader: FileReaderType,
        credentials: Option<FileCredentials>,
    ) -> Self {
        let settings = Settings {
            name: Filename(file_name),
            config,
            reader,
        };
        let state = match credentials {
            None => State::Open(AuthKey::none()),
            Some(x) => State::GetAuth(x),
        };
        Self::new(settings, state)
    }

    /// Abort the reader with `err`, and close the file once without retries
    fn end_early(&mut self, handle: FileHandle, err: FileError) {
        self.settings.reader.aborted(err);
        self.state = State::Close(handle, Some(err));
    }

    /// The state that reads the next block, or `None` if this block was the last
    async fn handle_read_response(
        settings: &mut Settings,
        rs: ReadState,
        header: ObjectHeader<'_>,
    ) -> Result<Option<ReadState>, FileError> {
        let obj = match header.details {
            HeaderDetails::FreeFormat(FreeFormatVariation::Group70Var5(obj)) => obj,
            _ => {
                tracing::warn!(
                    "File READ response contains unexpected variation: {}",
                    header.variation()
                );
                return Err(FileError::BadResponse);
            }
        };

        if obj.file_handle != u32::from(rs.handle) {
            tracing::warn!(
                "File READ response has handle {:#010X}, expected {:#010X}",
                obj.file_handle,
                u32::from(rs.handle)
            );
            return Err(FileError::WrongHandle);
        }

        let rx_block = BlockNumber::new(obj.block_number);

        if rx_block.bottom_bits() != rs.block.bottom_bits() {
            tracing::warn!(
                "Expected file block {} but received block {}",
                rs.block.bottom_bits(),
                rx_block.bottom_bits()
            );
            return Err(FileError::BadBlockNum);
        }

        let new_total = match rs.total_rx.checked_add(obj.file_data.len()) {
            None => {
                tracing::error!("Total rx file data overflow");
                return Err(FileError::MaxLengthExceeded);
            }
            Some(x) => x,
        };

        if new_total > settings.config.max_file_size {
            tracing::warn!(
                "Received bytes ({new_total}) exceeds configured maximum {}",
                settings.config.max_file_size
            );
            return Err(FileError::MaxLengthExceeded);
        }

        if settings
            .reader
            .block_received(rx_block.bottom_bits(), obj.file_data)
            .await
            .is_abort()
        {
            tracing::warn!("File transfer aborted by user");
            return Err(FileError::AbortByUser);
        }

        Ok(if rx_block.is_last() {
            None
        } else {
            let mut block = BlockNumber::new(obj.block_number);
            block.increment().map_err(|_| FileError::BadBlockNum)?;

            Some(ReadState {
                handle: rs.handle,
                block,
                total_rx: new_total,
            })
        })
    }
}

impl ResponseTask for FileReadTask {
    type Output = ();
    type Error = FileError;

    fn function(&self) -> FunctionCode {
        match self.state {
            State::GetAuth(_) => FunctionCode::AuthenticateFile,
            State::Open(_) => FunctionCode::OpenFile,
            State::Read(_) => FunctionCode::Read,
            State::Close(..) => FunctionCode::CloseFile,
        }
    }

    fn task_type(&self) -> TaskType {
        TaskType::FileRead
    }

    fn write(&self, writer: &mut HeaderWriter) -> Result<(), TaskError> {
        let result = match &self.state {
            State::GetAuth(auth) => write_auth(auth, writer),
            State::Open(auth) => write_open(
                &self.settings.name.0,
                *auth,
                Permissions::default(),
                0,
                FileMode::Read,
                self.settings.config.max_block_size,
                writer,
            ),
            State::Read(rs) => write_read(*rs, writer),
            State::Close(handle, _) => write_close(*handle, writer),
        };
        Ok(result?)
    }

    async fn on_response(
        &mut self,
        _association: &mut Association,
        response: Response<'_>,
    ) -> Result<Step<()>, FileError> {
        let header = response.get_only_object_header().map_err(FileError::from);

        match self.state {
            State::GetAuth(_) => {
                self.state = State::Open(parse_auth_response(header?)?);
                Ok(Step::Next)
            }
            State::Open(_) => {
                let file = parse_open_response(header?)?;
                if self.settings.reader.opened(file.file_size).is_abort() {
                    tracing::warn!("File transfer aborted by user");
                    self.end_early(file.file_handle, FileError::AbortByUser);
                } else {
                    self.state = State::Read(ReadState::new(file.file_handle));
                }
                Ok(Step::Next)
            }
            State::Read(rs) => {
                let result = match header {
                    Ok(header) => Self::handle_read_response(&mut self.settings, rs, header).await,
                    Err(err) => Err(err),
                };
                match result {
                    Ok(None) => {
                        self.settings.reader.completed();
                        self.state = State::Close(rs.handle, None);
                    }
                    Ok(Some(rs)) => self.state = State::Read(rs),
                    Err(err) => self.end_early(rs.handle, err),
                }
                Ok(Step::Next)
            }
            State::Close(handle, ref mut early) => {
                let closed = header.and_then(|header| check_close_response(handle, header));
                match early.take() {
                    None => closed.map(Step::Done),
                    Some(err) => closed_early(err, closed).map(Step::Done),
                }
            }
        }
    }

    fn complete(mut self, _association: Option<&mut Association>, result: Result<(), FileError>) {
        if let Err(err) = result {
            if let State::Close(_, Some(early)) = self.state {
                tracing::warn!(
                    "Unable to close the file after the transfer ended ({early}): {err}"
                );
            }
            self.settings.reader.aborted(err);
        }
    }
}

/// How a task whose transfer ended early with `err` ends, once the outstation answered the CLOSE
fn closed_early(err: FileError, closed: Result<(), FileError>) -> Result<(), FileError> {
    if let Err(close_err) = closed {
        tracing::warn!("Unable to close the file after the transfer ended ({err}): {close_err}");
    }

    match err {
        // the master ended the transfer itself, which only fails the task if the CLOSE fails
        FileError::AbortByUser | FileError::MaxLengthExceeded => closed,
        FileError::BadResponse
        | FileError::BadStatus(_)
        | FileError::WrongHandle
        | FileError::NoPermission
        | FileError::BadBlockNum
        | FileError::TaskError(_) => Err(err),
    }
}

fn write_read(rs: ReadState, writer: &mut HeaderWriter) -> Result<(), WriteError> {
    let obj = Group70Var5 {
        file_handle: rs.handle.into(),
        block_number: rs.block.wire_value(),
        file_data: &[],
    };
    writer.write_free_format(&obj)
}
