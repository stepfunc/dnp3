use oo_bindgen::model::*;

use crate::shared::SharedDefinitions;

pub(crate) fn define(
    lib: &mut LibraryBuilder,
    shared: &SharedDefinitions,
) -> BackTraced<ClassHandle> {
    let request = lib.declare_class("request")?;

    let constructor = lib
        .define_constructor(request.clone())?
        .doc("Create a new request")?
        .build()?;

    let request_new_class_fn = lib
        .define_function("request_new_class")?
        .param("class0", Primitive::Bool, "Ask for class 0 (static data)")?
        .param("class1", Primitive::Bool, "Ask for class 1 events")?
        .param("class2", Primitive::Bool, "Ask for class 2 events")?
        .param("class3", Primitive::Bool, "Ask for class 3 events")?
        .returns(
          request.clone(),
            "Handle to the created request",
        )?
        .doc(
            doc("Create a new request asking for classes")
            .details("An identical request can be created manually with {class:request.add_all_objects_header()} and variations {enum:variation.group60_var1}, {enum:variation.group60_var2}, {enum:variation.group60_var3} and {enum:variation.group60_var4}.")
        )?
        .build_static("class_request")?;

    let request_new_all_objects_fn = lib
        .define_function("request_new_all_objects")?
        .param(
            "variation",
            shared.variation_enum.clone(),
            "Variation to ask for",
        )?
        .returns(
          request.clone(),
            "Handle to the created request",
        )?
        .doc(
            doc("Create a new request asking for all objects of a particular variation.")
            .details("An identical request can be created manually with {class:request.[constructor]} and {class:request.add_all_objects_header()}.")
        )?
        .build_static("all_objects")?;

    let request_new_one_byte_range_fn = lib
        .define_function("request_new_one_byte_range")?
        .param(
            "variation",
            shared.variation_enum.clone(),
            "Variation to ask for",
        )?
        .param("start", Primitive::U8, "Start index to ask")?
        .param("stop", Primitive::U8, "Stop index to ask (inclusive)")?
        .returns(
          request.clone(),
            "Handle to the created request",
        )?
        .doc(
            doc("Create a new request asking for range of objects (using 8-bit start/stop).")
            .details("An identical request can be created manually with {class:request.[constructor]} and {class:request.add_one_byte_range_header()}.")
        )?
        .build_static("one_byte_range")?;

    let request_new_two_byte_range_fn = lib
        .define_function("request_new_two_byte_range")?
        .param(
            "variation",
            shared.variation_enum.clone(),
            "Variation to ask for",
        )?
        .param("start", Primitive::U16, "Start index to ask")?
        .param("stop", Primitive::U16, "Stop index to ask (inclusive)")?
        .returns(
          request.clone(),
            "Handle to the created request",
        )?
        .doc(
            doc("Create a new request asking for range of objects (using 16-bit start/stop).")
            .details("An identical request can be created manually with {class:request.[constructor]} and {class:request.add_two_byte_range_header()}.")
        )?
        .build_static("two_byte_range")?;

    let request_new_one_byte_limited_count_fn = lib
        .define_function("request_new_one_byte_limited_count")?
        .param(
            "variation",
            shared.variation_enum.clone(),
            "Variation to ask for",
        )?
        .param("count", Primitive::U8, "Maximum number of events")?
        .returns(
          request.clone(),
            "Handle to the created request",
        )?
        .doc(
            doc("Create a new request asking for a limited count of objects (using 8-bit start/stop).")
            .details("An identical request can be created manually with {class:request.[constructor]} and {class:request.add_one_byte_limited_count_header()}.")
        )?
        .build_static("one_byte_limited_count")?;

    let request_new_two_byte_limited_count_fn = lib
        .define_function("request_new_two_byte_limited_count")?
        .param(
            "variation",
            shared.variation_enum.clone(),
            "Variation to ask for",
        )?
        .param("count", Primitive::U16, "Maximum number of events")?
        .returns(
          request.clone(),
            "Handle to the created request",
        )?
        .doc(
            doc("Create a new request asking for a limited count of objects (using 16-bit start/stop).")
            .details("An identical request can be created manually with {class:request.[constructor]} and {class:request.add_two_byte_limited_count_header()}.")
        )?
        .build_static("two_byte_limited_count")?;

    let destructor = lib
        .define_destructor(
            request.clone(),
            "Destroy a request created with {class:request.[constructor]} or {class:request.class_request()}."
        )?;

    let add_specific_attribute_fn = lib
        .define_method("add_specific_attribute", request.clone())?
        .param(
            "variation",
            Primitive::U8,
            "Variation of the device attribute",
        )?
        .param(
            "set",
            Primitive::U8,
            "The set (point) to which the attribute belongs",
        )?
        .doc(doc(
            "Add a one-byte start/stop header for use with a READ request",
        ))?
        .build()?;

    let add_string_attribute_fn = lib
        .define_method("add_string_attribute", request.clone())?
        .param("variation", Primitive::U8, "Variation of the attribute")?
        .param(
            "set",
            Primitive::U8,
            "The set (point) to which the attribute belongs",
        )?
        .param("value", StringType, "Value of the attribute")?
        .doc(doc(
            "Add a one-byte start/stop header containing for use with a WRITE request",
        ))?
        .build()?;

    let add_uint_attribute_fn = lib
        .define_method("add_uint_attribute", request.clone())?
        .param("variation", Primitive::U8, "Variation of the attribute")?
        .param(
            "set",
            Primitive::U8,
            "The set (point) to which the attribute belongs",
        )?
        .param("value", Primitive::U32, "Value of the attribute")?
        .doc(doc(
            "Add a one-byte start/stop header containing for use with a WRITE request",
        ))?
        .build()?;

    let add_one_byte_range_header = lib
        .define_method("add_one_byte_range_header", request.clone())?
        .param(
            "variation",
            shared.variation_enum.clone(),
            "Variation to ask for",
        )?
        .param("start", Primitive::U8, "Start index to ask")?
        .param("stop", Primitive::U8, "Stop index to ask (inclusive)")?
        .doc("Add a one-byte start/stop header for use with a READ request")?
        .build()?;

    let add_two_byte_range_header = lib
        .define_method("add_two_byte_range_header", request.clone())?
        .param(
            "variation",
            shared.variation_enum.clone(),
            "Variation to ask for",
        )?
        .param("start", Primitive::U16, "Start index to ask")?
        .param("stop", Primitive::U16, "Stop index to ask (inclusive)")?
        .doc("Add a two-byte start/stop header for use with a READ request")?
        .build()?;

    let add_all_objects_header = lib
        .define_method("add_all_objects_header", request.clone())?
        .param(
            "variation",
            shared.variation_enum.clone(),
            "Variation to ask for",
        )?
        .doc("Add an all objects variation request")?
        .build()?;

    let add_one_byte_limited_count_header = lib
        .define_method("add_one_byte_limited_count_header", request.clone())?
        .param(
            "variation",
            shared.variation_enum.clone(),
            "Variation to ask for",
        )?
        .param("count", Primitive::U8, "Maximum number of events")?
        .doc("Add a one-byte limited count variation header for use with a READ request")?
        .build()?;

    let add_two_byte_limited_count_header = lib
        .define_method("add_two_byte_limited_count_header", request.clone())?
        .param(
            "variation",
            shared.variation_enum.clone(),
            "Variation to ask for",
        )?
        .param("count", Primitive::U16, "Maximum number of events")?
        .doc("Add a two-byte limited count variation header for use with a READ request")?
        .build()?;

    let add_time_and_interval = lib
        .define_method("add_time_and_interval", request.clone())?
        .param(
            "time",
            Primitive::U64,
            "DNP3 48-bit timestamp representing count of milliseconds since epoch UTC",
        )?
        .param(
            "interval_ms",
            Primitive::U32,
            "Interval expressed in milliseconds",
        )?
        .doc(
            doc("Add a single g51v1 time-and-interval")
                .details("This is useful when constructing freeze-at-time requests"),
        )?
        .build()?;

    let request = lib
        .define_class(&request)?
        .constructor(constructor)?
        .destructor(destructor)?
        .static_method(request_new_class_fn)?
        .static_method(request_new_all_objects_fn)?
        .static_method(request_new_one_byte_range_fn)?
        .static_method(request_new_two_byte_range_fn)?
        .static_method(request_new_one_byte_limited_count_fn)?
        .static_method(request_new_two_byte_limited_count_fn)?
        .method(add_one_byte_range_header)?
        .method(add_two_byte_range_header)?
        .method(add_all_objects_header)?
        .method(add_one_byte_limited_count_header)?
        .method(add_two_byte_limited_count_header)?
        .method(add_time_and_interval)?
        .method(add_specific_attribute_fn)?
        .method(add_string_attribute_fn)?
        .method(add_uint_attribute_fn)?
        .doc(
            doc("Custom request")
            .details("Whenever a method takes a request as a parameter, the request is internally copied. Therefore, it is possible to reuse the same requests over and over.")
        )?
        .build()?;

    Ok(request)
}
