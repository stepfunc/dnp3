use oo_bindgen::class::ClassDeclarationHandle;
use oo_bindgen::enum_type::EnumHandle;
use oo_bindgen::error_type::{ErrorType, ExceptionType};
use oo_bindgen::interface::InterfaceHandle;
use oo_bindgen::iterator::IteratorHandle;
use oo_bindgen::structs::*;
use oo_bindgen::types::{BasicType, DurationType};
use oo_bindgen::{doc, UniversalOr};
use oo_bindgen::{BindingError, LibraryBuilder};
use std::time::Duration;

pub struct SharedDefinitions {
    pub error_type: ErrorType,
    pub port_state_listener: InterfaceHandle,
    pub variation_enum: EnumHandle,
    pub runtime_class: ClassDeclarationHandle,
    pub decode_level: UniversalStructHandle,
    pub serial_port_settings: FunctionArgStructHandle,
    pub link_error_mode: EnumHandle,
    pub retry_strategy: FunctionArgStructHandle,
    pub control_struct: CallbackArgStructHandle,
    pub g12v1_struct: UniversalStructHandle,
    pub binary_point: UniversalStructHandle,
    pub binary_it: IteratorHandle,
    pub double_bit_binary_point: UniversalStructHandle,
    pub double_bit_binary_it: IteratorHandle,
    pub binary_output_status_point: UniversalStructHandle,
    pub binary_output_status_it: IteratorHandle,
    pub counter_point: UniversalStructHandle,
    pub counter_it: IteratorHandle,
    pub frozen_counter_point: UniversalStructHandle,
    pub frozen_counter_it: IteratorHandle,
    pub analog_point: UniversalStructHandle,
    pub analog_it: IteratorHandle,
    pub analog_output_status_point: UniversalStructHandle,
    pub analog_output_status_it: IteratorHandle,
    pub octet_string: FunctionReturnStructHandle,
    pub octet_string_it: IteratorHandle,
}

