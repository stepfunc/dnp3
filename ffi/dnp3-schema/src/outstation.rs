use std::time::Duration;

use class::ClassHandle;
use oo_bindgen::callback::InterfaceHandle;
use oo_bindgen::native_enum::*;
use oo_bindgen::native_function::*;
use oo_bindgen::native_struct::*;
use oo_bindgen::*;

use crate::shared::SharedDefinitions;

pub fn define(
    lib: &mut LibraryBuilder,
    shared_def: &SharedDefinitions,
) -> Result<(), BindingError> {
    // Everything required to create an outstation

    let database = crate::database::define(lib, shared_def)?;
    let outstation = define_outstation(lib, shared_def, &database)?;
    let outstation_config = define_outstation_config(lib, shared_def)?;
    let event_buffer_config = define_event_buffer_config(lib)?;
    let outstation_application = define_outstation_application(lib)?;
    let outstation_information = define_outstation_information(lib, shared_def)?;
    let control_handler = define_control_handler(lib, &database, shared_def)?;
    let address_filter = define_address_filter(lib)?;

    // Define the TCP server
    let tcp_server = lib.declare_class("TCPServer")?;

    let tcp_server_new_fn = lib
        .declare_native_function("tcpserver_new")?
        .param(
            "runtime",
            Type::ClassRef(shared_def.runtime_class.clone()),
            "Runtime to execute the server on",
        )?
        .param(
            "link_error_mode",
            Type::Enum(shared_def.link_error_mode.clone()),
            "Controls how link errors are handled with respect to the TCP session",
        )?
        .param(
            "address",
            Type::String,
            "Address to bind the server to e.g. 127.0.0.1:20000",
        )?
        .return_type(ReturnType::new(
            Type::ClassRef(tcp_server.clone()),
            "New TCP server instance",
        ))?
        .doc(doc("Create a new TCP server.").details("To start it, use {class:TCPServer.bind()}."))?
        .build()?;

    let tcp_server_destroy_fn = lib.declare_native_function("tcpserver_destroy")?
        .param("server", Type::ClassRef(tcp_server.clone()), "Server to shutdown")?
        .return_type(ReturnType::void())?
        .doc("Gracefully shutdown all the outstations associated to this server, stops the server and release resources.")?
        .build()?;

    let tcp_server_add_outstation_fn = lib.declare_native_function("tcpserver_add_outstation")?
        .param("server", Type::ClassRef(tcp_server.clone()), "TCP server to add the outstation to")?
        .param("config", Type::Struct(outstation_config), "Outstation configuration")?
        .param("event_config", Type::Struct(event_buffer_config), "Event buffer configuration")?
        .param("application", Type::Interface(outstation_application), "Outstation application callbacks")?
        .param("information", Type::Interface(outstation_information), "Outstation information callbacks")?
        .param("control_handler", Type::Interface(control_handler), "Outstation control handler")?
        .param("filter", Type::ClassRef(address_filter.declaration()), "Address filter")?
        .return_type(ReturnType::new(Type::ClassRef(outstation.declaration()), "Outstation handle"))?
        .doc(doc("Add an outstation to the server.")
            .details("The returned {class:Outstation} can be used to modify points of the outstation.")
            .details("In order for the outstation to run, the TCP server must be running. Use {class:TCPServer.bind()} to run it."))?
        .build()?;

    let tcp_server_bind_fn = lib.declare_native_function("tcpserver_bind")?
        .param("server", Type::ClassRef(tcp_server.clone()), "Server to bind")?
        .return_type(ReturnType::Type(Type::Bool, "true if the bind was successful".into()))?
        .doc("Bind the server to the port and starts listening. Also starts all the outstations associated to it.")?
        .build()?;

    lib.define_class(&tcp_server)?
        .constructor(&tcp_server_new_fn)?
        .destructor(&tcp_server_destroy_fn)?
        .method("add_outstation", &tcp_server_add_outstation_fn)?
        .method("bind", &tcp_server_bind_fn)?
        .doc(doc("TCP server that listens for connections and routes the messages to outstations.")
        .details("To add outstations to it, use {class:TCPServer.add_outstation()}. Once all the outstations are added, the server can be started with {class:TCPServer.bind()}.")
        .details("{class:TCPServer.[destructor]} is used to gracefully shutdown all the outstations and the server."))?
        .build()?;

    Ok(())
}

