use crate::gv;
use oo_bindgen::model::*;
use std::time::Duration;

pub struct SharedDefinitions {
    pub error_type: ErrorType<Unvalidated>,
    pub port_state_listener: AsynchronousInterface,
    pub variation_enum: EnumHandle,
    pub runtime_class: ClassDeclarationHandle,
    pub connect_options: ClassHandle,
    pub decode_level: UniversalStructHandle,
    pub serial_port_settings: FunctionArgStructHandle,
    pub link_error_mode: EnumHandle,
    pub retry_strategy: FunctionArgStructHandle,
    pub control_field_struct: CallbackArgStructHandle,
    pub g12v1_struct: UniversalStructHandle,
    pub function_code: EnumHandle,
    pub binary_point: UniversalStructHandle,
    pub binary_it: AbstractIteratorHandle,
    pub double_bit_binary_point: UniversalStructHandle,
    pub double_bit_binary_it: AbstractIteratorHandle,
    pub binary_output_status_point: UniversalStructHandle,
    pub binary_output_status_it: AbstractIteratorHandle,
    pub counter_point: UniversalStructHandle,
    pub counter_it: AbstractIteratorHandle,
    pub frozen_counter_point: UniversalStructHandle,
    pub frozen_counter_it: AbstractIteratorHandle,
    pub analog_point: UniversalStructHandle,
    pub analog_it: AbstractIteratorHandle,
    pub analog_output_status_point: UniversalStructHandle,
    pub analog_output_status_it: AbstractIteratorHandle,
    pub octet_string: FunctionReturnStructHandle,
    pub octet_string_it: AbstractIteratorHandle,
    pub min_tls_version: EnumHandle,
    pub certificate_mode: EnumHandle,
}

