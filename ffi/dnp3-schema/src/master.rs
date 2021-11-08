use oo_bindgen::class::ClassHandle;
use oo_bindgen::*;

use crate::shared::SharedDefinitions;
use oo_bindgen::enum_type::{EnumBuilder, EnumHandle};
use oo_bindgen::interface::InterfaceHandle;
use oo_bindgen::name::Name;
use oo_bindgen::structs::{
    ConstructorDefault, ConstructorType, FunctionArgStructHandle, Number, UniversalStructHandle,
};
use oo_bindgen::types::{BasicType, DurationType, StringType};
use std::time::Duration;

pub fn define(lib: &mut LibraryBuilder, shared: &SharedDefinitions) -> BackTraced<()> {
    let read_handler = crate::handler::define(lib, shared)?;

    let endpoint_list = define_endpoint_list(lib)?;

    let master_channel_config = define_master_channel_config(lib, shared)?;

    let tcp_client_state_listener = define_tcp_client_state_listener(lib)?;

    let master_channel_class = lib.declare_class("master_channel")?;

    let connect_strategy = define_connect_strategy(lib)?;

    let master_channel_create_tcp_fn = lib
        .define_function("master_channel_create_tcp")?
        .param(
            "runtime",
            shared.runtime_class.clone(),
            "Runtime to use to drive asynchronous operations of the master",
        )?
        .param(
            "link_error_mode",
            shared.link_error_mode.clone(),
            "Controls how link errors are handled with respect to the TCP session",
        )?
        .param(
            "config",
            master_channel_config.clone(),
            "Generic configuration for the channel",
        )?
        .param(
            "endpoints",
            endpoint_list.declaration(),
            "List of IP endpoints.",
        )?
        .param(
            "connect_strategy",
            connect_strategy,
            "Controls the timing of (re)connection attempts",
        )?
        .param(
            "listener",
            tcp_client_state_listener,
            "TCP connection listener used to receive updates on the status of the connection",
        )?
        .returns(
            master_channel_class.clone(),
            "Handle to the master created, {null} if an error occurred",
        )?
        .fails_with(shared.error_type.clone())?
        .doc("Create a master channel that connects to the specified TCP endpoint(s)")?
        .build()?;

    let master_channel_create_serial_fn = lib
        .define_function("master_channel_create_serial")?
        .param("runtime",shared.runtime_class.clone(), "Runtime to use to drive asynchronous operations of the master")?
        .param("config",master_channel_config, "Generic configuration for the channel")?
        .param("path", StringType, "Path to the serial device. Generally /dev/tty0 on Linux and COM1 on Windows.")?
        .param("serial_params",shared.serial_port_settings.clone(), "Serial port settings")?
        .param("open_retry_delay", DurationType::Milliseconds, "delay between attempts to open the serial port")?
        .param("listener",shared.port_state_listener.clone(), "Listener to receive updates on the status of the serial port")?
        .returns(master_channel_class.clone(), "Handle to the master created, {null} if an error occurred")?
        .fails_with(shared.error_type.clone())?
        .doc(
            doc("Create a master channel on the specified serial port")
                .details("The returned master must be gracefully shutdown with {class:master_channel.[destructor]} when done.")
        )?
        .build()?;

    let channel_destructor = lib.define_destructor(
        master_channel_class.clone(),
        "Shutdown a {class:master_channel} and release all resources",
    )?;

    let enable_method = lib
        .define_method("enable", master_channel_class.clone())?
        .returns_nothing()?
        .fails_with(shared.error_type.clone())?
        .doc("start communications")?
        .build()?;

    let disable_method = lib
        .define_method("disable", master_channel_class.clone())?
        .returns_nothing()?
        .fails_with(shared.error_type.clone())?
        .doc("stop communications")?
        .build()?;

    let association_id = define_association_id(lib)?;
    let poll_id = define_poll_id(lib)?;

    let association_config = define_association_config(lib, shared)?;

    let association_handler_interface = define_association_handler(lib)?;

    let request_class = crate::request::define(lib, shared)?;

    let add_association_method = lib
        .define_method("add_association", master_channel_class.clone())?
        .param(
            "address",
            BasicType::U16,
            "DNP3 data-link address of the remote outstation",
        )?
        .param("config", association_config, "Association configuration")?
        .param(
            "read_handler",
            read_handler,
            "Interface uses to load measurement data",
        )?
        .param(
            "association_handler",
            association_handler_interface,
            "Association specific callbacks such as time synchronization",
        )?
        .returns(association_id.clone(), "Id of the association")?
        .fails_with(shared.error_type.clone())?
        .doc("Add an association to the channel")?
        .build()?;

    let remove_association_method = lib
        .define_method("remove_association", master_channel_class.clone())?
        .param("id", association_id.clone(), "Id of the association")?
        .returns_nothing()?
        .fails_with(shared.error_type.clone())?
        .doc("Remove an association from the channel")?
        .build()?;

    let add_poll_method = lib.define_method("add_poll", master_channel_class.clone())?
        .param("id", association_id.clone(), "Association on which to add the poll")?
        .param("request", request_class.declaration(), "Request to perform")?
        .param("period", DurationType::Milliseconds, "Period to wait between each poll (in ms)")?
        .returns(poll_id.clone(), "Id of the created poll")?
        .fails_with(shared.error_type.clone())?
        .doc(
            doc("Add a periodic poll to an association")
                .details("Each result of the poll will be sent to the {interface:read_handler} of the association.")
        )?
        .build()?;

    let remove_poll_method = lib
        .define_method("remove_poll", master_channel_class.clone())?
        .param("poll_id", poll_id.clone(), "Id of the created poll")?
        .returns_nothing()?
        .fails_with(shared.error_type.clone())?
        .doc(
            doc("Add a periodic poll to an association")
                .details("Each result of the poll will be sent to the {interface:read_handler} of the association.")
        )?
        .build()?;

    let demand_poll_method = lib.define_method("demand_poll", master_channel_class.clone())?
        .param("poll_id", poll_id, "Id of the poll")?
        .returns_nothing()?
        .fails_with(shared.error_type.clone())?
        .doc(
            doc("Demand the immediate execution of a poll previously created with {class:master_channel.add_poll()}.")
                .details("This method returns immediately. The result will be sent to the registered {interface:read_handler}.")
                .details("This method resets the internal timer of the poll.")
        )?
        .build()?;

    let set_decode_level_method = lib
        .define_method("set_decode_level", master_channel_class.clone())?
        .param(
            "decode_level",
            shared.decode_level.clone(),
            "Decoding level",
        )?
        .returns_nothing()?
        .fails_with(shared.error_type.clone())?
        .doc("Set the decoding level for the channel")?
        .build()?;

    let get_decode_level_method = lib
        .define_method("get_decode_level", master_channel_class.clone())?
        .returns(shared.decode_level.clone(), "Decode level")?
        .fails_with(shared.error_type.clone())?
        .doc("Get the decoding level for the channel")?
        .build()?;

    let read_callback = define_read_callback(lib)?;

    let read_fn = lib
        .define_function("master_channel_read")?
        .param("channel",master_channel_class.clone(), "{class:master_channel} on which to perform the operation")?
        .param("association",association_id.clone(), "Association on which to perform the read")?
        .param("request",request_class.declaration(), "Request to send")?
        .param("callback", read_callback, "Callback that will be invoked once the read is complete")?
        .returns_nothing()?
        .fails_with(shared.error_type.clone())?
        .doc(
            doc("Perform a read on the association.")
                .details("The callback will be called once the read is completely received, but the actual values will be sent to the {interface:read_handler} of the association.")
        )?
        .build()?;

    let command = define_command_builder(lib, shared)?;
    let command_mode = define_command_mode(lib)?;
    let command_cb = define_command_callback(lib)?;

    let operate_fn = lib
        .define_function("master_channel_operate")?
        .param(
            "channel",
            master_channel_class.clone(),
            "{class:master_channel} on which to perform the operation",
        )?
        .param(
            "association",
            association_id.clone(),
            "Id of the association",
        )?
        .param("mode", command_mode, "Operation mode")?
        .param("command", command.declaration(), "Command to send")?
        .param(
            "callback",
            command_cb,
            "Callback that will receive the result of the command",
        )?
        .returns_nothing()?
        .fails_with(shared.error_type.clone())?
        .doc("Asynchronously perform a command operation on the association")?
        .build()?;

    let time_sync_mode = define_time_sync_mode(lib)?;
    let time_sync_cb = define_time_sync_callback(lib)?;

    let perform_time_sync_fn = lib
        .define_function("master_channel_sync_time")?
        .param(
            "channel",
            master_channel_class.clone(),
            "{class:master_channel} on which to perform the operation",
        )?
        .param(
            "association",
            association_id.clone(),
            "Id of the association",
        )?
        .param("mode", time_sync_mode, "Time sync mode")?
        .param(
            "callback",
            time_sync_cb,
            "Callback that will receive the result of the time sync",
        )?
        .returns_nothing()?
        .fails_with(shared.error_type.clone())?
        .doc("Asynchronously perform a time sync operation to the association")?
        .build()?;

    let restart_cb = define_restart_callback(lib)?;

    let cold_restart_fn = lib
        .define_function("master_channel_cold_restart")?
        .param(
            "channel",
            master_channel_class.clone(),
            "{class:master_channel} on which to perform the operation",
        )?
        .param(
            "association",
            association_id.clone(),
            "Id of the association",
        )?
        .param(
            "callback",
            restart_cb.clone(),
            "Callback that will receive the result of the restart",
        )?
        .returns_nothing()?
        .fails_with(shared.error_type.clone())?
        .doc("Asynchronously perform a cold restart operation to the association")?
        .build()?;

    let warm_restart_fn = lib
        .define_function("master_channel_warm_restart")?
        .param(
            "channel",
            master_channel_class.clone(),
            "{class:master_channel} on which to perform the operation",
        )?
        .param(
            "association",
            association_id.clone(),
            "Id of the association",
        )?
        .param(
            "callback",
            restart_cb,
            "Callback that will receive the result of the restart",
        )?
        .returns_nothing()?
        .fails_with(shared.error_type.clone())?
        .doc("Asynchronously perform a warm restart operation to the association")?
        .build()?;

    let link_status_cb = define_link_status_callback(lib)?;

    let check_link_status_fn = lib
        .define_function("master_channel_check_link_status")?
        .param(
            "channel",
            master_channel_class.clone(),
            "{class:master_channel} on which to perform the operation",
        )?
        .param("association", association_id, "Id of the association")?
        .param(
            "callback",
            link_status_cb,
            "Callback that will receive the result of the link status",
        )?
        .returns_nothing()?
        .fails_with(shared.error_type.clone())?
        .doc("Asynchronously perform a link status check")?
        .build()?;

    lib.define_class(&master_channel_class)?
        .destructor(channel_destructor)?
        .static_method("create_tcp_channel", &master_channel_create_tcp_fn)?
        .static_method("create_serial_channel", &master_channel_create_serial_fn)?
        .method(enable_method)?
        .method(disable_method)?
        .method(add_association_method)?
        .method(remove_association_method)?
        .method(set_decode_level_method)?
        .method(get_decode_level_method)?
        .method(add_poll_method)?
        .method(remove_poll_method)?
        .method(demand_poll_method)?
        .async_method("read", &read_fn)?
        .async_method("operate", &operate_fn)?
        .async_method("synchronize_time", &perform_time_sync_fn)?
        .async_method("cold_restart", &cold_restart_fn)?
        .async_method("warm_restart", &warm_restart_fn)?
        .async_method("check_link_status", &check_link_status_fn)?
        .custom_destroy("shutdown")?
        .doc(
            doc("Represents a communication channel for a master station")
            .details("To communicate with a particular outstation, you need to add an association with {class:master_channel.add_association()}.")
            .warning("The class methods that return a value (e.g. as {class:master_channel.add_association()}) cannot be called from within a callback.")
        )?
        .build()?;

    Ok(())
}