fn define_outstation(
    lib: &mut LibraryBuilder,
    shared_def: &SharedDefinitions,
    database: &ClassHandle,
) -> Result<ClassHandle, BindingError> {
    let transaction_interface = lib
        .define_one_time_callback("OutstationTransaction", "Outstation transaction interface")?
        .callback(
            "execute",
            "Execute the transaction with the provided database",
        )?
        .param(
            "database",
            Type::ClassRef(database.declaration()),
            "Database",
        )?
        .return_type(ReturnType::void())?
        .build()?
        .build()?;

    let outstation = lib.declare_class("Outstation")?;

    let outstation_destroy_fn = lib.declare_native_function("outstation_destroy")?
        .param("outstation", Type::ClassRef(outstation.clone()), "Outstation to destroy")?
        .return_type(ReturnType::void())?
        .doc(doc("Free resources of the outstation.").warning("This does not shutdown the outstation. Only {class:TCPServer.[destructor]} will properly shutdown the outstation."))?
        .build()?;

    let outstation_transaction_fn = lib
        .declare_native_function("outstation_transaction")?
        .param(
            "outstation",
            Type::ClassRef(outstation.clone()),
            "Outstation",
        )?
        .param(
            "callback",
            Type::OneTimeCallback(transaction_interface),
            "Method to execute as a transaction",
        )?
        .return_type(ReturnType::void())?
        .doc("Execute transaction to modify the internal database of the outstation")?
        .build()?;

    let outstation_set_decode_level_fn = lib
        .declare_native_function("outstation_set_decode_level")?
        .param(
            "outstation",
            Type::ClassRef(outstation.clone()),
            "{class:Outstation} on which to set the decoding level",
        )?
        .param(
            "level",
            Type::Struct(shared_def.decode_level.clone()),
            "Decode log",
        )?
        .return_type(ReturnType::void())?
        .doc("Set decoding log level")?
        .build()?;

    lib.define_class(&outstation)?
        .destructor(&outstation_destroy_fn)?
        .method("transaction", &outstation_transaction_fn)?
        .method("set_decode_level", &outstation_set_decode_level_fn)?
        .doc(doc("Outstation handle").details("Use this handle to modify the internal database."))?
        .build()
}

fn define_outstation_config(
    lib: &mut LibraryBuilder,
    shared: &SharedDefinitions,
) -> Result<NativeStructHandle, BindingError> {
    let class_zero_config = lib.declare_native_struct("ClassZeroConfig")?;
    let class_zero_config = lib
        .define_native_struct(&class_zero_config)?
        .add(
            "binary",
            StructElementType::Bool(Some(true)),
            "Include Binary Inputs in Class 0 reads",
        )?
        .add(
            "double_bit_binary",
            StructElementType::Bool(Some(true)),
            "Include Double-Bit Binary Inputs in Class 0 reads",
        )?
        .add(
            "binary_output_status",
            StructElementType::Bool(Some(true)),
            "Include Binary Output Status in Class 0 reads",
        )?
        .add(
            "counter",
            StructElementType::Bool(Some(true)),
            "Include Counters in Class 0 reads",
        )?
        .add(
            "frozen_counter",
            StructElementType::Bool(Some(true)),
            "Include Frozen Counters in Class 0 reads",
        )?
        .add(
            "analog",
            StructElementType::Bool(Some(true)),
            "Include Analog Inputs in Class 0 reads",
        )?
        .add(
            "analog_output_status",
            StructElementType::Bool(Some(true)),
            "Include Analog Output Status in Class 0 reads",
        )?
        .add(
            "octet_strings",
            StructElementType::Bool(Some(false)),
            doc("Include Binary Inputs in Class 0 reads")
                .warning("For conformance, this should be false."),
        )?
        .doc("Controls which types are reported during a Class 0 read.")?
        .build()?;

    let features = lib.declare_native_struct("OutstationFeatures")?;
    let features = lib
        .define_native_struct(&features)?
        .add(
            "self_address",
            StructElementType::Bool(Some(false)),
            "Respond to the self address",
        )?
        .add(
            "broadcast",
            StructElementType::Bool(Some(true)),
            "Process valid broadcast messages",
        )?
        .add(
            "unsolicited",
            StructElementType::Bool(Some(true)),
            "Respond to enable/disable unsolicited response and produce unsolicited responses",
        )?
        .doc("Optional outstation features that can be enabled or disabled")?
        .build()?;

    let outstation_config = lib.declare_native_struct("OutstationConfig")?;
    let outstation_config = lib
        .define_native_struct(&outstation_config)?
        .add(
            "outstation_address",
            Type::Uint16,
            "Link-layer outstation address",
        )?
        .add("master_address", Type::Uint16, "Link-layer master address")?
        .add(
            "solicited_buffer_size",
            StructElementType::Uint16(Some(2048)),
            doc("Solicited response buffer size").details("Must be at least 249 bytes"),
        )?
        .add(
            "unsolicited_buffer_size",
            StructElementType::Uint16(Some(2048)),
            doc("Unsolicited response buffer size").details("Must be at least 249 bytes"),
        )?
        .add(
            "rx_buffer_size",
            StructElementType::Uint16(Some(2048)),
            doc("Receive buffer size").details("Must be at least 249 bytes"),
        )?
        .add(
            "decode_level",
            StructElementType::Struct(shared.decode_level.clone()),
            "Decoding level",
        )?
        .add(
            "confirm_timeout",
            StructElementType::Duration(DurationMapping::Milliseconds, Some(Duration::from_secs(5))),
            "Confirmation timeout",
        )?
        .add(
            "select_timeout",
            StructElementType::Duration(DurationMapping::Milliseconds, Some(Duration::from_secs(5))),
            "Select timeout",
        )?
        .add("features", Type::Struct(features), "Optional features")?
        .add(
            "max_unsolicited_retries",
            StructElementType::Uint32(Some(u32::MAX)),
            "Maximum number of unsolicited retries",
        )?
        .add(
            "unsolicited_retry_delay",
            StructElementType::Duration(DurationMapping::Milliseconds, Some(Duration::from_secs(5))),
            "Delay to wait before retrying an unsolicited response",
        )?
        .add(
            "keep_alive_timeout",
            StructElementType::Duration(DurationMapping::Milliseconds, Some(Duration::from_secs(60))),
            doc("Delay of inactivity before sending a REQUEST_LINK_STATUS to the master")
                .details("A value of zero means no automatic keep-alives."),
        )?
        .add("max_read_request_headers", StructElementType::Uint16(Some(64)), doc("Maximum number of headers that will be processed in a READ request.").details("Internally, this controls the size of a pre-allocated buffer used to process requests. A minimum value of `DEFAULT_READ_REQUEST_HEADERS` is always enforced. Requesting more than this number will result in the PARAMETER_ERROR IIN bit being set in the response."))?
        .add("max_controls_per_request", StructElementType::Uint16(Some(64)), doc("Maximum number of controls in a single request."))?
        .add("class_zero", Type::Struct(class_zero_config), "Controls responses to Class 0 reads")?
        .doc("Outstation configuration")?
        .build()?;

    Ok(outstation_config)
}

