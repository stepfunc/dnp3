use crate::app::format::write::HeaderWriter;
use crate::app::format::WriteError;
use crate::app::parse::free_format::FreeFormatVariation;
use crate::app::parse::parser::{HeaderDetails, ObjectHeader};
use crate::app::{FileStatus, Group70Var2, Group70Var4};
use crate::master::{AuthKey, FileCredentials, FileError, FileHandle};

pub(crate) mod authenticate;
pub(crate) mod close;
pub(crate) mod directory;
pub(crate) mod get_info;
pub(crate) mod open;
pub(crate) mod read;
pub(crate) mod write_block;

pub(crate) struct Filename(pub(crate) String);

// we don't really care what the ID is as we don't support polling for file stuff
// we can just be cute and write Step Function (SF) on the wire.
pub(crate) const REQUEST_ID: u16 = u16::from_le_bytes([b'S', b'F']);

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
        file_handle: handle.into(),
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

    Ok(AuthKey::new(obj.auth_key))
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

    Ok((obj.file_size, FileHandle::new(obj.file_handle)))
}

fn process_close_response(header: ObjectHeader) -> Result<(), FileError> {
    let obj = match header.details {
        HeaderDetails::TwoByteFreeFormat(1, FreeFormatVariation::Group70Var4(obj)) => obj,
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
