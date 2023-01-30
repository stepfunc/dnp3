use crate::app::file::*;
use crate::app::format::write::HeaderWriter;
use crate::app::format::WriteError;
use crate::app::parse::free_format::FreeFormatVariation;
use crate::app::parse::parser::{HeaderDetails, ObjectHeader, Response};
use crate::app::{FunctionCode, Timestamp};
use crate::master::tasks::NonReadTask;
use crate::master::{FileReadError, FileReader, TaskError};

pub(crate) struct AuthData {
    pub(crate) user_name: String,
    pub(crate) password: String,
}

pub(crate) struct Filename(pub(crate) String);

#[derive(Copy, Clone, Default)]
struct AuthKey(u32);
#[derive(Copy, Clone, Default)]
struct FileHandle(u32);
#[derive(Copy, Clone, Default)]
struct BlockNumber(u32);

impl BlockNumber {
    const LAST_BIT: u32 = 0x80_00_00_00;
    const BOTTOM_BITS: u32 = !Self::LAST_BIT;

    fn is_last(self) -> bool {
        (self.0 & Self::LAST_BIT) != 0
    }

    fn bottom_bits(self) -> u32 {
        self.0 & Self::BOTTOM_BITS
    }
}

impl PartialEq for BlockNumber {
    fn eq(&self, other: &Self) -> bool {
        let b1 = self.0 & Self::BOTTOM_BITS;
        let b2 = other.0 & Self::BOTTOM_BITS;
        b1 == b2
    }
}

pub(crate) struct Settings {
    pub(crate) name: Filename,
    pub(crate) max_block_size: u16,
    pub(crate) reader: Box<dyn FileReader>,
}

#[derive(Copy, Clone)]
struct ReadState {
    handle: FileHandle,
    block: BlockNumber,
}

impl ReadState {
    fn new(handle: FileHandle) -> Self {
        Self {
            handle,
            block: Default::default(),
        }
    }
}

/// States of the file transfer
enum State {
    /// Obtain and authentication key using file name and authentication data
    GetAuth(AuthData),
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

impl FileReadTask {
    fn new(settings: Settings, state: State) -> Self {
        Self { settings, state }
    }

    pub(crate) fn auth(settings: Settings, auth: AuthData) -> Self {
        Self::new(settings, State::GetAuth(auth))
    }

    pub(crate) fn open(
        file_name: String,
        max_block_size: u16,
        reader: Box<dyn FileReader>,
    ) -> Self {
        let settings = Settings {
            name: Filename(file_name),
            max_block_size,
            reader,
        };
        Self::new(settings, State::Open(AuthKey::default()))
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
        self.settings.reader.aborted(FileReadError::TaskError(err));
    }

    pub(crate) fn handle(mut self, response: Response) -> Option<NonReadTask> {
        let headers = match response.objects {
            Ok(x) => x,
            Err(err) => {
                tracing::warn!("File operation received malformed response: {err}");
                self.settings
                    .reader
                    .aborted(FileReadError::TaskError(TaskError::MalformedResponse(err)));
                return None;
            }
        };

        let header = match headers.get_only_header() {
            None => {
                tracing::warn!("File operation response contains unexpected number of headers");
                self.settings.reader.aborted(FileReadError::TaskError(
                    TaskError::UnexpectedResponseHeaders,
                ));
                return None;
            }
            Some(x) => x,
        };

        let next = match self.state {
            State::GetAuth(_) => Self::handle_auth_response(self.settings, header),
            State::Open(_) => Self::handle_open_response(self.settings, header),
            State::Read(rs) => Self::handle_read_response(self.settings, rs, header),
            State::Close(_) => Self::handle_close_response(header),
        };

        next.map(NonReadTask::FileRead)
    }

    fn handle_auth_response(mut settings: Settings, header: ObjectHeader) -> Option<FileReadTask> {
        fn inner(header: ObjectHeader) -> Result<State, FileReadError> {
            let obj = match header.details {
                HeaderDetails::TwoByteFreeFormat(_, FreeFormatVariation::Group70Var2(obj)) => obj,
                _ => {
                    tracing::warn!(
                        "File AUTHENTICATE response contains unexpected variation: {}",
                        header.variation
                    );
                    return Err(FileReadError::BadResponse);
                }
            };

            if obj.auth_key == 0 {
                tracing::warn!("Outstation returned auth key == 0: no permission to access file");
                return Err(FileReadError::NoPermission);
            }

            Ok(State::Open(AuthKey(obj.auth_key)))
        }

        match inner(header) {
            Ok(state) => Some(FileReadTask::new(settings, state)),
            Err(err) => {
                settings.reader.aborted(err);
                None
            }
        }
    }

