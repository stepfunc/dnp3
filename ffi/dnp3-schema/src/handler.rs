use oo_bindgen::*;

use crate::shared::SharedDefinitions;
use oo_bindgen::enum_type::EnumHandle;
use oo_bindgen::interface::InterfaceHandle;
use oo_bindgen::structs::CallbackArgStructHandle;
use oo_bindgen::types::BasicType;

pub fn define(
    lib: &mut LibraryBuilder,
    shared_def: &SharedDefinitions,
) -> BackTraced<InterfaceHandle> {
    let response_function = lib
        .define_enum("response_function")?
        .push("response", "Solicited response")?
        .push("unsolicited_response", "Unsolicited response")?
        .doc("Type of response")?
        .build()?;

    let iin = declare_iin_struct(lib)?;

    let response_header = lib.declare_callback_arg_struct("response_header")?;
    let response_header = lib
        .define_callback_argument_struct(response_header)?
        .add(
            "control",
            shared_def.control_struct.clone(),
            "Application control field",
        )?
        .add("func", response_function, "Response type")?
        .add("iin", iin, "IIN bytes")?
        .doc("Response header information")?
        .end_fields()?
        // TODO - constructor
        .build()?;

    let qualifier_code_enum = lib
        .define_enum("qualifier_code")?
        .push("range8", "8-bit start stop (0x00)")?
        .push("range16", "16-bit start stop (0x01)")?
        .push("all_objects", "All objects (0x06)")?
        .push("count8", "8-bit count (0x07)")?
        .push("count16", "16-bit count (0x08)")?
        .push("count_and_prefix_8", "8-bit count and prefix (0x17)")?
        .push("count_and_prefix_16", "16-bit count and prefix (0x28)")?
        .push("free_format_16", "16-bit free format (0x5B)")?
        .doc("Qualifier code used in the response")?
        .build()?;

    let header_info = lib.declare_callback_arg_struct("header_info")?;
    let header_info = lib
        .define_callback_argument_struct(header_info)?
        .add(
            "variation",
            shared_def.variation_enum.clone(),
            "Group/Variation used in the response",
        )?
        .add(
            "qualifier",
            qualifier_code_enum,
            "Qualifier used in the response",
        )?
        .doc("Object header information")?
        .end_fields()?
        // TODO - constructor
        .build()?;

    let read_type = define_read_type_enum(lib)?;

    let read_handler_interface = lib
        .define_asynchronous_interface(
            "read_handler",
            "General handler that will receive all values read from the outstation.",
        )?
        .begin_callback("begin_fragment", "Marks the beginning of a fragment")?
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
        .end_callback()?
        .begin_callback("end_fragment", "Marks the end of a fragment")?
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
        .end_callback()?
        .begin_callback("handle_binary", "Handle binary input data")?
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
        .end_callback()?
        .begin_callback(
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

        .end_callback()?
        .begin_callback(
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

        .end_callback()?
        .begin_callback("handle_counter", "Handle counter data")?
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

        .end_callback()?
        .begin_callback("handle_frozen_counter", "Handle frozen counter input data")?
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

        .end_callback()?
        .begin_callback("handle_analog", "Handle analog input data")?
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

        .end_callback()?
        .begin_callback(
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

        .end_callback()?
        .begin_callback("handle_octet_string", "Handle octet string data")?
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

        .end_callback()?
        .build()?;

    Ok(read_handler_interface)
}

fn declare_iin_struct(lib: &mut LibraryBuilder) -> BackTraced<CallbackArgStructHandle> {
    let iin1 = lib.declare_callback_arg_struct("iin1")?;
    let iin1 = lib
        .define_callback_argument_struct(iin1)?
        .doc("First IIN byte")?
        .add(
            "broadcast",
            BasicType::Bool,
            "Broadcast message was received",
        )?
        .add(
            "class_1_events",
            BasicType::Bool,
            "Outstation has unreported Class 1 events",
        )?
        .add(
            "class_2_events",
            BasicType::Bool,
            "Outstation has unreported Class 2 events",
        )?
        .add(
            "class_3_events",
            BasicType::Bool,
            "Outstation has unreported Class 3 events",
        )?
        .add(
            "need_time",
            BasicType::Bool,
            "Outstation requires time synchronization",
        )?
        .add(
            "local_control",
            BasicType::Bool,
            "One or more of the outstation’s points are in local control mode",
        )?
        .add(
            "device_trouble",
            BasicType::Bool,
            "An abnormal, device-specific condition exists in the outstation",
        )?
        .add("device_restart", BasicType::Bool, "Outstation restarted")?
        .end_fields()?
        // TODO - constructor
        .build()?;

    let iin2 = lib.declare_callback_arg_struct("iin2")?;
    let iin2 = lib
        .define_callback_argument_struct(iin2)?
        .doc("Second IIN byte")?
        .add("no_func_code_support", BasicType::Bool, "Outstation does not support this function code")?
        .add("object_unknown", BasicType::Bool, "Outstation does not support requested operation for objects in the request")?
        .add("parameter_error", BasicType::Bool, "Outstation does not support requested operation for objects in the request")?
        .add("event_buffer_overflow", BasicType::Bool, "An event buffer overflow condition exists in the outstation, and at least one unconfirmed event was lost")?
        .add("already_executing", BasicType::Bool, "The operation requested is already executing (optional support)")?
        .add("config_corrupt", BasicType::Bool, "The outstation detected corrupt configuration (optional support)")?
        .add("reserved_2", BasicType::Bool, "Reserved for future use - should always be set to 0")?
        .add("reserved_1", BasicType::Bool, "Reserved for future use - should always be set to 0")?
        .end_fields()?
        // TODO - constructor
        .build()?;

    let iin = lib.declare_callback_arg_struct("iin")?;
    let iin = lib
        .define_callback_argument_struct(iin)?
        .add("iin1", iin1, "First IIN byte")?
        .add("iin2", iin2, "Second IIN byte")?
        .doc("Pair of IIN bytes")?
        .end_fields()?
        // TODO - constructor
        .build()?;

    Ok(iin)
}

fn define_read_type_enum(lib: &mut LibraryBuilder) -> BackTraced<EnumHandle> {
    let read_type = lib
        .define_enum("read_type")?
        .push("startup_integrity", "Startup integrity poll")?
        .push("unsolicited", "Unsolicited message")?
        .push("single_poll", "Single poll requested by the user")?
        .push("periodic_poll", "Periodic poll configured by the user")?
        .doc("Describes the source of a read event")?
        .build()?;

    Ok(read_type)
}
