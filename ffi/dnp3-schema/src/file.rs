use crate::master::TaskErrors;
use oo_bindgen::model::*;

pub(crate) struct FileDefinitions {
    pub(crate) permissions: UniversalStructHandle,
    pub(crate) file_error: ErrorType<Unvalidated>,
    pub(crate) file_info: UniversalStructHandle,
    pub(crate) file_mode: EnumHandle,
    pub(crate) file_open_cb: FutureInterfaceHandle,
    pub(crate) file_op_cb: FutureInterfaceHandle,
    pub(crate) file_auth_cb: FutureInterfaceHandle,
}

pub(crate) fn define(lib: &mut LibraryBuilder, nothing: EnumHandle) -> BackTraced<FileDefinitions> {
    let file_type = define_file_type(lib)?;
    let permissions = define_permissions(lib)?;
    let open_file = define_open_file(lib)?;
    let file_error = define_file_error(lib)?;

    let file_info = lib.declare_universal_struct("file_info")?;
    let file_info = lib
        .define_universal_struct(file_info)?
        .doc(
            doc("Information about a file or directory returned from the outstation")
                .details("This is a user-facing representation of Group 70 Variation 7")
        )?
        .add("file_name", StringType, "Name of the file or directory")?
        .add("file_type", file_type, "Simple file or directory")?
        .add("size", Primitive::U32, doc("Size of the file in bytes").details("If a directory, this represents the number of files and directories contained within."))?
        .add("time_created", Primitive::U64, doc("DNP3 timestamp").details("Milliseconds since January 1st, 1970 UTC. Only the lower 48-bits are used"))?
        .add("permissions", permissions.clone(), "Outstation permissions for the file")?
        .end_fields()?
        .build()?;

    let file_open_cb = lib.define_future_interface(
        "file_open_callback",
        "Callback interface used when opening a file",
        open_file.clone(),
        "Value describing the open file",
        file_error.clone(),
    )?;

    let file_op_cb = lib.define_future_interface(
        "file_operation_callback",
        "Callback interface used when closing a file or writing a block of file data",
        nothing,
        "Indicates a successful write operation",
        file_error.clone(),
    )?;

    let file_auth_cb = lib.define_future_interface(
        "file_auth_callback",
        "Callback interface used when obtaining an authentication key",
        Primitive::U32,
        "File authentication key",
        file_error.clone(),
    )?;

    Ok(FileDefinitions {
        permissions,
        file_error,
        file_info,
        file_mode: define_file_mode(lib)?,
        file_open_cb,
        file_op_cb,
        file_auth_cb,
    })
}

fn define_open_file(lib: &mut LibraryBuilder) -> BackTraced<UniversalStructHandle> {
    let open_file = lib.declare_universal_struct("open_file")?;
    let open_file = lib
        .define_universal_struct(open_file)?
        .doc("The result of opening a file on the outstation")?
        .add("file_handle", Primitive::U32,
             doc("The handle assigned to the file by the outstation")
            .details("This must be used in subsequent requests to manipulate the file")
        )?
        .add("file_size", Primitive::U32, "Size of the file returned by the outstation")?
        .add("max_block_size", Primitive::U16,
             doc("Maximum block size returned by the outstation")
                 .details("The master must respect this value when writing data to a file or the transfer may not succeed"))?
        .end_fields()?
        .build()?;
    Ok(open_file)
}

fn define_file_mode(lib: &mut LibraryBuilder) -> BackTraced<EnumHandle> {
    let value = lib
        .define_enum("file_mode")?
        .doc("Different modes in which files may be opened")?
        .push("read", "Specifies that an existing file is to be opened for reading")?
        .push("write", "Specifies that the file is to be opened for writing, truncating any existing file to length 0")?
        .push("append", "Specifies that the file is to be opened for writing, appending to the end of the file")?
        .build()?;

    Ok(value)
}

fn define_permissions(lib: &mut LibraryBuilder) -> BackTraced<UniversalStructHandle> {
    let permission_set = define_permission_set(lib)?;

    let world = Name::create("world")?;
    let group = Name::create("group")?;
    let owner = Name::create("owner")?;

    let permissions = lib.declare_universal_struct("permissions")?;
    let permissions = lib
        .define_universal_struct(permissions)?
        .doc("Permissions for world, group, and owner")?
        .add(world.clone(), permission_set.clone(), "World permissions")?
        .add(group.clone(), permission_set.clone(), "Group permissions")?
        .add(owner.clone(), permission_set, "Owner permissions")?
        .end_fields()?
        .add_full_initializer("init")?
        .begin_initializer(
            "none",
            InitializerType::Static,
            "Permissions with nothing enabled",
        )?
        .default_struct(&world)?
        .default_struct(&group)?
        .default_struct(&owner)?
        .end_initializer()?
        .build()?;

    Ok(permissions)
}

fn define_permission_set(lib: &mut LibraryBuilder) -> BackTraced<UniversalStructHandle> {
    let execute = Name::create("execute")?;
    let write = Name::create("write")?;
    let read = Name::create("read")?;

    let set = lib.declare_universal_struct("permission_set")?;
    let set = lib
        .define_universal_struct(set)?
        .doc("Defines read, write, execute permissions for particular group or user")?
        .add(execute.clone(), Primitive::Bool, "Permission to execute")?
        .add(write.clone(), Primitive::Bool, "Permission to write")?
        .add(read.clone(), Primitive::Bool, "Permission to read")?
        .end_fields()?
        .add_full_initializer("init")?
        .begin_initializer(
            "none",
            InitializerType::Normal,
            "Permission set with nothing enabled",
        )?
        .default(&execute, false)?
        .default(&write, false)?
        .default(&read, false)?
        .end_initializer()?
        .build()?;

    Ok(set)
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
        .add_error(
            "wrong_handle",
            "File handle returned by the outstation did not match the request",
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