pub fn define(lib: &mut LibraryBuilder) -> BackTraced<SharedDefinitions> {
    let error_type = lib
        .define_error_type(
            "param_error",
            "param_exception",
            ExceptionType::UncheckedException,
        )?
        .add_error(
            "invalid_timeout",
            "The supplied timeout value is too small or too large",
        )?
        .add_error("null_parameter", "Null parameter")?
        .add_error(
            "no_support",
            "Native library was compiled without support for this feature",
        )?
        .add_error(
            "association_does_not_exist",
            "The specified association does not exist",
        )?
        .add_error(
            "association_duplicate_address",
            "Duplicate association address",
        )?
        .add_error("invalid_socket_address", "Invalid socket address")?
        .add_error("invalid_dnp3_address", "Invalid link-layer DNP3 address")?
        .add_error("invalid_buffer_size", "Invalid buffer size")?
        .add_error(
            "address_filter_conflict",
            "Conflict in the address filter specification",
        )?
        .add_error("server_already_started", "Server already started")?
        .add_error(
            "server_bind_error",
            "Server failed to bind to the specified port",
        )?
        .add_error("master_already_shutdown", "Master was already shutdown")?
        .add_error("runtime_creation_failure", "Failed to create Tokio runtime")?
        .add_error("runtime_destroyed", "Runtime has already been disposed")?
        .add_error(
            "runtime_cannot_block_within_async",
            "Runtime cannot execute blocking call within asynchronous context",
        )?
        .add_error(
            "logging_already_configured",
            "Logging can only be configured once",
        )?
        .add_error("point_does_not_exist", "Point does not exist")?
        .add_error("invalid_peer_certificate", "Invalid peer certificate file")?
        .add_error(
            "invalid_local_certificate",
            "Invalid local certificate file",
        )?
        .add_error("invalid_private_key", "Invalid private key file")?
        .add_error("invalid_dns_name", "Invalid DNS name")?
        .add_error("other_tls_error", "Other TLS error")?
        .doc("Error type used throughout the library")?
        .build()?;

    crate::constants::define(lib)?;
    let decode_level = crate::decoding::define(lib)?;
    let runtime_class = sfio_tokio_ffi::define(lib, error_type.clone())?;

    let control_field_struct = lib.declare_callback_argument_struct("control_field")?;
    let control_field_struct = lib
        .define_callback_argument_struct(control_field_struct)?
        .add("fir", Primitive::Bool, "First fragment in the message")?
        .add("fin", Primitive::Bool, "Final fragment of the message")?
        .add("con", Primitive::Bool, "Requires confirmation")?
        .add("uns", Primitive::Bool, "Unsolicited response")?
        .add("seq", Primitive::U8, "Sequence number")?
        .doc("APDU Control field")?
        .end_fields()?
        .build()?;

    let trip_close_code = lib
        .define_enum("trip_close_code")?
        .variant("nul", 0, "NUL (0)")?
        .variant("close", 1, "CLOSE (1)")?
        .variant("trip", 2, "TRIP (2)")?
        .variant("reserved", 3, "RESERVED (3)")?
        .doc(
            "Trip-Close Code field, used in conjunction with {enum:op_type} to specify a control operation")?
        .build()?;

    let op_type = lib
        .define_enum("op_type")?
        .variant("nul", 0, "NUL (0)")?
        .variant("pulse_on", 1, "PULSE_ON (1)")?
        .variant("pulse_off", 2, "PULSE_OFF (2)")?
        .variant("latch_on", 3, "LATCH_ON (3)")?
        .variant("latch_off", 4, "LATCH_OFF(4)")?
        .doc("Operation Type field, used in conjunction with {enum:trip_close_code} to specify a control operation")?
        .build()?;

    let tcc_field = Name::create("tcc")?;
    let clear_field = Name::create("clear")?;
    let queue_field = Name::create("queue")?;

    let control_code = lib.declare_universal_struct("control_code")?;
    let control_code = lib
        .define_universal_struct(control_code)?
        .add(&tcc_field, trip_close_code, "This field is used in conjunction with {struct:control_code.op_type} to specify a control operation")?
        .add(&clear_field, Primitive::Bool, "Support for this field is optional. When the clear bit is set, the device shall remove pending control commands for that index and stop any control operation that is in progress for that index. The indexed point shall go to the state that it would have if the command were allowed to complete normally.")?
        .add(&queue_field, Primitive::Bool, "This field is obsolete and should always be 0")?
        .add("op_type", op_type, "This field is used in conjunction with the {struct:control_code.tcc} field to specify a control operation")?
        .doc("CROB ({struct:group12_var1}) control code")?
        .end_fields()?
        .begin_initializer("init", InitializerType::Normal, "Initialize a {struct:control_code} instance")?
        .default(&queue_field, false)?
        .end_initializer()?
        .begin_initializer("from_op_type", InitializerType::Static, doc("Initialize a {struct:control_code} instance from a {enum:op_type}").details("{struct:control_code.tcc} will be set to {enum:trip_close_code.nul}, {struct:control_code.clear} will be set to false and {struct:control_code.queue} will be set to false."))?
        .default_variant(&tcc_field, "nul")?
        .default(&clear_field, false)?
        .default(&queue_field, false)?
        .end_initializer()?
        .begin_initializer("from_tcc_and_op_type", InitializerType::Static, doc("Initialize a {struct:control_code} instance from a {enum:trip_close_code} and a {enum:op_type}.").details("{struct:control_code.clear} will be set to false and {struct:control_code.queue} will be set to false."))?
        .default(&clear_field, false)?
        .default(&queue_field, false)?
        .end_initializer()?
        .build()?;

    let code_field = Name::create("code")?;
    let count_field = Name::create("count")?;
    let on_time_field = Name::create("on_time")?;
    let off_time_field = Name::create("off_time")?;

    let g12v1_struct = lib.declare_universal_struct(gv(12, 1))?;
    let g12v1_struct = lib
        .define_universal_struct(g12v1_struct)?
        .add(&code_field, control_code, "Control code")?
        .add(&count_field, Primitive::U8, "Count")?
        .add(
            &on_time_field,
            Primitive::U32,
            "Duration the output drive remains active (in milliseconds)",
        )?
        .add(
            &off_time_field,
            Primitive::U32,
            "Duration the output drive remains non-active (in milliseconds)",
        )?
        .doc("Control Relay Output Block")?
        .end_fields()?
        .add_full_initializer("init")?
        .begin_initializer("from_code", InitializerType::Static, doc("Construct a {struct:group12_var1} from a {struct:control_code}.").details("{struct:group12_var1.count} = 1, {struct:group12_var1.on_time} = 1000 and {struct:group12_var1.off_time} = 1000."))?
        .default(&count_field, NumberValue::U8(1))?
        .default(&on_time_field, NumberValue::U32(1000))?
        .default(&off_time_field, NumberValue::U32(1000))?
        .end_initializer()?
        .build()?;

    // ======
    // Points
    // ======
    let flags_struct = declare_flags_struct(lib)?;

    let timestamp_struct = declare_timestamp_struct(lib)?;

    let double_bit_enum = lib
        .define_enum("double_bit")?
        .push("intermediate", "Transition between conditions")?
        .push("determined_off", "Determined to be OFF")?
        .push("determined_on", "Determined to be ON")?
        .push("indeterminate", "Abnormal or custom condition")?
        .doc("Double-bit binary input value")?
        .build()?;

    let (binary_point, binary_it) = build_iterator(
        "binary_input",
        Primitive::Bool,
        lib,
        &flags_struct,
        &timestamp_struct,
    )?;
    let (double_bit_binary_point, double_bit_binary_it) = build_iterator(
        "double_bit_binary_input",
        double_bit_enum,
        lib,
        &flags_struct,
        &timestamp_struct,
    )?;
    let (binary_output_status_point, binary_output_status_it) = build_iterator(
        "binary_output_status",
        Primitive::Bool,
        lib,
        &flags_struct,
        &timestamp_struct,
    )?;
    let (counter_point, counter_it) = build_iterator(
        "counter",
        Primitive::U32,
        lib,
        &flags_struct,
        &timestamp_struct,
    )?;
    let (frozen_counter_point, frozen_counter_it) = build_iterator(
        "frozen_counter",
        Primitive::U32,
        lib,
        &flags_struct,
        &timestamp_struct,
    )?;
    let (analog_point, analog_it) = build_iterator(
        "analog_input",
        Primitive::Double,
        lib,
        &flags_struct,
        &timestamp_struct,
    )?;
    let (analog_output_status_point, analog_output_status_it) = build_iterator(
        "analog_output_status",
        Primitive::Double,
        lib,
        &flags_struct,
        &timestamp_struct,
    )?;

    let (octet_string, octet_string_it) = build_octet_string(lib)?;

    let connect_options = define_connect_options(lib, error_type.clone())?;

    Ok(SharedDefinitions {
        error_type,
        port_state_listener: define_port_state_listener(lib)?,
        variation_enum: crate::variation::define(lib)?,
        runtime_class,
        connect_options,
        decode_level,
        retry_strategy: define_retry_strategy(lib)?,
        serial_port_settings: define_serial_port_settings(lib)?,
        link_error_mode: define_link_error_mode(lib)?,
        min_tls_version: define_min_tls_version(lib)?,
        certificate_mode: define_certificate_mode(lib)?,
        control_field_struct,
        g12v1_struct,
        function_code: define_function_code(lib)?,
        binary_point,
        binary_it,
        double_bit_binary_point,
        double_bit_binary_it,
        binary_output_status_point,
        binary_output_status_it,
        counter_point,
        counter_it,
        frozen_counter_point,
        frozen_counter_it,
        analog_point,
        analog_it,
        analog_output_status_point,
        analog_output_status_it,
        octet_string,
        octet_string_it,
    })
}