fn define_connect_strategy(lib: &mut LibraryBuilder) -> BackTraced<FunctionArgStructHandle> {
    let min_connect_delay = Name::create("min_connect_delay")?;
    let max_connect_delay = Name::create("max_connect_delay")?;
    let reconnect_delay = Name::create("reconnect_delay")?;

    let strategy = lib.declare_function_arg_struct("connect_strategy")?;
    let strategy = lib
        .define_function_argument_struct(strategy)?
        .add(
            &min_connect_delay,
            DurationType::Milliseconds,
            "Minimum delay between two connection attempts, doubles up to the maximum delay",
        )?
        .add(
            &max_connect_delay,
            DurationType::Milliseconds,
            "Maximum delay between two connection attempts",
        )?
        .add(
            &reconnect_delay,
            DurationType::Milliseconds,
            "Delay before attempting a connection after a disconnect",
        )?
        .doc("Timing parameters for connection attempts")?
        .end_fields()?
        .begin_constructor(
            "init",
            ConstructorType::Normal,
            "Initialize to default values",
        )?
        .default(&min_connect_delay, Duration::from_secs(1))?
        .default(&max_connect_delay, Duration::from_secs(10))?
        .default(&reconnect_delay, Duration::from_secs(1))?
        .end_constructor()?
        .build()?;

    Ok(strategy)
}

