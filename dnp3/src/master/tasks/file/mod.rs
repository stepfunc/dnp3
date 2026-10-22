use crate::app::format::write::HeaderWriter;
use crate::app::format::WriteError;
use crate::app::parse::free_format::FreeFormatVariation;
use crate::app::parse::parser::{HeaderDetails, ObjectHeader};
use crate::app::{FileStatus, Group70Var2, Group70Var3, Group70Var4, Permissions, Timestamp};
use crate::master::{AuthKey, FileCredentials, FileError, FileHandle, FileMode, OpenFile};

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
pub(crate) const REQUEST_ID: u16 = u16::from_le_bytes(*b"SF");

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

pub(super) fn write_open(
    file_name: &str,
    auth_key: AuthKey,
    permissions: Permissions,
    file_size: u32,
    mode: FileMode,
    max_block_size: u16,
    writer: &mut HeaderWriter,
) -> Result<(), WriteError> {
    let obj = Group70Var3 {
        time_of_creation: Timestamp::zero(),
        permissions,
        auth_key: auth_key.into(),
        file_size,
        mode,
        max_block_size,
        request_id: REQUEST_ID,
        file_name,
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

pub(super) fn parse_auth_response(header: ObjectHeader) -> Result<AuthKey, FileError> {
    let obj = match header.details {
        HeaderDetails::FreeFormat(FreeFormatVariation::Group70Var2(obj)) => obj,
        _ => {
            tracing::warn!(
                "File AUTHENTICATE response contains unexpected variation: {}",
                header.variation()
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

pub(super) fn parse_open_response(header: ObjectHeader) -> Result<OpenFile, FileError> {
    let obj = parse_status_object(header, "OPEN")?;

    if obj.status_code != FileStatus::Success {
        tracing::warn!("Unable to open file (status code == {:?})", obj.status_code);
        return Err(FileError::BadStatus(obj.status_code));
    }

    Ok(OpenFile {
        max_block_size: obj.max_block_size,
        file_handle: FileHandle::new(obj.file_handle),
        file_size: obj.file_size,
    })
}

/// Check that a CLOSE response is the status object for `handle`, and that it reports success
pub(super) fn check_close_response(
    handle: FileHandle,
    header: ObjectHeader<'_>,
) -> Result<(), FileError> {
    let obj = parse_status_object(header, "CLOSE")?;

    if obj.file_handle != u32::from(handle) {
        tracing::warn!(
            "File CLOSE response has handle {:#010X}, expected {:#010X}",
            obj.file_handle,
            u32::from(handle)
        );
        return Err(FileError::WrongHandle);
    }

    if obj.status_code != FileStatus::Success {
        tracing::warn!(
            "Unable to close file (status code == {:?})",
            obj.status_code
        );
        return Err(FileError::BadStatus(obj.status_code));
    }

    Ok(())
}

/// Extract the g70v4 status object that answers an OPEN or CLOSE
fn parse_status_object<'a>(
    header: ObjectHeader<'a>,
    function: &str,
) -> Result<Group70Var4<'a>, FileError> {
    match header.details {
        HeaderDetails::FreeFormat(FreeFormatVariation::Group70Var4(obj)) => Ok(obj),
        _ => {
            tracing::warn!(
                "File {function} response contains unexpected variation: {}",
                header.variation()
            );
            Err(FileError::BadResponse)
        }
    }
}