    fn handle_open_response(mut settings: Settings, header: ObjectHeader) -> Option<FileReadTask> {
        fn inner(header: ObjectHeader) -> Result<(u32, FileHandle), FileReadError> {
            let obj = match header.details {
                HeaderDetails::TwoByteFreeFormat(_, FreeFormatVariation::Group70Var4(obj)) => obj,
                _ => {
                    tracing::warn!(
                        "File OPEN response contains unexpected variation: {}",
                        header.variation
                    );
                    return Err(FileReadError::BadResponse);
                }
            };

            if obj.status_code != FileStatus::Success {
                tracing::warn!("Unable to open file (status code == {:?})", obj.status_code);
                return Err(FileReadError::BadStatus(obj.status_code));
            }

            Ok((obj.file_size, FileHandle(obj.file_handle)))
        }

        match inner(header) {
            Ok((file_size, handle)) => {
                if settings.reader.opened(file_size) {
                    Some(FileReadTask::new(
                        settings,
                        State::Read(ReadState::new(handle)),
                    ))
                } else {
                    tracing::warn!("File transfer aborted by user");
                    Some(FileReadTask::new(settings, State::Close(handle)))
                }
            }
            Err(err) => {
                settings.reader.aborted(err);
                None
            }
        }
    }

    fn handle_read_response(
        mut settings: Settings,
        rs: ReadState,
        header: ObjectHeader,
    ) -> Option<FileReadTask> {
        fn inner(
            reader: &mut dyn FileReader,
            rs: ReadState,
            header: ObjectHeader,
        ) -> Result<Option<ReadState>, FileReadError> {
            let obj = match header.details {
                HeaderDetails::TwoByteFreeFormat(_, FreeFormatVariation::Group70Var5(obj)) => obj,
                _ => {
                    tracing::warn!(
                        "File READ response contains unexpected variation: {}",
                        header.variation
                    );
                    return Err(FileReadError::BadResponse);
                }
            };

            let rx_block = BlockNumber(obj.block_number);

            if rx_block.bottom_bits() != rs.block.bottom_bits() {
                tracing::warn!(
                    "Expected file block {} but received block {}",
                    rs.block.bottom_bits(),
                    rx_block.bottom_bits()
                );
                return Err(FileReadError::BadBlockNum);
            }

            if !reader.block_received(rx_block.bottom_bits(), obj.file_data) {
                tracing::warn!("File transfer aborted by user");
                return Err(FileReadError::AbortByUser);
            }

            Ok(if rx_block.is_last() {
                None
            } else {
                Some(ReadState {
                    handle: rs.handle,
                    block: BlockNumber(obj.block_number + 1),
                })
            })
        }

        match inner(settings.reader.as_mut(), rs, header) {
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
        let obj = match header.details {
            HeaderDetails::TwoByteFreeFormat(_, FreeFormatVariation::Group70Var4(obj)) => obj,
            _ => {
                tracing::warn!(
                    "File READ response contains unexpected variation: {}",
                    header.variation
                );
                return None;
            }
        };

        if obj.status_code != FileStatus::Success {
            tracing::warn!(
                "Unable to close file (status code == {:?})",
                obj.status_code
            );
        }

        None
    }
}

fn write_auth(auth: &AuthData, writer: &mut HeaderWriter) -> Result<(), WriteError> {
    let obj = Group70Var2 {
        auth_key: 0,
        user_name: &auth.user_name,
        password: &auth.password,
    };
    writer.write_free_format(&obj)
}

fn write_open(
    settings: &Settings,
    key: AuthKey,
    writer: &mut HeaderWriter,
) -> Result<(), WriteError> {
    let obj = Group70Var3 {
        time_of_creation: Timestamp::zero(),
        permissions: Permissions::default(),
        auth_key: key.0,
        file_size: 0,
        mode: FileMode::Read,
        max_block_size: settings.max_block_size,
        request_id: 4, // TODO
        file_name: &settings.name.0,
    };
    writer.write_free_format(&obj)
}

fn write_read(rs: ReadState, writer: &mut HeaderWriter) -> Result<(), WriteError> {
    let obj = Group70Var5 {
        file_handle: rs.handle.0,
        block_number: rs.block.0,
        file_data: &[],
    };
    writer.write_free_format(&obj)
}

fn write_close(handle: FileHandle, writer: &mut HeaderWriter) -> Result<(), WriteError> {
    let obj = Group70Var4 {
        file_handle: handle.0,
        file_size: 0,
        max_block_size: 0,
        request_id: 5, // TODO
        status_code: FileStatus::Success,
        text: "",
    };
    writer.write_free_format(&obj)
}
