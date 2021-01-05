use oo_bindgen::class::ClassDeclarationHandle;
use oo_bindgen::native_enum::*;
use oo_bindgen::native_function::*;
use oo_bindgen::native_struct::*;
use oo_bindgen::*;

pub fn define(
    lib: &mut LibraryBuilder,
    decode_log_level_enum: NativeEnumHandle,
) -> Result<(NativeStructHandle, ClassDeclarationHandle), BindingError> {
    // Forward declare the class
    let runtime_class = lib.declare_class("Runtime")?;

    // Declare the C-style structs
    let config_struct = lib.declare_native_struct("RuntimeConfig")?;
    let config_struct = lib
        .define_native_struct(&config_struct)?
        .add(
            "num_core_threads",
            Type::Uint16,
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
            Type::StructRef(config_struct.declaration()),
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
            Type::Duration(DurationMapping::Milliseconds),
            "Minimum delay between two retries",
        )?
        .add(
            "max_delay",
            Type::Duration(DurationMapping::Milliseconds),
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
        .add("reconnection_strategy", Type::Struct(retry_strategy.clone()), "Reconnection strategy to use")?
        .add(
            "response_timeout",
            Type::Duration(DurationMapping::Milliseconds),
            "Timeout for receiving a response"
        )?
        .add("tx_buffer_size", Type::Uint16, doc("TX buffer size").details("Should be at least 249"))?
        .add("rx_buffer_size", Type::Uint16, doc("RX buffer size").details("Should be at least 2048"))?
        .add("bubble_framing_errors", Type::Bool, "Close connection when a framing error occurs")?
        .doc("Master configuration")?
        .build()?;

    let master_class = lib.declare_class("Master")?;

    let add_master_tcp_fn = lib
        .declare_native_function("runtime_add_master_tcp")?
        .param("runtime", Type::ClassRef(runtime_class.clone()), "Runtime to use to drive asynchronous operations of the master")?
        .param("config", Type::Struct(master_config.clone()), "Master configuration")?
        .param("endpoint", Type::String, "IP address or DNS name and the port to connect to. e.g. \"127.0.0.1:20000\" or \"dnp3.myorg.com:20000\".")?
        .param("listener", Type::Interface(client_state_listener.clone()), "Client connection listener to receive updates on the status of the connection")?
        .return_type(ReturnType::new(Type::ClassRef(master_class.clone()), "Handle to the master created, {null} if an error occured"))?
        .doc(
            doc("Add a master TCP connection")
            .details("The returned master must be gracefully shutdown with {class:Master.[destructor]} when done.")
        )?
        .build()?;

    /* TODO
    let serial_params = define_serial_params(lib)?;

    let add_master_serial_fn = lib
        .declare_native_function("runtime_add_master_serial")?
        .param("runtime", Type::ClassRef(runtime_class.clone()), "Runtime to use to drive asynchronous operations of the master")?
        .param("config", Type::Struct(master_config), "Master configuration")?
        .param("path", Type::String, "Path to the serial device. Generally /dev/tty0 on Linux and COM1 on Windows.")?
        .param("serial_params", Type::Struct(serial_params), "Serial parameters.")?
        .param("listener", Type::Interface(client_state_listener), "Client connection listener to receive updates on the status of the connection")?
        .return_type(ReturnType::new(Type::ClassRef(master_class.clone()), "Handle to the master created, {null} if an error occured"))?
        .doc(
            doc("Add a master serial connection")
            .details("The returned master must be gracefully shutdown with {class:Master.[destructor]} when done.")
        )?
        .build()?;
     */

    // Declare the object-oriented class
    let _runtime_class = lib
        .define_class(&runtime_class)?
        .constructor(&new_fn)?
        .destructor(&destroy_fn)?
        .method("AddMasterTcp", &add_master_tcp_fn)?
        // TODO - .method("AddMasterSerial", &add_master_serial_fn)?
        .doc("Event-queue based runtime handle")?
        .build()?;

    Ok((retry_strategy, master_class))
}

/*
fn define_serial_params(lib: &mut LibraryBuilder) -> Result<NativeStructHandle, BindingError> {
    let data_bits = lib
        .define_native_enum("DataBits")?
        .push("Five", "5 bits per character")?
        .push("Six", "6 bits per character")?
        .push("Seven", "7 bits per character")?
        .push("Eight", "8 bits per character")?
        .doc("Number of bits per character")?
        .build()?;

    let flow_control = lib
        .define_native_enum("FlowControl")?
        .push("None", "No flow control")?
        .push("Software", "Flow control using XON/XOFF bytes")?
        .push("Hardware", "Flow control using RTS/CTS signals")?
        .doc("Flow control modes")?
        .build()?;

    let parity = lib
        .define_native_enum("Parity")?
        .push("None", "No parity bit")?
        .push("Odd", "Parity bit sets odd number of 1 bits")?
        .push("Even", "Parity bit sets even number of 1 bits")?
        .doc("Parity checking modes")?
        .build()?;

    let stop_bits = lib
        .define_native_enum("StopBits")?
        .push("One", "One stop bit")?
        .push("Two", "Two stop bits")?
        .doc("Number of stop bits")?
        .build()?;

    let serial_params = lib.declare_native_struct("SerialPortSettings")?;
    lib.define_native_struct(&serial_params)?
        .add(
            "baud_rate",
            Type::Uint32,
            "Baud rate (in symbols-per-second)",
        )?
        .add(
            "data_bits",
            Type::Enum(data_bits),
            "Number of bits used to represent a character sent on the line",
        )?
        .add(
            "flow_control",
            Type::Enum(flow_control),
            "Type of signalling to use for controlling data transfer",
        )?
        .add(
            "parity",
            Type::Enum(parity),
            "Type of parity to use for error checking",
        )?
        .add(
            "stop_bits",
            Type::Enum(stop_bits),
            "Number of bits to use to signal the end of a character",
        )?
        .doc(doc("Serial port settings").details("Used by {class:Runtime.AddMasterSerial()}."))?
        .build()
}
 */