fn define_connect_options(
    lib: &mut LibraryBuilder,
    error_type: ErrorType<Unvalidated>,
) -> BackTraced<ClassHandle> {
    let options = lib.declare_class("connect_options")?;

    let constructor = lib
        .define_constructor(options.clone())?
        .doc("Initialize to the defaults")?
        .build()?;

    let destructor = lib.define_destructor(options.clone(), "Destroy an instance")?;

    let set_timeout = lib
        .define_method("set_timeout", options.clone())?
        .doc("Set a timeout for the TCP connection that might be less than the default for the OS")?
        .param("timeout", DurationType::Seconds, "Timeout value")?
        .build()?;

    let set_local_endpoint = lib
        .define_method("set_local_endpoint", options.clone())?
        .doc(
            doc("Set the local address to which the socket is bound")
                .details("If not specified, then any available adapter may be used with an OS assigned port.")
        )?
        .param("endpoint", StringType, "String in 'address:port' format, where address can be IPv4 or IPv6. Using 0 for the port results in an OS assigned port")?
        .fails_with(error_type)?
        .build()?;

    let options = lib
        .define_class(&options)?
        .doc("Options that control how TCP connections are established")?
        .constructor(constructor)?
        .destructor(destructor)?
        .method(set_timeout)?
        .method(set_local_endpoint)?
        .build()?;

    Ok(options)
}

