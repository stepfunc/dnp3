use oo_bindgen::native_function::Type;
use oo_bindgen::native_struct::NativeStructHandle;
use oo_bindgen::{BindingError, LibraryBuilder};

pub struct SharedDefinitions {
    pub control_struct: NativeStructHandle,
    pub g12v1_struct: NativeStructHandle,
}

pub fn define(lib: &mut LibraryBuilder) -> Result<SharedDefinitions, BindingError> {
    let control = lib.declare_native_struct("Control")?;
    let control = lib
        .define_native_struct(&control)?
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
        .add("queue", Type::Bool, "This field is obsolete and should always be 0")?
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

    Ok(SharedDefinitions {
        control_struct: control,
        g12v1_struct,
    })
}
