use oo_bindgen::callback::InterfaceHandle;
use oo_bindgen::class::ClassDeclarationHandle;
use oo_bindgen::doc;
use oo_bindgen::iterator::IteratorHandle;
use oo_bindgen::native_enum::NativeEnumHandle;
use oo_bindgen::native_function::{DurationMapping, ReturnType, Type};
use oo_bindgen::native_struct::{NativeStructHandle, StructElementType};
use oo_bindgen::{BindingError, LibraryBuilder};

pub struct SharedDefinitions {
    pub port_state_listener: InterfaceHandle,
    pub variation_enum: NativeEnumHandle,
    pub runtime_class: ClassDeclarationHandle,
    pub decode_level: NativeStructHandle,
    pub serial_port_settings: NativeStructHandle,
    pub link_error_mode: NativeEnumHandle,
    pub retry_strategy: NativeStructHandle,
    pub control_struct: NativeStructHandle,
    pub g12v1_struct: NativeStructHandle,
    pub binary_point: NativeStructHandle,
    pub binary_it: IteratorHandle,
    pub double_bit_binary_point: NativeStructHandle,
    pub double_bit_binary_it: IteratorHandle,
    pub binary_output_status_point: NativeStructHandle,
    pub binary_output_status_it: IteratorHandle,
    pub counter_point: NativeStructHandle,
    pub counter_it: IteratorHandle,
    pub frozen_counter_point: NativeStructHandle,
    pub frozen_counter_it: IteratorHandle,
    pub analog_point: NativeStructHandle,
    pub analog_it: IteratorHandle,
    pub analog_output_status_point: NativeStructHandle,
    pub analog_output_status_it: IteratorHandle,
}

pub fn define(lib: &mut LibraryBuilder) -> Result<SharedDefinitions, BindingError> {
    crate::constants::define(lib)?;

    let control_struct = lib.declare_native_struct("Control")?;
    let control_struct = lib
        .define_native_struct(&control_struct)?
        .add("fir", Type::Bool, "First fragment in the message")?
        .add("fin", Type::Bool, "Final fragment of the message")?
        .add("con", Type::Bool, "Requires confirmation")?
        .add("uns", Type::Bool, "Unsolicited response")?
        .add("seq", Type::Uint8, "Sequence number")?
        .doc("APDU Control field")?
        .build()?;

    let trip_close_code = lib
        .define_native_enum("TripCloseCode")?
        .variant("Nul", 0, "NUL (0)")?
        .variant("Close", 1, "CLOSE (1)")?
        .variant("Trip", 2, "TRIP (2)")?
        .variant("Reserved", 3, "RESERVED (3)")?
        .doc(
            "Trip-Close Code field, used in conjunction with {enum:OpType} to specify a control operation")?
        .build()?;

    let op_type = lib
        .define_native_enum("OpType")?
        .variant("Nul", 0, "NUL (0)")?
        .variant("PulseOn", 1, "PULSE_ON (1)")?
        .variant("PulseOff", 2, "PULSE_OFF (2)")?
        .variant("LatchOn", 3, "LATCH_ON (3)")?
        .variant("LatchOff", 4, "LATCH_OFF(4)")?
        .doc("Operation Type field, used in conjunction with {enum:TripCloseCode} to specify a control operation")?
        .build()?;

    let control_code = lib.declare_native_struct("ControlCode")?;
    let control_code = lib
        .define_native_struct(&control_code)?
        .add("tcc", Type::Enum(trip_close_code), "This field is used in conjunction with the `op_type` field to specify a control operation")?
        .add("clear", Type::Bool, "Support for this field is optional. When the clear bit is set, the device shall remove pending control commands for that index and stop any control operation that is in progress for that index. The indexed point shall go to the state that it would have if the command were allowed to complete normally.")?
        .add("queue", StructElementType::Bool(Some(false)), "This field is obsolete and should always be 0")?
        .add("op_type", Type::Enum(op_type), "This field is used in conjunction with the `tcc` field to specify a control operation")?
        .doc("CROB ({struct:G12V1}) control code")?
        .build()?;

    let g12v1_struct = lib.declare_native_struct("G12V1")?;
    let g12v1_struct = lib
        .define_native_struct(&g12v1_struct)?
        .add("code", Type::Struct(control_code), "Control code")?
        .add("count", Type::Uint8, "Count")?
        .add(
            "on_time",
            Type::Uint32,
            "Duration the output drive remains active (in milliseconds)",
        )?
        .add(
            "off_time",
            Type::Uint32,
            "Duration the output drive remains non-active (in milliseconds)",
        )?
        .doc("Control Relay Output Block")?
        .build()?;

    // ======
    // Points
    // ======
    let flags_struct = declare_flags_struct(lib)?;

    let timestamp_struct = declare_timestamp_struct(lib)?;

    let double_bit_enum = lib
        .define_native_enum("DoubleBit")?
        .push("Intermediate", "Transition between conditions")?
        .push("DeterminedOff", "Determined to be OFF")?
        .push("DeterminedOn", "Determined to be ON")?
        .push("Indeterminate", "Abnormal or custom condition")?
        .doc("Double-bit binary input value")?
        .build()?;

    let (binary_point, binary_it) =
        build_iterator("Binary", Type::Bool, lib, &flags_struct, &timestamp_struct)?;
    let (double_bit_binary_point, double_bit_binary_it) = build_iterator(
        "DoubleBitBinary",
        Type::Enum(double_bit_enum),
        lib,
        &flags_struct,
        &timestamp_struct,
    )?;
    let (binary_output_status_point, binary_output_status_it) = build_iterator(
        "BinaryOutputStatus",
        Type::Bool,
        lib,
        &flags_struct,
        &timestamp_struct,
    )?;
    let (counter_point, counter_it) = build_iterator(
        "Counter",
        Type::Uint32,
        lib,
        &flags_struct,
        &timestamp_struct,
    )?;
    let (frozen_counter_point, frozen_counter_it) = build_iterator(
        "FrozenCounter",
        Type::Uint32,
        lib,
        &flags_struct,
        &timestamp_struct,
    )?;
    let (analog_point, analog_it) = build_iterator(
        "Analog",
        Type::Double,
        lib,
        &flags_struct,
        &timestamp_struct,
    )?;
    let (analog_output_status_point, analog_output_status_it) = build_iterator(
        "AnalogOutputStatus",
        Type::Double,
        lib,
        &flags_struct,
        &timestamp_struct,
    )?;

    Ok(SharedDefinitions {
        port_state_listener: define_port_state_listener(lib)?,
        variation_enum: crate::variation::define(lib)?,
        runtime_class: crate::runtime::define(lib)?,
        decode_level: crate::logging::define(lib)?,
        retry_strategy: define_retry_strategy(lib)?,
        serial_port_settings: define_serial_params(lib)?,
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
    })
}