fn define_event_buffer_config(
    lib: &mut LibraryBuilder,
) -> Result<NativeStructHandle, BindingError> {
    let event_buffer_config = lib.declare_native_struct("EventBufferConfig")?;
    let event_buffer_config = lib
        .define_native_struct(&event_buffer_config)?
        .add(
            "max_binary",
            Type::Uint16,
            "Maximum number of Binary Input events (g2)",
        )?
        .add(
            "max_double_bit_binary",
            Type::Uint16,
            "Maximum number of Double-Bit Binary Input events (g4)",
        )?
        .add(
            "max_binary_output_status",
            Type::Uint16,
            "Maximum number of Binary Output Status events (g11)",
        )?
        .add(
            "max_counter",
            Type::Uint16,
            "Maximum number of Counter events (g22)",
        )?
        .add(
            "max_frozen_counter",
            Type::Uint16,
            "Maximum number of Frozen Counter events (g23)",
        )?
        .add(
            "max_analog",
            Type::Uint16,
            "Maximum number of Analog Input events (g32)",
        )?
        .add(
            "max_analog_output_status",
            Type::Uint16,
            "Maximum number of Analog Output Status events (g42)",
        )?
        .add(
            "max_octet_string",
            Type::Uint16,
            doc("Maximum number of Octet String events (g111)"),
        )?
        .doc(
            doc("Maximum number of events for each type")
                .details("A value of zero means that events will not be buffered for that type."),
        )?
        .build()?;

    let event_buffer_config_all_types = lib
        .declare_native_function("event_buffer_config_all_types")?
        .param("max", Type::Uint16, "Maximum value to set all types")?
        .return_type(ReturnType::new(
            Type::Struct(event_buffer_config.clone()),
            "Event buffer configuration",
        ))?
        .doc("Initialize an event buffer configuration with the same maximum values for all types")?
        .build()?;

    let event_buffer_config_no_events = lib
        .declare_native_function("event_buffer_config_no_events")?
        .return_type(ReturnType::new(
            Type::Struct(event_buffer_config.clone()),
            "Event buffer configuration",
        ))?
        .doc("Initialize an event buffer configuration to support no events")?
        .build()?;

    lib.define_struct(&event_buffer_config)?
        .static_method("all_types", &event_buffer_config_all_types)?
        .static_method("no_events", &event_buffer_config_no_events)?
        .build();

    Ok(event_buffer_config)
}