fn define_association_id(lib: &mut LibraryBuilder) -> BackTraced<UniversalStructHandle> {
    let id = lib.declare_universal_struct("association_id")?;
    let id = lib
        .define_opaque_struct(id)?
        .add(
            "address",
            BasicType::U16,
            "Outstation address of the association",
        )?
        .doc("Association identifier")?
        .end_fields()?
        .build()?;

    Ok(id)
}

fn define_poll_id(lib: &mut LibraryBuilder) -> BackTraced<UniversalStructHandle> {
    let id = lib.declare_universal_struct("poll_id")?;
    let id = lib
        .define_opaque_struct(id)?
        .add(
            "association_id",
            BasicType::U16,
            "Outstation address of the association",
        )?
        .add(
            "id",
            BasicType::U64,
            "Unique poll id assigned by the association",
        )?
        .doc("Poll identifier")?
        .end_fields()?
        .build()?;

    Ok(id)
}

fn define_read_callback(lib: &mut LibraryBuilder) -> BackTraced<InterfaceHandle> {
    let read_result = lib
        .define_enum("read_result")?
        .push("success", "Read was perform successfully")?
        .add_task_errors()?
        .doc("Result of a read operation")?
        .build()?;

    let callback = lib
        .define_asynchronous_interface("read_task_callback", "Handler for read tasks")?
        .begin_callback(
            "on_complete",
            "Called when the read task reached completion or failed",
        )?
        .param("result", read_result, "Result of the read task")?
        .returns_nothing()?
        .end_callback()?
        .build()?;

    Ok(callback)
}

