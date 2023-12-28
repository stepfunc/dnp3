use crate::shared::SharedDefinitions;
use oo_bindgen::model::*;

pub(crate) fn define(
    lib: &mut LibraryBuilder,
    shared_def: &SharedDefinitions,
) -> BackTraced<AsynchronousInterface> {
    let response_function = lib
        .define_enum("response_function")?
        .push("response", "Solicited response")?
        .push("unsolicited_response", "Unsolicited response")?
        .doc("Type of response")?
        .build()?;

    let iin = declare_iin_struct(lib)?;

    let response_header = lib.declare_callback_argument_struct("response_header")?;
    let response_header = lib
        .define_callback_argument_struct(response_header)?
        .add(
            "control_field",
            shared_def.control_field_struct.clone(),
            "Application control field",
        )?
        .add("func", response_function, "Response type")?
        .add("iin", iin, "IIN bytes")?
        .doc("Response header information")?
        .end_fields()?
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

    let header_info = lib.declare_callback_argument_struct("header_info")?;
    let header_info = lib
        .define_callback_argument_struct(header_info)?
        .add(
            "variation",
            shared_def.variation_enum.clone(),
            "underlying variation in the response",
        )?
        .add(
            "qualifier",
            qualifier_code_enum,
            "Qualifier code used in the response",
        )?
        .add(
            "is_event",
            Primitive::Bool,
            "true if the received variation is an event type, false otherwise",
        )?
        .add(
            "has_flags",
            Primitive::Bool,
            "true if a flags byte is present on the underlying variation, false otherwise",
        )?
        .doc("Information about the object header and specific variation")?
        .end_fields()?
        .build()?;

    let read_type = define_read_type_enum(lib)?;

    let iterator_doc = "{iterator} of values from the response";

    let read_handler_interface = lib
        .define_interface(
            "read_handler",
            doc("Callback interface used to received measurement values received from the outstation.")
                .details("Methods are always invoked in the following order. {interface:read_handler.begin_fragment()} is called first, followed by one or more of the measurement handlers, and finally {interface:read_handler.end_fragment()} is called."),
        )?
        .begin_callback("begin_fragment", "Called when a valid response fragment is received, but before any measurements are processed")?
        .param(
            "read_type",
            read_type.clone(),
            "Describes what triggered the callback, e.g. response to a poll vs an unsolicited response",
        )?
        .param("header", response_header.clone(), "Header of the fragment")?
        .returns_nothing_by_default()?
        .end_callback()?
        .begin_callback("end_fragment", "Called when all the data from a response fragment has been processed")?
        .param(
            "read_type",
            read_type,
            "Describes what triggered the read event",
        )?
        .param("header", response_header, "Header of the fragment")?
        .returns_nothing_by_default()?
        .end_callback()?
        .begin_callback("handle_binary_input", "Handle binary input data")?
        .param(
            "info",
            header_info.clone(),
            "Group/variation and qualifier information",
        )?
        .param("values", shared_def.binary_it.clone(), iterator_doc)?
        .returns_nothing_by_default()?
        .end_callback()?
        .begin_callback(
            "handle_double_bit_binary_input",
            "Handle double-bit binary input data",
        )?
        .param(
            "info",
            header_info.clone(),
            "Group/variation and qualifier information",
        )?
        .param(
            "values",
            shared_def.double_bit_binary_it.clone(),
            iterator_doc,
        )?
        .returns_nothing_by_default()?
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
            "values",
            shared_def.binary_output_status_it.clone(),
            iterator_doc,
        )?
        .returns_nothing_by_default()?
        .end_callback()?
        .begin_callback("handle_counter", "Handle counter data")?
        .param(
            "info",
            header_info.clone(),
            "Group/variation and qualifier information",
        )?
        .param("values", shared_def.counter_it.clone(), iterator_doc)?
        .returns_nothing_by_default()?
        .end_callback()?
        .begin_callback("handle_frozen_counter", "Handle frozen counter input data")?
        .param(
            "info",
            header_info.clone(),
            "Group/variation and qualifier information",
        )?
        .param("values", shared_def.frozen_counter_it.clone(), iterator_doc)?
        .returns_nothing_by_default()?
        .end_callback()?
        .begin_callback("handle_analog_input", "Handle analog input data")?
        .param(
            "info",
            header_info.clone(),
            "Group/variation and qualifier information",
        )?
        .param("values", shared_def.analog_it.clone(), iterator_doc)?
        .returns_nothing_by_default()?
        .end_callback()?
        .begin_callback("handle_frozen_analog_input", "Handle frozen analog input data")?
        .param(
            "info",
            header_info.clone(),
            "Group/variation and qualifier information",
        )?
        .param("values", shared_def.frozen_analog_it.clone(), iterator_doc)?
        .returns_nothing_by_default()?
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
            "values",
            shared_def.analog_output_status_it.clone(),
            iterator_doc,
        )?
        .returns_nothing_by_default()?
        .end_callback()?
        .begin_callback(
            "handle_binary_output_command_event",
            "Handle binary output command events",
        )?
        .param(
            "info",
            header_info.clone(),
            "Group/variation and qualifier information",
        )?
        .param(
            "values",
            shared_def.binary_command_event_it.clone(),
            iterator_doc,
        )?
        .returns_nothing_by_default()?
        .end_callback()?
        .begin_callback(
            "handle_analog_output_command_event",
            "Handle analog output command events",
        )?
        .param(
            "info",
            header_info.clone(),
            "Group/variation and qualifier information",
        )?
        .param(
            "values",
            shared_def.analog_command_event_it.clone(),
            iterator_doc,
        )?
        .returns_nothing_by_default()?
        .end_callback()?
        .begin_callback(
            "handle_unsigned_integer",
            "Handle unsigned integer values (g102)",
        )?
        .param(
            "info",
            header_info.clone(),
            "Group/variation and qualifier information",
        )?
        .param(
            "values",
            shared_def.unsigned_integer_it.clone(),
            iterator_doc,
        )?
        .returns_nothing_by_default()?
        .end_callback()?
        .begin_callback("handle_octet_string", "Handle octet string data")?
        .param(
            "info",
            header_info.clone(),
            "Group/variation and qualifier information",
        )?
        .param("values", shared_def.octet_string_it.clone(), iterator_doc)?
        .returns_nothing_by_default()?
        .end_callback()?
        // group 0 callbacks
        .begin_callback("handle_string_attr", "Handle a known or unknown visible string device attribute")?
        .param(
            "info",
            header_info.clone(),
            "Group/variation and qualifier information",
        )?
        .param(
            "attr",
            shared_def.attr.string_attr.clone(),
            "Enumeration describing the attribute (possibly unknown) associated with the value"
        )?
        .param(
            "set",
            Primitive::U8,
            "The set associated with this attribute. Examining this argument is only important if the attr argument is unknown."
        )?
        .param(
            "variation",
            Primitive::U8,
            "The variation associated with this attribute. Examining this argument is only important if the attr argument is unknown."
        )?
        .param("value", StringType, "attribute value")?
        .returns_nothing_by_default()?
        .end_callback()?
        .begin_callback("handle_variation_list_attr", "Handle a known or unknown list of attribute variations")?
        .param(
            "info",
            header_info.clone(),
            "Group/variation and qualifier information",
        )?
        .param(
            "attr",
            shared_def.attr.variation_list_attr.clone(),
            "Enumeration describing the attribute (possibly unknown) associated with the value"
        )?
        .param(
            "set",
            Primitive::U8,
            "The set associated with this attribute. Examining this argument is only important if the attr argument is unknown."
        )?
        .param(
            "variation",
            Primitive::U8,
            "The variation associated with this attribute. Examining this argument is only important if the attr argument is unknown."
        )?
        .param("value", shared_def.attr.attr_item_iter.clone(), "Iterator over a list of variation / properties pairs")?
        .returns_nothing_by_default()?
        .end_callback()?
        .begin_callback("handle_uint_attr", "Handle an unsigned integer device attribute")?
        .param(
            "info",
            header_info.clone(),
            "Group/variation and qualifier information",
        )?
        .param(
            "attr",
            shared_def.attr.uint_attr.clone(),
            "Enumeration describing the attribute (possibly unknown) associated with the value"
        )?
        .param(
            "set",
            Primitive::U8,
            "The set associated with this attribute. Examining this argument is only important if the attr argument is unknown."
        )?
        .param(
            "variation",
            Primitive::U8,
            "The variation associated with this attribute. Examining this argument is only important if the attr argument is unknown."
        )?
        .param("value", Primitive::U32, "attribute value")?
        .returns_nothing_by_default()?
        .end_callback()?
        .begin_callback("handle_bool_attr",
                        doc("Handle a boolean device attribute")
                            .details("These are actually signed integer values on the wire. This method is only called for known values")
        )?
        .param(
            "info",
            header_info.clone(),
            "Group/variation and qualifier information",
        )?
        .param(
            "attr",
            shared_def.attr.bool_attr.clone(),
            "Enumeration describing the attribute associated with the value"
        )?
        .param(
            "set",
            Primitive::U8,
            "The set associated with this attribute. Examining this argument is only important if the attr argument is unknown."
        )?
        .param(
            "variation",
            Primitive::U8,
            "The variation associated with this attribute. Examining this argument is only important if the attr argument is unknown."
        )?
        .param("value", Primitive::Bool, "attribute value")?
        .returns_nothing_by_default()?
        .end_callback()?
        .begin_callback("handle_int_attr",
                        doc("Handle a signed integer device attribute")
                            .details("There are no defined attributes for this type that aren't mapped to booleans so there is no enumeration")
        )?
        .param(
            "info",
            header_info.clone(),
            "Group/variation and qualifier information",
        )?
        .param(
            "attr",
            shared_def.attr.int_attr.clone(),
            "Enumeration describing the attribute associated with the value"
        )?
        .param(
            "set",
            Primitive::U8,
            "The set associated with this attribute. Examining this argument is only important if the attr argument is unknown."
        )?
        .param(
            "variation",
            Primitive::U8,
            "The variation associated with this attribute. Examining this argument is only important if the attr argument is unknown."
        )?
        .param("value", Primitive::S32, "attribute value")?
        .returns_nothing_by_default()?
        .end_callback()?
        .begin_callback("handle_time_attr", "Handle a DNP3 time device attribute")?
        .param(
            "info",
            header_info.clone(),
            "Group/variation and qualifier information",
        )?
        .param(
            "attr",
            shared_def.attr.time_attr.clone(),
            "Enumeration describing the attribute associated with the value"
        )?
        .param(
            "set",
            Primitive::U8,
            "The set associated with this attribute. Examining this argument is only important if the attr argument is unknown."
        )?
        .param(
            "variation",
            Primitive::U8,
            "The variation associated with this attribute. Examining this argument is only important if the attr argument is unknown."
        )?
        .param("value", Primitive::U64, "48-bit timestamp representing milliseconds since Unix epoch")?
        .returns_nothing_by_default()?
        .end_callback()?
        .begin_callback("handle_float_attr", "Handle a floating point device attribute")?
        .param(
            "info",
            header_info.clone(),
            "Group/variation and qualifier information",
        )?
        .param(
            "attr",
            shared_def.attr.float_attr.clone(),
            "Enumeration describing the attribute associated with the value"
        )?
        .param(
            "set",
            Primitive::U8,
            "The set associated with this attribute. Examining this argument is only important if the attr argument is unknown."
        )?
        .param(
            "variation",
            Primitive::U8,
            "The variation associated with this attribute. Examining this argument is only important if the attr argument is unknown."
        )?
        .param("value", Primitive::Double, "Attribute value")?
        .returns_nothing_by_default()?
        .end_callback()?
        .begin_callback("handle_octet_string_attr", "Handle an octet string device attribute")?
        .param(
            "info",
            header_info.clone(),
            "Group/variation and qualifier information",
        )?
        .param(
            "attr",
            shared_def.attr.octet_string_attr.clone(),
            "Enumeration describing the attribute associated with the value"
        )?
        .param(
            "set",
            Primitive::U8,
            "The set associated with this attribute. Examining this argument is only important if the attr argument is unknown."
        )?
        .param(
            "variation",
            Primitive::U8,
            "The variation associated with this attribute. Examining this argument is only important if the attr argument is unknown."
        )?
        .param("value", shared_def.byte_it.clone(), "Iterator over bytes in the octet-string")?
        .returns_nothing_by_default()?
        .end_callback()?
        .begin_callback("handle_bit_string_attr", "Handle a bit string device attribute")?
        .param(
            "info",
            header_info,
            "Group/variation and qualifier information",
        )?
        .param(
            "attr",
            shared_def.attr.bit_string_attr.clone(),
            "Enumeration describing the attribute associated with the value"
        )?
        .param(
            "set",
            Primitive::U8,
            "The set associated with this attribute. Examining this argument is only important if the attr argument is unknown."
        )?
        .param(
            "variation",
            Primitive::U8,
            "The variation associated with this attribute. Examining this argument is only important if the attr argument is unknown."
        )?
        .param("value", shared_def.byte_it.clone(), "Iterator over bytes in the bit-string")?
        .returns_nothing_by_default()?
        .end_callback()?
        .build_async()?;

    Ok(read_handler_interface)
}

