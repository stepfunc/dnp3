use crate::database::DatabaseTypes;
use crate::shared::SharedDefinitions;
use oo_bindgen::model::*;
use std::time::Duration;

struct OutstationTypes {
    database_transaction: SynchronousInterface,
    outstation_config: FunctionArgStructHandle,
    outstation_application: AsynchronousInterface,
    outstation_information: AsynchronousInterface,
    control_handler: AsynchronousInterface,
    connection_state_listener: AsynchronousInterface,
}

impl OutstationTypes {
    fn define(lib: &mut LibraryBuilder, shared: &SharedDefinitions) -> BackTraced<Self> {
        let DatabaseTypes {
            database_transaction,
            database_handle,
        } = crate::database::define(lib, shared)?;

        Ok(Self {
            database_transaction,
            outstation_config: define_outstation_config(lib, shared)?,
            outstation_application: define_outstation_application(lib, shared, &database_handle)?,
            outstation_information: define_outstation_information(lib, shared)?,
            control_handler: define_control_handler(lib, &database_handle, shared)?,
            connection_state_listener: define_connection_state_listener(lib)?,
        })
    }
}

pub(crate) fn define_buffer_state(lib: &mut LibraryBuilder) -> BackTraced<UniversalStructHandle> {
    let class_count = lib.declare_universal_struct("class_count")?;
    let class_count = lib
        .define_universal_struct(class_count)?
        .doc("Remaining number of events in the buffer after a confirm on a per-class basis")?
        .add(
            "num_class_1",
            Primitive::U32,
            "Number of class 1 events remaining in the buffer",
        )?
        .add(
            "num_class_2",
            Primitive::U32,
            "Number of class 2 events remaining in the buffer",
        )?
        .add(
            "num_class_3",
            Primitive::U32,
            "Number of class 3 events remaining in the buffer",
        )?
        .end_fields()?
        .build()?;

    let type_count = lib.declare_universal_struct("type_count")?;
    let type_count = lib
        .define_universal_struct(type_count)?
        .doc("Remaining number of events in the buffer after a confirm on a per-type basis")?
        .add(
            "num_binary_input",
            Primitive::U32,
            "Number of binary input events remaining in the buffer",
        )?
        .add(
            "num_double_bit_binary_input",
            Primitive::U32,
            "Number of double-bit binary input events remaining in the buffer",
        )?
        .add(
            "num_binary_output_status",
            Primitive::U32,
            "Number of binary output status events remaining in the buffer",
        )?
        .add(
            "num_counter",
            Primitive::U32,
            "Number of counter events remaining in the buffer",
        )?
        .add(
            "num_frozen_counter",
            Primitive::U32,
            "Number of frozen counter events remaining in the buffer",
        )?
        .add(
            "num_analog",
            Primitive::U32,
            "Number of analog events remaining in the buffer",
        )?
        .add(
            "num_analog_output_status",
            Primitive::U32,
            "Number of analog output status events remaining in the buffer",
        )?
        .add(
            "num_octet_string",
            Primitive::U32,
            "Number octet string events remaining in the buffer",
        )?
        .end_fields()?
        .build()?;

    let buffer_state = lib.declare_universal_struct("buffer_state")?;
    let buffer_state = lib
        .define_universal_struct(buffer_state)?
        .doc("Information about the state of buffer after a CONFIRM has been processed")?
        .add(
            "classes",
            class_count,
            "Remaining number of events in the buffer on a per-class basis",
        )?
        .add(
            "types",
            type_count,
            "Remaining number of events in the buffer on a per-type basis",
        )?
        .end_fields()?
        .build()?;

    Ok(buffer_state)
}

pub(crate) fn define(lib: &mut LibraryBuilder, shared_def: &SharedDefinitions) -> BackTraced<()> {
    // Everything required to create an outstation

    let types = OutstationTypes::define(lib, shared_def)?;
    let outstation = define_outstation(lib, shared_def, &types)?;
    let address_filter = define_address_filter(lib, shared_def)?;
    let tls_server_config = define_tls_server_config(lib, shared_def)?;

    // Define the TCP server
    let outstation_server = lib.declare_class("outstation_server")?;

    let outstation_server_tcp_create = lib
        .define_function("outstation_server_create_tcp_server")?
        .param(
            "runtime",
            shared_def.runtime_class.clone(),
            "Runtime to execute the server on",
        )?
        .param(
            "link_error_mode",
            shared_def.link_error_mode.clone(),
            "Controls how link errors are handled with respect to the TCP session",
        )?
        .param(
            "address",
            StringType,
            "Address to bind the server to e.g. 127.0.0.1:20000",
        )?
        .returns(outstation_server.clone(), "New TCP server instance")?
        .fails_with(shared_def.error_type.clone())?
        .doc(
            doc("Create a new TCP server.")
                .details("To start it, use {class:outstation_server.bind()}."),
        )?
        .build_static("create_tcp_server")?;

    let outstation_server_tls_create = lib
        .define_function("outstation_server_create_tls_server")?
        .param(
            "runtime",
            shared_def.runtime_class.clone(),
            "Runtime to execute the server on",
        )?
        .param(
            "link_error_mode",
            shared_def.link_error_mode.clone(),
            "Controls how link errors are handled with respect to the session",
        )?
        .param(
            "address",
            StringType,
            "Address to bind the server to e.g. 127.0.0.1:20000",
        )?
        .param("tls_config", tls_server_config, "TLS server configuration")?
        .returns(outstation_server.clone(), "New TLS server instance")?
        .fails_with(shared_def.error_type.clone())?
        .doc(
            doc("Create a new TLS server.")
                .details("To start it, use {class:outstation_server.bind()}."),
        )?
        .build_static("create_tls_server")?;

    let add_outstation = lib.define_method("add_outstation", outstation_server.clone())?
        .param("config",types.outstation_config, "Outstation configuration")?
        .param("application", types.outstation_application, "Outstation application callbacks")?
        .param("information", types.outstation_information, "Outstation information callbacks")?
        .param("control_handler", types.control_handler, "Outstation control handler")?
        .param("listener", types.connection_state_listener, "Listener for the connection state")?
        .param("filter",address_filter.declaration(), "Address filter")?
        .returns(outstation.declaration(), "Outstation handle")?
        .fails_with(shared_def.error_type.clone())?
        .doc(doc("Add an outstation to the server.")
            .details("The returned {class:outstation} can be used to modify points of the outstation.")
            .details("In order for the outstation to run, the TCP server must be running. Use {class:outstation_server.bind()} to run it."))?
        .build()?;

    let bind = lib.define_method("bind", outstation_server.clone())?
        .fails_with(shared_def.error_type.clone())?
        .doc("Bind the server to the port and starts listening. Also starts all the outstations associated to it.")?
        .build()?;

    let destructor = lib
        .define_destructor(outstation_server.clone(), "Gracefully shutdown all the outstations associated to this server, stops the server and release resources.")?;

    lib.define_class(&outstation_server)?
        .static_method(outstation_server_tcp_create)?
        .static_method(outstation_server_tls_create)?
        .destructor(destructor)?
        .method(add_outstation)?
        .method(bind)?
        .custom_destroy("shutdown")?
        .doc(doc("TCP server that listens for connections and routes the messages to outstations.")
        .details("To add outstations to it, use {class:outstation_server.add_outstation()}. Once all the outstations are added, the server can be started with {class:outstation_server.bind()}.")
        .details("{class:outstation_server.[destructor]} is used to gracefully shutdown all the outstations and the server."))?
        .build()?;

    Ok(())
}