fn define_association_config(
    lib: &mut LibraryBuilder,
    shared: &SharedDefinitions,
) -> BackTraced<FunctionArgStructHandle> {
    let event_classes = define_event_classes(lib)?;
    let classes = define_classes(lib)?;

    let auto_time_sync_enum = lib
        .define_enum("auto_time_sync")?
        .push("none", "Do not perform automatic time sync")?
        .push(
            "lan",
            "Perform automatic time sync with Record Current Time (0x18) function code",
        )?
        .push(
            "non_lan",
            "Perform automatic time sync with Delay Measurement (0x17) function code",
        )?
        .doc("Automatic time synchronization configuration")?
        .build()?;

    let auto_time_sync = Name::create("auto_time_sync")?;
    let auto_tasks_retry_strategy = Name::create("auto_tasks_retry_strategy")?;
    let keep_alive_timeout = Name::create("keep_alive_timeout")?;
    let auto_integrity_scan_on_buffer_overflow =
        Name::create("auto_integrity_scan_on_buffer_overflow")?;
    let max_queued_user_requests = Name::create("max_queued_user_requests")?;
    let association_config = lib.declare_function_arg_struct("association_config")?;

    let association_config = lib
        .define_function_argument_struct(association_config)?
        .doc("Association configuration")?
        .add(
            "disable_unsol_classes",
          event_classes.clone(),
            "Classes to disable unsolicited responses at startup",
        )?
        .add(
            "enable_unsol_classes",
          event_classes.clone(),
            "Classes to enable unsolicited responses at startup",
        )?
        .add(
            "startup_integrity_classes",
          classes,
            doc("Startup integrity classes to ask on master startup and when an outstation restart is detected.").details("For conformance, this should be Class 1230.")
        )?
        .add(
            &auto_time_sync,
            auto_time_sync_enum,
            "Automatic time synchronization configuration",
        )?
        .add(
            &auto_tasks_retry_strategy,
          shared.retry_strategy.clone(),
            "Automatic tasks retry strategy",
        )?
        .add(&keep_alive_timeout,
             DurationType::Seconds,
             doc("Delay of inactivity before sending a REQUEST_LINK_STATUS to the outstation").details("A value of zero means no automatic keep-alive.")
        )?
        .add(&auto_integrity_scan_on_buffer_overflow,
             BasicType::Bool,
             doc("Automatic integrity scan when an EVENT_BUFFER_OVERFLOW is detected")
        )?
        .add("event_scan_on_events_available",
           event_classes,
             doc("Classes to automatically send reads when the IIN bit is asserted")
        )?
        .add(&max_queued_user_requests,
            BasicType::U16,
            doc("maximum number of user requests (e.g. commands, adhoc reads, etc) that will be queued before back-pressure is applied by failing requests")
        )?
        .end_fields()?
        .begin_constructor("init", ConstructorType::Normal, "Initialize the configuration with the specified values")?
        .default(&auto_time_sync, ConstructorDefault::Enum("none".to_string()))?
        .default_struct(&auto_tasks_retry_strategy)?
        .default(&keep_alive_timeout, Duration::from_secs(60))?
        .default(&auto_integrity_scan_on_buffer_overflow, true)?
        .default(&max_queued_user_requests, Number::U16(16))?
        .end_constructor()?
        .build()?;

    Ok(association_config)
}

fn define_tcp_client_state_listener(lib: &mut LibraryBuilder) -> BackTraced<InterfaceHandle> {
    let client_state_enum = lib
        .define_enum("client_state")?
        .push("disabled", "Client is disabled and idle until disabled")?
        .push(
            "connecting",
            "Client is trying to establish a connection to the remote device",
        )?
        .push("connected", "Client is connected to the remote device")?
        .push(
            "wait_after_failed_connect",
            "Failed to establish a connection, waiting before retrying",
        )?
        .push(
            "wait_after_disconnect",
            "Client was disconnected, waiting before retrying",
        )?
        .push("shutdown", "Client is shutting down")?
        .doc(
            doc("State of the client connection.")
                .details("Use by the {interface:client_state_listener}."),
        )?
        .build()?;

    let listener = lib
        .define_asynchronous_interface(
            "client_state_listener",
            "Callback for monitoring the client TCP connection state",
        )?
        .begin_callback("on_change", "Called when the client state changed")?
        .param("state", client_state_enum, "New state")?
        .returns_nothing()?
        .end_callback()?
        .build()?;

    Ok(listener)
}

