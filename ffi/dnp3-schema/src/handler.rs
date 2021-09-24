use oo_bindgen::callback::InterfaceHandle;
use oo_bindgen::native_struct::NativeStructHandle;
use oo_bindgen::*;

use crate::shared::SharedDefinitions;
use oo_bindgen::native_enum::NativeEnumHandle;
use oo_bindgen::types::BasicType;

pub fn define(
    lib: &mut LibraryBuilder,
    shared_def: &SharedDefinitions,
) -> Result<InterfaceHandle, BindingError> {
    let response_function = lib
        .define_native_enum("ResponseFunction")?
        .push("Response", "Solicited response")?
        .push("UnsolicitedResponse", "Unsolicited response")?
        .doc("Type of response")?
        .build()?;

    let iin = declare_iin_struct(lib)?;

    let response_header = lib.declare_native_struct("ResponseHeader")?;
    let response_header = lib
        .define_native_struct(&response_header)?
        .add(
            "control",
            shared_def.control_struct.clone(),
            "Application control field",
        )?
        .add("func", response_function, "Response type")?
        .add("iin", iin, "IIN bytes")?
        .doc("Response header information")?
        .build()?;

    let qualifier_code_enum = lib
        .define_native_enum("QualifierCode")?
        .push("Range8", "8-bit start stop (0x00)")?
        .push("Range16", "16-bit start stop (0x01)")?
        .push("AllObjects", "All objects (0x06)")?
        .push("Count8", "8-bit count (0x07)")?
        .push("Count16", "16-bit count (0x08)")?
        .push("CountAndPrefix8", "8-bit count and prefix (0x17)")?
        .push("CountAndPrefix16", "16-bit count and prefix (0x28)")?
        .push("FreeFormat16", "16-bit free format (0x5B)")?
        .doc("Qualifier code used in the response")?
        .build()?;

    let header_info = lib.declare_native_struct("HeaderInfo")?;
    let header_info = lib
        .define_native_struct(&header_info)?
        .add(
            "variation",
            shared_def.variation_enum.clone(),
            "Group/Variation used in the response",
        )?
        .add(
            "qualifier",
            qualifier_code_enum,
            "Qualitifer used in the response",
        )?
        .doc("Object header information")?
        .build()?;

    let read_type = define_read_type_enum(lib)?;

    let read_handler_interface = lib
        .define_interface(
            "ReadHandler",
            "General handler that will receive all values read from the outstation.",
        )?
        .callback("begin_fragment", "Marks the beginning of a fragment")?
        .param(
            "read_type",
            read_type.clone(),
            "Describes what triggered the read event"
        )?
        .param(
            "header",
            response_header.clone(),
            "Header of the fragment",
        )?
        .returns_nothing()?
        .build()?
        .callback("end_fragment", "Marks the end of a fragment")?
        .param(
            "read_type",
            read_type,
            "Describes what triggered the read event"
        )?
        .param(
            "header",
           response_header,
            "Header of the fragment",
        )?
        .returns_nothing()?
        .build()?
        .callback("handle_binary", "Handle binary input data")?
        .param(
            "info",
           header_info.clone(),
            "Group/variation and qualifier information",
        )?
        .param(
            "it",
           shared_def.binary_it.clone(),
                "Iterator of point values in the response. This iterator is valid only within this call. Do not copy it."
        )?
        .returns_nothing()?
        .build()?
        .callback(
            "handle_double_bit_binary",
            "Handle double-bit binary input data",
        )?
        .param(
            "info",
           header_info.clone(),
            "Group/variation and qualifier information",
        )?
        .param(
            "it",
           shared_def.double_bit_binary_it.clone(),
            "Iterator of point values in the response. This iterator is valid only within this call. Do not copy it."
        )?
        .returns_nothing()?
        .build()?
        .callback(
            "handle_binary_output_status",
            "Handle binary output status data",
        )?
        .param(
            "info",
           header_info.clone(),
            "Group/variation and qualifier information",
        )?
        .param(
            "it",
           shared_def.binary_output_status_it.clone(),
            "Iterator of point values in the response. This iterator is valid only within this call. Do not copy it."
        )?
        .returns_nothing()?
        .build()?
        .callback("handle_counter", "Handle counter data")?
        .param(
            "info",
           header_info.clone(),
            "Group/variation and qualifier information",
        )?
        .param(
            "it",
           shared_def.counter_it.clone(),
            "Iterator of point values in the response. This iterator is valid only within this call. Do not copy it."
        )?
        .returns_nothing()?
        .build()?
        .callback("handle_frozen_counter", "Handle frozen counter input data")?
        .param(
            "info",
           header_info.clone(),
            "Group/variation and qualifier information",
        )?
        .param(
            "it",
           shared_def.frozen_counter_it.clone(),
            "Iterator of point values in the response. This iterator is valid only within this call. Do not copy it."
        )?
        .returns_nothing()?
        .build()?
        .callback("handle_analog", "Handle analog input data")?
        .param(
            "info",
           header_info.clone(),
            "Group/variation and qualifier information",
        )?
        .param(
            "it",
           shared_def.analog_it.clone(),
            "Iterator of point values in the response. This iterator is valid only within this call. Do not copy it."
        )?
        .returns_nothing()?
        .build()?
        .callback(
            "handle_analog_output_status",
            "Handle analog output status data",
        )?
        .param(
            "info",
           header_info.clone(),
            "Group/variation and qualifier information",
        )?
        .param(
            "it",
           shared_def.analog_output_status_it.clone(),
            "Iterator of point values in the response. This iterator is valid only within this call. Do not copy it."
        )?
        .returns_nothing()?
        .build()?
        .callback("handle_octet_string", "Handle octet string data")?
        .param(
            "info",
           header_info,
            "Group/variation and qualifier information",
        )?
        .param(
            "it",
           shared_def.octet_string_it.clone(),
            "Iterator of point values in the response. This iterator is valid only within this call. Do not copy it."
        )?
        .returns_nothing()?
        .build()?
        .destroy_callback("on_destroy")?
        .build()?;

    Ok(read_handler_interface)
}