fn define_outstation_application(
    lib: &mut LibraryBuilder,
) -> Result<InterfaceHandle, BindingError> {
    let restart_delay_type = lib
        .define_native_enum("RestartDelayType")?
        .push("NotSupported", "Restart mode not supported")?
        .push("Seconds", "Value is in seconds (corresponds to g51v1)")?
        .push(
            "Milliseconds",
            "Value is in milliseconds (corresponds to g51v2)",
        )?
        .doc("Type of restart delay value. Used by {struct:RestartDelay}.")?
        .build()?;

    let restart_delay = lib.declare_native_struct("RestartDelay")?;
    let restart_delay = lib.define_native_struct(&restart_delay)?
        .add("restart_type", Type::Enum(restart_delay_type), "Indicates what {struct:RestartDelay.value} is.")?
        .add("value", Type::Uint16, "Expected delay before the outstation comes back online.")?
        .doc(doc("Restart delay used by {interface:OutstationApplication.cold_restart()} and {interface:OutstationApplication.warm_restart()}")
        .details("If {struct:RestartDelay.restart_type} is not {enum:RestartDelayType.NotSupported}, then the {struct:RestartDelay.value} is valid. Otherwise, the outstation will return IIN2.0 NO_FUNC_CODE_SUPPORT."))?
        .build()?;

    let restart_delay_not_supported_fn = lib
        .declare_native_function("restart_delay_not_supported")?
        .return_type(ReturnType::new(
            Type::Struct(restart_delay.clone()),
            "Unsupported restart delay",
        ))?
        .doc("Creates a restart delay that indicates that this operation is not supported.")?
        .build()?;

    let restart_delay_seconds_fn = lib
        .declare_native_function("restart_delay_seconds")?
        .param("value", Type::Uint16, "Expected restart delay (in seconds)")?
        .return_type(ReturnType::new(
            Type::Struct(restart_delay.clone()),
            "Valid restart delay",
        ))?
        .doc("Creates a restart delay with a value specified in seconds.")?
        .build()?;

    let restart_delay_millis_fn = lib
        .declare_native_function("restart_delay_millis")?
        .param(
            "value",
            Type::Uint16,
            "Expected restart delay (in milliseconds)",
        )?
        .return_type(ReturnType::new(
            Type::Struct(restart_delay.clone()),
            "Valid restart delay",
        ))?
        .doc("Creates a restart delay with a value specified in milliseconds.")?
        .build()?;

    lib.define_struct(&restart_delay)?
        .static_method("NotSupported", &restart_delay_not_supported_fn)?
        .static_method("ValidSeconds", &restart_delay_seconds_fn)?
        .static_method("ValidMillis", &restart_delay_millis_fn)?
        .build();

    lib.define_interface("OutstationApplication", "Dynamic information required by the outstation from the user application")?
        .callback("get_processing_delay_ms", doc("Returns the DELAY_MEASUREMENT delay")
            .details("The value returned by this method is used in conjunction with the DELAY_MEASUREMENT function code and returned in a g52v2 time delay object as part of a non-LAN time synchronization procedure.")
            .details("It represents the processing delay from receiving the request to sending the response. This parameter should almost always use the default value of zero as only an RTOS or bare metal system would have access to this level of timing. Modern hardware can almost always respond in less than 1 millisecond anyway.")
            .details("For more information, see IEEE-1815 2012, p. 64."))?
            .return_type(ReturnType::new(Type::Uint16, "Processing delay, in milliseconds"))?
            .build()?
        .callback("cold_restart", doc("Request that the outstation perform a cold restart (IEEE-1815 2012, p. 58)")
            .details("The outstation will not automatically restart. It is the responsibility of the user application to handle this request and take the appropriate action."))?
            .return_type(ReturnType::new(Type::Struct(restart_delay.clone()), "The restart delay"))?
            .build()?
        .callback("warm_restart", doc("Request that the outstation perform a warm restart (IEEE-1815 2012, p. 58)")
            .details("The outstation will not automatically restart. It is the responsibility of the user application to handle this request and take the appropriate action."))?
            .return_type(ReturnType::new(Type::Struct(restart_delay), "The restart delay"))?
            .build()?
        .destroy_callback("on_destroy")?
        .build()
}