fn define_outstation(
    lib: &mut LibraryBuilder,
    shared: &SharedDefinitions,
    types: &OutstationTypes,
) -> BackTraced<ClassHandle> {
    let outstation = lib.declare_class("outstation")?;

    let outstation_create_tcp_client_fn = lib
        .define_function("outstation_create_tcp_client")?
        .param(
            "runtime",
            shared.runtime_class.clone(),
            "runtime on which to spawn the outstation",
        )?
        .param(
            "link_error_mode",
            shared.link_error_mode.clone(),
            "Controls how link errors are handled with respect to the TCP session",
        )?
        .param(
            "endpoints",
            shared.endpoint_list.declaration(),
            "List of IP endpoints.",
        )?
        .param(
            "connect_strategy",
            shared.connect_strategy.clone(),
            "Controls the timing of (re)connection attempts",
        )?
        .param(
            "connect_options",
            shared.connect_options.declaration(),
            "Options that control the TCP connection process",
        )?
        .param(
            "config",
            types.outstation_config.clone(),
            "outstation configuration",
        )?
        .param(
            "application",
            types.outstation_application.clone(),
            "application interface",
        )?
        .param(
            "information",
            types.outstation_information.clone(),
            "informational events interface",
        )?
        .param(
            "control_handler",
            types.control_handler.clone(),
            "control handler interface",
        )?
        .param(
            "listener",
            shared.client_state_listener.clone(),
            "Connection listener used to receive updates on the status of the connection",
        )?
        .returns(outstation.clone(), "Outstation instance")?
        .fails_with(shared.error_type.clone())?
        .doc(doc("Create an outstation instance running as a TCP client"))?
        .build_static("create_tcp_client")?;

    let outstation_create_tls_client_fn = lib
        .define_function("outstation_create_tls_client")?
        .param(
            "runtime",
            shared.runtime_class.clone(),
            "runtime on which to spawn the outstation",
        )?
        .param(
            "link_error_mode",
            shared.link_error_mode.clone(),
            "Controls how link errors are handled with respect to the TCP session",
        )?
        .param(
            "endpoints",
            shared.endpoint_list.declaration(),
            "List of IP endpoints.",
        )?
        .param(
            "connect_strategy",
            shared.connect_strategy.clone(),
            "Controls the timing of (re)connection attempts",
        )?
        .param(
            "connect_options",
            shared.connect_options.declaration(),
            "Options that control the TCP connection process",
        )?
        .param(
            "config",
            types.outstation_config.clone(),
            "outstation configuration",
        )?
        .param(
            "application",
            types.outstation_application.clone(),
            "application interface",
        )?
        .param(
            "information",
            types.outstation_information.clone(),
            "informational events interface",
        )?
        .param(
            "control_handler",
            types.control_handler.clone(),
            "control handler interface",
        )?
        .param(
            "listener",
            shared.client_state_listener.clone(),
            "Connection listener used to receive updates on the status of the connection",
        )?
        .param(
            "tls_config",
            shared.tls_client_config.clone(),
            "TLS client configuration",
        )?
        .returns(outstation.clone(), "Outstation instance")?
        .fails_with(shared.error_type.clone())?
        .doc(doc("Create an outstation instance running as a TLS client"))?
        .build_static("create_tls_client")?;

    let outstation_create_serial_session_fn = lib
        .define_function("outstation_create_serial_session")?
        .param(
            "runtime",
            shared.runtime_class.clone(),
            "runtime on which to spawn the outstation",
        )?
        .param("serial_path", StringType, "Path of the serial device")?
        .param(
            "settings",
            shared.serial_port_settings.clone(),
            "settings for the serial port",
        )?
        .param(
            "config",
            types.outstation_config.clone(),
            "outstation configuration",
        )?
        .param(
            "application",
            types.outstation_application.clone(),
            "application interface",
        )?
        .param(
            "information",
            types.outstation_information.clone(),
            "informational events interface",
        )?
        .param(
            "control_handler",
            types.control_handler.clone(),
            "control handler interface",
        )?
        .returns(
            outstation.clone(),
            "Outstation instance or {null} if the port cannot be opened",
        )?
        .fails_with(shared.error_type.clone())?
        .doc(
            doc("Create an outstation instance running on a serial port")
                .details("The port is opened immediately on the calling thread and fails if not successful")
                .warning("Most users should prefer the fault tolerant version of the this method {class:outstation.create_serial_session_fault_tolerant()}")
        )?
        .build_static("create_serial_session")?;

    let outstation_create_serial_session_fault_tolerant_fn = lib
        .define_function("outstation_create_serial_session_fault_tolerant")?
        .param(
            "runtime",
            shared.runtime_class.clone(),
            "runtime on which to spawn the outstation",
        )?
        .param("serial_path", StringType, "Path of the serial device")?
        .param(
            "settings",
            shared.serial_port_settings.clone(),
            "settings for the serial port",
        )?
        .param("open_retry_delay", DurationType::Milliseconds, "delay between attempts to open the serial port")?
        .param(
            "config",
            types.outstation_config.clone(),
            "outstation configuration",
        )?
        .param(
            "application",
            types.outstation_application.clone(),
            "application interface",
        )?
        .param(
            "information",
            types.outstation_information.clone(),
            "informational events interface",
        )?
        .param(
            "control_handler",
            types.control_handler.clone(),
            "control handler interface",
        )?
        .returns(
            outstation.clone(),
            "Outstation instance or {null} if the port cannot be opened",
        )?
        .fails_with(shared.error_type.clone())?
        .doc(
            doc("This method is implemented in terms of {class:outstation.create_serial_session_2()} but without a port listener")
        )?
        .build_static("create_serial_session_fault_tolerant")?;

    let outstation_create_serial_session_2_fn = lib
        .define_function("outstation_create_serial_session_2")?
        .param(
            "runtime",
            shared.runtime_class.clone(),
            "runtime on which to spawn the outstation",
        )?
        .param("serial_path", StringType, "Path of the serial device")?
        .param(
            "settings",
            shared.serial_port_settings.clone(),
            "settings for the serial port",
        )?
        .param("open_retry_delay", DurationType::Milliseconds, "delay between attempts to open the serial port")?
        .param(
            "config",
            types.outstation_config.clone(),
            "outstation configuration",
        )?
        .param(
            "application",
            types.outstation_application.clone(),
            "application interface",
        )?
        .param(
            "information",
            types.outstation_information.clone(),
            "informational events interface",
        )?
        .param(
            "control_handler",
            types.control_handler.clone(),
            "control handler interface",
        )?
        .param("port_listener",
               shared.port_state_listener.clone(),
            "port state listener"
        )?
        .returns(
            outstation.clone(),
            "Outstation instance or {null} if the port cannot be opened",
        )?
        .fails_with(shared.error_type.clone())?
        .doc(
            doc("Create an outstation instance running on a serial port which is tolerant to the serial port being added and removed")
        )?
        .build_static("create_serial_session_2")?;

    let udp_config = define_outstation_udp_config(lib, shared)?;

    let outstation_create_udp = lib
        .define_function("outstation_create_udp")?
        .param(
            "runtime",
            shared.runtime_class.clone(),
            "runtime on which to spawn the outstation",
        )?
        .param("udp_config", udp_config, "UDP configuration")?
        .param(
            "config",
            types.outstation_config.clone(),
            "outstation configuration",
        )?
        .param(
            "application",
            types.outstation_application.clone(),
            "application interface",
        )?
        .param(
            "information",
            types.outstation_information.clone(),
            "informational events interface",
        )?
        .param(
            "control_handler",
            types.control_handler.clone(),
            "control handler interface",
        )?
        .returns(
            outstation.clone(),
            "Outstation instance or {null} if the port cannot be opened",
        )?
        .fails_with(shared.error_type.clone())?
        .doc(
            doc("Create an outstation instance running on a serial port which is tolerant to the serial port being added and removed")
        )?
        .build_static("create_udp")?;

    let destructor = lib.define_destructor(
        outstation.clone(),
        doc("Free resources of the outstation.").warning("This does not shutdown the outstation. Only {class:outstation_server.[destructor]} will properly shutdown the outstation.")
    )?;

    let execute_transaction = lib
        .define_method("transaction", outstation.clone())?
        .param(
            "callback",
            types.database_transaction.clone(),
            "Interface on which to execute the transaction",
        )?
        .doc("Acquire a mutex on the underlying database and apply a set of changes as a transaction")?
        .build()?;

    let set_decode_level = lib
        .define_method("set_decode_level", outstation.clone())?
        .param("level", shared.decode_level.clone(), "Decode log")?
        .fails_with(shared.error_type.clone())?
        .doc("Set decoding log level")?
        .build()?;

    let enable = lib
        .define_method("enable", outstation.clone())?
        .fails_with(shared.error_type.clone())?
        .doc("enable communications")?
        .build()?;

    let disable = lib
        .define_method("disable", outstation.clone())?
        .fails_with(shared.error_type.clone())?
        .doc("disable communications")?
        .build()?;

    let outstation = lib
        .define_class(&outstation)?
        .destructor(destructor)?
        .static_method(outstation_create_serial_session_fn)?
        .static_method(outstation_create_serial_session_fault_tolerant_fn)?
        .static_method(outstation_create_serial_session_2_fn)?
        .static_method(outstation_create_tcp_client_fn)?
        .static_method(outstation_create_tls_client_fn)?
        .static_method(outstation_create_udp)?
        .method(enable)?
        .method(disable)?
        .method(execute_transaction)?
        .method(set_decode_level)?
        .doc(doc("Outstation handle").details("Use this handle to modify the internal database."))?
        .build()?;

    Ok(outstation)
}

