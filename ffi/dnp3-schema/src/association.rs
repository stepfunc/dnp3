use oo_bindgen::class::*;
use oo_bindgen::native_function::*;
use oo_bindgen::*;

use crate::shared::SharedDefinitions;

pub fn define(
    lib: &mut LibraryBuilder,
    shared_def: &SharedDefinitions,
) -> Result<ClassDeclarationHandle, BindingError> {
    let association_class = lib.declare_class("Association")?;

    let destroy_fn = lib
        .declare_native_function("association_destroy")?
        .param(
            "association",
            Type::ClassRef(association_class.clone()),
            "Association to destroy",
        )?
        .return_type(ReturnType::void())?
        .doc(doc("Remove an association").details(
            "This method will gracefully end all communications with this particular outstation.",
        ))?
        .build()?;

    // Poll stuff
    let poll = lib.declare_class("Poll")?;

    let poll_demand_fn = lib
        .declare_native_function("poll_demand")?
        .param(
            "poll",
            Type::ClassRef(poll.clone()),
            "Poll handle to demand",
        )?
        .return_type(ReturnType::void())?
        .doc(
                doc("Demand the immediate execution of a poll previously created with {class:Association.AddPoll()}.")
                .details("This method returns immediatly. The result will be sent to the registered {interface:ReadHandler}.")
                .details("This method resets the internal timer of the poll.")
        )?
        .build()?;

    let poll_destroy_fn = lib
        .declare_native_function("poll_destroy")?
        .param(
            "poll",
            Type::ClassRef(poll.clone()),
            "Poll handle to destroy",
        )?
        .return_type(ReturnType::void())?
        .doc("Remove a poll. The poll won't be performed again.")?
        .build()?;

    let poll = lib
        .define_class(&poll)?
        .destructor(&poll_destroy_fn)?
        .method("Demand", &poll_demand_fn)?
        .doc("Poll handle to demand executions of polls with {class:Poll.Demand()}.")?
        .build()?;

    let request_class = crate::request::define(lib, &shared_def)?;

    let add_poll_fn = lib.declare_native_function("association_add_poll")?
        .param("association", Type::ClassRef(association_class.clone()), "Association to add the poll to")?
        .param("request", Type::ClassRef(request_class.declaration()), "Request to perform")?
        .param("period", Type::Duration(DurationMapping::Milliseconds), "Period to wait between each poll (in ms)")?
        .return_type(ReturnType::new(Type::ClassRef(poll.declaration()), "Handle to the created poll."))?
        .doc(
            doc("Add a periodic poll to the association.")
            .details("Each result of the poll will be sent to the {interface:ReadHandler} of the association.")
            .warning("This cannot be called from within a callback.")
        )?
        .build()?;

    // Read stuff
    let read_result = lib
        .define_native_enum("ReadResult")?
        .push("Success", "Read was perform successfully")?
        .push("TaskError", "The read was not performed properly")?
        .doc("Result of a read operation")?
        .build()?;

    let read_cb = lib
        .define_one_time_callback("ReadTaskCallback", "Handler for read tasks")?
        .callback(
            "on_complete",
            "Called when the read task reached completion or failed",
        )?
        .param("result", Type::Enum(read_result), "Result of the read task")?
        .return_type(ReturnType::void())?
        .build()?
        .build()?;

    let read_fn = lib
        .declare_native_function("association_read")?
        .param("association", Type::ClassRef(association_class.clone()), "Association to read")?
        .param("request", Type::ClassRef(request_class.declaration()), "Request to send")?
        .param("callback", Type::OneTimeCallback(read_cb), "Callback that will be called once the read is complete")?
        .return_type(ReturnType::void())?
        .doc(
            doc("Perform a read on the association.")
            .details("The callback will be called once the read is completely received, but the actual values will be sent to the {interface:ReadHandler} of the association.")
        )?
        .build()?;

    // Command stuff
    let command_mode = lib
        .define_native_enum("CommandMode")?
        .push("DirectOperate", "Perform a Direct Operate (0x05)")?
        .push(
            "SelectBeforeOperate",
            "Perform a Select & Operate (0x03 then 0x04)",
        )?
        .doc("Command operation mode")?
        .build()?;

    let command = lib.declare_class("Command")?;

    let command_new_fn = lib
        .declare_native_function("command_new")?
        .return_type(ReturnType::new(
            Type::ClassRef(command.clone()),
            "Handle to the created command",
        ))?
        .doc("Create a new command")?
        .build()?;

    let command_destroy_fn = lib
        .declare_native_function("command_destroy")?
        .param(
            "command",
            Type::ClassRef(command.clone()),
            "Command to destroy",
        )?
        .return_type(ReturnType::void())?
        .doc("Destroy command")?
        .build()?;

    let command_add_u8_g12v1_fn = lib
        .declare_native_function("command_add_u8_g12v1")?
        .param(
            "command",
            Type::ClassRef(command.clone()),
            "Command to modify",
        )?
        .param(
            "idx",
            Type::Uint8,
            "Index of the point to send the command to",
        )?
        .param(
            "header",
            Type::Struct(shared_def.g12v1_struct.clone()),
            "CROB data",
        )?
        .return_type(ReturnType::void())?
        .doc("Add a CROB with 1-byte prefix index")?
        .build()?;

    let command_add_u16_g12v1_fn = lib
        .declare_native_function("command_add_u16_g12v1")?
        .param(
            "command",
            Type::ClassRef(command.clone()),
            "Command to modify",
        )?
        .param(
            "idx",
            Type::Uint16,
            "Index of the point to send the command to",
        )?
        .param(
            "header",
            Type::Struct(shared_def.g12v1_struct.clone()),
            "CROB data",
        )?
        .return_type(ReturnType::void())?
        .doc("Add a CROB with 2-byte prefix index")?
        .build()?;

    let command_add_u8_g41v1_fn = lib
        .declare_native_function("command_add_u8_g41v1")?
        .param(
            "command",
            Type::ClassRef(command.clone()),
            "Command to modify",
        )?
        .param(
            "idx",
            Type::Uint8,
            "Index of the point to send the command to",
        )?
        .param("value", Type::Sint32, "Value to set the analog output to")?
        .return_type(ReturnType::void())?
        .doc("Add a Analog Output command (signed 32-bit integer) with 1-byte prefix index")?
        .build()?;

    let command_add_u16_g41v1_fn = lib
        .declare_native_function("command_add_u16_g41v1")?
        .param(
            "command",
            Type::ClassRef(command.clone()),
            "Command to modify",
        )?
        .param(
            "idx",
            Type::Uint16,
            "Index of the point to send the command to",
        )?
        .param("value", Type::Sint32, "Value to set the analog output to")?
        .return_type(ReturnType::void())?
        .doc("Add a Analog Output command (signed 32-bit integer) with 2-byte prefix index")?
        .build()?;

    let command_add_u8_g41v2_fn = lib
        .declare_native_function("command_add_u8_g41v2")?
        .param(
            "command",
            Type::ClassRef(command.clone()),
            "Command to modify",
        )?
        .param(
            "idx",
            Type::Uint8,
            "Index of the point to send the command to",
        )?
        .param("value", Type::Sint16, "Value to set the analog output to")?
        .return_type(ReturnType::void())?
        .doc("Add a Analog Output command (signed 16-bit integer) with 1-byte prefix index")?
        .build()?;

    let command_add_u16_g41v2_fn = lib
        .declare_native_function("command_add_u16_g41v2")?
        .param(
            "command",
            Type::ClassRef(command.clone()),
            "Command to modify",
        )?
        .param(
            "idx",
            Type::Uint16,
            "Index of the point to send the command to",
        )?
        .param("value", Type::Sint16, "Value to set the analog output to")?
        .return_type(ReturnType::void())?
        .doc("Add a Analog Output command (signed 16-bit integer) with 2-byte prefix index")?
        .build()?;

    let command_add_u8_g41v3_fn = lib
        .declare_native_function("command_add_u8_g41v3")?
        .param(
            "command",
            Type::ClassRef(command.clone()),
            "Command to modify",
        )?
        .param(
            "idx",
            Type::Uint8,
            "Index of the point to send the command to",
        )?
        .param("value", Type::Float, "Value to set the analog output to")?
        .return_type(ReturnType::void())?
        .doc("Add a Analog Output command (single-precision float) with 1-byte prefix index")?
        .build()?;

    let command_add_u16_g41v3_fn = lib
        .declare_native_function("command_add_u16_g41v3")?
        .param(
            "command",
            Type::ClassRef(command.clone()),
            "Command to modify",
        )?
        .param(
            "idx",
            Type::Uint16,
            "Index of the point to send the command to",
        )?
        .param("value", Type::Float, "Value to set the analog output to")?
        .return_type(ReturnType::void())?
        .doc("Add a Analog Output command (single-precision float) with 2-byte prefix index")?
        .build()?;

    let command_add_u8_g41v4_fn = lib
        .declare_native_function("command_add_u8_g41v4")?
        .param(
            "command",
            Type::ClassRef(command.clone()),
            "Command to modify",
        )?
        .param(
            "idx",
            Type::Uint8,
            "Index of the point to send the command to",
        )?
        .param("value", Type::Double, "Value to set the analog output to")?
        .return_type(ReturnType::void())?
        .doc("Add a Analog Output command (double-precision float) with 1-byte prefix index")?
        .build()?;

    let command_add_u16_g41v4_fn = lib
        .declare_native_function("command_add_u16_g41v4")?
        .param(
            "command",
            Type::ClassRef(command.clone()),
            "Command to modify",
        )?
        .param(
            "idx",
            Type::Uint16,
            "Index of the point to send the command to",
        )?
        .param("value", Type::Double, "Value to set the analog output to")?
        .return_type(ReturnType::void())?
        .doc("Add a Analog Output command (double-precision float) with 2-byte prefix index")?
        .build()?;

    let command = lib
        .define_class(&command)?
        .constructor(&command_new_fn)?
        .destructor(&command_destroy_fn)?
        .method("AddU8G12V1", &command_add_u8_g12v1_fn)?
        .method("AddU16G12V1", &command_add_u16_g12v1_fn)?
        .method("AddU8G41V1", &command_add_u8_g41v1_fn)?
        .method("AddU16G41V1", &command_add_u16_g41v1_fn)?
        .method("AddU8G41V2", &command_add_u8_g41v2_fn)?
        .method("AddU16G41V2", &command_add_u16_g41v2_fn)?
        .method("AddU8G41V3", &command_add_u8_g41v3_fn)?
        .method("AddU16G41V3", &command_add_u16_g41v3_fn)?
        .method("AddU8G41V4", &command_add_u8_g41v4_fn)?
        .method("AddU16G41V4", &command_add_u16_g41v4_fn)?
        .doc("Command handle used to send SBO or DO commands")?
        .build()?;

    let command_result = lib
        .define_native_enum("CommandResult")?
        .push("Success", "Command was a success")?
        .push("TaskError", "Failed b/c of a generic task execution error")?
        .push(
            "BadStatus",
            "Outstation indicated that a command was not SUCCESS",
        )?
        .push(
            "HeaderCountMismatch",
            "Number of headers in the response doesn't match the number in the request",
        )?
        .push(
            "HeaderTypeMismatch",
            "Header in the response doesn't match the request",
        )?
        .push(
            "ObjectCountMismatch",
            "Number of objects in one of the headers doesn't match the request",
        )?
        .push(
            "ObjectValueMismatch",
            "Value in one of the objects in the response doesn't match the request",
        )?
        .doc("Result of a command")?
        .build()?;

    let command_cb = lib
        .define_one_time_callback("CommandTaskCallback", "Handler for command tasks")?
        .callback(
            "on_complete",
            "Called when the command task reached completion or failed",
        )?
        .param(
            "result",
            Type::Enum(command_result),
            "Result of the command task",
        )?
        .return_type(ReturnType::void())?
        .build()?
        .build()?;

    let operate_fn = lib
        .declare_native_function("association_operate")?
        .param(
            "association",
            Type::ClassRef(association_class.clone()),
            "Association to send the command to",
        )?
        .param("mode", Type::Enum(command_mode), "Operation mode")?
        .param(
            "command",
            Type::ClassRef(command.declaration()),
            "Command to send",
        )?
        .param(
            "callback",
            Type::OneTimeCallback(command_cb),
            "Callback that will receive the result of the command",
        )?
        .return_type(ReturnType::void())?
        .doc("Asynchronously send a command to the association")?
        .build()?;

    // Time sync stuff
    let timesync_mode = lib
        .define_native_enum("TimeSyncMode")?
        .push(
            "Lan",
            "Perform a LAN time sync with Record Current Time (0x18) function code",
        )?
        .push(
            "NonLan",
            "Perform a non-LAN time sync with Delay Measurement (0x17) function code",
        )?
        .doc("Time synchronization mode")?
        .build()?;

    let timesync_result = lib
        .define_native_enum("TimeSyncResult")?
        .push("Success", "Time synchronization operation was a success")?
        .push("TaskError", "Failed b/c of a generic task execution error")?
        .push("ClockRollback", "Detected a clock rollback")?
        .push(
            "SystemTimeNotUnix",
            "The system time cannot be converted to a Unix timestamp",
        )?
        .push(
            "BadOutstationTimeDelay",
            "Outstation time delay exceeded the response delay",
        )?
        .push("Overflow", "Overflow in calculation")?
        .push(
            "StillNeedsTime",
            "Outstation did not clear the NEED_TIME IIN bit",
        )?
        .push("SystemTimeNotAvailable", "System time not available")?
        .push("IinError", "Outstation indicated an error")?
        .doc("Result of a time sync operation")?
        .build()?;

    let timesync_cb = lib
        .define_one_time_callback(
            "TimeSyncTaskCallback",
            "Handler for time synchronization tasks",
        )?
        .callback(
            "on_complete",
            "Called when the timesync task reached completion or failed",
        )?
        .param(
            "result",
            Type::Enum(timesync_result),
            "Result of the time synchronization task",
        )?
        .return_type(ReturnType::void())?
        .build()?
        .build()?;

    let perform_time_sync_fn = lib
        .declare_native_function("association_perform_time_sync")?
        .param(
            "association",
            Type::ClassRef(association_class.clone()),
            "Association to perform the timesync to",
        )?
        .param("mode", Type::Enum(timesync_mode), "Timesync mode")?
        .param(
            "callback",
            Type::OneTimeCallback(timesync_cb),
            "Callback that will receive the result of the timesync",
        )?
        .return_type(ReturnType::void())?
        .doc("Asynchronously perform a timesync operation to the association")?
        .build()?;

    let restart_success = lib
        .define_native_enum("RestartSuccess")?
        .push("Success", "Restart was perform successfully")?
        .push("TaskError", "The restart was not performed properly")?
        .doc("Result of a read operation")?
        .build()?;

    let restart_result = lib.declare_native_struct("RestartResult")?;
    let restart_result = lib.define_native_struct(&restart_result)?
        .add("delay", Type::Duration(DurationMapping::Milliseconds), "Delay value returned by the outstation. Valid only if {struct:RestartResult.success} is {enum:RestartSuccess.Success}.")?
        .add("success", Type::Enum(restart_success), "Success status of the restart task")?
        .doc("Result of a restart task")?
        .build()?;

    let restart_cb = lib
        .define_one_time_callback("RestartTaskCallback", "Handler for restart tasks")?
        .callback(
            "on_complete",
            "Called when the restart task reached completion or failed",
        )?
        .param(
            "result",
            Type::Struct(restart_result),
            "Result of the restart task",
        )?
        .return_type(ReturnType::void())?
        .build()?
        .build()?;

    let cold_restart_fn = lib
        .declare_native_function("association_cold_restart")?
        .param(
            "association",
            Type::ClassRef(association_class.clone()),
            "Association to perform the cold restart",
        )?
        .param(
            "callback",
            Type::OneTimeCallback(restart_cb.clone()),
            "Callback that will receive the result of the restart",
        )?
        .return_type(ReturnType::void())?
        .doc("Asynchronously perform a cold restart operation to the association")?
        .build()?;

    let warm_restart_fn = lib
        .declare_native_function("association_warm_restart")?
        .param(
            "association",
            Type::ClassRef(association_class.clone()),
            "Association to perform the warm restart",
        )?
        .param(
            "callback",
            Type::OneTimeCallback(restart_cb),
            "Callback that will receive the result of the restart",
        )?
        .return_type(ReturnType::void())?
        .doc("Asynchronously perform a warm restart operation to the association")?
        .build()?;

    let link_status_enum = lib
        .define_native_enum("LinkStatusResult")?
        .push(
            "Success",
            "The outstation responded with a valid LINK_STATUS",
        )?
        .push(
            "UnexpectedResponse",
            "There was activity on the link, but it wasn't a LINK_STATUS",
        )?
        .push(
            "TaskError",
            "The task failed for some reason (e.g. the master was shutdown)",
        )?
        .doc("Result of a link status check. See {class:Association.CheckLinkStatus()}")?
        .build()?;

    let link_status_cb = lib
        .define_one_time_callback("LinkStatusCallback", "Handler for link status check")?
        .callback("on_complete", "Called when a link status is received")?
        .param(
            "result",
            Type::Enum(link_status_enum),
            "Result of the link status",
        )?
        .return_type(ReturnType::void())?
        .build()?
        .build()?;

    let check_link_status_fn = lib
        .declare_native_function("association_check_link_status")?
        .param(
            "association",
            Type::ClassRef(association_class.clone()),
            "Association to perform the link status check",
        )?
        .param(
            "callback",
            Type::OneTimeCallback(link_status_cb),
            "Callback that will receive the result of the link status",
        )?
        .return_type(ReturnType::void())?
        .doc("Asynchronously perform a link status check")?
        .build()?;

    let association_class = lib
        .define_class(&association_class)?
        .destructor(&destroy_fn)?
        .method("AddPoll", &add_poll_fn)?
        .async_method("Read", &read_fn)?
        .async_method("Operate", &operate_fn)?
        .async_method("PerformTimeSync", &perform_time_sync_fn)?
        .async_method("ColdRestart", &cold_restart_fn)?
        .async_method("WarmRestart", &warm_restart_fn)?
        .async_method("CheckLinkStatus", &check_link_status_fn)?
        .doc("Master-outstation association to interact with")?
        .build()?;

    Ok(association_class.declaration.clone())
}