fn define_master_channel_config(
    lib: &mut LibraryBuilder,
    shared: &SharedDefinitions,
) -> BackTraced<FunctionArgStructHandle> {
    let config = lib.declare_function_arg_struct("master_channel_config")?;

    let decode_level = Name::create("decode_level")?;
    let response_timeout = Name::create("response_timeout")?;
    let tx_buffer_size = Name::create("tx_buffer_size")?;
    let rx_buffer_size = Name::create("rx_buffer_size")?;

    let config = lib.define_function_argument_struct(config)?
        .doc("Generic configuration for a MasterChannel")?
        .add("address", BasicType::U16, "Local DNP3 data-link address")?
        .add(decode_level.clone(), shared.decode_level.clone(), "Decoding level for this master. You can modify this later on with {class:master_channel.set_decode_level()}.")?
        .add(
            response_timeout.clone(),
            DurationType::Milliseconds,
            "Timeout for receiving a response"
        )?
        .add(tx_buffer_size.clone(), BasicType::U16, doc("TX buffer size").details("Must be at least 249"))?
        .add(rx_buffer_size.clone(), BasicType::U16, doc("RX buffer size").details("Must be at least 2048"))?
        .end_fields()?
        .begin_constructor("init", ConstructorType::Normal, "Initialize {struct:master_channel_config} to default values")?
        .default_struct(&decode_level)?
        .default(&response_timeout, Duration::from_secs(5))?
        .default(&tx_buffer_size, Number::U16(2048))?
        .default(&rx_buffer_size, Number::U16(2048))?
        .end_constructor()?
        .build()?;

    Ok(config)
}

fn define_endpoint_list(lib: &mut LibraryBuilder) -> BackTraced<ClassHandle> {
    let endpoint_list_class = lib.declare_class("endpoint_list")?;

    let endpoint_list_new = lib.define_function("endpoint_list_new")?
        .param("main_endpoint", StringType, "Main endpoint")?
        .returns(endpoint_list_class.clone(), "New endpoint list")?
        .doc(doc("Create a new list of IP endpoints.").details("You can write IP addresses or DNS names and the port to connect to. e.g. \"127.0.0.1:20000\" or \"dnp3.myorg.com:20000\"."))?
        .build()?;

    let destructor = lib.define_destructor(
        endpoint_list_class.clone(),
        "Destroy a previously allocated endpoint list",
    )?;

    let add_method = lib.define_method("add", endpoint_list_class.clone())?
        .param("endpoint", StringType, "Endpoint to add to the list")?
        .returns_nothing()?
        .doc(doc("Add an IP endpoint to the list.").details("You can write IP addresses or DNS names and the port to connect to. e.g. \"127.0.0.1:20000\" or \"dnp3.myorg.com:20000\"."))?
        .build()?;

    let endpoint_list_class = lib.define_class(&endpoint_list_class)?
        .constructor(&endpoint_list_new)?
        .destructor(destructor)?
        .method(add_method)?
        .doc(doc("List of IP endpoints.").details("You can write IP addresses or DNS names and the port to connect to. e.g. \"127.0.0.1:20000\" or \"dnp3.myorg.com:20000\"."))?
        .build()?;

    Ok(endpoint_list_class)
}

fn define_utc_timestamp(lib: &mut LibraryBuilder) -> BackTraced<UniversalStructHandle> {
    let value = Name::create("value")?;
    let is_valid = Name::create("is_valid")?;

    let timestamp_utc = lib.declare_universal_struct("utc_timestamp")?;
    let timestamp_utc = lib.define_universal_struct(timestamp_utc)?
        .add(&value, BasicType::U64, doc("Value of the timestamp (in milliseconds from UNIX Epoch).").warning("Only 48 bits are available for timestamps."))?
        .add(&is_valid, BasicType::Bool, "True if the timestamp is valid, false otherwise.")?
        .doc(doc("Timestamp value returned by {interface:association_handler.get_current_time()}.").details("{struct:utc_timestamp.value} is only valid if {struct:utc_timestamp.is_valid} is true."))?
        .end_fields()?
        .begin_constructor("valid", ConstructorType::Static, "Construct a valid {struct:utc_timestamp}")?
        .default(&is_valid, true)?
        .end_constructor()?
        .begin_constructor("invalid", ConstructorType::Static, "Construct an invalid {struct:utc_timestamp}")?
        .default(&is_valid, false)?
        .default(&value, Number::U64(0))?
        .end_constructor()?
        .build()?;

    Ok(timestamp_utc)
}