fn define_outstation_udp_config(
    lib: &mut LibraryBuilder,
    shared: &SharedDefinitions,
) -> BackTraced<FunctionArgStructHandle> {
    let value = lib.declare_function_argument_struct("outstation_udp_config")?;

    let retry_delay = Name::create("retry_delay")?;
    let link_read_mode = Name::create("link_read_mode")?;
    let socket_mode = Name::create("socket_mode")?;

    let value = lib
        .define_function_argument_struct(value)?
        .add(
            "local_endpoint",
            StringType,
            "Local endpoint to which the UDP socket is bound. Must be a socket address (ip:port)",
        )?
        .add(
            "remote_endpoint",
            StringType,
            "Remote endpoint where outbound requests are sent. Must be a socket address (ip:port)",
        )?
        .add(
            "socket_mode",
            shared.udp_socket_mode.clone(),
            "UDP socket mode to use",
        )?
        .add(
            link_read_mode.clone(),
            shared.link_read_mode.clone(),
            "Read mode to use, this should typically be set to {enum:link_read_mode.datagram}",
        )?
        .add(
            retry_delay.clone(),
            DurationType::Milliseconds,
            "Period to wait before retrying after a failure to bind or connect the UDP socket",
        )?
        .doc("UDP outstation configuration")?
        .end_fields()?
        .begin_initializer(
            "init",
            InitializerType::Normal,
            "Initialize the configuration with default settings for unspecified parameter",
        )?
        .default(&retry_delay, Duration::from_secs(5))?
        .default_variant(&link_read_mode, "datagram")?
        .default_variant(&socket_mode, "one_to_one")?
        .end_initializer()?
        .build()?;

    Ok(value)
}

fn define_class_zero_config(lib: &mut LibraryBuilder) -> BackTraced<FunctionArgStructHandle> {
    let binary = Name::create("binary")?;
    let double_bit_binary = Name::create("double_bit_binary")?;
    let binary_output_status = Name::create("binary_output_status")?;
    let counter = Name::create("counter")?;
    let frozen_counter = Name::create("frozen_counter")?;
    let analog = Name::create("analog")?;
    let analog_output_status = Name::create("analog_output_status")?;
    let octet_strings = Name::create("octet_string")?;

    let class_zero_config = lib.declare_function_argument_struct("class_zero_config")?;
    let class_zero_config = lib
        .define_function_argument_struct(class_zero_config)?
        .add(
            &binary,
            Primitive::Bool,
            "Include Binary Inputs in Class 0 reads",
        )?
        .add(
            &double_bit_binary,
            Primitive::Bool,
            "Include Double-Bit Binary Inputs in Class 0 reads",
        )?
        .add(
            &binary_output_status,
            Primitive::Bool,
            "Include Binary Output Status in Class 0 reads",
        )?
        .add(
            &counter,
            Primitive::Bool,
            "Include Counters in Class 0 reads",
        )?
        .add(
            &frozen_counter,
            Primitive::Bool,
            "Include Frozen Counters in Class 0 reads",
        )?
        .add(
            &analog,
            Primitive::Bool,
            "Include Analog Inputs in Class 0 reads",
        )?
        .add(
            &analog_output_status,
            Primitive::Bool,
            "Include Analog Output Status in Class 0 reads",
        )?
        .add(
            &octet_strings,
            Primitive::Bool,
            doc("Include Octet Strings in Class 0 reads")
                .warning("For conformance, this should be false."),
        )?
        .doc("Controls which types are reported during a Class 0 read.")?
        .end_fields()?
        .begin_initializer(
            "init",
            InitializerType::Normal,
            "Initialize to default values",
        )?
        .default(&binary, true)?
        .default(&double_bit_binary, true)?
        .default(&binary_output_status, true)?
        .default(&counter, true)?
        .default(&frozen_counter, true)?
        .default(&analog, true)?
        .default(&analog_output_status, true)?
        .default(&octet_strings, false)?
        .end_initializer()?
        .build()?;

    Ok(class_zero_config)
}

fn define_outstation_features(lib: &mut LibraryBuilder) -> BackTraced<FunctionArgStructHandle> {
    let self_address = Name::create("self_address")?;
    let broadcast = Name::create("broadcast")?;
    let unsolicited = Name::create("unsolicited")?;
    let respond_to_any_master = Name::create("respond_to_any_master")?;

    let features = lib.declare_function_argument_struct("outstation_features")?;
    let features = lib
        .define_function_argument_struct(features)?
        .add(
            &self_address,
            Primitive::Bool,
            "Respond to the self address",
        )?
        .add(
            &broadcast,
            Primitive::Bool,
            "Process valid broadcast messages",
        )?
        .add(
            &unsolicited,
            Primitive::Bool,
            "Respond to enable/disable unsolicited response and produce unsolicited responses",
        )?
        .add(
            &respond_to_any_master,
            Primitive::Bool,
            doc("Outstation will process every request as if it came from the configured master address")
                .details("This feature is a hack that can make configuration of some systems easier/more flexible, but should not be used when unsolicited reporting is also required.")
        )?
        .doc("Optional outstation features that can be enabled or disabled")?
        .end_fields()?
        .begin_initializer(
            "init",
            InitializerType::Normal,
            "Initialize to default values",
        )?
        .default(&self_address, false)?
        .default(&broadcast, true)?
        .default(&unsolicited, true)?
        .default(&respond_to_any_master, false)?
        .end_initializer()?
        .build()?;

    Ok(features)
}

