use crate::app::file::*;
use crate::app::format::write::HeaderWriter;
use crate::app::format::WriteError;
use crate::app::{FunctionCode, Timestamp};

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

pub(crate) struct Settings {
    pub(crate) name: Filename,
    pub(crate) max_block_size: u16,
}

/// States of the file transfer
enum State {
    /// Obtain and authentication key using file name and authentication data
    GetAuth(AuthData),
    /// Open the file - We might state in this state w/ the default AuthKey if auth not required
    Open(AuthKey),
    /// Read the next block
    Read(FileHandle, BlockNumber),
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
    pub(crate) fn auth(settings: Settings, auth: AuthData) -> Self {
        Self {
            settings,
            state: State::GetAuth(auth),
        }
    }

    pub(crate) fn open(settings: Settings) -> Self {
        Self {
            settings,
            state: State::Open(AuthKey::default()),
        }
    }

    pub(crate) fn function(&self) -> FunctionCode {
        match self.state {
            State::GetAuth(_) => FunctionCode::AuthenticateFile,
            State::Open(_) => FunctionCode::OpenFile,
            State::Read(_, _) => FunctionCode::Read,
            State::Close(_) => FunctionCode::CloseFile,
        }
    }

    pub(crate) fn write(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        match &self.state {
            State::GetAuth(auth) => write_auth(auth, writer),
            State::Open(auth) => write_open(&self.settings, *auth, writer),
            State::Read(handle, block) => write_read(*handle, *block, writer),
            State::Close(handle) => write_close(*handle, writer),
        }
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

fn write_read(
    handle: FileHandle,
    block: BlockNumber,
    writer: &mut HeaderWriter,
) -> Result<(), WriteError> {
    let obj = Group70Var5 {
        file_handle: handle.0,
        block_number: block.0,
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