fn define_association_handler(lib: &mut LibraryBuilder) -> BackTraced<InterfaceHandle> {
    let timestamp_utc = define_utc_timestamp(lib)?;

    let timestamp_utc = lib.define_asynchronous_interface(
        "association_handler",
        "Callbacks for a particular outstation association",
    )?
    .begin_callback(
        "get_current_time",
        doc("Returns the current time or an invalid time if none is available")
            .details("This callback is used when the master performs time synchronization for a particular outstation.")
            .details("This could return the system clock or some other clock's time"),
    )?
    .returns(
        timestamp_utc,
        "The current time",
    )?
    .end_callback()?
    .build()?;

    Ok(timestamp_utc)
}

fn define_event_classes(lib: &mut LibraryBuilder) -> BackTraced<FunctionArgStructHandle> {
    let class1 = Name::create("class1")?;
    let class2 = Name::create("class2")?;
    let class3 = Name::create("class3")?;

    let event_classes = lib.declare_function_arg_struct("event_classes")?;
    let event_classes = lib
        .define_function_argument_struct(event_classes)?
        .add(&class1, BasicType::Bool, "Class 1 events")?
        .add(&class2, BasicType::Bool, "Class 2 events")?
        .add(&class3, BasicType::Bool, "Class 3 events")?
        .doc("Event classes")?
        .end_fields()?
        .add_full_constructor("init")?
        .begin_constructor(
            "all",
            ConstructorType::Static,
            "Initialize all classes to true",
        )?
        .default(&class1, true)?
        .default(&class2, true)?
        .default(&class3, true)?
        .end_constructor()?
        .begin_constructor(
            "none",
            ConstructorType::Static,
            "Initialize all classes to false",
        )?
        .default(&class1, false)?
        .default(&class2, false)?
        .default(&class3, false)?
        .end_constructor()?
        .build()?;

    Ok(event_classes)
}

fn define_classes(lib: &mut LibraryBuilder) -> BackTraced<FunctionArgStructHandle> {
    let class0 = Name::create("class0")?;
    let class1 = Name::create("class1")?;
    let class2 = Name::create("class2")?;
    let class3 = Name::create("class3")?;

    let classes = lib.declare_function_arg_struct("classes")?;
    let classes = lib
        .define_function_argument_struct(classes)?
        .add(&class0, BasicType::Bool, "Class 0 (static data)")?
        .add(&class1, BasicType::Bool, "Class 1 events")?
        .add(&class2, BasicType::Bool, "Class 2 events")?
        .add(&class3, BasicType::Bool, "Class 3 events")?
        .doc("Class 0, 1, 2 and 3 config")?
        .end_fields()?
        .add_full_constructor("init")?
        .begin_constructor(
            "all",
            ConstructorType::Static,
            "Initialize all classes to true",
        )?
        .default(&class0, true)?
        .default(&class1, true)?
        .default(&class2, true)?
        .default(&class3, true)?
        .end_constructor()?
        .begin_constructor(
            "none",
            ConstructorType::Static,
            "Initialize all classes to false",
        )?
        .default(&class0, false)?
        .default(&class1, false)?
        .default(&class2, false)?
        .default(&class3, false)?
        .end_constructor()?
        .build()?;

    Ok(classes)
}

fn define_command_mode(lib: &mut LibraryBuilder) -> BackTraced<EnumHandle> {
    let mode = lib
        .define_enum("command_mode")?
        .push("direct_operate", "Perform a Direct Operate (0x05)")?
        .push(
            "select_before_operate",
            "Perform a Select and Operate (0x03 then 0x04)",
        )?
        .doc("Command operation mode")?
        .build()?;

    Ok(mode)
}

trait TaskErrors: Sized {
    fn add_task_errors(self) -> BackTraced<Self>;
}

impl TaskErrors for EnumBuilder<'_> {
    fn add_task_errors(self) -> BackTraced<Self> {
        let builder = self
            .push("too_many_requests", "too many user requests queued")?
            .push(
                "bad_response",
                "response was malformed or contained object headers",
            )?
            .push(
                "response_timeout",
                "timeout occurred before receiving a response",
            )?
            .push(
                "write_error",
                "insufficient buffer space to serialize the request",
            )?
            .push("no_connection", "no connection")?
            .push("shutdown", "master was shutdown")?
            .push("association_removed", "association was removed mid-task")?;

        Ok(builder)
    }
}

fn define_command_callback(lib: &mut LibraryBuilder) -> BackTraced<InterfaceHandle> {
    let command_result = lib
        .define_enum("command_result")?
        .push("success", "Command was a success")?
        .push(
            "bad_status",
            "Outstation indicated that a command was not SUCCESS",
        )?
        .push(
            "header_mismatch",
            "Number of headers or objects in the response didn't match the number in the request",
        )?
        .add_task_errors()?
        .doc("Result of a command")?
        .build()?;

    let callback = lib
        .define_asynchronous_interface("command_task_callback", "Handler for command tasks")?
        .begin_callback(
            "on_complete",
            "Called when the command task reached completion or failed",
        )?
        .param("result", command_result, "Result of the command task")?
        .returns_nothing()?
        .end_callback()?
        .build()?;

    Ok(callback)
}