pub fn define(lib: &mut LibraryBuilder) -> Result<SharedDefinitions, BindingError> {
    let error_type = lib
        .define_error_type(
            "ParamError",
            "ParamException",
            ExceptionType::UncheckedException,
        )?
        .add_error("NullParameter", "Null parameter")?
        .add_error(
            "AssociationDoesNotExist",
            "The specified association does not exist",
        )?
        .add_error(
            "AssociationDuplicateAddress",
            "Duplicate association address",
        )?
        .add_error("InvalidSocketAddress", "Invalid socket address")?
        .add_error("InvalidDnp3Address", "Invalid link-layer DNP3 address")?
        .add_error("InvalidBufferSize", "Invalid buffer size")?
        .add_error(
            "AddressFilterConflict",
            "Conflict in the address filter specification",
        )?
        .add_error("ServerAlreadyStarted", "Server already started")?
        .add_error(
            "ServerBindError",
            "Server failed to bind to the specified port",
        )?
        .add_error("MasterAlreadyShutdown", "Master was already shutdown")?
        .add_error("RuntimeCreationFailure", "Failed to create tokio runtime")?
        .add_error("RuntimeDestroyed", "Runtime was already disposed of")?
        .add_error(
            "RuntimeCannotBlockWithinAsync",
            "Runtime cannot execute blocking call within asynchronous context",
        )?
        .add_error(
            "LoggingAlreadyConfigured",
            "Logging can only be configured once",
        )?
        .add_error("PointDoesNotExist", "Point does not exist")?
        .doc("Error type used throughout the library")?
        .build()?;

    crate::constants::define(lib)?;
    let decode_level = crate::logging::define(lib, error_type.clone())?;
    let runtime_class = crate::runtime::define(lib, error_type.clone())?;

    let control_struct = lib.declare_callback_arg_struct("Control")?;
    let control_struct = lib
        .define_callback_argument_struct(control_struct)?
        .add("fir", BasicType::Bool, "First fragment in the message")?
        .add("fin", BasicType::Bool, "Final fragment of the message")?
        .add("con", BasicType::Bool, "Requires confirmation")?
        .add("uns", BasicType::Bool, "Unsolicited response")?
        .add("seq", BasicType::U8, "Sequence number")?
        .doc("APDU Control field")?
        .end_fields()?
        .build()?;

    let trip_close_code = lib
        .define_enum("TripCloseCode")
        .variant("Nul", 0, "NUL (0)")?
        .variant("Close", 1, "CLOSE (1)")?
        .variant("Trip", 2, "TRIP (2)")?
        .variant("Reserved", 3, "RESERVED (3)")?
        .doc(
            "Trip-Close Code field, used in conjunction with {enum:OpType} to specify a control operation")?
        .build()?;

    let op_type = lib
        .define_enum("OpType")
        .variant("Nul", 0, "NUL (0)")?
        .variant("PulseOn", 1, "PULSE_ON (1)")?
        .variant("PulseOff", 2, "PULSE_OFF (2)")?
        .variant("LatchOn", 3, "LATCH_ON (3)")?
        .variant("LatchOff", 4, "LATCH_OFF(4)")?
        .doc("Operation Type field, used in conjunction with {enum:TripCloseCode} to specify a control operation")?
        .build()?;

    let queue_field = FieldName::new("queue");

    let control_code = lib.declare_universal_struct("ControlCode")?;
    let control_code = lib
        .define_universal_struct(control_code)?
        .add("tcc", trip_close_code, "This field is used in conjunction with the `op_type` field to specify a control operation")?
        .add("clear", BasicType::Bool, "Support for this field is optional. When the clear bit is set, the device shall remove pending control commands for that index and stop any control operation that is in progress for that index. The indexed point shall go to the state that it would have if the command were allowed to complete normally.")?
        .add(&queue_field, BasicType::Bool, "This field is obsolete and should always be 0")?
        .add("op_type", op_type, "This field is used in conjunction with the `tcc` field to specify a control operation")?
        .doc("CROB ({struct:G12V1}) control code")?
        .end_fields()?
        .begin_constructor("init", ConstructorType::Normal, "Initialize a {struct:ControlCode} instance")?
        .default(&queue_field, false)?
        .end_constructor()?
        .build()?;

    let g12v1_struct = lib.declare_universal_struct("G12V1")?;
    let g12v1_struct = lib
        .define_universal_struct(g12v1_struct)?
        .add("code", control_code, "Control code")?
        .add("count", BasicType::U8, "Count")?
        .add(
            "on_time",
            BasicType::U32,
            "Duration the output drive remains active (in milliseconds)",
        )?
        .add(
            "off_time",
            BasicType::U32,
            "Duration the output drive remains non-active (in milliseconds)",
        )?
        .doc("Control Relay Output Block")?
        .end_fields()?
        .add_full_constructor("init")?
        .build()?;

    // ======
    // Points
    // ======
    let flags_struct = declare_flags_struct(lib)?;

    let timestamp_struct = declare_timestamp_struct(lib)?;

    let double_bit_enum = lib
        .define_enum("DoubleBit")
        .push("Intermediate", "Transition between conditions")?
        .push("DeterminedOff", "Determined to be OFF")?
        .push("DeterminedOn", "Determined to be ON")?
        .push("Indeterminate", "Abnormal or custom condition")?
        .doc("Double-bit binary input value")?
        .build()?;

    let (binary_point, binary_it) = build_iterator(
        "Binary",
        BasicType::Bool,
        lib,
        &flags_struct,
        &timestamp_struct,
    )?;
    let (double_bit_binary_point, double_bit_binary_it) = build_iterator(
        "DoubleBitBinary",
        double_bit_enum,
        lib,
        &flags_struct,
        &timestamp_struct,
    )?;
    let (binary_output_status_point, binary_output_status_it) = build_iterator(
        "BinaryOutputStatus",
        BasicType::Bool,
        lib,
        &flags_struct,
        &timestamp_struct,
    )?;
    let (counter_point, counter_it) = build_iterator(
        "Counter",
        BasicType::U32,
        lib,
        &flags_struct,
        &timestamp_struct,
    )?;
    let (frozen_counter_point, frozen_counter_it) = build_iterator(
        "FrozenCounter",
        BasicType::U32,
        lib,
        &flags_struct,
        &timestamp_struct,
    )?;
    let (analog_point, analog_it) = build_iterator(
        "Analog",
        BasicType::Double64,
        lib,
        &flags_struct,
        &timestamp_struct,
    )?;
    let (analog_output_status_point, analog_output_status_it) = build_iterator(
        "AnalogOutputStatus",
        BasicType::Double64,
        lib,
        &flags_struct,
        &timestamp_struct,
    )?;

    let (octet_string, octet_string_it) = build_octet_string(lib)?;

    Ok(SharedDefinitions {
        error_type,
        port_state_listener: define_port_state_listener(lib)?,
        variation_enum: crate::variation::define(lib)?,
        runtime_class,
        decode_level,
        retry_strategy: define_retry_strategy(lib)?,
        serial_port_settings: define_serial_port_settings(lib)?,
        link_error_mode: define_link_error_mode(lib)?,
        control_struct,
        g12v1_struct,
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

fn define_retry_strategy(
    lib: &mut LibraryBuilder,
) -> Result<FunctionArgStructHandle, BindingError> {
    let min_delay = FieldName::new("min_delay");
    let max_delay = FieldName::new("max_delay");

    let retry_strategy = lib.declare_function_arg_struct("RetryStrategy")?;
    lib.define_function_argument_struct(retry_strategy)?
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
        .begin_constructor("init", ConstructorType::Normal, "Initialize to defaults")?
        .default(&min_delay, Duration::from_secs(1))?
        .default(&max_delay, Duration::from_secs(10))?
        .end_constructor()?
        .build()
}

fn define_link_error_mode(lib: &mut LibraryBuilder) -> Result<EnumHandle, BindingError> {
    lib
        .define_enum("LinkErrorMode")
        .push("Discard", "Framing errors are discarded. The link-layer parser is reset on any error, and the parser begins scanning for 0x0564. This is always the behavior for serial ports.")?
        .push("Close", "Framing errors are bubbled up to calling code, closing the session. Suitable for physical layers that provide error correction like TCP.")?
        .doc("Controls how errors in parsed link-layer frames are handled. This behavior is configurable for physical layers with built-in error correction like TCP as the connection might be through a terminal server.")?
        .build()
}

fn define_serial_port_settings(
    lib: &mut LibraryBuilder,
) -> Result<FunctionArgStructHandle, BindingError> {
    let data_bits_enum = lib
        .define_enum("DataBits")
        .push("Five", "5 bits per character")?
        .push("Six", "6 bits per character")?
        .push("Seven", "7 bits per character")?
        .push("Eight", "8 bits per character")?
        .doc("Number of bits per character")?
        .build()?;

    let flow_control_enum = lib
        .define_enum("FlowControl")
        .push("None", "No flow control")?
        .push("Software", "Flow control using XON/XOFF bytes")?
        .push("Hardware", "Flow control using RTS/CTS signals")?
        .doc("Flow control modes")?
        .build()?;

    let parity_enum = lib
        .define_enum("Parity")
        .push("None", "No parity bit")?
        .push("Odd", "Parity bit sets odd number of 1 bits")?
        .push("Even", "Parity bit sets even number of 1 bits")?
        .doc("Parity checking modes")?
        .build()?;

    let stop_bits_enum = lib
        .define_enum("StopBits")
        .push("One", "One stop bit")?
        .push("Two", "Two stop bits")?
        .doc("Number of stop bits")?
        .build()?;

    let baud_rate = FieldName::new("baud_rate");
    let data_bits = FieldName::new("data_bits");
    let flow_control = FieldName::new("flow_control");
    let parity = FieldName::new("parity");
    let stop_bits = FieldName::new("stop_bits");

    let serial_settings = lib.declare_function_arg_struct("SerialPortSettings")?;
    lib.define_function_argument_struct(serial_settings)?
        .add(
            &baud_rate,
            BasicType::U32,
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
        .begin_constructor(
            "init",
            ConstructorType::Normal,
            "Initialize to default values",
        )?
        .default(&baud_rate, Number::U32(9600))?
        .default_variant(&data_bits, "Eight")?
        .default_variant(&flow_control, "None")?
        .default_variant(&parity, "None")?
        .default_variant(&stop_bits, "One")?
        .end_constructor()?
        .build()
}

fn declare_flags_struct(lib: &mut LibraryBuilder) -> Result<UniversalStructHandle, BindingError> {
    let flags_struct = lib.declare_universal_struct("Flags")?;
    let flags_struct = lib
        .define_universal_struct(flags_struct)?
        .add(
            "value",
            BasicType::U8,
            "bit-mask representing a set of individual flag bits",
        )?
        .doc("Collection of individual flag bits represented by an underlying mask value")?
        .end_fields()?
        .add_full_constructor("init")?
        .build()?;

    Ok(flags_struct)
}

fn define_port_state_listener(lib: &mut LibraryBuilder) -> Result<InterfaceHandle, BindingError> {
    let port_state = lib
        .define_enum("PortState")
        .push("Disabled", "Disabled until enabled")?
        .push("Wait", "Waiting to perform an open retry")?
        .push("Open", "Port is open")?
        .push("Shutdown", "Task has been shut down")?
        .doc("State of the serial port")?
        .build()?;

    let port_state_listener = lib
        .define_asynchronous_interface(
            "PortStateListener",
            "Callback interface for receiving updates about the state of a serial port",
        )
        .begin_callback("on_change", "Invoked when the serial port changes state")?
        .param("state", port_state, "New state of the port")?
        .returns_nothing()?
        .end_callback()?
        .build()?;

    Ok(port_state_listener)
}

fn declare_timestamp_struct(
    lib: &mut LibraryBuilder,
) -> Result<UniversalStructHandle, BindingError> {
    let time_quality_enum = lib
        .define_enum("TimeQuality")
        .push(
            "Synchronized",
            "The timestamp is UTC synchronized at the remote device",
        )?
        .push(
            "Unsynchronized",
            "The device indicates the timestamp may be not be synchronized",
        )?
        .push(
            "Invalid",
            "Timestamp is not valid, ignore the value and use a local timestamp",
        )?
        .doc("Timestamp quality")?
        .build()?;

    let value = FieldName::new("value");
    let quality = FieldName::new("quality");

    let timestamp_struct = lib.declare_universal_struct("Timestamp")?;
    let timestamp_struct = lib
        .define_universal_struct(timestamp_struct)?
        .add(&value, BasicType::U64, "Timestamp value")?
        .add(&quality, time_quality_enum, "Timestamp quality")?
        .doc("Timestamp value")?
        .end_fields()?
        .begin_constructor(
            "invalid_timestamp",
            ConstructorType::Static,
            "Creates an invalid timestamp struct",
        )?
        .default(&value, Number::U64(0))?
        .default_variant(&quality, "Invalid")?
        .end_constructor()?
        .begin_constructor(
            "synchronized_timestamp",
            ConstructorType::Static,
            "Creates a synchronized timestamp struct",
        )?
        .default_variant(&quality, "Synchronized")?
        .end_constructor()?
        .begin_constructor(
            "unsynchronized_timestamp",
            ConstructorType::Static,
            "Creates an unsynchronized timestamp struct",
        )?
        .default_variant(&quality, "Unsynchronized")?
        .end_constructor()?
        .build()?;

    Ok(timestamp_struct)
}

fn build_iterator<T: Into<UniversalStructField>>(
    name: &str,
    value_type: T,
    lib: &mut LibraryBuilder,
    flags_struct: &UniversalStructHandle,
    timestamp_struct: &UniversalStructHandle,
) -> Result<(UniversalStructHandle, IteratorHandle), BindingError> {
    let value_struct_decl = lib.declare_universal_struct(name)?;
    let value_struct = lib
        .define_universal_struct(value_struct_decl.clone())?
        .add("index", BasicType::U16, "Point index")?
        .add("value", value_type, "Point value")?
        .add("flags", flags_struct.clone(), "Point flags")?
        .add("time", timestamp_struct.clone(), "Point timestamp")?
        .doc(format!("{} point", name))?
        .end_fields()?
        .add_full_constructor("init")?
        .build()?;

    let value_iterator = lib.declare_class(&format!("{}Iterator", name))?;
    let iterator_next_fn = lib
        .define_function(&format!("{}_next", name.to_lowercase()))
        .param("it", value_iterator, "Iterator")?
        .returns(
            value_struct_decl,
            "Next value of the iterator or {null} if the iterator reached the end",
        )?
        .doc("Get the next value of the iterator")?
        .build()?;

    let value_iterator = lib.define_iterator(
        &iterator_next_fn,
        UniversalOr::Universal(value_struct.clone()),
    )?;

    Ok((value_struct, value_iterator))
}

fn build_octet_string(
    lib: &mut LibraryBuilder,
) -> Result<(FunctionReturnStructHandle, IteratorHandle), BindingError> {
    // Octet string stuff
    let byte_struct_decl = lib.declare_function_return_struct("Byte")?;
    let byte_struct = lib
        .define_function_return_struct(byte_struct_decl.clone())?
        .add("value", BasicType::U8, "Byte value")?
        .doc("Single byte struct")?
        .end_fields()?
        // TODO - constructor
        .build()?;

    let byte_it = lib.declare_class("ByteIterator")?;
    let byte_it_next_fn = lib
        .define_function("byte_next")
        .param("it", byte_it, "Iterator")?
        .returns(
            byte_struct_decl,
            "Next value of the iterator or {null} if the iterator reached the end",
        )?
        .doc("Get the next value of the iterator")?
        .build()?;

    let byte_it = lib.define_iterator_with_lifetime(&byte_it_next_fn, byte_struct.into())?;

    let octet_string_struct_decl = lib.declare_function_return_struct("OctetString")?;
    let octet_string_struct = lib
        .define_function_return_struct(octet_string_struct_decl.clone())?
        .add("index", BasicType::U16, "Point index")?
        .add("value", byte_it, "Point value")?
        .doc("Octet String point")?
        .end_fields()?
        // TODO - constructor
        .build()?;

    let octet_string_iterator = lib.declare_class("OctetStringIterator")?;
    let iterator_next_fn = lib
        .define_function("octetstring_next")
        .param("it", octet_string_iterator, "Iterator")?
        .returns(
            octet_string_struct_decl,
            "Next value of the iterator or {null} if the iterator reached the end",
        )?
        .doc("Get the next value of the iterator")?
        .build()?;

    let octet_string_iterator =
        lib.define_iterator_with_lifetime(&iterator_next_fn, octet_string_struct.clone().into())?;

    Ok((octet_string_struct, octet_string_iterator))
}
