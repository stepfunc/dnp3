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
    fn define(lib: &mut LibraryBuilder, shared_def: &SharedDefinitions) -> BackTraced<Self> {
        let DatabaseTypes {
            database_transaction,
            database_handle,
        } = crate::database::define(lib, shared_def)?;

        Ok(Self {
            database_transaction,
            outstation_config: define_outstation_config(lib, shared_def)?,
            outstation_application: define_outstation_application(lib, &database_handle)?,
            outstation_information: define_outstation_information(lib, shared_def)?,
            control_handler: define_control_handler(lib, &database_handle, shared_def)?,
            connection_state_listener: define_connection_state_listener(lib)?,
        })
    }
}

pub fn define(lib: &mut LibraryBuilder, shared_def: &SharedDefinitions) -> BackTraced<()> {
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
    shared_def: &SharedDefinitions,
    types: &OutstationTypes,
) -> BackTraced<ClassHandle> {
    let outstation = lib.declare_class("outstation")?;

    let outstation_create_serial_session_fn = lib
        .define_function("outstation_create_serial_session")?
        .param(
            "runtime",
            shared_def.runtime_class.clone(),
            "runtime on which to spawn the outstation",
        )?
        .param("serial_path", StringType, "Path of the serial device")?
        .param(
            "settings",
            shared_def.serial_port_settings.clone(),
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
        .fails_with(shared_def.error_type.clone())?
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
            shared_def.runtime_class.clone(),
            "runtime on which to spawn the outstation",
        )?
        .param("serial_path", StringType, "Path of the serial device")?
        .param(
            "settings",
            shared_def.serial_port_settings.clone(),
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
        .fails_with(shared_def.error_type.clone())?
        .doc(
            doc("Create an outstation instance running on a serial port which is tolerant to the serial port being added and removed")
        )?
        .build_static("create_serial_session_fault_tolerant")?;

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
        .param("level", shared_def.decode_level.clone(), "Decode log")?
        .fails_with(shared_def.error_type.clone())?
        .doc("Set decoding log level")?
        .build()?;

    let outstation = lib
        .define_class(&outstation)?
        .destructor(destructor)?
        .static_method(outstation_create_serial_session_fn)?
        .static_method(outstation_create_serial_session_fault_tolerant_fn)?
        .method(execute_transaction)?
        .method(set_decode_level)?
        .doc(doc("Outstation handle").details("Use this handle to modify the internal database."))?
        .build()?;

    Ok(outstation)
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
            .returns(freeze_result.clone(), "Result of the freeze operation")?
            .end_callback()?
        .begin_callback("freeze_counters_range", "Freeze a range of counters")?
            .param("start", Primitive::U16, "Start index to freeze (inclusive)")?
            .param("stop", Primitive::U16, "Stop index to freeze (inclusive)")?
            .param("freeze_type", freeze_type, "Type of freeze operation")?
            .param("database_handle",database_handle.declaration(), "Database handle")?
            .returns(freeze_result, "Result of the freeze operation")?
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
    let command_status = define_command_status(lib)?;

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
        .returns(command_status.clone(), "Command status")?
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
        .returns(command_status.clone(), "Command status")?
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
        .returns(command_status.clone(), "Command status")?
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
        .returns(command_status.clone(), "Command status")?
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
        .returns(command_status.clone(), "Command status")?
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
        .returns(command_status.clone(), "Command status")?
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
        .returns(command_status.clone(), "Command status")?
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
        .returns(command_status.clone(), "Command status")?
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
        .returns(command_status.clone(), "Command status")?
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
        .returns(command_status, "Command status")?
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

    let tls_server_config = lib.declare_function_argument_struct("tls_server_config")?;
    let tls_server_config = lib.define_function_argument_struct(tls_server_config)?
        .add("dns_name", StringType, "Expected name to validate in the presented certificate (only in {enum:certificate_mode.authority_based} mode)")?
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
            "Path to the the PEM-encoded private key",
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
        .doc("TLS server configuration")?
        .end_fields()?
        .begin_initializer("init", InitializerType::Normal, "construct the configuration with defaults")?
        .default_variant(&min_tls_version, "v12")?
        .default_variant(&certificate_mode, "authority_based")?
        .end_initializer()?
        .build()?;

    Ok(tls_server_config)
}

fn define_command_status(lib: &mut LibraryBuilder) -> BackTraced<EnumHandle> {
    let command_status = lib.define_enum("command_status")?
    .push("success", "command was accepted, initiated, or queued (value == 0)")?
    .push("timeout", "command timed out before completing (value == 1)")?
    .push("no_select", "command requires being selected before operate, configuration issue (value == 2)")?
    .push("format_error", "bad control code or timing values (value == 3)")?
    .push("not_supported", "command is not implemented (value == 4)")?
    .push("already_active", "command is all ready in progress or its all ready in that mode (value == 5)")?
    .push("hardware_error", "something is stopping the command, often a local/remote interlock (value == 6)")?
    .push("local", "the function governed by the control is in local only control (value == 7)")?
    .push("too_many_ops", "the command has been done too often and has been throttled (value == 8)")?
    .push("not_authorized", "the command was rejected because the device denied it or an RTU intercepted it (value == 9)")?
    .push("automation_inhibit", "command not accepted because it was prevented or inhibited by a local automation process, such as interlocking logic or synchrocheck (value == 10)")?
    .push("processing_limited", "command not accepted because the device cannot process any more activities than are presently in progress (value == 11)")?
    .push("out_of_range", "command not accepted because the value is outside the acceptable range permitted for this point (value == 12)")?
    .push("downstream_local", "command not accepted because the outstation is forwarding the request to another downstream device which reported LOCAL (value == 13)")?
    .push("already_complete", "command not accepted because the outstation has already completed the requested operation (value == 14)")?
    .push("blocked", "command not accepted because the requested function is specifically blocked at the outstation (value == 15)")?
    .push("canceled", "command not accepted because the operation was cancelled (value == 16)")?
    .push("blocked_other_master", "command not accepted because another master is communicating with the outstation and has exclusive rights to operate this control point (value == 17)")?
    .push("downstream_fail", "command not accepted because the outstation is forwarding the request to another downstream device which cannot be reached or is otherwise incapable of performing the request (value == 18)")?
    .push("non_participating", "(deprecated) indicates the outstation shall not issue or perform the control operation (value == 126)")?
    .push("unknown", "captures any value not defined in the enumeration")?
    .doc("Enumeration received from an outstation in response to command request")?
    .build()?;

    Ok(command_status)
}