fn define_command_builder(
    lib: &mut LibraryBuilder,
    shared: &SharedDefinitions,
) -> BackTraced<ClassHandle> {
    let command_set = lib.declare_class("command_set")?;

    let command_set_new_fn = lib
        .define_function("command_set_new")?
        .returns(command_set.clone(), "Handle to the created set of commands")?
        .doc("Create a new set of commands")?
        .build()?;

    let command_set_destructor =
        lib.define_destructor(command_set.clone(), "Destroy a set of commands")?;

    let finish_header = lib
        .define_method("finish_header", command_set.clone())?
        .returns_nothing()?
        .doc("Finish any partially completed header. This allows for the construction of two headers with the same type and index")?
        .build()?;

    let add_u8_g12v1 = lib
        .define_method("add_g12v1_u8", command_set.clone())?
        .param(
            "idx",
            BasicType::U8,
            "Index of the point to send the command to",
        )?
        .param("header", shared.g12v1_struct.clone(), "CROB data")?
        .returns_nothing()?
        .doc("Add a CROB with 1-byte prefix index")?
        .build()?;

    let add_u16_g12v1 = lib
        .define_method("add_g12v1_u16", command_set.clone())?
        .param(
            "idx",
            BasicType::U16,
            "Index of the point to send the command to",
        )?
        .param("header", shared.g12v1_struct.clone(), "CROB data")?
        .returns_nothing()?
        .doc("Add a CROB with 2-byte prefix index")?
        .build()?;

    let add_u8_g41v1 = lib
        .define_method("add_g41v1_u8", command_set.clone())?
        .param(
            "idx",
            BasicType::U8,
            "Index of the point to send the command to",
        )?
        .param("value", BasicType::S32, "Value to set the analog output to")?
        .returns_nothing()?
        .doc("Add a Analog Output command (signed 32-bit integer) with 1-byte prefix index")?
        .build()?;

    let add_u16_g41v1 = lib
        .define_method("add_g41v1_u16", command_set.clone())?
        .param(
            "idx",
            BasicType::U16,
            "Index of the point to send the command to",
        )?
        .param("value", BasicType::S32, "Value to set the analog output to")?
        .returns_nothing()?
        .doc("Add a Analog Output command (signed 32-bit integer) with 2-byte prefix index")?
        .build()?;

    let add_u8_g41v2 = lib
        .define_method("add_g41v2_u8", command_set.clone())?
        .param(
            "idx",
            BasicType::U8,
            "Index of the point to send the command to",
        )?
        .param("value", BasicType::S16, "Value to set the analog output to")?
        .returns_nothing()?
        .doc("Add a Analog Output command (signed 16-bit integer) with 1-byte prefix index")?
        .build()?;

    let add_u16_g41v2 = lib
        .define_method("add_g41v2_u16", command_set.clone())?
        .param(
            "idx",
            BasicType::U16,
            "Index of the point to send the command to",
        )?
        .param("value", BasicType::S16, "Value to set the analog output to")?
        .returns_nothing()?
        .doc("Add a Analog Output command (signed 16-bit integer) with 2-byte prefix index")?
        .build()?;

    let add_u8_g41v3 = lib
        .define_method("add_g41v3_u8", command_set.clone())?
        .param(
            "idx",
            BasicType::U8,
            "Index of the point to send the command to",
        )?
        .param(
            "value",
            BasicType::Float32,
            "Value to set the analog output to",
        )?
        .returns_nothing()?
        .doc("Add a Analog Output command (single-precision float) with 1-byte prefix index")?
        .build()?;

    let add_u16_g41v3 = lib
        .define_method("add_g41v3_u16", command_set.clone())?
        .param(
            "idx",
            BasicType::U16,
            "Index of the point to send the command to",
        )?
        .param(
            "value",
            BasicType::Float32,
            "Value to set the analog output to",
        )?
        .returns_nothing()?
        .doc("Add a Analog Output command (single-precision float) with 2-byte prefix index")?
        .build()?;

    let add_u8_g41v4 = lib
        .define_method("add_g41v4_u8", command_set.clone())?
        .param(
            "idx",
            BasicType::U8,
            "Index of the point to send the command to",
        )?
        .param(
            "value",
            BasicType::Double64,
            "Value to set the analog output to",
        )?
        .returns_nothing()?
        .doc("Add a Analog Output command (double-precision float) with 1-byte prefix index")?
        .build()?;

    let add_u16_g41v4 = lib
        .define_method("add_g41v4_u16", command_set.clone())?
        .param(
            "idx",
            BasicType::U16,
            "Index of the point to send the command to",
        )?
        .param(
            "value",
            BasicType::Double64,
            "Value to set the analog output to",
        )?
        .returns_nothing()?
        .doc("Add a Analog Output command (double-precision float) with 2-byte prefix index")?
        .build()?;

    let command_set = lib
        .define_class(&command_set)?
        .constructor(&command_set_new_fn)?
        .destructor(command_set_destructor)?
        .method(add_u8_g12v1)?
        .method(add_u16_g12v1)?
        .method(add_u8_g41v1)?
        .method(add_u16_g41v1)?
        .method(add_u8_g41v2)?
        .method(add_u16_g41v2)?
        .method(add_u8_g41v3)?
        .method(add_u16_g41v3)?
        .method(add_u8_g41v4)?
        .method(add_u16_g41v4)?
        .method(finish_header)?
        .doc("Builder type used to construct command requests")?
        .build()?;

    Ok(command_set)
}

