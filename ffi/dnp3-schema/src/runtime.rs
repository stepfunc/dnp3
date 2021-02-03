use std::time::Duration;

use crate::shared::SharedDefinitions;
use class::ClassHandle;
use oo_bindgen::class::ClassDeclarationHandle;
use oo_bindgen::native_enum::*;
use oo_bindgen::native_function::*;
use oo_bindgen::native_struct::*;
use oo_bindgen::*;

pub fn define(
    lib: &mut LibraryBuilder,
    shared: &SharedDefinitions,
    decode_log_level_enum: NativeEnumHandle,
) -> Result<
    (
        ClassDeclarationHandle,
        NativeStructHandle,
        ClassDeclarationHandle,
    ),
    BindingError,
> {
    // Forward declare the class
    let runtime_class = lib.declare_class("Runtime")?;

    // Declare the C-style structs
    let config_struct = lib.declare_native_struct("RuntimeConfig")?;
    let config_struct = lib
        .define_native_struct(&config_struct)?
        .add(
            "num_core_threads",
            StructElementType::Uint16(Some(0)),
            doc("Number of runtime threads to spawn. For a guess of the number of CPU cores, use 0.")
            .details("Even if tons of connections are expected, it is preferred to use a value around the number of CPU cores for better performances. The library uses an efficient thread pool polling mechanism."),
        )?
        .doc("Runtime configuration")?
        .build()?;

    // Declare the native functions
    let new_fn = lib
        .declare_native_function("runtime_new")?
        .param(
            "config",
            Type::Struct(config_struct),
            "Runtime configuration",
        )?
        .return_type(ReturnType::new(
            Type::ClassRef(runtime_class.clone()),
            "Handle to the created runtime, {null} if an error occured",
        ))?
        .doc(
            doc("Creates a new runtime for running the protocol stack.")
            .warning("The runtime should be kept alive for as long as it's needed and it should be released with {class:Runtime.[destructor]}")
        )?
        .build()?;

    let destroy_fn = lib
        .declare_native_function("runtime_destroy")?
        .param("runtime", Type::ClassRef(runtime_class.clone()), "Runtime to destroy")?
        .return_type(ReturnType::void())?
        .doc(
            doc("Destroy a runtime.")
            .details("This method will gracefully wait for all asynchronous operation to end before returning")
        )?
        .build()?;

    let retry_strategy = lib.declare_native_struct("RetryStrategy")?;
    let retry_strategy = lib
        .define_native_struct(&retry_strategy)?
        .add(
            "min_delay",
            StructElementType::Duration(
                DurationMapping::Milliseconds,
                Some(Duration::from_secs(1)),
            ),
            "Minimum delay between two retries",
        )?
        .add(
            "max_delay",
            StructElementType::Duration(
                DurationMapping::Milliseconds,
                Some(Duration::from_secs(10)),
            ),
            "Maximum delay between two retries",
        )?
        .doc(doc("Retry strategy configuration.").details(
            "The strategy uses an exponential back-off with a minimum and maximum value.",
        ))?
        .build()?;

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

    let client_state_listener = lib
        .define_interface(
            "ClientStateListener",
            doc("Callback for monitoring the client connection state")
                .details("This is registered at creation in {class:Runtime.AddMasterTcp()}."),
        )?
        .callback("on_change", "Called when the client state changed")?
        .param("state", Type::Enum(client_state_enum), "New state")?
        .return_type(ReturnType::void())?
        .build()?
        .destroy_callback("on_destroy")?
        .build()?;

    let master_config = lib.declare_native_struct("MasterConfiguration")?;
    let master_config = lib.define_native_struct(&master_config)?
        .add("address", Type::Uint16, "Local DNP3 data-link address")?
        .add("level", Type::Enum(decode_log_level_enum), "Decoding log-level for this master. You can modify this later on with {class:Master.SetDecodeLogLevel()}.")?
        .add("reconnection_strategy", Type::Struct(retry_strategy.clone()), "Reconnection retry strategy to use")?
        .add("reconnection_delay", StructElementType::Duration(DurationMapping::Milliseconds, Some(Duration::from_millis(0))), doc("Optional reconnection delay when a connection is lost.").details("A value of 0 means no delay."))?
        .add(
            "response_timeout",
            Type::Duration(DurationMapping::Milliseconds),
            "Timeout for receiving a response"
        )?
        .add("tx_buffer_size", StructElementType::Uint16(Some(2048)), doc("TX buffer size").details("Should be at least 249"))?
        .add("rx_buffer_size", StructElementType::Uint16(Some(2048)), doc("RX buffer size").details("Should be at least 2048"))?
        .doc("Master configuration")?
        .build()?;

    let master_class = lib.declare_class("Master")?;

    let endpoint_list = define_endpoint_list(lib)?;

    let add_master_tcp_fn = lib
        .declare_native_function("runtime_add_master_tcp")?
        .param("runtime", Type::ClassRef(runtime_class.clone()), "Runtime to use to drive asynchronous operations of the master")?
        .param("link_error_mode", Type::Enum(shared.link_error_mode.clone()), "Controls how link errors are handled with respect to the TCP session")?
        .param("config", Type::Struct(master_config.clone()), "Master configuration")?
        .param("endpoints", Type::ClassRef(endpoint_list.declaration()), "List of IP endpoints.")?
        .param("listener", Type::Interface(client_state_listener.clone()), "Client connection listener to receive updates on the status of the connection")?
        .return_type(ReturnType::new(Type::ClassRef(master_class.clone()), "Handle to the master created, {null} if an error occured"))?
        .doc(
            doc("Add a master TCP connection")
            .details("The returned master must be gracefully shutdown with {class:Master.[destructor]} when done.")
        )?
        .build()?;

    let add_master_serial_fn = lib
        .declare_native_function("runtime_add_master_serial")?
        .param("runtime", Type::ClassRef(runtime_class.clone()), "Runtime to use to drive asynchronous operations of the master")?
        .param("config", Type::Struct(master_config), "Master configuration")?
        .param("path", Type::String, "Path to the serial device. Generally /dev/tty0 on Linux and COM1 on Windows.")?
        .param("serial_params", Type::Struct(shared.serial_port_settings.clone()), "Serial port settings")?
        .param("listener", Type::Interface(client_state_listener), "Client connection listener to receive updates on the status of the connection")?
        .return_type(ReturnType::new(Type::ClassRef(master_class.clone()), "Handle to the master created, {null} if an error occured"))?
        .doc(
            doc("Add a master serial connection")
            .details("The returned master must be gracefully shutdown with {class:Master.[destructor]} when done.")
        )?
        .build()?;

    // Declare the object-oriented class
    let _runtime_class = lib
        .define_class(&runtime_class)?
        .constructor(&new_fn)?
        .destructor(&destroy_fn)?
        .method("AddMasterTcp", &add_master_tcp_fn)?
        .method("AddMasterSerial", &add_master_serial_fn)?
        .doc("Event-queue based runtime handle")?
        .build()?;

    Ok((runtime_class, retry_strategy, master_class))
}

fn define_endpoint_list(lib: &mut LibraryBuilder) -> Result<ClassHandle, BindingError> {
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
