use crate::master::TaskErrors;
use oo_bindgen::model::*;

pub(crate) struct FileDefinitions {
    // pub(crate) file_status: EnumHandle,
    pub(crate) file_error: ErrorType<Unvalidated>,
    pub(crate) file_info: CallbackArgStructHandle,
}

pub(crate) fn define(lib: &mut LibraryBuilder) -> BackTraced<FileDefinitions> {
    let file_type = define_file_type(lib)?;

    let permissions = define_permissions(lib)?;

    let file_info = lib.declare_callback_argument_struct("file_info")?;
    let file_info = lib
        .define_callback_argument_struct(file_info)?
        .doc(
            doc("Information about a file or directory returned from the outstation")
                .details("This is a user-facing representation of Group 70 Variation 7")
        )?
        .add("file_name", StringType, "Name of the file or directory")?
        .add("file_type", file_type, "Simple file or directory")?
        .add("size", Primitive::U32, doc("Size of the file in bytes").details("If a directory, this represents the number of files and directories contained within."))?
        .add("time_created", Primitive::U64, doc("DNP3 timestamp").details("Milliseconds since January 1st, 1970 UTC. Only the lower 48-bits are used"))?
        .add("permissions", permissions, "Outstation permissions for the file")?
        .end_fields()?
        .build()?;

    Ok(FileDefinitions {
        //file_status: define_file_status(lib)?,
        file_error: define_file_error(lib)?,
        file_info,
    })
}

fn define_permissions(lib: &mut LibraryBuilder) -> BackTraced<CallbackArgStructHandle> {
    let permission_set = lib.declare_callback_argument_struct("permission_set")?;
    let permission_set = lib
        .define_callback_argument_struct(permission_set)?
        .doc("Defines read, write, execute permissions for particular group or user")?
        .add("execute", Primitive::Bool, "Permission to execute")?
        .add("write", Primitive::Bool, "Permission to write")?
        .add("read", Primitive::Bool, "Permission to read")?
        .end_fields()?
        .build()?;

    let permissions = lib.declare_callback_argument_struct("permissions")?;
    let permissions = lib
        .define_callback_argument_struct(permissions)?
        .doc("Permissions for world, group, and owner")?
        .add("world", permission_set.clone(), "World permissions")?
        .add("group", permission_set.clone(), "Group permissions")?
        .add("owner", permission_set, "Owner permissions")?
        .end_fields()?
        .build()?;

    Ok(permissions)
}

fn define_file_error(lib: &mut LibraryBuilder) -> BackTraced<ErrorType<Unvalidated>> {
    let value = lib
        .define_error_type(
            "file_error",
            "file_exception",
            ExceptionType::UncheckedException,
        )?
        .doc("Errors that can occur during file transfer")?
        .add_error("bad_status", "Outstation returned an error status code")?
        .add_error(
            "no_permission",
            "Outstation indicated no permission to access file",
        )?
        .add_error("bad_block_num", "Received an unexpected block number")?
        .add_error("abort_by_user", "File transfer aborted by user")?
        .add_error(
            "max_length_exceeded",
            "Exceeded the maximum length specified by the user",
        )?
        .add_task_errors()?
        .build()?;

    Ok(value)
}

fn define_file_type(lib: &mut LibraryBuilder) -> BackTraced<EnumHandle> {
    let value = lib
        .define_enum("file_type")?
        .doc("File type enumeration used in Group 70 objects")?
        .push("directory", "File is a directory")?
        .push(
            "simple",
            "File is a simple file type suitable for sequential file transfer",
        )?
        .push("other", "Some other unspecified value")?
        .build()?;

    Ok(value)
}

/*
fn define_file_status(lib: &mut LibraryBuilder) -> BackTraced<EnumHandle> {
    let value = lib
        .define_enum("file_status")?
        .doc("File status enumeration used in Group 70 objects")?
        .push("success", "Requested operation was successful")?
        .push("permission_denied", "Permission was denied due to improper authentication key, user name, or password")?
        .push("invalid_mode", "An unsupported or unknown operation mode was requested")?
        .push("file_not_found", "Requested file does not exist")?
        .push("file_locked", "Requested file is already in use")?
        .push("too_many_open", "File could not be opened because of limit on the number of open files")?
        .push("invalid_handle", "There is no file opened with the handle in the request")?
        .push("write_block_size", "Outstation is unable to negotiate a suitable write block size")?
        .push("comm_lost", "Communications were lost or cannot be establishes with end device where file resides")?
        .push("cannot_abort", "An abort request was unsuccessful because the outstation is unable or not programmed to abort")?
        .push("not_opened", "File handle does not reference an opened file")?
        .push("handle_expired", "File closed due to inactivity timeout")?
        .push("buffer_overrun", "Too much file data was received for outstation to process")?
        .push("fatal", "An error occurred in the file processing that prevents any further activity with this file")?
        .push("block_seq", "The block number did not have the expected sequence number")?
        .push("undefined", "Some other error not listed here occurred")?
        .build()?;

    Ok(value)
}
*/