fn define_outstation_config(
    lib: &mut LibraryBuilder,
    shared: &SharedDefinitions,
) -> BackTraced<FunctionArgStructHandle> {
    let event_buffer_config = define_event_buffer_config(lib)?;
    let class_zero_config = define_class_zero_config(lib)?;
    let outstation_features = define_outstation_features(lib)?;

    let solicited_buffer_size = Name::create("solicited_buffer_size")?;
    let unsolicited_buffer_size = Name::create("unsolicited_buffer_size")?;
    let rx_buffer_size = Name::create("rx_buffer_size")?;
    let decode_level = Name::create("decode_level")?;
    let confirm_timeout = Name::create("confirm_timeout")?;
    let select_timeout = Name::create("select_timeout")?;
    let features = Name::create("features")?;
    let max_unsolicited_retries = Name::create("max_unsolicited_retries")?;
    let unsolicited_retry_delay = Name::create("unsolicited_retry_delay")?;
    let keep_alive_timeout = Name::create("keep_alive_timeout")?;
    let max_read_request_headers = Name::create("max_read_request_headers")?;
    let max_controls_per_request = Name::create("max_controls_per_request")?;
    let class_zero = Name::create("class_zero")?;

    let outstation_config = lib.declare_function_argument_struct("outstation_config")?;
    let outstation_config = lib
        .define_function_argument_struct(outstation_config)?
        .doc("Outstation configuration")?
        .add(
            "outstation_address",
            Primitive::U16,
            "Link-layer outstation address",
        )?
        .add("master_address", Primitive::U16, "Link-layer master address")?
        .add("event_buffer_config", event_buffer_config, "Event buffer sizes configuration")?
        .add(
            &solicited_buffer_size,
            Primitive::U16,
            doc("Solicited response buffer size").details("Must be at least 249 bytes"),
        )?
        .add(
            &unsolicited_buffer_size,
            Primitive::U16,
            doc("Unsolicited response buffer size").details("Must be at least 249 bytes"),
        )?
        .add(
            &rx_buffer_size,
            Primitive::U16,
            doc("Receive buffer size").details("Must be at least 249 bytes"),
        )?
        .add(
            &decode_level,
            shared.decode_level.clone(),
            "Decoding level",
        )?
        .add(
            &confirm_timeout,
            DurationType::Milliseconds,
            "Confirmation timeout",
        )?
        .add(
            &select_timeout,
            DurationType::Milliseconds,
            "Select timeout",
        )?
        .add(&features, outstation_features, "Optional features")?
        .add(
            &max_unsolicited_retries,
            Primitive::U32,
            "Maximum number of unsolicited retries",
        )?
        .add(
            &unsolicited_retry_delay,
            DurationType::Milliseconds,
            "Delay to wait before retrying an unsolicited response",
        )?
        .add(
            &keep_alive_timeout,
            DurationType::Milliseconds,
            doc("Delay of inactivity before sending a REQUEST_LINK_STATUS to the master")
                .details("A value of zero means no automatic keep-alive will be sent."),
        )?
        .add(&max_read_request_headers, Primitive::U16, doc("Maximum number of headers that will be processed in a READ request.").details("Internally, this controls the size of a pre-allocated buffer used to process requests. A minimum value of `DEFAULT_READ_REQUEST_HEADERS` is always enforced. Requesting more than this number will result in the PARAMETER_ERROR IIN bit being set in the response."))?
        .add(&max_controls_per_request, Primitive::U16, doc("Maximum number of controls in a single request."))?
        .add(&class_zero, class_zero_config, "Controls responses to Class 0 reads")?
        .end_fields()?
        .begin_initializer("init", InitializerType::Normal, "Initialize to defaults")?
        .default(&solicited_buffer_size, NumberValue::U16(2048))?
        .default(&unsolicited_buffer_size, NumberValue::U16(2048))?
        .default(&rx_buffer_size, NumberValue::U16(2048))?
        .default_struct(&decode_level)?
        .default(&confirm_timeout, Duration::from_secs(5))?
        .default(&select_timeout, Duration::from_secs(5))?
        .default(&features, InitializerDefault::DefaultStruct)?
        .default(&max_unsolicited_retries, NumberValue::U32(u32::MAX))?
        .default(&unsolicited_retry_delay, Duration::from_secs(5))?
        .default(&keep_alive_timeout, Duration::from_secs(60))?
        .default(&max_read_request_headers, NumberValue::U16(64))?
        .default(&max_controls_per_request, NumberValue::U16(u16::MAX))?
        .default_struct(&class_zero)?
        .end_initializer()?
        .build()?;

    Ok(outstation_config)
}

fn define_event_buffer_config(lib: &mut LibraryBuilder) -> BackTraced<FunctionArgStructHandle> {
    let max_binary = Name::create("max_binary")?;
    let max_double_bit_binary = Name::create("max_double_bit_binary")?;
    let max_binary_output_status = Name::create("max_binary_output_status")?;
    let max_counter = Name::create("max_counter")?;
    let max_frozen_counter = Name::create("max_frozen_counter")?;
    let max_analog = Name::create("max_analog")?;
    let max_analog_output_status = Name::create("max_analog_output_status")?;
    let max_octet_string = Name::create("max_octet_string")?;

    let event_buffer_config = lib.declare_function_argument_struct("event_buffer_config")?;
    let event_buffer_config = lib
        .define_function_argument_struct(event_buffer_config)?
        .add(
            &max_binary,
            Primitive::U16,
            "Maximum number of Binary Input events (g2)",
        )?
        .add(
            &max_double_bit_binary,
            Primitive::U16,
            "Maximum number of Double-Bit Binary Input events (g4)",
        )?
        .add(
            &max_binary_output_status,
            Primitive::U16,
            "Maximum number of Binary Output Status events (g11)",
        )?
        .add(
            &max_counter,
            Primitive::U16,
            "Maximum number of Counter events (g22)",
        )?
        .add(
            &max_frozen_counter,
            Primitive::U16,
            "Maximum number of Frozen Counter events (g23)",
        )?
        .add(
            &max_analog,
            Primitive::U16,
            "Maximum number of Analog Input events (g32)",
        )?
        .add(
            &max_analog_output_status,
            Primitive::U16,
            "Maximum number of Analog Output Status events (g42)",
        )?
        .add(
            &max_octet_string,
            Primitive::U16,
            doc("Maximum number of Octet String events (g111)"),
        )?
        .doc(
            doc("Maximum number of events for each type")
                .details("A value of zero means that events will not be buffered for that type."),
        )?
        .end_fields()?
        .add_full_initializer("init")?
        .begin_initializer(
            "no_events",
            InitializerType::Static,
            "Create a configuration where no events are buffered.",
        )?
        .default(&max_binary, NumberValue::U16(0))?
        .default(&max_double_bit_binary, NumberValue::U16(0))?
        .default(&max_binary_output_status, NumberValue::U16(0))?
        .default(&max_counter, NumberValue::U16(0))?
        .default(&max_frozen_counter, NumberValue::U16(0))?
        .default(&max_analog, NumberValue::U16(0))?
        .default(&max_analog_output_status, NumberValue::U16(0))?
        .default(&max_octet_string, NumberValue::U16(0))?
        .end_initializer()?
        .build()?;

    Ok(event_buffer_config)
}