fn declare_iin_struct(lib: &mut LibraryBuilder) -> BackTraced<CallbackArgStructHandle> {
    let iin1 = lib.declare_callback_argument_struct("iin1")?;
    let iin1 = lib
        .define_callback_argument_struct(iin1)?
        .doc("First IIN byte")?
        .add(
            "broadcast",
            Primitive::Bool,
            "Broadcast message was received",
        )?
        .add(
            "class_1_events",
            Primitive::Bool,
            "Outstation has unreported Class 1 events",
        )?
        .add(
            "class_2_events",
            Primitive::Bool,
            "Outstation has unreported Class 2 events",
        )?
        .add(
            "class_3_events",
            Primitive::Bool,
            "Outstation has unreported Class 3 events",
        )?
        .add(
            "need_time",
            Primitive::Bool,
            "Outstation requires time synchronization",
        )?
        .add(
            "local_control",
            Primitive::Bool,
            "One or more of the outstation’s points are in local control mode",
        )?
        .add(
            "device_trouble",
            Primitive::Bool,
            "An abnormal, device-specific condition exists in the outstation",
        )?
        .add("device_restart", Primitive::Bool, "Outstation restarted")?
        .end_fields()?
        .build()?;

    let iin2 = lib.declare_callback_argument_struct("iin2")?;
    let iin2 = lib
        .define_callback_argument_struct(iin2)?
        .doc("Second IIN byte")?
        .add("no_func_code_support", Primitive::Bool, "Outstation does not support this function code")?
        .add("object_unknown", Primitive::Bool, "Outstation does not support requested operation for objects in the request")?
        .add("parameter_error", Primitive::Bool, "Outstation does not support requested operation for objects in the request")?
        .add("event_buffer_overflow", Primitive::Bool, "An event buffer overflow condition exists in the outstation, and at least one unconfirmed event was lost")?
        .add("already_executing", Primitive::Bool, "The operation requested is already executing (optional support)")?
        .add("config_corrupt", Primitive::Bool, "The outstation detected corrupt configuration (optional support)")?
        .add("reserved_2", Primitive::Bool, "Reserved for future use - should always be set to 0")?
        .add("reserved_1", Primitive::Bool, "Reserved for future use - should always be set to 0")?
        .end_fields()?
        .build()?;

    let iin = lib.declare_callback_argument_struct("iin")?;
    let iin = lib
        .define_callback_argument_struct(iin)?
        .add("iin1", iin1, "First IIN byte")?
        .add("iin2", iin2, "Second IIN byte")?
        .doc("Pair of IIN bytes")?
        .end_fields()?
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