fn define_outstation_information(
    lib: &mut LibraryBuilder,
    shared_def: &SharedDefinitions,
) -> Result<InterfaceHandle, BindingError> {
    let function_code = define_function_code(lib)?;

    let request_header = lib.declare_native_struct("RequestHeader")?;
    let request_header = lib
        .define_native_struct(&request_header)?
        .add(
            "control",
            Type::Struct(shared_def.control_struct.clone()),
            "Control field",
        )?
        .add(
            "function",
            Type::Enum(function_code.clone()),
            "Function code",
        )?
        .doc("Application-layer header for requests")?
        .build()?;

    let broadcast_action = lib.define_native_enum("BroadcastAction")?
        .push("Processed", "Outstation processed the broadcast")?
        .push("IgnoredByConfiguration", "Outstation ignored the broadcast message b/c it is disabled by configuration")?
        .push("BadObjectHeaders", "Outstation was unable to parse the object headers and ignored the request")?
        .push("UnsupportedFunction", "Outstation ignore the broadcast message b/c the function is not supported via Broadcast")?
        .doc("Enumeration describing how the outstation processed a broadcast request")?
        .build()?;

    lib.define_interface("OutstationInformation", doc("Informational callbacks that the outstation doesn't rely on to function").details("It may be useful to certain applications to assess the health of the communication or to count statistics"))?
        .callback("process_request_from_idle", "Called when a request is processed from the IDLE state")?
            .param("header", Type::Struct(request_header.clone()), "Request header")?
            .return_type(ReturnType::void())?
            .build()?
        .callback("broadcast_received", "Called when a broadcast request is received by the outstation")?
            .param("function_code", Type::Enum(function_code), "Function code received")?
            .param("action", Type::Enum(broadcast_action), "Broadcast action")?
            .return_type(ReturnType::void())?
            .build()?
        .callback("enter_solicited_confirm_wait", "Outstation has begun waiting for a solicited confirm")?
            .param("ecsn", Type::Uint8, "Expected sequence number")?
            .return_type(ReturnType::void())?
            .build()?
        .callback("solicited_confirm_timeout", "Failed to receive a solicited confirm before the timeout occurred")?
            .param("ecsn", Type::Uint8, "Expected sequence number")?
            .return_type(ReturnType::void())?
            .build()?
        .callback("solicited_confirm_received", "Received the expected confirm")?
            .param("ecsn", Type::Uint8, "Expected sequence number")?
            .return_type(ReturnType::void())?
            .build()?
        .callback("solicited_confirm_wait_new_request", "Received a new request while waiting for a solicited confirm, aborting the response series")?
            .param("header", Type::Struct(request_header), "Request header")?
            .return_type(ReturnType::void())?
            .build()?
        .callback("wrong_solicited_confirm_seq", "Received a solicited confirm with the wrong sequence number")?
            .param("ecsn", Type::Uint8, "Expected sequence number")?
            .param("seq", Type::Uint8, "Received sequence number")?
            .return_type(ReturnType::void())?
            .build()?
        .callback("unexpected_confirm", "Received a confirm when not expecting one")?
            .param("unsolicited", Type::Bool, "True if it's an unsolicited response confirm, false if it's a solicited response confirm")?
            .param("seq", Type::Uint8, "Received sequence number")?
            .return_type(ReturnType::void())?
            .build()?
        .callback("enter_unsolicited_confirm_wait", "Outstation has begun waiting for an unsolicited confirm")?
            .param("ecsn", Type::Uint8, "Expected sequence number")?
            .return_type(ReturnType::void())?
            .build()?
        .callback("unsolicited_confirm_timeout", "Failed to receive an unsolicited confirm before the timeout occurred")?
            .param("ecsn", Type::Uint8, "Expected sequence number")?
            .param("retry", Type::Bool, "Is it a retry")?
            .return_type(ReturnType::void())?
            .build()?
        .callback("unsolicited_confirmed", "Master confirmed an unsolicited message")?
            .param("ecsn", Type::Uint8, "Expected sequence number")?
            .return_type(ReturnType::void())?
            .build()?
        .callback("clear_restart_iin", "Master cleared the restart IIN bit")?
            .return_type(ReturnType::void())?
            .build()?
        .destroy_callback("on_destroy")?
        .build()
}