fn define_application_iin(lib: &mut LibraryBuilder) -> BackTraced<UniversalStructHandle> {
    let need_time = Name::create("need_time")?;
    let local_control = Name::create("local_control")?;
    let device_trouble = Name::create("device_trouble")?;
    let config_corrupt = Name::create("config_corrupt")?;

    let application_iin = lib.declare_universal_struct("application_iin")?;
    let application_iin = lib
        .define_universal_struct(application_iin)?
        .add(
            need_time.clone(),
            Primitive::Bool,
            "IIN1.4 - Time synchronization is required",
        )?
        .add(
            local_control.clone(),
            Primitive::Bool,
            "IIN1.5 - Some output points are in local mode",
        )?
        .add(
            device_trouble.clone(),
            Primitive::Bool,
            "IIN1.6 - Device trouble",
        )?
        .add(
            config_corrupt.clone(),
            Primitive::Bool,
            "IIN2.5 - Configuration corrupt",
        )?
        .doc("Application-controlled IIN bits")?
        .end_fields()?
        .begin_initializer(
            "init",
            InitializerType::Normal,
            "Initialize all fields in {struct:application_iin} to false",
        )?
        .default(&need_time, false)?
        .default(&local_control, false)?
        .default(&device_trouble, false)?
        .default(&config_corrupt, false)?
        .end_initializer()?
        .build()?;

    Ok(application_iin)
}

fn define_restart_delay(lib: &mut LibraryBuilder) -> BackTraced<UniversalStructHandle> {
    let restart_delay_type = lib
        .define_enum("restart_delay_type")?
        .push("not_supported", "Restart mode not supported")?
        .push("seconds", "Value is in seconds (corresponds to g51v1)")?
        .push(
            "milli_seconds",
            "Value is in milliseconds (corresponds to g51v2)",
        )?
        .doc("Type of restart delay value. Used by {struct:restart_delay}.")?
        .build()?;

    let restart_type = Name::create("restart_type")?;
    let value = Name::create("value")?;

    let restart_delay = lib.declare_universal_struct("restart_delay")?;
    let restart_delay = lib.define_universal_struct(restart_delay)?
        .add(restart_type.clone(), restart_delay_type, "Indicates what {struct:restart_delay.value} is.")?
        .add(value.clone(), Primitive::U16, "Expected delay before the outstation comes back online.")?
        .doc(doc("Restart delay used by {interface:outstation_application.cold_restart()} and {interface:outstation_application.warm_restart()}")
            .details("If {struct:restart_delay.restart_type} is not {enum:restart_delay_type.not_supported}, then the {struct:restart_delay.value} is valid. Otherwise, the outstation will return IIN2.0 NO_FUNC_CODE_SUPPORT."))?
        .end_fields()?
        // -----
        .begin_initializer("not_supported", InitializerType::Static, "RestartDelay indicating that the request is not supported")?
        .default_variant(&restart_type, "not_supported")?
        .default(&value, NumberValue::U16(0))?
        .end_initializer()?
        // -----
        .begin_initializer("seconds", InitializerType::Static, "RestartDelay with a count of seconds")?
        .default_variant(&restart_type, "seconds")?
        .end_initializer()?
        // -----
        .begin_initializer("milliseconds", InitializerType::Static, "RestartDelay with a count of milliseconds")?
        .default_variant(&restart_type, "milli_seconds")?
        .end_initializer()?
        // -----
        .build()?;

    Ok(restart_delay)
}