fn define_retry_strategy(lib: &mut LibraryBuilder) -> BackTraced<FunctionArgStructHandle> {
    let min_delay = Name::create("min_delay")?;
    let max_delay = Name::create("max_delay")?;

    let retry_strategy = lib.declare_function_argument_struct("retry_strategy")?;
    let retry_strategy = lib
        .define_function_argument_struct(retry_strategy)?
        .add(
            &min_delay,
            DurationType::Milliseconds,
            "Minimum delay between two retries",
        )?
        .add(
            &max_delay,
            DurationType::Milliseconds,
            "Maximum delay between two retries",
        )?
        .doc(doc("Retry strategy configuration.").details(
            "The strategy uses an exponential back-off with a minimum and maximum value.",
        ))?
        .end_fields()?
        .begin_initializer("init", InitializerType::Normal, "Initialize to defaults")?
        .default(&min_delay, Duration::from_secs(1))?
        .default(&max_delay, Duration::from_secs(10))?
        .end_initializer()?
        .build()?;

    Ok(retry_strategy)
}

fn define_link_error_mode(lib: &mut LibraryBuilder) -> BackTraced<EnumHandle> {
    let mode = lib
        .define_enum("link_error_mode")?
        .push("discard", "Framing errors are discarded. The link-layer parser is reset on any error, and the parser begins scanning for 0x0564. This is always the behavior for serial ports.")?
        .push("close", "Framing errors are bubbled up to calling code, closing the session. Suitable for physical layers that provide error correction like TCP.")?
        .doc("Controls how errors in parsed link-layer frames are handled. This behavior is configurable for physical layers with built-in error correction like TCP as the connection might be through a terminal server.")?
        .build()?;

    Ok(mode)
}

