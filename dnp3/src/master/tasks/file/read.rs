use super::*;
use crate::app::file::*;
use crate::app::parse::parser::Response;
use crate::app::{FunctionCode, Timestamp};
use crate::master::file::BlockNumber;
use crate::master::tasks::{AppTask, NonReadTask, Task};
use crate::master::{FileAction, FileMode, FileReadConfig, FileReader, TaskError};

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

    pub(crate) fn function(&self) -> FunctionCode {
        match self.state {
            State::GetAuth(_) => FunctionCode::AuthenticateFile,
            State::Open(_) => FunctionCode::OpenFile,
            State::Read(_) => FunctionCode::Read,
            State::Close(_) => FunctionCode::CloseFile,
        }
    }

    pub(crate) fn write(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        match &self.state {
            State::GetAuth(auth) => write_auth(auth, writer),
            State::Open(auth) => write_open(&self.settings, *auth, writer),
            State::Read(rs) => write_read(*rs, writer),
            State::Close(handle) => write_close(*handle, writer),
        }
    }

    pub(crate) fn on_task_error(mut self, err: TaskError) {
        self.settings.reader.aborted(FileError::TaskError(err));
    }

    pub(crate) async fn handle(
        mut self,
        response: Response<'_>,
    ) -> Result<Option<NonReadTask>, TaskError> {
        let header = match response.get_only_object_header() {
            Ok(x) => x,
            Err(err) => {
                self.settings.reader.aborted(err.into());
                return Err(err.into());
            }
        };

        let next = match self.state {
            State::GetAuth(_) => Self::handle_auth_response(self.settings, header),
            State::Open(_) => Self::handle_open_response(self.settings, header),
            State::Read(rs) => Self::handle_read_response(self.settings, rs, header).await,
            State::Close(_) => Self::handle_close_response(header),
        };

        Ok(next.map(NonReadTask::FileRead))
    }

    fn handle_auth_response(mut settings: Settings, header: ObjectHeader) -> Option<FileReadTask> {
        match handle_auth_response(header) {
            Ok(key) => Some(FileReadTask::new(settings, State::Open(key))),
            Err(err) => {
                settings.reader.aborted(err);
                None
            }
        }
    }

    fn handle_open_response(mut settings: Settings, header: ObjectHeader) -> Option<FileReadTask> {
        match handle_open_response(header) {
            Ok((file_size, handle)) => {
                if settings.reader.opened(file_size).is_abort() {
                    tracing::warn!("File transfer aborted by user");
                    Some(FileReadTask::new(settings, State::Close(handle)))
                } else {
                    Some(FileReadTask::new(
                        settings,
                        State::Read(ReadState::new(handle)),
                    ))
                }
            }
            Err(err) => {
                settings.reader.aborted(err);
                None
            }
        }
    }

    async fn handle_read_response(
        mut settings: Settings,
        rs: ReadState,
        header: ObjectHeader<'_>,
    ) -> Option<FileReadTask> {
        async fn inner(
            settings: &mut Settings,
            rs: ReadState,
            header: ObjectHeader<'_>,
        ) -> Result<Option<ReadState>, FileError> {
            let obj = match header.details {
                HeaderDetails::TwoByteFreeFormat(_, FreeFormatVariation::Group70Var5(obj)) => obj,
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

        match inner(&mut settings, rs, header).await {
            Ok(None) => {
                settings.reader.completed();
                Some(FileReadTask::new(settings, State::Close(rs.handle)))
            }
            Ok(Some(rs)) => Some(FileReadTask::new(settings, State::Read(rs))),
            Err(err) => {
                settings.reader.aborted(err);
                None
            }
        }
    }

    fn handle_close_response(header: ObjectHeader) -> Option<FileReadTask> {
        let _ = process_close_response(header);
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
        permissions: Permissions::default(),
        auth_key: key.into(),
        file_size: 0,
        mode: FileMode::Read,
        max_block_size: settings.config.max_block_size,
        request_id: REQUEST_ID,
        file_name: &settings.name.0,
    };
    writer.write_free_format(&obj)
}

fn write_read(rs: ReadState, writer: &mut HeaderWriter) -> Result<(), WriteError> {
    let obj = Group70Var5 {
        file_handle: rs.handle.into(),
        block_number: rs.block.wire_value(),
        file_data: &[],
    };
    writer.write_free_format(&obj)
}