fn define_time_sync_callback(lib: &mut LibraryBuilder) -> BackTraced<InterfaceHandle> {
    let timesync_result = lib
        .define_enum("time_sync_result")?
        .push("success", "Time synchronization operation was a success")?
        .push("clock_rollback", "Detected a clock rollback")?
        .push(
            "system_time_not_unix",
            "The system time cannot be converted to a Unix timestamp",
        )?
        .push(
            "bad_outstation_time_delay",
            "Outstation time delay exceeded the response delay",
        )?
        .push("overflow", "Overflow in calculation")?
        .push(
            "still_needs_time",
            "Outstation did not clear the NEED_TIME IIN bit",
        )?
        .push("system_time_not_available", "System time not available")?
        .push("iin_error", "Outstation indicated an error")?
        .add_task_errors()?
        .doc("Result of a time sync operation")?
        .build()?;

    let callback = lib
        .define_asynchronous_interface(
            "time_sync_task_callback",
            "Handler for time synchronization tasks",
        )?
        .begin_callback(
            "on_complete",
            "Called when the timesync task reached completion or failed",
        )?
        .param(
            "result",
            timesync_result,
            "Result of the time synchronization task",
        )?
        .returns_nothing()?
        .end_callback()?
        .build()?;

    Ok(callback)
}

fn define_time_sync_mode(lib: &mut LibraryBuilder) -> BackTraced<EnumHandle> {
    let mode = lib
        .define_enum("time_sync_mode")?
        .push(
            "lan",
            "Perform a LAN time sync with Record Current Time (0x18) function code",
        )?
        .push(
            "non_lan",
            "Perform a non-LAN time sync with Delay Measurement (0x17) function code",
        )?
        .doc("Time synchronization mode")?
        .build()?;

    Ok(mode)
}

fn define_restart_callback(lib: &mut LibraryBuilder) -> BackTraced<InterfaceHandle> {
    let restart_error = lib
        .define_enum("restart_error")?
        .push("ok", "Restart was perform successfully")?
        .add_task_errors()?
        .doc("Result of a restart operation")?
        .build()?;

    let restart_result = lib.declare_callback_arg_struct("restart_result")?;
    let restart_result = lib.define_callback_argument_struct(restart_result)?
        .add("error", restart_error, "Success/failure of the restart task")?
        .add("delay", DurationType::Milliseconds, "Delay value returned by the outstation. Valid only if {struct:restart_result.error} is {enum:restart_error.ok}.")?
        .doc("Result of a restart task")?
        .end_fields()?
        .add_full_constructor("init")?
        .build()?;

    let callback = lib
        .define_asynchronous_interface("restart_task_callback", "Handler for restart tasks")?
        .begin_callback(
            "on_complete",
            "Called when the restart task reached completion or failed",
        )?
        .param("result", restart_result, "Result of the restart task")?
        .returns_nothing()?
        .end_callback()?
        .build()?;

    Ok(callback)
}

fn define_link_status_callback(lib: &mut LibraryBuilder) -> BackTraced<InterfaceHandle> {
    let link_status_enum = lib
        .define_enum("link_status_result")?
        .push(
            "success",
            "The outstation responded with a valid LINK_STATUS",
        )?
        .push(
            "unexpected_response",
            "There was activity on the link, but it wasn't a LINK_STATUS",
        )?
        .push(
            "task_error",
            "The task failed for some reason (e.g. the master was shutdown)",
        )?
        .doc("Result of a link status check. See {class:master_channel.check_link_status()}")?
        .build()?;

    let callback = lib
        .define_asynchronous_interface("link_status_callback", "Handler for link status check")?
        .begin_callback("on_complete", "Called when a link status is received")?
        .param("result", link_status_enum, "Result of the link status")?
        .returns_nothing()?
        .end_callback()?
        .build()?;

    Ok(callback)
}