fn define_outstation_application(
    lib: &mut LibraryBuilder,
    shared: &SharedDefinitions,
    database_handle: &ClassHandle,
) -> BackTraced<AsynchronousInterface> {
    let restart_delay = define_restart_delay(lib)?;

    let write_time_result = lib.define_enum("write_time_result")?
        .push("ok", "The write time operation succeeded.")?
        .push("parameter_error", "The request parameters are nonsensical.")?
        .push("not_supported", "Writing time is not supported by this outstation (translated to NO_FUNC_CODE_SUPPORT).")?
        .doc("Write time result used by {interface:outstation_application.write_absolute_time()}")?
        .build()?;

    let freeze_type = lib.define_enum("freeze_type")?
        .push("immediate_freeze", "Copy the current value of a counter to the associated point")?
        .push("freeze_and_clear", "Copy the current value of a counter to the associated point and clear the current value to 0.")?
        .doc("Freeze operation type")?
        .build()?;

    let freeze_result = lib
        .define_enum("freeze_result")?
        .push("ok", "Freeze operation was successful.")?
        .push("parameter_error", "The request parameters are nonsensical.")?
        .push(
            "not_supported",
            "The demanded freeze operation is not supported by this device.",
        )?
        .doc("Result of a freeze operation")?
        .build()?;

    let application_iin = define_application_iin(lib)?;

    let buffer_state = define_buffer_state(lib)?;

    let freeze_not_supported = freeze_result.value("not_supported")?;

    let set_doc = "Set to which the attribute belongs";
    let var_doc = "Variation of the attribute";
    let attr_return_doc = "If true, the value will be modified in the in memory database and the outstation will return a successful response. If false, no change will be made and the outstation will return PARAM_ERROR";
    let attr_enum_doc = "Enumeration describing which attribute it is, possibly unknown";

    let application = lib.define_interface("outstation_application", "Dynamic information required by the outstation from the user application")?
        .begin_callback("get_processing_delay_ms", doc("Returns the DELAY_MEASUREMENT delay")
            .details("The value returned by this method is used in conjunction with the DELAY_MEASUREMENT function code and returned in a g52v2 time delay object as part of a non-LAN time synchronization procedure.")
            .details("It represents the processing delay from receiving the request to sending the response. This parameter should almost always use the default value of zero as only an RTOS or bare metal system would have access to this level of timing. Modern hardware can almost always respond in less than 1 millisecond anyway.")
            .details("For more information, see IEEE-1815 2012, p. 64."))?
            .returns(Primitive::U16, "Processing delay, in milliseconds")?
            .end_callback()?
        .begin_callback("write_absolute_time", "Handle a write of the absolute time during time synchronization procedures.")?
            .param("time", Primitive::U64, "Received time in milliseconds since EPOCH (only 48 bits are used)")?
            .returns(write_time_result, "Result of the write time operation")?
            .end_callback()?
        .begin_callback("get_application_iin", "Returns the application-controlled IIN bits")?
            .returns(application_iin, "Application IIN bits")?
            .end_callback()?
        .begin_callback("cold_restart", doc("Request that the outstation perform a cold restart (IEEE-1815 2012, p. 58)")
            .details("The outstation will not automatically restart. It is the responsibility of the user application to handle this request and take the appropriate action."))?
            .returns(restart_delay.clone(), "The restart delay")?
            .end_callback()?
        .begin_callback("warm_restart", doc("Request that the outstation perform a warm restart (IEEE-1815 2012, p. 58)")
            .details("The outstation will not automatically restart. It is the responsibility of the user application to handle this request and take the appropriate action."))?
            .returns(restart_delay, "The restart delay")?
            .end_callback()?
        .begin_callback("freeze_counters_all", "Freeze all the counters")?
            .param("freeze_type", freeze_type.clone(), "Type of freeze operation")?
            .param("database_handle",database_handle.declaration(), "Database handle")?
            .returns_with_default(freeze_not_supported.clone(), "Result of the freeze operation")?
            .end_callback()?
        .begin_callback("freeze_counters_all_at_time",
                        doc("Freeze all the counters at a requested time and interval")
                            .details("Refer to the table on page 57 of IEEE 1815-2012 to interpret the time and interval parameters correctly")
             )?
            .param("database_handle",database_handle.declaration(), "Database handle")?
            .param("time", Primitive::U64, "48-bit DNP3 timestamp in milliseconds since epoch UTC")?
            .param("interval", Primitive::U32, "Count of milliseconds representing the interval between freezes relative to the timestamp")?
            .returns_with_default(freeze_not_supported.clone(), "Result of the freeze operation")?
            .end_callback()?
        .begin_callback("freeze_counters_range", "Freeze a range of counters")?
            .param("start", Primitive::U16, "Start index to freeze (inclusive)")?
            .param("stop", Primitive::U16, "Stop index to freeze (inclusive)")?
            .param("freeze_type", freeze_type, "Type of freeze operation")?
            .param("database_handle",database_handle.declaration(), "Database handle")?
            .returns_with_default(freeze_not_supported.clone(), "Result of the freeze operation")?
            .end_callback()?
        .begin_callback("freeze_counters_range_at_time",
                        doc("Freeze a range of counters at a requested time and interval")
                            .details("Refer to the table on page 57 of IEEE 1815-2012 to interpret the time and interval parameters correctly")
             )?
            .param("start", Primitive::U16, "Start index to freeze (inclusive)")?
            .param("stop", Primitive::U16, "Stop index to freeze (inclusive)")?
            .param("database_handle",database_handle.declaration(), "Database handle")?
           .param("time", Primitive::U64, "48-bit DNP3 timestamp in milliseconds since epoch UTC")?
            .param("interval", Primitive::U32, "Count of milliseconds representing the interval between freezes relative to the timestamp")?
            .returns_with_default(freeze_not_supported, "Result of the freeze operation")?
        .end_callback()?
        .begin_callback("support_write_analog_dead_bands",
                        doc("Controls outstation support for writing group 34, analog input dead-bands")
                            .details("Returning false, indicates that the writes to group34 should not be processed and requests to do so should be rejected with IIN2.NO_FUNC_CODE_SUPPORT")
                            .details("Returning true will allow the request to process the actual values with a sequence of calls:")
                            .details("1) A single call to {interface:outstation_application.begin_write_analog_dead_bands()}")
                            .details("2) Zero or more calls to {interface:outstation_application.write_analog_dead_band()}")
                            .details("3) A single call to {interface:outstation_application.end_write_analog_dead_bands()}")
             )?
            .returns_with_default(PrimitiveValue::Bool(false), "True if the outstation should process the request")?
            .end_callback()?
        .begin_callback("begin_write_analog_dead_bands", "Called when the outstation begins processing a header to write analog dead-bands")?
            .returns_nothing_by_default()?
            .end_callback()?
        .begin_callback("write_analog_dead_band",
                        doc("Called when the outstation begins processing a header to write analog dead-bands")
                            .details("Called for each analog dead-band in the write request where an analog input is defined at the specified index.")
                            .details("The dead-band is automatically updated in the database. This callback allows application code to persist the modified value to non-volatile memory if desired")
            )?
            .param("index", Primitive::U16, "Index of the analog input")?
            .param("dead_band", Primitive::Double, "New dead-band value")?
            .returns_nothing_by_default()?
            .end_callback()?
        .begin_callback("end_write_analog_dead_bands",
                        doc("Called when the outstation completes processing a header to write analog dead-bands")
                            .details("Multiple dead-bands changes can be accumulated in calls to {interface:outstation_application.write_analog_dead_band()} and then be processed as a batch in this method.")
             )?
            .returns_nothing_by_default()?
            .end_callback()?
        // attribute callbacks
        .begin_callback("write_string_attr",
                        doc("Write a string attribute. This method is only called if the corresponding attribute has been configured as writable")
        )?
        .param("set", Primitive::U8, set_doc)?
        .param("variation", Primitive::U8, var_doc)?
        .param("attr_type", shared.attr.string_attr.clone(), attr_enum_doc)?
        .param("value", StringType, "Value of the attribute")?
        .returns_with_default(PrimitiveValue::Bool(false), attr_return_doc)?
        .end_callback()?
        .begin_callback("write_float_attr",
                        doc("Write a 32-bit floating point attribute. This method is only called if the corresponding attribute has been configured as writable")
        )?
        .param("set", Primitive::U8, set_doc)?
        .param("variation", Primitive::U8, var_doc)?
        .param("attr_type", shared.attr.float_attr.clone(), attr_enum_doc)?
        .param("value", Primitive::Float, "Value of the attribute")?
        .returns_with_default(PrimitiveValue::Bool(false), attr_return_doc)?
        .end_callback()?
        .begin_callback("write_double_attr",
                        doc("Write a 64-bit floating point attribute. This method is only called if the corresponding attribute has been configured as writable")
        )?
        .param("set", Primitive::U8, set_doc)?
        .param("variation", Primitive::U8, var_doc)?
        .param("attr_type", shared.attr.float_attr.clone(), attr_enum_doc)?
        .param("value", Primitive::Double, "Value of the attribute")?
        .returns_with_default(PrimitiveValue::Bool(false), attr_return_doc)?
        .end_callback()?
        .begin_callback("write_uint_attr",
                        doc("Write an unsigned integer attribute. This method is only called if the corresponding attribute has been configured as writable")
        )?
        .param("set", Primitive::U8, set_doc)?
        .param("variation", Primitive::U8, var_doc)?
        .param("attr_type", shared.attr.uint_attr.clone(), attr_enum_doc)?
        .param("value", Primitive::U32, "Value of the attribute")?
        .returns_with_default(PrimitiveValue::Bool(false), attr_return_doc)?
        .end_callback()?
        .begin_callback("write_int_attr",
                        doc("Write a signed integer attribute. This method is only called if the corresponding attribute has been configured as writable")
        )?
        .param("set", Primitive::U8, set_doc)?
        .param("variation", Primitive::U8, var_doc)?
        .param("attr_type", shared.attr.int_attr.clone(), attr_enum_doc)?
        .param("value", Primitive::S32, "Value of the attribute")?
        .returns_with_default(PrimitiveValue::Bool(false), attr_return_doc)?
        .end_callback()?
        .begin_callback("write_octet_string_attr",
                        doc("Write an octet-string attribute. This method is only called if the corresponding attribute has been configured as writable")
        )?
        .param("set", Primitive::U8, set_doc)?
        .param("variation", Primitive::U8, var_doc)?
        .param("attr_type", shared.attr.octet_string_attr.clone(), attr_enum_doc)?
        .param("value", shared.byte_it.clone(), "Iterator over bytes of the value")?
        .returns_with_default(PrimitiveValue::Bool(false), attr_return_doc)?
        .end_callback()?
        .begin_callback("write_bit_string_attr",
                        doc("Write a bit-string attribute. This method is only called if the corresponding attribute has been configured as writable")
        )?
        .param("set", Primitive::U8, set_doc)?
        .param("variation", Primitive::U8, var_doc)?
        .param("attr_type", shared.attr.bit_string_attr.clone(), attr_enum_doc)?
        .param("value", shared.byte_it.clone(), "Iterator over bytes of the value")?
        .returns_with_default(PrimitiveValue::Bool(false), attr_return_doc)?
        .end_callback()?
        .begin_callback("write_time_attr",
                        doc("Write a DNP3 time attribute. This method is only called if the corresponding attribute has been configured as writable.")
        )?
        .param("set", Primitive::U8, set_doc)?
        .param("variation", Primitive::U8, var_doc)?
        .param("attr_type", shared.attr.time_attr.clone(), attr_enum_doc)?
        .param("value", Primitive::U64, "48-bit DNP3 timestamp value")?
        .returns_with_default(PrimitiveValue::Bool(false), attr_return_doc)?
        .end_callback()?
        // event CONFIRM handling
        // begin CONFIRM
        .begin_callback("begin_confirm", doc("Called when a CONFIRM is received to a response or unsolicited response, but before any previously transmitted events are cleared from the buffer"))?
        .returns_nothing_by_default()?
        .end_callback()?
        // event cleared
        .begin_callback("event_cleared", doc("Called when an event is cleared from the buffer due to master acknowledgement"))?
        .param("id", Primitive::U64, "Unique identifier previously assigned to the event by the database in an update method")?
        .returns_nothing_by_default()?
        .end_callback()?
        // end CONFIRM
        .begin_callback("end_confirm", doc(" Called when all relevant events have been cleared"))?
        .param("state", buffer_state, "information about the post-CONFIRM state of the buffer")?
        .returns_nothing_by_default()?
        .end_callback()?
        .build_async()?;

    Ok(application)
}