fn define_control_handler(
    lib: &mut LibraryBuilder,
    database: &ClassHandle,
    shared_def: &SharedDefinitions,
) -> Result<InterfaceHandle, BindingError> {
    let command_status = define_command_status(lib)?;

    let operate_type = lib
        .define_native_enum("OperateType")?
        .push(
            "SelectBeforeOperate",
            "control point was properly selected before the operate request",
        )?
        .push(
            "DirectOperate",
            "operate the control via a DirectOperate request",
        )?
        .push(
            "DirectOperateNoAck",
            "operate the control via a DirectOperateNoAck request",
        )?
        .doc("Enumeration describing how the master requested the control operation")?
        .build()?;

    lib.define_interface("ControlHandler", "Callbacks for handling controls")?
        .callback("begin_fragment", "Notifies the start of a command fragment")?
            .return_type(ReturnType::void())?
            .build()?
        .callback("end_fragment", "Notifies the end of a command fragment")?
            .return_type(ReturnType::void())?
            .build()?
        .callback("select_g12v1", doc("Select a CROB, but do not operate")
            .details("Implementors can think of this function ask the question \"is this control supported\"?")
            .details("Most implementations should not alter the database in this method. It is only provided in the event that some event counters reflected via the API get updated on SELECT, but this would be highly abnormal."))?
            .param("control", Type::Struct(shared_def.g12v1_struct.clone()), "Received CROB")?
            .param("index", Type::Uint16, "Index of the point")?
            .param("database", Type::ClassRef(database.declaration()), "Database")?
            .return_type(ReturnType::new(Type::Enum(command_status.clone()), "Command status"))?
            .build()?
        .callback("operate_g12v1", "Operate a control point")?
            .param("control", Type::Struct(shared_def.g12v1_struct.clone()), "Received CROB")?
            .param("index", Type::Uint16, "Index of the point")?
            .param("op_type", Type::Enum(operate_type.clone()), "Operate type")?
            .param("database", Type::ClassRef(database.declaration()), "Database")?
            .return_type(ReturnType::new(Type::Enum(command_status.clone()), "Command status"))?
            .build()?
        .callback("select_g41v1", doc("Select an analog output, but do not operate")
            .details("Implementors can think of this function ask the question \"is this control supported\"?")
            .details("Most implementations should not alter the database in this method. It is only provided in the event that some event counters reflected via the API get updated on SELECT, but this would be highly abnormal."))?
            .param("control", Type::Sint32, "Received analog output value")?
            .param("index", Type::Uint16, "Index of the point")?
            .param("database", Type::ClassRef(database.declaration()), "Database")?
            .return_type(ReturnType::new(Type::Enum(command_status.clone()), "Command status"))?
            .build()?
        .callback("operate_g41v1", "Operate a control point")?
            .param("control", Type::Sint32, "Received analog output value")?
            .param("index", Type::Uint16, "Index of the point")?
            .param("op_type", Type::Enum(operate_type.clone()), "Operate type")?
            .param("database", Type::ClassRef(database.declaration()), "Database")?
            .return_type(ReturnType::new(Type::Enum(command_status.clone()), "Command status"))?
            .build()?
        .callback("select_g41v2", doc("Select an analog output, but do not operate")
            .details("Implementors can think of this function ask the question \"is this control supported\"?")
            .details("Most implementations should not alter the database in this method. It is only provided in the event that some event counters reflected via the API get updated on SELECT, but this would be highly abnormal."))?
            .param("value", Type::Sint16, "Received analog output value")?
            .param("index", Type::Uint16, "Index of the point")?
            .param("database", Type::ClassRef(database.declaration()), "Database")?
            .return_type(ReturnType::new(Type::Enum(command_status.clone()), "Command status"))?
            .build()?
        .callback("operate_g41v2", "Operate a control point")?
            .param("value", Type::Sint16, "Received analog output value")?
            .param("index", Type::Uint16, "Index of the point")?
            .param("op_type", Type::Enum(operate_type.clone()), "Operate type")?
            .param("database", Type::ClassRef(database.declaration()), "Database")?
            .return_type(ReturnType::new(Type::Enum(command_status.clone()), "Command status"))?
            .build()?
        .callback("select_g41v3", doc("Select an analog output, but do not operate")
            .details("Implementors can think of this function ask the question \"is this control supported\"?")
            .details("Most implementations should not alter the database in this method. It is only provided in the event that some event counters reflected via the API get updated on SELECT, but this would be highly abnormal."))?
            .param("value", Type::Float, "Received analog output value")?
            .param("index", Type::Uint16, "Index of the point")?
            .param("database", Type::ClassRef(database.declaration()), "Database")?
            .return_type(ReturnType::new(Type::Enum(command_status.clone()), "Command status"))?
            .build()?
        .callback("operate_g41v3", "Operate a control point")?
            .param("value", Type::Float, "Received analog output value")?
            .param("index", Type::Uint16, "Index of the point")?
            .param("op_type", Type::Enum(operate_type.clone()), "Operate type")?
            .param("database", Type::ClassRef(database.declaration()), "Database")?
            .return_type(ReturnType::new(Type::Enum(command_status.clone()), "Command status"))?
            .build()?
        .callback("select_g41v4", doc("Select an analog output, but do not operate")
            .details("Implementors can think of this function ask the question \"is this control supported\"?")
            .details("Most implementations should not alter the database in this method. It is only provided in the event that some event counters reflected via the API get updated on SELECT, but this would be highly abnormal."))?
            .param("value", Type::Double, "Received analog output value")?
            .param("index", Type::Uint16, "Index of the point")?
            .param("database", Type::ClassRef(database.declaration()), "Database")?
            .return_type(ReturnType::new(Type::Enum(command_status.clone()), "Command status"))?
            .build()?
        .callback("operate_g41v4", "Operate a control point")?
            .param("value", Type::Double, "Received analog output value")?
            .param("index", Type::Uint16, "Index of the point")?
            .param("op_type", Type::Enum(operate_type), "Operate type")?
            .param("database", Type::ClassRef(database.declaration()), "Database")?
            .return_type(ReturnType::new(Type::Enum(command_status), "Command status"))?
            .build()?
        .destroy_callback("on_destroy")?
        .build()
}