fn define_serial_port_settings(lib: &mut LibraryBuilder) -> BackTraced<FunctionArgStructHandle> {
    let data_bits_enum = lib
        .define_enum("data_bits")?
        .push("five", "5 bits per character")?
        .push("six", "6 bits per character")?
        .push("seven", "7 bits per character")?
        .push("eight", "8 bits per character")?
        .doc("Number of bits per character")?
        .build()?;

    let flow_control_enum = lib
        .define_enum("flow_control")?
        .push("none", "No flow control")?
        .push("software", "Flow control using XON/XOFF bytes")?
        .push("hardware", "Flow control using RTS/CTS signals")?
        .doc("Flow control modes")?
        .build()?;

    let parity_enum = lib
        .define_enum("parity")?
        .push("none", "No parity bit")?
        .push("odd", "Parity bit sets odd number of 1 bits")?
        .push("even", "Parity bit sets even number of 1 bits")?
        .doc("Parity checking modes")?
        .build()?;

    let stop_bits_enum = lib
        .define_enum("stop_bits")?
        .push("one", "One stop bit")?
        .push("two", "Two stop bits")?
        .doc("Number of stop bits")?
        .build()?;

    let baud_rate = Name::create("baud_rate")?;
    let data_bits = Name::create("data_bits")?;
    let flow_control = Name::create("flow_control")?;
    let parity = Name::create("parity")?;
    let stop_bits = Name::create("stop_bits")?;

    let serial_settings = lib.declare_function_argument_struct("serial_settings")?;
    let serial_settings = lib
        .define_function_argument_struct(serial_settings)?
        .add(
            &baud_rate,
            Primitive::U32,
            "Baud rate (in symbols-per-second)",
        )?
        .add(
            &data_bits,
            data_bits_enum,
            "Number of bits used to represent a character sent on the line",
        )?
        .add(
            &flow_control,
            flow_control_enum,
            "Type of signalling to use for controlling data transfer",
        )?
        .add(
            &parity,
            parity_enum,
            "Type of parity to use for error checking",
        )?
        .add(
            &stop_bits,
            stop_bits_enum,
            "Number of bits to use to signal the end of a character",
        )?
        .doc("Serial port settings")?
        .end_fields()?
        .begin_initializer(
            "init",
            InitializerType::Normal,
            "Initialize to default values",
        )?
        .default(&baud_rate, NumberValue::U32(9600))?
        .default_variant(&data_bits, "eight")?
        .default_variant(&flow_control, "none")?
        .default_variant(&parity, "none")?
        .default_variant(&stop_bits, "one")?
        .end_initializer()?
        .build()?;

    Ok(serial_settings)
}

fn define_min_tls_version(lib: &mut LibraryBuilder) -> BackTraced<EnumHandle> {
    let handle = lib
        .define_enum("min_tls_version")?
        .push("v12", "Allow TLS 1.2 and 1.3")?
        .push("v13", "Only allow TLS 1.3")?
        .doc("Minimum TLS version to allow")?
        .build()?;

    Ok(handle)
}

fn define_certificate_mode(lib: &mut LibraryBuilder) -> BackTraced<EnumHandle> {
    let handle = lib.define_enum("certificate_mode")?
        .push("authority_based",
              doc("Validates the peer certificate against one or more configured trust anchors")
                  .details("This mode uses the default certificate verifier in `rustls` to ensure that the chain of certificates presented by the peer is valid against one of the configured trust anchors.")
                  .details("The name verification is relaxed to allow for certificates that do not contain the SAN extension. In these cases the name is verified using the Common Name instead.")
        )?
        .push(
            "self_signed",
            doc("Validates that the peer presents a single certificate which is a byte-for-byte match against the configured peer certificate")
                .details("The certificate is parsed only to ensure that the `NotBefore` and `NotAfter` are valid for the current system time.")
        )?
        .doc(
            doc("Determines how the certificate(s) presented by the peer are validated")
                .details("This validation always occurs **after** the handshake signature has been verified."))?
        .build()?;

    Ok(handle)
}

fn declare_flags_struct(lib: &mut LibraryBuilder) -> BackTraced<UniversalStructHandle> {
    let flags_struct = lib.declare_universal_struct("flags")?;
    let flags_struct = lib
        .define_universal_struct(flags_struct)?
        .add(
            "value",
            Primitive::U8,
            "bit-mask representing a set of individual flag bits",
        )?
        .doc("Collection of individual flag bits represented by an underlying mask value")?
        .end_fields()?
        .add_full_initializer("init")?
        .build()?;

    Ok(flags_struct)
}

fn define_port_state_listener(lib: &mut LibraryBuilder) -> BackTraced<AsynchronousInterface> {
    let port_state = lib
        .define_enum("port_state")?
        .push("disabled", "Disabled until enabled")?
        .push("wait", "Waiting to perform an open retry")?
        .push("open", "Port is open")?
        .push("shutdown", "Task has been shut down")?
        .doc("State of the serial port")?
        .build()?;

    let port_state_listener = lib
        .define_interface(
            "port_state_listener",
            "Callback interface for receiving updates about the state of a serial port",
        )?
        .begin_callback("on_change", "Invoked when the serial port changes state")?
        .param("state", port_state, "New state of the port")?
        .end_callback()?
        .build_async()?;

    Ok(port_state_listener)
}

