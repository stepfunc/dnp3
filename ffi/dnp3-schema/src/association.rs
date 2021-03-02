use oo_bindgen::class::*;
use oo_bindgen::native_function::*;
use oo_bindgen::*;

use crate::shared::SharedDefinitions;

pub fn define(
    lib: &mut LibraryBuilder,
    shared_def: &SharedDefinitions,
) -> Result<ClassDeclarationHandle, BindingError> {

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