fn define_retry_strategy(lib: &mut LibraryBuilder) -> Result<NativeStructHandle, BindingError> {
    let retry_strategy = lib.declare_native_struct("RetryStrategy")?;
    lib.define_native_struct(&retry_strategy)?
        .add(
            "min_delay",
            StructElementType::Duration(
                DurationMapping::Milliseconds,
                Some(std::time::Duration::from_secs(1)),
            ),
            "Minimum delay between two retries",
        )?
        .add(
            "max_delay",
            StructElementType::Duration(
                DurationMapping::Milliseconds,
                Some(std::time::Duration::from_secs(10)),
            ),
            "Maximum delay between two retries",
        )?
        .doc(doc("Retry strategy configuration.").details(
            "The strategy uses an exponential back-off with a minimum and maximum value.",
        ))?
        .build()
}

fn define_link_error_mode(lib: &mut LibraryBuilder) -> Result<NativeEnumHandle, BindingError> {
    lib
        .define_native_enum("LinkErrorMode")?
        .push("Discard", "Framing errors are discarded. The link-layer parser is reset on any error, and the parser begins scanning for 0x0564. This is always the behavior for serial ports.")?
        .push("Close", "Framing errors are bubbled up to calling code, closing the session. Suitable for physical layers that provide error correction like TCP.")?
        .doc("Controls how errors in parsed link-layer frames are handled. This behavior is configurable for physical layers with built-in error correction like TCP as the connection might be through a terminal server.")?
        .build()
}

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
            StructElementType::Uint32(Some(9600)),
            "Baud rate (in symbols-per-second)",
        )?
        .add(
            "data_bits",
            StructElementType::Enum(data_bits, Some("Eight".to_string())),
            "Number of bits used to represent a character sent on the line",
        )?
        .add(
            "flow_control",
            StructElementType::Enum(flow_control, Some("None".to_string())),
            "Type of signalling to use for controlling data transfer",
        )?
        .add(
            "parity",
            StructElementType::Enum(parity, Some("None".to_string())),
            "Type of parity to use for error checking",
        )?
        .add(
            "stop_bits",
            StructElementType::Enum(stop_bits, Some("One".to_string())),
            "Number of bits to use to signal the end of a character",
        )?
        .doc("Serial port settings")?
        .build()
}

fn declare_flags_struct(lib: &mut LibraryBuilder) -> Result<NativeStructHandle, BindingError> {
    let flags_struct = lib.declare_native_struct("Flags")?;
    let flags_struct = lib
        .define_native_struct(&flags_struct)?
        .add(
            "value",
            Type::Uint8,
            "bit-mask representing a set of individual flag bits",
        )?
        .doc("Collection of individual flag bits represented by an underlying mask value")?
        .build()?;

    Ok(flags_struct)
}