fn declare_timestamp_struct(lib: &mut LibraryBuilder) -> BackTraced<UniversalStructHandle> {
    let time_quality_enum = lib
        .define_enum("time_quality")?
        .push(
            "synchronized_time",
            "The timestamp is UTC synchronized at the remote device",
        )?
        .push(
            "unsynchronized_time",
            "The device indicates the timestamp may be not be synchronized",
        )?
        .push(
            "invalid_time",
            "Timestamp is not valid, ignore the value and use a local timestamp",
        )?
        .doc("Timestamp quality")?
        .build()?;

    let value = Name::create("value")?;
    let quality = Name::create("quality")?;

    let timestamp_struct = lib.declare_universal_struct("timestamp")?;
    let timestamp_struct = lib
        .define_universal_struct(timestamp_struct)?
        .add(&value, Primitive::U64, doc("Count of milliseconds since UNIX epoch").warning("Only the lower 48-bits are used in DNP3 timestamps and time synchronization"))?
        .add(&quality, time_quality_enum, "Enumeration that indicates the timestamp's validity")?
        .doc("Timestamp associated with particular measurement from the outstation. The validity of the value depends on the quality.")?
        .end_fields()?
        .begin_initializer(
            "invalid_timestamp",
            InitializerType::Static,
            "Creates an invalid timestamp struct",
        )?
        .default(&value, NumberValue::U64(0))?
        .default_variant(&quality, "invalid_time")?
        .end_initializer()?
        .begin_initializer(
            "synchronized_timestamp",
            InitializerType::Static,
            "Creates a synchronized timestamp struct",
        )?
        .default_variant(&quality, "synchronized_time")?
        .end_initializer()?
        .begin_initializer(
            "unsynchronized_timestamp",
            InitializerType::Static,
            "Creates an unsynchronized timestamp struct",
        )?
        .default_variant(&quality, "unsynchronized_time")?
        .end_initializer()?
        .build()?;

    Ok(timestamp_struct)
}