fn declare_iin_struct(lib: &mut LibraryBuilder) -> Result<NativeStructHandle, BindingError> {
    let iin1 = lib.declare_native_struct("IIN1")?;
    let iin1 = lib
        .define_native_struct(&iin1)?
        .add("value", BasicType::Uint8, "Byte value")?
        .doc("First IIN byte")?
        .build()?;

    let iin1_flag = lib
        .define_native_enum("IIN1Flag")?
        .push("Broadcast", "Indicate that the message was broadcasted")?
        .push(
            "Class1Events",
            "Outstation has Class 1 events not reported yet",
        )?
        .push(
            "Class2Events",
            "Outstation has Class 2 events not reported yet",
        )?
        .push(
            "Class3Events",
            "Outstation has Class 3 events not reported yet",
        )?
        .push(
            "NeedTime",
            "Outstation indicates it requires time synchronization from the master",
        )?
        .push(
            "LocalControl",
            "At least one point of the outstation is in the local operation mode",
        )?
        .push("DeviceTrouble", "Outstation reports abnormal condition")?
        .push("DeviceRestart", "Outstation has restarted")?
        .doc("First IIN bit flags")?
        .build()?;

    let iin1_is_set_fn = lib
        .declare_native_function("iin1_is_set")?
        .param("iin1", iin1.declaration(), "IIN1 to check")?
        .param("flag", iin1_flag, "Flag to check")?
        .returns(BasicType::Bool, "true if the flag is set, false otherwise")?
        .doc("Check if a particular flag is set in the IIN1 byte")?
        .build()?;

    lib.define_struct(&iin1)?
        .method("IsSet", &iin1_is_set_fn)?
        .build();

    let iin2 = lib.declare_native_struct("IIN2")?;
    let iin2 = lib
        .define_native_struct(&iin2)?
        .add("value", BasicType::Uint8, "Byte value")?
        .doc("Second IIN byte")?
        .build()?;

    let iin2_flag = lib
        .define_native_enum("IIN2Flag")?
        .push(
            "NoFuncCodeSupport",
            "Function code is not supported by the outstation",
        )?
        .push("ObjectUnknown", "Request contains an unknown point")?
        .push(
            "ParameterError",
            "Unable to parse request or invalid qualifier code",
        )?
        .push(
            "EventBufferOverflow",
            "Event buffer overflow, at least one event was lost",
        )?
        .push(
            "AlreadyExecuting",
            "Cannot perform operation because an execution is already in progress",
        )?
        .push(
            "ConfigCorrupt",
            "Outstation reports a configuration corruption",
        )?
        .doc("Second IIN bit flags")?
        .build()?;

    let iin2_is_set_fn = lib
        .declare_native_function("iin2_is_set")?
        .param("iin2", iin2.declaration(), "IIN2 to check")?
        .param("flag", iin2_flag, "Flag to check")?
        .returns(BasicType::Bool, "true if the flag is set, false otherwise")?
        .doc("Check if a particular flag is set in the IIN2 byte")?
        .build()?;

    lib.define_struct(&iin2)?
        .method("IsSet", &iin2_is_set_fn)?
        .build();

    let iin = lib.declare_native_struct("IIN")?;
    let iin = lib
        .define_native_struct(&iin)?
        .add("iin1", iin1, "First IIN byte")?
        .add("iin2", iin2, "Second IIN byte")?
        .doc("Pair of IIN bytes")?
        .build()?;

    Ok(iin)
}

fn define_read_type_enum(lib: &mut LibraryBuilder) -> Result<NativeEnumHandle, BindingError> {
    lib.define_native_enum("ReadType")?
        .push("StartupIntegrity", "Startup integrity poll")?
        .push("Unsolicited", "Unsolicited message")?
        .push("SinglePoll", "Single poll requested by the user")?
        .push("PeriodicPoll", "Periodic poll configured by the user")?
        .doc("Describes the source of a read event")?
        .build()
}
