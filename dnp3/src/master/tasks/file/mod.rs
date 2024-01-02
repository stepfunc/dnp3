use crate::app::format::write::HeaderWriter;
use crate::app::format::WriteError;
use crate::app::parse::free_format::FreeFormatVariation;
use crate::app::parse::parser::{HeaderDetails, ObjectHeader, Response};
use crate::app::{FileStatus, Group70Var2, Group70Var4};
use crate::master::{FileCredentials, FileError, TaskError};

pub(crate) mod get_info;
pub(crate) mod read;
pub(crate) mod write;

pub(crate) struct Filename(pub(crate) String);

#[derive(Copy, Clone, Default)]
pub(super) struct AuthKey(pub(super) u32);
#[derive(Copy, Clone, Default)]
pub(super) struct FileHandle(pub(super) u32);
#[derive(Copy, Clone, Default)]
pub(super) struct BlockNumber(pub(super) u32);

impl BlockNumber {
    const LAST_BIT: u32 = 0x80_00_00_00;

    pub(super) const MAX_VALUE: u32 = !Self::LAST_BIT;

    pub(super) fn is_last(self) -> bool {
        (self.0 & Self::LAST_BIT) != 0
    }

    pub(super) fn set_last(self) -> Self {
        Self(self.0 | Self::LAST_BIT)
    }

    pub(super) fn bottom_bits(self) -> u32 {
        // The maximum value is also a mask for the bottom bits
        self.0 & Self::MAX_VALUE
    }

    pub(super) fn increment(&mut self) -> Result<(), u32> {
        if self.bottom_bits() < Self::MAX_VALUE {
            self.0 = self.bottom_bits() + 1;
            Ok(())
        } else {
            Err(Self::MAX_VALUE)
        }
    }
}

// TODO - see if this is really needed for correct behavior
impl PartialEq for BlockNumber {
    fn eq(&self, other: &Self) -> bool {
        let b1 = self.0 & Self::MAX_VALUE;
        let b2 = other.0 & Self::MAX_VALUE;
        b1 == b2
    }
}

// we don't really care what the ID is as we don't support polling for file stuff
// we can just be cute and write Step Function (SF) on the wire.
const REQUEST_ID: u16 = u16::from_le_bytes([b'S', b'F']);

pub(super) fn write_auth(
    credentials: &FileCredentials,
    writer: &mut HeaderWriter,
) -> Result<(), WriteError> {
    let obj = Group70Var2 {
        auth_key: 0,
        user_name: &credentials.user_name,
        password: &credentials.password,
    };
    writer.write_free_format(&obj)
}

pub(super) fn write_close(handle: FileHandle, writer: &mut HeaderWriter) -> Result<(), WriteError> {
    let obj = Group70Var4 {
        file_handle: handle.0,
        file_size: 0,
        max_block_size: 0,
        request_id: REQUEST_ID,
        status_code: FileStatus::Success,
        text: "",
    };
    writer.write_free_format(&obj)
}

pub(super) fn handle_auth_response(header: ObjectHeader) -> Result<AuthKey, FileError> {
    let obj = match header.details {
        HeaderDetails::TwoByteFreeFormat(_, FreeFormatVariation::Group70Var2(obj)) => obj,
        _ => {
            tracing::warn!(
                "File AUTHENTICATE response contains unexpected variation: {}",
                header.variation
            );
            return Err(FileError::BadResponse);
        }
    };

    if obj.auth_key == 0 {
        tracing::warn!("Outstation returned auth key == 0: no permission to access file");
        return Err(FileError::NoPermission);
    }

    Ok(AuthKey(obj.auth_key))
}

fn handle_open_response(header: ObjectHeader) -> Result<(u32, FileHandle), FileError> {
    let obj = match header.details {
        HeaderDetails::TwoByteFreeFormat(_, FreeFormatVariation::Group70Var4(obj)) => obj,
        _ => {
            tracing::warn!(
                "File OPEN response contains unexpected variation: {}",
                header.variation
            );
            return Err(FileError::BadResponse);
        }
    };

    if obj.status_code != FileStatus::Success {
        tracing::warn!("Unable to open file (status code == {:?})", obj.status_code);
        return Err(FileError::BadStatus(obj.status_code));
    }

    Ok((obj.file_size, FileHandle(obj.file_handle)))
}

fn get_only_header(response: Response) -> Result<ObjectHeader, TaskError> {
    let headers = match response.objects {
        Ok(x) => x,
        Err(err) => {
            tracing::warn!("File operation received malformed response: {err}");
            return Err(TaskError::MalformedResponse(err));
        }
    };

    let header = match headers.get_only_header() {
        None => {
            tracing::warn!("File operation response contains unexpected number of headers");
            return Err(TaskError::UnexpectedResponseHeaders);
        }
        Some(x) => x,
    };

    Ok(header)
}

fn process_close_response(header: ObjectHeader) -> Result<(), FileError> {
    let obj = match header.details {
        HeaderDetails::TwoByteFreeFormat(_, FreeFormatVariation::Group70Var4(obj)) => obj,
        _ => {
            tracing::warn!(
                "File CLOSE response contains unexpected variation: {}",
                header.variation
            );
            return Err(FileError::BadResponse);
        }
    };

    if obj.status_code != FileStatus::Success {
        tracing::warn!(
            "Unable to close file (status code == {:?})",
            obj.status_code
        );
        return Err(FileError::BadStatus(obj.status_code));
    }

    Ok(())
}