fn define_function_code(lib: &mut LibraryBuilder) -> BackTraced<EnumHandle> {
    let function = lib.define_enum("function_code")?
        .push("confirm", "Master sends this to an outstation to confirm the receipt of an Application Layer fragment (value == 0)")?
        .push("read", "Outstation shall return the data specified by the objects in the request (value == 1)")?
        .push("write", "Outstation shall store the data specified by the objects in the request (value == 2)")?
        .push("select", "Outstation shall select (or arm) the output points specified by the objects in the request in preparation for a subsequent operate command (value == 3)")?
        .push("operate", "Outstation shall activate the output points selected (or armed) by a previous select function code command (value == 4)")?
        .push("direct_operate", "Outstation shall immediately actuate the output points specified by the objects in the request (value == 5)")?
        .push("direct_operate_no_response", "Same as DirectOperate but outstation shall not send a response (value == 6)")?
        .push("immediate_freeze", "Outstation shall copy the point data values specified by the objects in the request to a separate freeze buffer (value == 7)")?
        .push("immediate_freeze_no_response", "Same as ImmediateFreeze but outstation shall not send a response (value == 8)")?
        .push("freeze_clear", "Outstation shall copy the point data values specified by the objects in the request into a separate freeze buffer and then clear the values (value == 9)")?
        .push("freeze_clear_no_response", "Same as FreezeClear but outstation shall not send a response (value == 10)")?
        .push("freeze_at_time", "Outstation shall copy the point data values specified by the objects in the request to a separate freeze buffer at the time and/or time intervals specified in a special time data information object (value == 11)")?
        .push("freeze_at_time_no_response", "Same as FreezeAtTime but outstation shall not send a response (value == 12)")?
        .push("cold_restart", "Outstation shall perform a complete reset of all hardware and software in the device (value == 13)")?
        .push("warm_restart", "Outstation shall reset only portions of the device (value == 14)")?
        .push("initialize_data", "Obsolete-Do not use for new designs (value == 15)")?
        .push("initialize_application", "Outstation shall place the applications specified by the objects in the request into the ready to run state (value == 16)")?
        .push("start_application", "Outstation shall start running the applications specified by the objects in the request (value == 17)")?
        .push("stop_application", "Outstation shall stop running the applications specified by the objects in the request (value == 18)")?
        .push("save_configuration", "This code is deprecated-Do not use for new designs (value == 19)")?
        .push("enable_unsolicited", "Enables outstation to initiate unsolicited responses from points specified by the objects in the request (value == 20)")?
        .push("disable_unsolicited", "Prevents outstation from initiating unsolicited responses from points specified by the objects in the request (value == 21)")?
        .push("assign_class", "Outstation shall assign the events generated by the points specified by the objects in the request to one of the classes (value == 22)")?
        .push("delay_measure", "Outstation shall report the time it takes to process and initiate the transmission of its response (value == 23)")?
        .push("record_current_time", "Outstation shall save the time when the last octet of this message is received (value == 24)")?
        .push("open_file", "Outstation shall open a file (value == 25)")?
        .push("close_file", "Outstation shall close a file (value == 26)")?
        .push("delete_file", "Outstation shall delete a file (value == 27)")?
        .push("get_file_info", "Outstation shall retrieve information about a file (value == 28)")?
        .push("authenticate_file", "Outstation shall return a file authentication key (value == 29)")?
        .push("abort_file", "Outstation shall abort a file transfer operation (value == 30)")?
        .push("response", "Master shall interpret this fragment as an Application Layer response to an ApplicationLayer request (value == 129)")?
        .push("unsolicited_response", "Master shall interpret this fragment as an unsolicited response that was not prompted by an explicit request (value == 130)")?
        .doc("Application layer function code")?
        .build()?;

    Ok(function)
}

fn build_iterator<T: Into<UniversalStructField>>(
    name: &str,
    value_type: T,
    lib: &mut LibraryBuilder,
    flags_struct: &UniversalStructHandle,
    timestamp_struct: &UniversalStructHandle,
) -> Result<(UniversalStructHandle, AbstractIteratorHandle), BindingError> {
    let value_struct_decl = lib.declare_universal_struct(name)?;
    let value_struct = lib
        .define_universal_struct(value_struct_decl)?
        .add("index", Primitive::U16, "Point index")?
        .add("value", value_type, "Point value")?
        .add("flags", flags_struct.clone(), "Point flags")?
        .add("time", timestamp_struct.clone(), "Point timestamp")?
        .doc(format!("{} point", name))?
        .end_fields()?
        .add_full_initializer("init")?
        .build()?;

    let value_iterator = lib.define_iterator(format!("{}_iterator", name), value_struct.clone())?;

    Ok((value_struct, value_iterator))
}

fn build_octet_string(
    lib: &mut LibraryBuilder,
) -> Result<(FunctionReturnStructHandle, AbstractIteratorHandle), BindingError> {
    let byte_it = lib.define_iterator_with_lifetime("byte_iterator", Primitive::U8)?;

    let octet_string_struct_decl = lib.declare_function_return_struct("octet_string")?;
    let octet_string_struct = lib
        .define_function_return_struct(octet_string_struct_decl)?
        .add("index", Primitive::U16, "Point index")?
        .add("value", byte_it, "Point value")?
        .doc("Octet String point")?
        .end_fields()?
        .build()?;

    let octet_string_iterator =
        lib.define_iterator_with_lifetime("octet_string_iterator", octet_string_struct.clone())?;

    Ok((octet_string_struct, octet_string_iterator))
}