fn define_address_filter(lib: &mut LibraryBuilder) -> Result<ClassHandle, BindingError> {
    let address_filter = lib.declare_class("AddressFilter")?;

    let address_filter_any_fn = lib
        .declare_native_function("address_filter_any")?
        .return_type(ReturnType::new(
            Type::ClassRef(address_filter.clone()),
            "Address filter",
        ))?
        .doc("Create an address filter that accepts any IP address")?
        .build()?;

    let address_filter_new_fn = lib
        .declare_native_function("address_filter_new")?
        .param("address", Type::String, "IP address to accept")?
        .return_type(ReturnType::new(
            Type::ClassRef(address_filter.clone()),
            "Address filter",
        ))?
        .doc("Create an address filter that accepts any IP address")?
        .build()?;

    let address_filter_add_fn = lib
        .declare_native_function("address_filter_add")?
        .param(
            "address_filter",
            Type::ClassRef(address_filter.clone()),
            "Address filter to modify",
        )?
        .param("address", Type::String, "IP address to add")?
        .return_type(ReturnType::void())?
        .doc("Add an accepted IP address to the filter")?
        .build()?;

    let address_filter_destroy_fn = lib
        .declare_native_function("address_filter_destroy")?
        .param(
            "address_filter",
            Type::ClassRef(address_filter.clone()),
            "Address filter to destroy",
        )?
        .return_type(ReturnType::void())?
        .doc("Destroy an address filter")?
        .build()?;

    lib.define_class(&address_filter)?
        .constructor(&address_filter_new_fn)?
        .destructor(&address_filter_destroy_fn)?
        .static_method("Any", &address_filter_any_fn)?
        .method("Add", &address_filter_add_fn)?
        .doc("")?
        .build()
}