fn define_outstation_information(
    lib: &mut LibraryBuilder,
    shared_def: &SharedDefinitions,
) -> BackTraced<AsynchronousInterface> {
    let request_header = lib.declare_callback_argument_struct("request_header")?;
    let request_header = lib
        .define_callback_argument_struct(request_header)?
        .add(
            "control_field",
            shared_def.control_field_struct.clone(),
            "Control field",
        )?
        .add(
            "function",
            shared_def.function_code.clone(),
            "Function code",
        )?
        .doc("Application-layer header for requests")?
        .end_fields()?
        .build()?;

    let broadcast_action = lib.define_enum("broadcast_action")?
        .push("processed", "Outstation processed the broadcast")?
        .push("ignored_by_configuration", "Outstation ignored the broadcast message b/c it is disabled by configuration")?
        .push("bad_object_headers", "Outstation was unable to parse the object headers and ignored the request")?
        .push("unsupported_function", "Outstation ignore the broadcast message b/c the function is not supported via Broadcast")?
        .doc("Enumeration describing how the outstation processed a broadcast request")?
        .build()?;

    let information = lib.define_interface("outstation_information", doc("Informational callbacks that the outstation doesn't rely on to function").details("It may be useful to certain applications to assess the health of the communication or to count statistics"))?
        .begin_callback("process_request_from_idle", "Called when a request is processed from the IDLE state")?
            .param("header", request_header, "Request header")?

            .end_callback()?
        .begin_callback("broadcast_received", "Called when a broadcast request is received by the outstation")?
            .param("function_code", shared_def.function_code.clone(), "Function code received")?
            .param("action", broadcast_action, "Broadcast action")?

            .end_callback()?
        .begin_callback("enter_solicited_confirm_wait", "Outstation has begun waiting for a solicited confirm")?
            .param("ecsn", Primitive::U8, "Expected sequence number")?

            .end_callback()?
        .begin_callback("solicited_confirm_timeout", "Failed to receive a solicited confirm before the timeout occurred")?
            .param("ecsn", Primitive::U8, "Expected sequence number")?

            .end_callback()?
        .begin_callback("solicited_confirm_received", "Received the expected confirm")?
            .param("ecsn", Primitive::U8, "Expected sequence number")?

            .end_callback()?
        .begin_callback("solicited_confirm_wait_new_request", "Received a new request while waiting for a solicited confirm, aborting the response series")?

            .end_callback()?
        .begin_callback("wrong_solicited_confirm_seq", "Received a solicited confirm with the wrong sequence number")?
            .param("ecsn", Primitive::U8, "Expected sequence number")?
            .param("seq", Primitive::U8, "Received sequence number")?

            .end_callback()?
        .begin_callback("unexpected_confirm", "Received a confirm when not expecting one")?
            .param("unsolicited", Primitive::Bool, "True if it's an unsolicited response confirm, false if it's a solicited response confirm")?
            .param("seq", Primitive::U8, "Received sequence number")?

            .end_callback()?
        .begin_callback("enter_unsolicited_confirm_wait", "Outstation has begun waiting for an unsolicited confirm")?
            .param("ecsn", Primitive::U8, "Expected sequence number")?

            .end_callback()?
        .begin_callback("unsolicited_confirm_timeout", "Failed to receive an unsolicited confirm before the timeout occurred")?
            .param("ecsn", Primitive::U8, "Expected sequence number")?
            .param("retry", Primitive::Bool, "Is it a retry")?

            .end_callback()?
        .begin_callback("unsolicited_confirmed", "Master confirmed an unsolicited message")?
            .param("ecsn", Primitive::U8, "Expected sequence number")?

            .end_callback()?
        .begin_callback("clear_restart_iin", "Master cleared the restart IIN bit")?

            .end_callback()?
        .build_async()?;

    Ok(information)
}

