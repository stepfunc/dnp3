use std::time::Duration;

use crate::shared::SharedDefinitions;
use oo_bindgen::callback::InterfaceHandle;
use oo_bindgen::class::ClassHandle;
use oo_bindgen::native_function::*;
use oo_bindgen::native_struct::*;
use oo_bindgen::*;

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
        .param("listener", Type::Interface(tcp_client_state_listener.clone()), "TCP connection listener used to receive updates on the status of the connection")?
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
        .param("listener", Type::Interface(tcp_client_state_listener), "Client connection listener to receive updates on the status of the connection")?
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

    // define the association
    let association_class = crate::association::define(lib, shared)?;

    let event_classes = lib.declare_native_struct("EventClasses")?;
    let event_classes = lib
        .define_native_struct(&event_classes)?
        .add("class1", Type::Bool, "Class 1 events")?
        .add("class2", Type::Bool, "Class 2 events")?
        .add("class3", Type::Bool, "Class 3 events")?
        .doc("Event classes")?
        .build()?;

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

    let association_configuration = lib.declare_native_struct("AssociationConfiguration")?;
    let association_configuration = lib
        .define_native_struct(&association_configuration)?
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
            doc("Classes to automaticaly send reads when the IIN bit is asserted")
        )?
        .doc("Association configuration")?
        .build()?;

    let association_handlers = lib.declare_native_struct("AssociationHandlers")?;
    let association_handlers = lib
        .define_native_struct(&association_handlers)?
        .add("integrity_handler", Type::Interface(read_handler.clone()), "Handler for the initial integrity scan")?
        .add("unsolicited_handler", Type::Interface(read_handler.clone()), "Handler for unsolicited responses")?
        .add("default_poll_handler", Type::Interface(read_handler), "Handler for all other responses")?
        .doc(
            doc("Handlers that will receive readings.")
            .details("You can set all handlers to the same handler if knowing what type of event generated the value is not required.")
        )?
        .build()?;

    let time_provider_interface = define_time_provider(lib)?;

    let add_association_fn = lib
        .declare_native_function("master_add_association")?
        .param(
            "master",
            Type::ClassRef(master_class.clone()),
            "Master to add the association to",
        )?
        .param(
            "address",
            Type::Uint16,
            "DNP3 data-link address of the remote outstation",
        )?
        .param(
            "config",
            Type::Struct(association_configuration),
            "Association configuration",
        )?
        .param(
            "handlers",
            Type::Struct(association_handlers),
            "Handlers to call when receiving point data",
        )?
        .param(
            "time_provider",
            Type::Interface(time_provider_interface),
            "Time provider for the association",
        )?
        .return_type(ReturnType::new(
            Type::ClassRef(association_class),
            "Handle to the created association or NULL if an error occurred",
        ))?
        .doc("Add an association to the master")?
        .build()?;

    let set_decode_level_fn = lib
        .declare_native_function("master_set_decode_level")?
        .param(
            "master",
            Type::ClassRef(master_class.clone()),
            "Master to modify",
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

    lib.define_class(&master_class)?
        .destructor(&destroy_fn)?
        .static_method("CreateTCPSession", &master_create_tcp_session_fn)?
        .static_method("CreateSerialSession", &master_create_serial_session_fn)?
        .method("AddAssociation", &add_association_fn)?
        .method("SetDecodeLevel", &set_decode_level_fn)?
        .method("GetDecodeLevel", &get_decode_level_fn)?
        .doc(
            doc("Master channel of communication")
            .details("To communicate with a particular outstation, you need to add an association with {class:Master.AddAssociation()}.")
            .warning("This cannot be called from within a callback.")
        )?
        .build()?;

    Ok(())
}

fn define_tcp_client_state_listener(
    lib: &mut LibraryBuilder,
) -> std::result::Result<InterfaceHandle, BindingError> {
    let client_state_enum = lib
        .define_native_enum("ClientState")?
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
    let master_config = lib.declare_native_struct("MasterConfiguration")?;
    lib.define_native_struct(&master_config)?
        .add("address", Type::Uint16, "Local DNP3 data-link address")?
        .add("decode_level", StructElementType::Struct(shared.decode_level.clone()), "Decoding level for this master. You can modify this later on with {class:Master.SetDecodeLevel()}.")?
        .add("reconnection_strategy", Type::Struct(shared.retry_strategy.clone()), "Reconnection retry strategy to use")?
        .add("reconnection_delay", StructElementType::Duration(DurationMapping::Milliseconds, Some(Duration::from_millis(0))), doc("Optional reconnection delay when a connection is lost.").details("A value of 0 means no delay."))?
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