fn define_function_code(lib: &mut LibraryBuilder) -> Result<NativeEnumHandle, BindingError> {
    lib.define_native_enum("FunctionCode")?
        .push("Confirm", "Master sends this to an outstation to confirm the receipt of an Application Layer fragment (value == 0)")?
        .push("Read", "Outstation shall return the data specified by the objects in the request (value == 1)")?
        .push("Write", "Outstation shall store the data specified by the objects in the request (value == 2)")?
        .push("Select", "Outstation shall select (or arm) the output points specified by the objects in the request in preparation for a subsequent operate command (value == 3)")?
        .push("Operate", "Outstation shall activate the output points selected (or armed) by a previous select function code command (value == 4)")?
        .push("DirectOperate", "Outstation shall immediately actuate the output points specified by the objects in the request (value == 5)")?
        .push("DirectOperateNoResponse", "Same as DirectOperate but outstation shall not send a response (value == 6)")?
        .push("ImmediateFreeze", "Outstation shall copy the point data values specified by the objects in the request to a separate freeze buffer (value == 7)")?
        .push("ImmediateFreezeNoResponse", "Same as ImmediateFreeze but outstation shall not send a response (value == 8)")?
        .push("FreezeClear", "Outstation shall copy the point data values specified by the objects in the request into a separate freeze buffer and then clear the values (value == 9)")?
        .push("FreezeClearNoResponse", "Same as FreezeClear but outstation shall not send a response (value == 10)")?
        .push("FreezeAtTime", "Outstation shall copy the point data values specified by the objects in the request to a separate freeze buffer at the time and/or time intervals specified in a special time data information object (value == 11)")?
        .push("FreezeAtTimeNoResponse", "Same as FreezeAtTime but outstation shall not send a response (value == 12)")?
        .push("ColdRestart", "Outstation shall perform a complete reset of all hardware and software in the device (value == 13)")?
        .push("WarmRestart", "Outstation shall reset only portions of the device (value == 14)")?
        .push("InitializeData", "Obsolete-Do not use for new designs (value == 15)")?
        .push("InitializeApplication", "Outstation shall place the applications specified by the objects in the request into the ready to run state (value == 16)")?
        .push("StartApplication", "Outstation shall start running the applications specified by the objects in the request (value == 17)")?
        .push("StopApplication", "Outstation shall stop running the applications specified by the objects in the request (value == 18)")?
        .push("SaveConfiguration", "This code is deprecated-Do not use for new designs (value == 19)")?
        .push("EnableUnsolicited", "Enables outstation to initiate unsolicited responses from points specified by the objects in the request (value == 20)")?
        .push("DisableUnsolicited", "Prevents outstation from initiating unsolicited responses from points specified by the objects in the request (value == 21)")?
        .push("AssignClass", "Outstation shall assign the events generated by the points specified by the objects in the request to one of the classes (value == 22)")?
        .push("DelayMeasure", "Outstation shall report the time it takes to process and initiate the transmission of its response (value == 23)")?
        .push("RecordCurrentTime", "Outstation shall save the time when the last octet of this message is received (value == 24)")?
        .push("OpenFile", "Outstation shall open a file (value == 25)")?
        .push("CloseFile", "Outstation shall close a file (value == 26)")?
        .push("DeleteFile", "Outstation shall delete a file (value == 27)")?
        .push("GetFileInfo", "Outstation shall retrieve information about a file (value == 28)")?
        .push("AuthenticateFile", "Outstation shall return a file authentication key (value == 29)")?
        .push("AbortFile", "Outstation shall abort a file transfer operation (value == 30)")?
        .push("Response", "Master shall interpret this fragment as an Application Layer response to an ApplicationLayer request (value == 129)")?
        .push("UnsolicitedResponse", "Master shall interpret this fragment as an unsolicited response that was not prompted by an explicit request (value == 130)")?
        .doc("Application layer function code")?
        .build()
}

fn define_command_status(lib: &mut LibraryBuilder) -> Result<NativeEnumHandle, BindingError> {
    lib.define_native_enum("CommandStatus")?
    .push("Success", "command was accepted, initiated, or queued (value == 0)")?
    .push("Timeout", "command timed out before completing (value == 1)")?
    .push("NoSelect", "command requires being selected before operate, configuration issue (value == 2)")?
    .push("FormatError", "bad control code or timing values (value == 3)")?
    .push("NotSupported", "command is not implemented (value == 4)")?
    .push("AlreadyActive", "command is all ready in progress or its all ready in that mode (value == 5)")?
    .push("HardwareError", "something is stopping the command, often a local/remote interlock (value == 6)")?
    .push("Local", "the function governed by the control is in local only control (value == 7)")?
    .push("TooManyOps", "the command has been done too often and has been throttled (value == 8)")?
    .push("NotAuthorized", "the command was rejected because the device denied it or an RTU intercepted it (value == 9)")?
    .push("AutomationInhibit", "command not accepted because it was prevented or inhibited by a local automation process, such as interlocking logic or synchrocheck (value == 10)")?
    .push("ProcessingLimited", "command not accepted because the device cannot process any more activities than are presently in progress (value == 11)")?
    .push("OutOfRange", "command not accepted because the value is outside the acceptable range permitted for this point (value == 12)")?
    .push("DownstreamLocal", "command not accepted because the outstation is forwarding the request to another downstream device which reported LOCAL (value == 13)")?
    .push("AlreadyComplete", "command not accepted because the outstation has already completed the requested operation (value == 14)")?
    .push("Blocked", "command not accepted because the requested function is specifically blocked at the outstation (value == 15)")?
    .push("Canceled", "command not accepted because the operation was cancelled (value == 16)")?
    .push("BlockedOtherMaster", "command not accepted because another master is communicating with the outstation and has exclusive rights to operate this control point (value == 17)")?
    .push("DownstreamFail", "command not accepted because the outstation is forwarding the request to another downstream device which cannot be reached or is otherwise incapable of performing the request (value == 18)")?
    .push("NonParticipating", "(deprecated) indicates the outstation shall not issue or perform the control operation (value == 126)")?
    .push("Unknown", "aptures any value not defined in the enumeration")?
    .doc("Enumeration received from an outstation in response to command request")?
    .build()
}