fn define_control_handler(
    lib: &mut LibraryBuilder,
    database_handle: &ClassHandle,
    shared_def: &SharedDefinitions,
) -> BackTraced<AsynchronousInterface> {
    let operate_type = lib
        .define_enum("operate_type")?
        .push(
            "select_before_operate",
            "control point was properly selected before the operate request",
        )?
        .push(
            "direct_operate",
            "operate the control via a DirectOperate request",
        )?
        .push(
            "direct_operate_no_ack",
            "operate the control via a DirectOperateNoAck request",
        )?
        .doc("Enumeration describing how the master requested the control operation")?
        .build()?;

    let select_details_1 = "Implementors can think of this function as asking the question \"is this control supported\"?";
    let select_details_2 = "Most implementations should not alter the database in this method. It is only provided in the event that some event counters reflected via the API get updated on SELECT, but this would be highly abnormal.";
    let select_g12_doc = doc("Select a CROB, but do not operate")
        .details(select_details_1)
        .details(select_details_2);
    let select_g40_doc = doc("Select an analog output, but do not operate")
        .details(select_details_1)
        .details(select_details_2);

    let control_handler = lib
        .define_interface("control_handler", "Callbacks for handling controls")?
        //------
        .begin_callback("begin_fragment", "Notifies the start of a command fragment")?
        .end_callback()?
        .begin_callback("end_fragment", "Notifies the end of a command fragment")?
        .param("database", database_handle.declaration(), "Database handle")?
        .end_callback()?
        //------
        .begin_callback("select_g12v1", select_g12_doc)?
        .param("value", shared_def.g12v1_struct.clone(), "Received CROB")?
        .param("index", Primitive::U16, "Index of the point")?
        .param(
            "database_handle",
            database_handle.declaration(),
            "Database handle",
        )?
        .returns(shared_def.command_status.clone(), "Command status")?
        .end_callback()?
        //------
        .begin_callback("operate_g12v1", "Operate a control point")?
        .param("value", shared_def.g12v1_struct.clone(), "Received CROB")?
        .param("index", Primitive::U16, "Index of the point")?
        .param("op_type", operate_type.clone(), "Operate type")?
        .param(
            "database_handle",
            database_handle.declaration(),
            "Database handle",
        )?
        .returns(shared_def.command_status.clone(), "Command status")?
        .end_callback()?
        //------
        .begin_callback("select_g41v1", select_g40_doc.clone())?
        .param("value", Primitive::S32, "Received analog output value")?
        .param("index", Primitive::U16, "Index of the point")?
        .param(
            "database_handle",
            database_handle.declaration(),
            "Database handle",
        )?
        .returns(shared_def.command_status.clone(), "Command status")?
        .end_callback()?
        //------
        .begin_callback("operate_g41v1", "Operate a control point")?
        .param("value", Primitive::S32, "Received analog output value")?
        .param("index", Primitive::U16, "Index of the point")?
        .param("op_type", operate_type.clone(), "Operate type")?
        .param(
            "database_handle",
            database_handle.declaration(),
            "Database handle",
        )?
        .returns(shared_def.command_status.clone(), "Command status")?
        .end_callback()?
        //------
        .begin_callback("select_g41v2", select_g40_doc.clone())?
        .param("value", Primitive::S16, "Received analog output value")?
        .param("index", Primitive::U16, "Index of the point")?
        .param(
            "database_handle",
            database_handle.declaration(),
            "Database handle",
        )?
        .returns(shared_def.command_status.clone(), "Command status")?
        .end_callback()?
        //------
        .begin_callback("operate_g41v2", "Operate a control point")?
        .param("value", Primitive::S16, "Received analog output value")?
        .param("index", Primitive::U16, "Index of the point")?
        .param("op_type", operate_type.clone(), "Operate type")?
        .param(
            "database_handle",
            database_handle.declaration(),
            "Database handle",
        )?
        .returns(shared_def.command_status.clone(), "Command status")?
        .end_callback()?
        //------
        .begin_callback("select_g41v3", select_g40_doc.clone())?
        .param("value", Primitive::Float, "Received analog output value")?
        .param("index", Primitive::U16, "Index of the point")?
        .param(
            "database_handle",
            database_handle.declaration(),
            "Database handle",
        )?
        .returns(shared_def.command_status.clone(), "Command status")?
        .end_callback()?
        //------
        .begin_callback("operate_g41v3", "Operate a control point")?
        .param("value", Primitive::Float, "Received analog output value")?
        .param("index", Primitive::U16, "Index of the point")?
        .param("op_type", operate_type.clone(), "Operate type")?
        .param(
            "database_handle",
            database_handle.declaration(),
            "Database handle",
        )?
        .returns(shared_def.command_status.clone(), "Command status")?
        .end_callback()?
        //------
        .begin_callback("select_g41v4", select_g40_doc)?
        .param("value", Primitive::Double, "Received analog output value")?
        .param("index", Primitive::U16, "Index of the point")?
        .param(
            "database_handle",
            database_handle.declaration(),
            "Database handle",
        )?
        .returns(shared_def.command_status.clone(), "Command status")?
        .end_callback()?
        //------
        .begin_callback("operate_g41v4", "Operate a control point")?
        .param("value", Primitive::Double, "Received analog output value")?
        .param("index", Primitive::U16, "Index of the point")?
        .param("op_type", operate_type, "Operate type")?
        .param(
            "database_handle",
            database_handle.declaration(),
            "Database handle",
        )?
        .returns(shared_def.command_status.clone(), "Command status")?
        .end_callback()?
        //------
        .build_async()?;

    Ok(control_handler)
}

fn define_connection_state_listener(lib: &mut LibraryBuilder) -> BackTraced<AsynchronousInterface> {
    let state = lib
        .define_enum("connection_state")?
        .push("connected", "Connected to the master")?
        .push("disconnected", "Disconnected from the master")?
        .doc("Outstation connection state for connection-oriented transports, e.g. TCP")?
        .build()?;

    let listener = lib
        .define_interface(
            "connection_state_listener",
            "Callback interface for connection state events",
        )?
        .begin_callback("on_change", "Called when the connection state changes")?
        .param("state", state, "New state of the connection")?
        .end_callback()?
        .build_async()?;

    Ok(listener)
}

fn define_address_filter(
    lib: &mut LibraryBuilder,
    shared_def: &SharedDefinitions,
) -> BackTraced<ClassHandle> {
    let address_filter = lib.declare_class("address_filter")?;

    let address_filter_any_fn = lib
        .define_function("address_filter_any")?
        .returns(address_filter.clone(), "Address filter")?
        .doc("Create an address filter that accepts any IP address")?
        .build_static("any")?;

    let constructor = lib
        .define_constructor(address_filter.clone())?
        .param("address", StringType, "IP address to accept")?
        .fails_with(shared_def.error_type.clone())?
        .doc(
            doc("Create an address filter that matches a specific address or wildcards")
                .details("Examples: 192.168.1.26, 192.168.0.*, *.*.*.*")
                .details("Wildcards are only supported for IPv4 addresses"),
        )?
        .build()?;

    let add = lib
        .define_method("add", address_filter.clone())?
        .param("address", StringType, "IP address to add")?
        .fails_with(shared_def.error_type.clone())?
        .doc("Add an accepted IP address to the filter")?
        .build()?;

    let destructor = lib.define_destructor(address_filter.clone(), "Destroy an address filter")?;

    let address_filter = lib
        .define_class(&address_filter)?
        .constructor(constructor)?
        .destructor(destructor)?
        .static_method(address_filter_any_fn)?
        .method(add)?
        .doc(
            doc("Filters connecting client by their IP address to associate a connecting master with an outstation on the server")
                .details("Address filters must be DISJOINT, i.e. two filters cannot accept the same IP address. The {class:outstation_server.add_outstation()} method will fail if the filter conflicts with a previously added filter.")
        )?
        .build()?;

    Ok(address_filter)
}

fn define_tls_server_config(
    lib: &mut LibraryBuilder,
    shared: &SharedDefinitions,
) -> BackTraced<FunctionArgStructHandle> {
    let min_tls_version = Name::create("min_tls_version")?;
    let certificate_mode = Name::create("certificate_mode")?;
    let allow_client_name_wildcard = Name::create("allow_client_name_wildcard")?;

    let tls_server_config = lib.declare_function_argument_struct("tls_server_config")?;
    let tls_server_config = lib.define_function_argument_struct(tls_server_config)?
        .add("dns_name", StringType,
             doc("Subject name which is verified in the presented client certificate, from the SAN extension or in the common name field.")
            .warning("This argument is only used when used with {enum:certificate_mode.authority_based} "))?
        .add(
            "peer_cert_path",
            StringType,
            "Path to the PEM-encoded certificate of the peer",
        )?
        .add(
            "local_cert_path",
            StringType,
            "Path to the PEM-encoded local certificate",
        )?
        .add(
            "private_key_path",
            StringType,
            "Path to the PEM-encoded private key",
        )?
        .add(
            "password",
            StringType,
            doc("Optional password if the private key file is encrypted").details("Only PKCS#8 encrypted files are supported.").details("Pass empty string if the file is not encrypted.")
        )?
        .add(
            min_tls_version.clone(),
            shared.min_tls_version.clone(),
            "Minimum TLS version allowed",
        )?
        .add(certificate_mode.clone(), shared.certificate_mode.clone(), "Certificate validation mode")?
        .add(allow_client_name_wildcard.clone(), Primitive::Bool, "If set to true, a '*' may be used for {struct:tls_server_config.dns_name} to allow any authenticated client to connect")?
        .doc("TLS server configuration")?
        .end_fields()?
        .begin_initializer("init", InitializerType::Normal, "construct the configuration with defaults")?
        .default_variant(&min_tls_version, "v12")?
        .default_variant(&certificate_mode, "authority_based")?
        .default(&allow_client_name_wildcard, false)?
        .end_initializer()?
        .build()?;

    Ok(tls_server_config)
}
