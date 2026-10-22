use super::*;
use crate::app::file::*;
use crate::app::parse::parser::Response;
use crate::app::FunctionCode;
use crate::master::association::Association;
use crate::master::file::BlockNumber;
use crate::master::tasks::{AppTask, NonReadTask, ResponseTask, Step, Task};
use crate::master::{FileAction, FileMode, FileReadConfig, FileReader, TaskError, TaskType};

enum ReaderTypes {
    Trait(Box<dyn FileReader>),
}

pub(crate) struct FileReaderType {
    inner: Option<ReaderTypes>,
}

impl FileReaderType {
    pub(crate) fn from_reader(reader: Box<dyn FileReader>) -> Self {
        Self {
            inner: Some(ReaderTypes::Trait(reader)),
        }
    }

    fn opened(&mut self, size: u32) -> FileAction {
        if let Some(x) = self.inner.as_mut() {
            match x {
                ReaderTypes::Trait(x) => x.opened(size),
            }
        } else {
            FileAction::Abort
        }
    }

    async fn block_received(&mut self, block_num: u32, data: &[u8]) -> FileAction {
        if let Some(x) = self.inner.as_mut() {
            match x {
                ReaderTypes::Trait(x) => x.block_received(block_num, data).get().await,
            }
        } else {
            FileAction::Abort
        }
    }

    fn aborted(&mut self, err: FileError) {
        if let Some(x) = self.inner.take() {
            match x {
                ReaderTypes::Trait(mut x) => x.aborted(err),
            }
        }
    }

    fn completed(&mut self) {
        if let Some(x) = self.inner.take() {
            match x {
                ReaderTypes::Trait(mut x) => x.completed(),
            }
        }
    }
}

impl Drop for FileReaderType {
    fn drop(&mut self) {
        if let Some(x) = self.inner.take() {
            match x {
                ReaderTypes::Trait(mut x) => x.aborted(FileError::TaskError(TaskError::Shutdown)),
            }
        }
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
    Close(FileHandle),
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
                    header.variation
                );
                return Err(FileError::BadResponse);
            }
        };

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
    /// An error that ends the transfer, which the task reports as its success
    type Output = Option<FileError>;
    type Error = FileError;

    fn function(&self) -> FunctionCode {
        match self.state {
            State::GetAuth(_) => FunctionCode::AuthenticateFile,
            State::Open(_) => FunctionCode::OpenFile,
            State::Read(_) => FunctionCode::Read,
            State::Close(_) => FunctionCode::CloseFile,
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
            State::Close(handle) => write_close(*handle, writer),
        };
        Ok(result?)
    }

    async fn on_response(
        &mut self,
        _association: &mut Association,
        response: Response<'_>,
    ) -> Result<Step<Option<FileError>>, FileError> {
        let header = response.get_only_object_header()?;

        match self.state {
            State::GetAuth(_) => match parse_auth_response(header) {
                Ok(key) => {
                    self.state = State::Open(key);
                    Ok(Step::Next)
                }
                Err(err) => Ok(Step::Done(Some(err))),
            },
            State::Open(_) => match parse_open_response(header) {
                Ok(file) => {
                    if self.settings.reader.opened(file.file_size).is_abort() {
                        tracing::warn!("File transfer aborted by user");
                        self.settings.reader.aborted(FileError::AbortByUser);
                        self.state = State::Close(file.file_handle);
                    } else {
                        self.state = State::Read(ReadState::new(file.file_handle));
                    }
                    Ok(Step::Next)
                }
                Err(err) => Ok(Step::Done(Some(err))),
            },
            State::Read(rs) => {
                match Self::handle_read_response(&mut self.settings, rs, header).await {
                    Ok(None) => {
                        self.settings.reader.completed();
                        self.state = State::Close(rs.handle);
                        Ok(Step::Next)
                    }
                    Ok(Some(rs)) => {
                        self.state = State::Read(rs);
                        Ok(Step::Next)
                    }
                    Err(err) => Ok(Step::Done(Some(err))),
                }
            }
            // the file was already read, so a CLOSE that fails only gets logged
            State::Close(_) => {
                let _: Result<(), FileError> = parse_close_response(header)
                    .and_then(|obj| check_close_status(obj.status_code));
                Ok(Step::Done(None))
            }
        }
    }

    fn from_task_error(err: TaskError) -> FileError {
        FileError::TaskError(err)
    }

    fn task_error(err: FileError) -> TaskError {
        err.into()
    }

    fn complete(
        mut self,
        _association: Option<&mut Association>,
        result: Result<Option<FileError>, FileError>,
    ) {
        match result {
            Ok(None) => {}
            Ok(Some(err)) | Err(err) => self.settings.reader.aborted(err),
        }
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