fn define_port_state_listener(lib: &mut LibraryBuilder) -> Result<InterfaceHandle, BindingError> {
    let port_state = lib
        .define_native_enum("PortState")?
        .push("Disabled", "Disabled until enabled")?
        .push("Wait", "Waiting to perform an open retry")?
        .push("Open", "Port is open")?
        .push("Shutdown", "Task has been shut down")?
        .doc("State of the serial port")?
        .build()?;

    let port_state_listener = lib
        .define_interface(
            "PortStateListener",
            "Callback interface for receiving updates about the state of a serial port",
        )?
        .callback("on_change", "Invoked when the serial port changes state")?
        .param("state", Type::Enum(port_state), "New state of the port")?
        .return_type(ReturnType::Void)?
        .build()?
        .destroy_callback("on_destroy")?
        .build()?;

    Ok(port_state_listener)
}

fn declare_timestamp_struct(lib: &mut LibraryBuilder) -> Result<NativeStructHandle, BindingError> {
    let time_quality_enum = lib
        .define_native_enum("TimeQuality")?
        .push(
            "Synchronized",
            "The timestamp is UTC synchronized at the remote device",
        )?
        .push(
            "NotSynchronized",
            "The device indicates the timestamp may be not be synchronized",
        )?
        .push(
            "Invalid",
            "Timestamp is not valid, ignore the value and use a local timestamp",
        )?
        .doc("Timestamp quality")?
        .build()?;

    let timestamp_struct = lib.declare_native_struct("Timestamp")?;
    let timestamp_struct = lib
        .define_native_struct(&timestamp_struct)?
        .add("value", Type::Uint64, "Timestamp value")?
        .add(
            "quality",
            Type::Enum(time_quality_enum),
            "Timestamp quality",
        )?
        .doc("Timestamp value")?
        .build()?;

    let timestamp_invalid_fn = lib
        .declare_native_function("timestamp_invalid")?
        .return_type(ReturnType::new(
            Type::Struct(timestamp_struct.clone()),
            "Invalid timestamp",
        ))?
        .doc("Creates an invalid timestamp struct")?
        .build()?;

    let timestamp_synchronized_fn = lib
        .declare_native_function("timestamp_synchronized")?
        .param(
            "value",
            Type::Uint64,
            "Timestamp value in milliseconds since EPOCH",
        )?
        .return_type(ReturnType::new(
            Type::Struct(timestamp_struct.clone()),
            "Synchronized timestamp",
        ))?
        .doc("Creates a synchronized timestamp struct")?
        .build()?;

    let timestamp_not_synchronized_fn = lib
        .declare_native_function("timestamp_not_synchronized")?
        .param(
            "value",
            Type::Uint64,
            "Timestamp value in milliseconds since EPOCH",
        )?
        .return_type(ReturnType::new(
            Type::Struct(timestamp_struct.clone()),
            "Not synchronized timestamp",
        ))?
        .doc("Creates a not synchronized timestamp struct")?
        .build()?;

    lib.define_struct(&timestamp_struct)?
        .static_method("invalid_timestamp", &timestamp_invalid_fn)?
        .static_method("synchronized_timestamp", &timestamp_synchronized_fn)?
        .static_method("not_synchronized_timestamp", &timestamp_not_synchronized_fn)?
        .build();

    Ok(timestamp_struct)
}

fn build_iterator(
    name: &str,
    value_type: Type,
    lib: &mut LibraryBuilder,
    flags_struct: &NativeStructHandle,
    timestamp_struct: &NativeStructHandle,
) -> Result<(NativeStructHandle, IteratorHandle), BindingError> {
    let value_struct = lib.declare_native_struct(name)?;
    let value_struct = lib
        .define_native_struct(&value_struct)?
        .add("index", Type::Uint16, "Point index")?
        .add("value", value_type, "Point value")?
        .add("flags", Type::Struct(flags_struct.clone()), "Point flags")?
        .add(
            "time",
            Type::Struct(timestamp_struct.clone()),
            "Point timestamp",
        )?
        .doc(format!("{} point", name))?
        .build()?;

    let value_iterator = lib.declare_class(&format!("{}Iterator", name))?;
    let iterator_next_fn = lib
        .declare_native_function(&format!("{}_next", name.to_lowercase()))?
        .param("it", Type::ClassRef(value_iterator), "Iterator")?
        .return_type(ReturnType::new(
            Type::StructRef(value_struct.declaration()),
            "Next value of the iterator or {null} if the iterator reached the end",
        ))?
        .doc("Get the next value of the iterator")?
        .build()?;

    let value_iterator = lib.define_iterator(&iterator_next_fn, &value_struct)?;

    Ok((value_struct, value_iterator))
}
