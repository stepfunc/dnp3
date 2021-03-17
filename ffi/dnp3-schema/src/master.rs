use std::time::Duration;

use oo_bindgen::callback::{InterfaceHandle, OneTimeCallbackHandle};
use oo_bindgen::class::ClassHandle;
use oo_bindgen::native_function::*;
use oo_bindgen::native_struct::*;
use oo_bindgen::*;

use crate::shared::SharedDefinitions;
use oo_bindgen::native_enum::NativeEnumHandle;

pub fn define(lib: &mut LibraryBuilder, shared: &SharedDefinitions) -> Result<(), BindingError> {
    let read_handler = crate::handler::define(lib, shared)?;

    let endpoint_list = define_endpoint_list(lib)?;

    let master_config = define_master_config(lib, shared)?;

    let tcp_client_state_listener = define_tcp_client_state_listener(lib)?;

    let master_class = lib.declare_class("Master")?;

    let master_create_tcp_session_fn = lib
        .declare_native_function("master_create_tcp_session")?
        .param("runtime", Type::ClassRef(shared.runtime_class.clone()), "Runtime to use to drive asynchronous operations of the master")?
        .param("link_error_mode", Type::Enum(shared.link_error_mode.clone()), "Controls how link errors are handled with respect to the TCP session")?
        .param("config", Type::Struct(master_config.clone()), "Master configuration")?
        .param("endpoints", Type::ClassRef(endpoint_list.declaration()), "List of IP endpoints.")?
        .param("connect_strategy", Type::Struct(shared.retry_strategy.clone()), "Connection retry strategy to use")?
        .param("reconnect_delay", Type::Duration(DurationMapping::Milliseconds), "delay before reconnecting after a disconnect")?
        .param("listener", Type::Interface(tcp_client_state_listener), "TCP connection listener used to receive updates on the status of the connection")?
        .return_type(ReturnType::new(Type::ClassRef(master_class.clone()), "Handle to the master created, {null} if an error occurred"))?
        .doc(
            doc("Create a master TCP session connecting to the specified endpoint(s)")
                .details("The returned master must be gracefully shutdown with {class:Master.[destructor]} when done.")
        )?
        .build()?;

    let master_create_serial_session_fn = lib
        .declare_native_function("master_create_serial_session")?
        .param("runtime", Type::ClassRef(shared.runtime_class.clone()), "Runtime to use to drive asynchronous operations of the master")?
        .param("config", Type::Struct(master_config), "Master configuration")?
        .param("path", Type::String, "Path to the serial device. Generally /dev/tty0 on Linux and COM1 on Windows.")?
        .param("serial_params", Type::Struct(shared.serial_port_settings.clone()), "Serial port settings")?
        .param("open_retry_delay", Type::Duration(DurationMapping::Milliseconds), "delay between attempts to open the serial port")?
        .param("listener", Type::Interface(shared.port_state_listener.clone()), "Listener to receive updates on the status of the serial port")?
        .return_type(ReturnType::new(Type::ClassRef(master_class.clone()), "Handle to the master created, {null} if an error occurred"))?
        .doc(
            doc("Create a master session on the specified serial port")
                .details("The returned master must be gracefully shutdown with {class:Master.[destructor]} when done.")
        )?
        .build()?;

    let destroy_fn = lib
        .declare_native_function("master_destroy")?
        .param(
            "master",
            Type::ClassRef(master_class.clone()),
            "Master to destroy",
        )?
        .return_type(ReturnType::void())?
        .doc(
            doc("Remove and destroy a master.")
                .warning("This method must NOT be called from within the {class:Runtime} thread."),
        )?
        .build()?;

    let enable_fn = lib
        .declare_native_function("master_enable")?
        .param(
            "master",
            Type::ClassRef(master_class.clone()),
            "master to enable",
        )?
        .return_type(ReturnType::Void)?
        .doc("start communications")?
        .build()?;

    let disable_fn = lib
        .declare_native_function("master_disable")?
        .param(
            "master",
            Type::ClassRef(master_class.clone()),
            "master to disable",
        )?
        .return_type(ReturnType::Void)?
        .doc("stop communications")?
        .build()?;

    let association_id = define_association_id(lib)?;
    let poll_id = define_poll_id(lib)?;

    let association_config = define_association_config(lib, shared)?;

    let time_provider_interface = define_time_provider(lib)?;

    let request_class = crate::request::define(lib, shared)?;

    let add_association_fn = lib
        .declare_native_function("master_add_association")?
        .param(
            "master",
            Type::ClassRef(master_class.clone()),
            "Master on which the association will be created",
        )?
        .param(
            "address",
            Type::Uint16,
            "DNP3 data-link address of the remote outstation",
        )?
        .param(
            "config",
            Type::Struct(association_config),
            "Association configuration",
        )?
        .param(
            "read_handler",
            Type::Interface(read_handler),
            "Interface uses to load measurement data",
        )?
        .param(
            "time_provider",
            Type::Interface(time_provider_interface),
            "Time provider for the association",
        )?
        .return_type(ReturnType::new(
            Type::Struct(association_id.clone()),
            "Id of the association",
        ))?
        .doc("Add an association to the master")?
        .build()?;

    let remove_association_fn = lib
        .declare_native_function("master_remove_association")?
        .param(
            "master",
            Type::ClassRef(master_class.clone()),
            "Master on which to apply the operation",
        )?
        .param(
            "id",
            Type::Struct(association_id.clone()),
            "Id of the association",
        )?
        .return_type(ReturnType::Type(
            Type::Bool,
            "true if the operation was successful".into(),
        ))?
        .doc("Remove an association from the master")?
        .build()?;

    let add_poll_fn = lib.declare_native_function("master_add_poll")?
        .param(
            "master",
            Type::ClassRef(master_class.clone()),
            "Master on which to apply the operation",
        )?
        .param("id", Type::Struct(association_id.clone()), "Association on which to add the poll")?
        .param("request", Type::ClassRef(request_class.declaration()), "Request to perform")?
        .param("period", Type::Duration(DurationMapping::Milliseconds), "Period to wait between each poll (in ms)")?
        .return_type(ReturnType::new(Type::Struct(poll_id.clone()), "Id of the created poll"))?
        .doc(
            doc("Add a periodic poll to an association")
                .details("Each result of the poll will be sent to the {interface:ReadHandler} of the association.")
                .warning("This cannot be called from within a callback.")
        )?
        .build()?;

    let remove_poll_fn = lib.declare_native_function("master_remove_poll")?
        .param(
            "master",
            Type::ClassRef(master_class.clone()),
            "Master on which to apply the operation",
        )?
        .param("poll_id", Type::Struct(poll_id.clone()), "Id of the created poll")?
        .return_type(ReturnType::Void)?
        .doc(
            doc("Add a periodic poll to an association")
                .details("Each result of the poll will be sent to the {interface:ReadHandler} of the association.")
                .warning("This cannot be called from within a callback.")
        )?
        .build()?;

    let demand_poll_fn = lib.declare_native_function("master_demand_poll")?
        .param(
            "master",
            Type::ClassRef(master_class.clone()),
            "Master on which to apply the operation",
        )?
        .param("poll_id", Type::Struct(poll_id), "Id of the poll")?
        .return_type(ReturnType::void())?
        .doc(
            doc("Demand the immediate execution of a poll previously created with {class:Master.AddPoll()}.")
                .details("This method returns immediately. The result will be sent to the registered {interface:ReadHandler}.")
                .details("This method resets the internal timer of the poll.")
        )?
        .build()?;

    let set_decode_level_fn = lib
        .declare_native_function("master_set_decode_level")?
        .param(
            "master",
            Type::ClassRef(master_class.clone()),
            "Master on which to apply the operation",
        )?
        .param(
            "decode_level",
            Type::Struct(shared.decode_level.clone()),
            "Decoding level",
        )?
        .return_type(ReturnType::void())?
        .doc("Set the master decoding level for log messages")?
        .build()?;

    let get_decode_level_fn = lib
        .declare_native_function("master_get_decode_level")?
        .param(
            "master",
            Type::ClassRef(master_class.clone()),
            "{class:Master} to get the decode level from",
        )?
        .return_type(ReturnType::new(
            Type::Struct(shared.decode_level.clone()),
            "Decode level",
        ))?
        .doc(
            doc("Get the master decoding level for log messages")
                .warning("This cannot be called from within a callback."),
        )?
        .build()?;

    let read_callback = define_read_callback(lib)?;

    let read_fn = lib
        .declare_native_function("master_read")?
        .param("master", Type::ClassRef(master_class.clone()), "Master on which to perform the operation")?
        .param("association", Type::Struct(association_id.clone()), "Association on which to perform the read")?
        .param("request", Type::ClassRef(request_class.declaration()), "Request to send")?
        .param("callback", Type::OneTimeCallback(read_callback), "Callback that will be invoked once the read is complete")?
        .return_type(ReturnType::void())?
        .doc(
            doc("Perform a read on the association.")
                .details("The callback will be called once the read is completely received, but the actual values will be sent to the {interface:ReadHandler} of the association.")
        )?
        .build()?;

    let command = define_command_builder(lib, shared)?;
    let command_mode = define_command_mode(lib)?;
    let command_cb = define_command_callback(lib)?;

    let operate_fn = lib
        .declare_native_function("master_operate")?
        .param(
            "master",
            Type::ClassRef(master_class.clone()),
            "Master on which to perform the operation",
        )?
        .param(
            "association",
            Type::Struct(association_id.clone()),
            "Id of the association",
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
        .doc("Asynchronously perform a command operation on the association")?
        .build()?;

    let time_sync_mode = define_time_sync_mode(lib)?;
    let time_sync_cb = define_time_sync_callback(lib)?;

    let perform_time_sync_fn = lib
        .declare_native_function("master_sync_time")?
        .param(
            "master",
            Type::ClassRef(master_class.clone()),
            "Master on which to perform the operation",
        )?
        .param(
            "association",
            Type::Struct(association_id.clone()),
            "Id of the association",
        )?
        .param("mode", Type::Enum(time_sync_mode), "Time sync mode")?
        .param(
            "callback",
            Type::OneTimeCallback(time_sync_cb),
            "Callback that will receive the result of the time sync",
        )?
        .return_type(ReturnType::void())?
        .doc("Asynchronously perform a time sync operation to the association")?
        .build()?;

    let restart_cb = define_restart_callback(lib)?;

    let cold_restart_fn = lib
        .declare_native_function("master_cold_restart")?
        .param(
            "master",
            Type::ClassRef(master_class.clone()),
            "Master on which to perform the operation",
        )?
        .param(
            "association",
            Type::Struct(association_id.clone()),
            "Id of the association",
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
        .declare_native_function("master_warm_restart")?
        .param(
            "master",
            Type::ClassRef(master_class.clone()),
            "Master on which to perform the operation",
        )?
        .param(
            "association",
            Type::Struct(association_id.clone()),
            "Id of the association",
        )?
        .param(
            "callback",
            Type::OneTimeCallback(restart_cb),
            "Callback that will receive the result of the restart",
        )?
        .return_type(ReturnType::void())?
        .doc("Asynchronously perform a warm restart operation to the association")?
        .build()?;

    let link_status_cb = define_link_status_callback(lib)?;

    let check_link_status_fn = lib
        .declare_native_function("master_check_link_status")?
        .param(
            "master",
            Type::ClassRef(master_class.clone()),
            "Master on which to perform the operation",
        )?
        .param(
            "association",
            Type::Struct(association_id),
            "Id of the association",
        )?
        .param(
            "callback",
            Type::OneTimeCallback(link_status_cb),
            "Callback that will receive the result of the link status",
        )?
        .return_type(ReturnType::void())?
        .doc("Asynchronously perform a link status check")?
        .build()?;

    lib.define_class(&master_class)?
        .destructor(&destroy_fn)?
        .static_method("CreateTCPSession", &master_create_tcp_session_fn)?
        .static_method("CreateSerialSession", &master_create_serial_session_fn)?
        .method("Enable", &enable_fn)?
        .method("Disable", &disable_fn)?
        .method("AddAssociation", &add_association_fn)?
        .method("RemoveAssociation", &remove_association_fn)?
        .method("SetDecodeLevel", &set_decode_level_fn)?
        .method("GetDecodeLevel", &get_decode_level_fn)?
        .method("AddPoll", &add_poll_fn)?
        .method("RemovePoll", &remove_poll_fn)?
        .method("DemandPoll", &demand_poll_fn)?
        .async_method("Read", &read_fn)?
        .async_method("Operate", &operate_fn)?
        .async_method("SynchronizeTime", &perform_time_sync_fn)?
        .async_method("ColdRestart", &cold_restart_fn)?
        .async_method("WarmRestart", &warm_restart_fn)?
        .async_method("CheckLinkStatus", &check_link_status_fn)?
        .doc(
            doc("Master channel of communication")
            .details("To communicate with a particular outstation, you need to add an association with {class:Master.AddAssociation()}.")
            .warning("This cannot be called from within a callback.")
        )?
        .build()?;

    Ok(())
}

fn define_association_id(
    lib: &mut LibraryBuilder,
) -> std::result::Result<NativeStructHandle, BindingError> {
    let id = lib.declare_native_struct("AssociationId")?;
    lib.define_native_struct(&id)?
        .make_opaque()
        .add(
            "address",
            Type::Uint16,
            "Outstation address of the association",
        )?
        .doc("Association identifier")?
        .build()
}

fn define_poll_id(
    lib: &mut LibraryBuilder,
) -> std::result::Result<NativeStructHandle, BindingError> {
    let id = lib.declare_native_struct("PollId")?;
    lib.define_native_struct(&id)?
        .make_opaque()
        .add(
            "association_id",
            Type::Uint16,
            "Outstation address of the association",
        )?
        .add(
            "id",
            Type::Uint64,
            "Unique poll id assigned by the association",
        )?
        .doc("Poll identifier")?
        .build()
}

fn define_read_callback(
    lib: &mut LibraryBuilder,
) -> std::result::Result<OneTimeCallbackHandle, BindingError> {
    let read_result = lib
        .define_native_enum("ReadResult")?
        .push("Success", "Read was perform successfully")?
        .push("TaskError", "The read was not performed properly")?
        .doc("Result of a read operation")?
        .build()?;

    lib.define_one_time_callback("ReadTaskCallback", "Handler for read tasks")?
        .callback(
            "on_complete",
            "Called when the read task reached completion or failed",
        )?
        .param("result", Type::Enum(read_result), "Result of the read task")?
        .return_type(ReturnType::void())?
        .build()?
        .build()
}

fn define_association_config(
    lib: &mut LibraryBuilder,
    shared: &SharedDefinitions,
) -> std::result::Result<NativeStructHandle, BindingError> {
    let event_classes = define_event_classes(lib)?;
    let classes = define_classes(lib)?;

    let auto_time_sync_enum = lib
        .define_native_enum("AutoTimeSync")?
        .push("None", "Do not perform automatic time sync")?
        .push(
            "Lan",
            "Perform automatic time sync with Record Current Time (0x18) function code",
        )?
        .push(
            "NonLan",
            "Perform automatic time sync with Delay Measurement (0x17) function code",
        )?
        .doc("Automatic time synchronization configuration")?
        .build()?;

    let association_config = lib.declare_native_struct("AssociationConfig")?;
    lib
        .define_native_struct(&association_config)?
        .add(
            "disable_unsol_classes",
            Type::Struct(event_classes.clone()),
            "Classes to disable unsolicited responses at startup",
        )?
        .add(
            "enable_unsol_classes",
            Type::Struct(event_classes.clone()),
            "Classes to enable unsolicited responses at startup",
        )?
        .add(
            "startup_integrity_classes",
            Type::Struct(classes),
            doc("Startup integrity classes to ask on master startup and when an outstation restart is detected.").details("For conformance, this should be Class 1230.")
        )?
        .add(
            "auto_time_sync",
            StructElementType::Enum(auto_time_sync_enum, Some("None".to_string())),
            "Automatic time synchronization configuration",
        )?
        .add(
            "auto_tasks_retry_strategy",
            Type::Struct(shared.retry_strategy.clone()),
            "Automatic tasks retry strategy",
        )?
        .add("keep_alive_timeout",
             StructElementType::Duration(DurationMapping::Seconds, Some(Duration::from_secs(60))),
             doc("Delay of inactivity before sending a REQUEST_LINK_STATUS to the outstation").details("A value of zero means no automatic keep-alive.")
        )?
        .add("auto_integrity_scan_on_buffer_overflow",
             StructElementType::Bool(Some(true)),
             doc("Automatic integrity scan when an EVENT_BUFFER_OVERFLOW is detected")
        )?
        .add("event_scan_on_events_available",
             Type::Struct(event_classes),
             doc("Classes to automatically send reads when the IIN bit is asserted")
        )?
        .doc("Association configuration")?
        .build()
}

fn define_tcp_client_state_listener(
    lib: &mut LibraryBuilder,
) -> std::result::Result<InterfaceHandle, BindingError> {
    let client_state_enum = lib
        .define_native_enum("ClientState")?
        .push("Disabled", "Client is disabled and idle until disabled")?
        .push(
            "Connecting",
            "Client is trying to establish a connection to the remote device",
        )?
        .push("Connected", "Client is connected to the remote device")?
        .push(
            "WaitAfterFailedConnect",
            "Failed to establish a connection, waiting before retrying",
        )?
        .push(
            "WaitAfterDisconnect",
            "Client was disconnected, waiting before retrying",
        )?
        .push("Shutdown", "Client is shutting down")?
        .doc(
            doc("State of the client connection.")
                .details("Use by the {interface:ClientStateListener}."),
        )?
        .build()?;

    lib.define_interface(
        "ClientStateListener",
        "Callback for monitoring the client TCP connection state",
    )?
    .callback("on_change", "Called when the client state changed")?
    .param("state", Type::Enum(client_state_enum), "New state")?
    .return_type(ReturnType::void())?
    .build()?
    .destroy_callback("on_destroy")?
    .build()
}

fn define_master_config(
    lib: &mut LibraryBuilder,
    shared: &SharedDefinitions,
) -> std::result::Result<NativeStructHandle, BindingError> {
    let master_config = lib.declare_native_struct("MasterConfig")?;
    lib.define_native_struct(&master_config)?
        .add("address", Type::Uint16, "Local DNP3 data-link address")?
        .add("decode_level", StructElementType::Struct(shared.decode_level.clone()), "Decoding level for this master. You can modify this later on with {class:Master.SetDecodeLevel()}.")?
        .add(
            "response_timeout",
            StructElementType::Duration(DurationMapping::Milliseconds, Some(Duration::from_secs(5))),
            "Timeout for receiving a response"
        )?
        .add("tx_buffer_size", StructElementType::Uint16(Some(2048)), doc("TX buffer size").details("Should be at least 249"))?
        .add("rx_buffer_size", StructElementType::Uint16(Some(2048)), doc("RX buffer size").details("Should be at least 2048"))?
        .doc("Master configuration")?
        .build()
}

fn define_endpoint_list(
    lib: &mut LibraryBuilder,
) -> std::result::Result<ClassHandle, BindingError> {
    let endpoint_list_class = lib.declare_class("EndpointList")?;

    let endpoint_list_new = lib.declare_native_function("endpoint_list_new")?
        .param("main_endpoint", Type::String, "Main endpoint")?
        .return_type(ReturnType::new(Type::ClassRef(endpoint_list_class.clone()), "New endpoint list"))?
        .doc(doc("Create a new list of IP endpoints.").details("You can write IP addresses or DNS names and the port to connect to. e.g. \"127.0.0.1:20000\" or \"dnp3.myorg.com:20000\"."))?
        .build()?;

    let endpoint_list_destroy = lib
        .declare_native_function("endpoint_list_destroy")?
        .param(
            "list",
            Type::ClassRef(endpoint_list_class.clone()),
            "Endpoint list to destroy",
        )?
        .return_type(ReturnType::void())?
        .doc("Delete a previously allocated endpoint list.")?
        .build()?;

    let endpoint_list_add = lib.declare_native_function("endpoint_list_add")?
        .param("list", Type::ClassRef(endpoint_list_class.clone()), "Endpoint list to modify")?
        .param("endpoint", Type::String, "Endpoint to add to the list")?
        .return_type(ReturnType::void())?
        .doc(doc(".").details("You can write IP addresses or DNS names and the port to connect to. e.g. \"127.0.0.1:20000\" or \"dnp3.myorg.com:20000\"."))?
        .build()?;

    let endpoint_list_class = lib.define_class(&endpoint_list_class)?
        .constructor(&endpoint_list_new)?
        .destructor(&endpoint_list_destroy)?
        .method("Add", &endpoint_list_add)?
        .doc(doc("List of IP endpoints.").details("You can write IP addresses or DNS names and the port to connect to. e.g. \"127.0.0.1:20000\" or \"dnp3.myorg.com:20000\"."))?
        .build()?;

    Ok(endpoint_list_class)
}

fn define_time_provider(lib: &mut LibraryBuilder) -> Result<InterfaceHandle, BindingError> {
    let timestamp_struct = lib.declare_native_struct("TimeProviderTimestamp")?;
    let timestamp_struct = lib.define_native_struct(&timestamp_struct)?
        .add("value", Type::Uint64, doc("Value of the timestamp (in milliseconds from UNIX Epoch).").warning("Only 48 bits are available for timestamps."))?
        .add("is_valid", Type::Bool, "True if the timestamp is valid, false otherwise.")?
        .doc(doc("Timestamp value returned by {interface:TimeProvider}.").details("{struct:TimeProviderTimestamp.value} is only valid if {struct:TimeProviderTimestamp.is_valid} is true."))?
        .build()?;

    let valid_constructor = lib
        .declare_native_function("timeprovidertimestamp_valid")?
        .param(
            "value",
            Type::Uint64,
            "Timestamp value in milliseconds from UNIX Epoch",
        )?
        .return_type(ReturnType::new(
            Type::Struct(timestamp_struct.clone()),
            "Timestamp",
        ))?
        .doc("Create a valid timestamp value")?
        .build()?;

    let invalid_constructor = lib
        .declare_native_function("timeprovidertimestamp_invalid")?
        .return_type(ReturnType::new(
            Type::Struct(timestamp_struct.clone()),
            "Timestamp",
        ))?
        .doc("Create an invalid timestamp value")?
        .build()?;

    lib.define_struct(&timestamp_struct)?
        .static_method("valid", &valid_constructor)?
        .static_method("invalid", &invalid_constructor)?
        .build();

    lib.define_interface("TimeProvider", "Current time provider")?
        .callback(
            "get_time",
            doc("Returns the current time of the system.")
                .details("This callback is called when time synchronization is performed.")
                .details(
                    "This can use external clock synchronization or the system clock for example.",
                ),
        )?
        .return_type(ReturnType::new(
            Type::Struct(timestamp_struct),
            "The current time",
        ))?
        .build()?
        .destroy_callback("on_destroy")?
        .build()
}

fn define_event_classes(lib: &mut LibraryBuilder) -> Result<NativeStructHandle, BindingError> {
    let event_classes = lib.declare_native_struct("EventClasses")?;
    let event_classes = lib
        .define_native_struct(&event_classes)?
        .add("class1", Type::Bool, "Class 1 events")?
        .add("class2", Type::Bool, "Class 2 events")?
        .add("class3", Type::Bool, "Class 3 events")?
        .doc("Event classes")?
        .build()?;

    let event_classes_all_fn = lib
        .declare_native_function("event_classes_all")?
        .return_type(ReturnType::Type(
            Type::Struct(event_classes.clone()),
            "Initialized value".into(),
        ))?
        .doc("Initialize all three event classes to true")?
        .build()?;

    let event_classes_none_fn = lib
        .declare_native_function("event_classes_none")?
        .return_type(ReturnType::Type(
            Type::Struct(event_classes.clone()),
            "Initialized value".into(),
        ))?
        .doc("Initialize all three event classes to false")?
        .build()?;

    lib.define_struct(&event_classes)?
        .static_method("all", &event_classes_all_fn)?
        .static_method("none", &event_classes_none_fn)?
        .build();

    Ok(event_classes)
}

fn define_classes(lib: &mut LibraryBuilder) -> Result<NativeStructHandle, BindingError> {
    let classes = lib.declare_native_struct("Classes")?;
    let classes = lib
        .define_native_struct(&classes)?
        .add("class0", Type::Bool, "Class 0 (static data)")?
        .add("class1", Type::Bool, "Class 1 events")?
        .add("class2", Type::Bool, "Class 2 events")?
        .add("class3", Type::Bool, "Class 3 events")?
        .doc("Class 0, 1, 2 and 3 config")?
        .build()?;

    let classes_all_fn = lib
        .declare_native_function("classes_all")?
        .return_type(ReturnType::new(Type::Struct(classes.clone()), "Class 1230"))?
        .doc("Class 1230")?
        .build()?;

    let classes_none_fn = lib
        .declare_native_function("classes_none")?
        .return_type(ReturnType::new(Type::Struct(classes.clone()), "No class"))?
        .doc("No class")?
        .build()?;

    lib.define_struct(&classes)?
        .static_method("all", &classes_all_fn)?
        .static_method("none", &classes_none_fn)?
        .build();

    Ok(classes)
}

fn define_command_mode(
    lib: &mut LibraryBuilder,
) -> std::result::Result<NativeEnumHandle, BindingError> {
    lib.define_native_enum("CommandMode")?
        .push("DirectOperate", "Perform a Direct Operate (0x05)")?
        .push(
            "SelectBeforeOperate",
            "Perform a Select & Operate (0x03 then 0x04)",
        )?
        .doc("Command operation mode")?
        .build()
}

fn define_command_callback(
    lib: &mut LibraryBuilder,
) -> std::result::Result<OneTimeCallbackHandle, BindingError> {
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

    lib.define_one_time_callback("CommandTaskCallback", "Handler for command tasks")?
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
        .build()
}

fn define_command_builder(
    lib: &mut LibraryBuilder,
    shared: &SharedDefinitions,
) -> std::result::Result<ClassHandle, BindingError> {
    let command = lib.declare_class("Commands")?;

    let command_new_fn = lib
        .declare_native_function("commands_new")?
        .return_type(ReturnType::new(
            Type::ClassRef(command.clone()),
            "Handle to the created set of commands",
        ))?
        .doc("Create a new set of commands")?
        .build()?;

    let command_destroy_fn = lib
        .declare_native_function("commands_destroy")?
        .param(
            "command",
            Type::ClassRef(command.clone()),
            "Set of commands to destroy",
        )?
        .return_type(ReturnType::void())?
        .doc("Destroy set of commands")?
        .build()?;

    let command_finish_header_fn = lib
        .declare_native_function("commands_finish_header")?
        .param(
            "commands",
            Type::ClassRef(command.clone()),
            "Commands on which to finish the header",
        )?
        .return_type(ReturnType::void())?
        .doc("Finish any partially completed header. This allows for the construction of two headers with the same type and index")?
        .build()?;

    let command_add_u8_g12v1_fn = lib
        .declare_native_function("commands_add_g12v1_u8")?
        .param(
            "command",
            Type::ClassRef(command.clone()),
            "Commands to modify",
        )?
        .param(
            "idx",
            Type::Uint8,
            "Index of the point to send the command to",
        )?
        .param(
            "header",
            Type::Struct(shared.g12v1_struct.clone()),
            "CROB data",
        )?
        .return_type(ReturnType::void())?
        .doc("Add a CROB with 1-byte prefix index")?
        .build()?;

    let command_add_u16_g12v1_fn = lib
        .declare_native_function("commands_add_g12v1_u16")?
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
            Type::Struct(shared.g12v1_struct.clone()),
            "CROB data",
        )?
        .return_type(ReturnType::void())?
        .doc("Add a CROB with 2-byte prefix index")?
        .build()?;

    let command_add_u8_g41v1_fn = lib
        .declare_native_function("commands_add_g41v1_u8")?
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
        .declare_native_function("commands_add_g41v1_u16")?
        .param(
            "command",
            Type::ClassRef(command.clone()),
            "Commands to modify",
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
        .declare_native_function("commands_add_g41v2_u8")?
        .param(
            "command",
            Type::ClassRef(command.clone()),
            "Commands to modify",
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
        .declare_native_function("commands_add_g41v2_u16")?
        .param(
            "command",
            Type::ClassRef(command.clone()),
            "Commands to modify",
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
        .declare_native_function("commands_add_g41v3_u8")?
        .param(
            "command",
            Type::ClassRef(command.clone()),
            "Commands to modify",
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
        .declare_native_function("commands_add_g41v3_u16")?
        .param(
            "commands",
            Type::ClassRef(command.clone()),
            "Commands to modify",
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
        .declare_native_function("commands_add_g41v4_u8")?
        .param(
            "commands",
            Type::ClassRef(command.clone()),
            "Commands to modify",
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
        .declare_native_function("commands_add_g41v4_u16")?
        .param(
            "commands",
            Type::ClassRef(command.clone()),
            "Commands to modify",
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

    lib.define_class(&command)?
        .constructor(&command_new_fn)?
        .destructor(&command_destroy_fn)?
        .method("AddG12V1U8", &command_add_u8_g12v1_fn)?
        .method("AddG12V1U16", &command_add_u16_g12v1_fn)?
        .method("AddG41V1U8", &command_add_u8_g41v1_fn)?
        .method("AddG41V1U16", &command_add_u16_g41v1_fn)?
        .method("AddG41V2U8", &command_add_u8_g41v2_fn)?
        .method("AddG41V2U16", &command_add_u16_g41v2_fn)?
        .method("AddG41V3U8", &command_add_u8_g41v3_fn)?
        .method("AddG41V3U16", &command_add_u16_g41v3_fn)?
        .method("AddG41V4U8", &command_add_u8_g41v4_fn)?
        .method("AddG41V4U16", &command_add_u16_g41v4_fn)?
        .method("FinishHeader", &command_finish_header_fn)?
        .doc("Builder type used to construct command requests")?
        .build()
}

fn define_time_sync_callback(
    lib: &mut LibraryBuilder,
) -> Result<OneTimeCallbackHandle, BindingError> {
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

    lib.define_one_time_callback(
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
    .build()
}

fn define_time_sync_mode(lib: &mut LibraryBuilder) -> Result<NativeEnumHandle, BindingError> {
    lib.define_native_enum("TimeSyncMode")?
        .push(
            "Lan",
            "Perform a LAN time sync with Record Current Time (0x18) function code",
        )?
        .push(
            "NonLan",
            "Perform a non-LAN time sync with Delay Measurement (0x17) function code",
        )?
        .doc("Time synchronization mode")?
        .build()
}

fn define_restart_callback(
    lib: &mut LibraryBuilder,
) -> Result<OneTimeCallbackHandle, BindingError> {
    let restart_success = lib
        .define_native_enum("RestartSuccess")?
        .push("Success", "Restart was perform successfully")?
        .push("TaskError", "The restart was not performed properly")?
        .doc("Result of a read operation")?
        .build()?;

    let restart_result = lib.declare_native_struct("RestartResult")?;
    let restart_result = lib.define_native_struct(&restart_result)?
        .add("success", Type::Enum(restart_success), "Success status of the restart task")?
        .add("delay", Type::Duration(DurationMapping::Milliseconds), "Delay value returned by the outstation. Valid only if {struct:RestartResult.success} is {enum:RestartSuccess.Success}.")?
        .doc("Result of a restart task")?
        .build()?;

    lib.define_one_time_callback("RestartTaskCallback", "Handler for restart tasks")?
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
        .build()
}

fn define_link_status_callback(
    lib: &mut LibraryBuilder,
) -> Result<OneTimeCallbackHandle, BindingError> {
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
        .doc("Result of a link status check. See {class:Master.CheckLinkStatus()}")?
        .build()?;

    lib.define_one_time_callback("LinkStatusCallback", "Handler for link status check")?
        .callback("on_complete", "Called when a link status is received")?
        .param(
            "result",
            Type::Enum(link_status_enum),
            "Result of the link status",
        )?
        .return_type(ReturnType::void())?
        .build()?
        .build()
}
