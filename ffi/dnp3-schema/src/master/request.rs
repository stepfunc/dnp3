// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use oo_bindgen::model::*;

use crate::shared::SharedDefinitions;

pub(crate) struct RequestClasses {
    /// the headers of a READ, for reads and polls
    pub(crate) read_request: ClassHandle,
    /// the headers of a request that expects an empty response, such as a WRITE or a FREEZE
    pub(crate) request: ClassHandle,
}

pub(crate) fn define(
    lib: &mut LibraryBuilder,
    shared: &SharedDefinitions,
) -> BackTraced<RequestClasses> {
    Ok(RequestClasses {
        read_request: define_read_request(lib, shared)?,
        request: define_request(lib, shared)?,
    })
}

fn define_read_request(
    lib: &mut LibraryBuilder,
    shared: &SharedDefinitions,
) -> BackTraced<ClassHandle> {
    let read_request = lib.declare_class("read_request")?;

    let constructor = lib
        .define_constructor(read_request.clone())?
        .doc("Create a new read request with no headers")?
        .build()?;

    let destructor = lib.define_destructor(read_request.clone(), "Destroy a read request")?;

    let add_classes_fn = lib
        .define_method("add_classes", read_request.clone())?
        .param("class0", Primitive::Bool, "Ask for class 0 (static data)")?
        .param("class1", Primitive::Bool, "Ask for class 1 events")?
        .param("class2", Primitive::Bool, "Ask for class 2 events")?
        .param("class3", Primitive::Bool, "Ask for class 3 events")?
        .doc(
            doc("Add headers asking for classes")
                .details("The headers ask for events in classes 1, 2 and 3, then class 0."),
        )?
        .build()?;

    let add_specific_attribute_fn = lib
        .define_method("add_specific_attribute", read_request.clone())?
        .param(
            "variation",
            Primitive::U8,
            "Variation of the device attribute, 1 to 255 (254 reads every attribute of the set)",
        )?
        .param(
            "set",
            Primitive::U8,
            "The set (point) to which the attribute belongs",
        )?
        .doc("Add a header that reads a specific device attribute (group 0)")?
        .build()?;

    let header_methods = define_header_methods(lib, shared, &read_request)?;

    let mut builder = lib
        .define_class(&read_request)?
        .constructor(constructor)?
        .destructor(destructor)?
        .method(add_classes_fn)?;
    for method in header_methods {
        builder = builder.method(method)?;
    }
    let read_request = builder
        .method(add_specific_attribute_fn)?
        .doc(
            doc("The object headers of a READ, for one-time reads and polls")
            .details("Only headers that are valid in a READ can be added. A request that writes or freezes objects is a {class:request}.")
            .details("Whenever a method takes a read request as a parameter, the request is internally copied. Therefore, it is possible to reuse the same requests over and over.")
        )?
        .build()?;

    Ok(read_request)
}

fn define_request(lib: &mut LibraryBuilder, shared: &SharedDefinitions) -> BackTraced<ClassHandle> {
    let request = lib.declare_class("request")?;

    let constructor = lib
        .define_constructor(request.clone())?
        .doc("Create a new request with no headers")?
        .build()?;

    let destructor = lib.define_destructor(
        request.clone(),
        "Destroy a request created with {class:request.[constructor]}",
    )?;

    let add_classes_fn = lib
        .define_method("add_classes", request.clone())?
        .param("class0", Primitive::Bool, "Ask for class 0 (static data)")?
        .param("class1", Primitive::Bool, "Ask for class 1 events")?
        .param("class2", Primitive::Bool, "Ask for class 2 events")?
        .param("class3", Primitive::Bool, "Ask for class 3 events")?
        .doc(
            doc("Add headers naming classes")
                .details("The headers name events in classes 1, 2 and 3, then class 0."),
        )?
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
            "Add a header that writes a visible string device attribute (group 0) for use with a WRITE request",
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
            "Add a header that writes an unsigned integer device attribute (group 0) for use with a WRITE request",
        ))?
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
            doc("Add a single g50v2 time-and-interval")
                .details("This is useful when constructing freeze-at-time requests"),
        )?
        .build()?;

    let header_methods = define_header_methods(lib, shared, &request)?;

    let mut builder = lib
        .define_class(&request)?
        .constructor(constructor)?
        .destructor(destructor)?
        .method(add_classes_fn)?;
    for method in header_methods {
        builder = builder.method(method)?;
    }
    let request = builder
        .method(add_time_and_interval)?
        .method(add_string_attribute_fn)?
        .method(add_uint_attribute_fn)?
        .doc(
            doc("The object headers of a request that expects an empty response, such as a WRITE or a FREEZE")
            .details("Reads and polls take a {class:read_request} instead.")
            .details("Whenever a method takes a request as a parameter, the request is internally copied. Therefore, it is possible to reuse the same requests over and over.")
        )?
        .build()?;

    Ok(request)
}

/// The methods that add a READ-style header, for both request classes
fn define_header_methods(
    lib: &mut LibraryBuilder,
    shared: &SharedDefinitions,
    class: &ClassDeclarationHandle,
) -> BackTraced<Vec<Method<Unvalidated>>> {
    let add_one_byte_range_header = lib
        .define_method("add_one_byte_range_header", class.clone())?
        .param(
            "variation",
            shared.variation_enum.clone(),
            "Variation to ask for",
        )?
        .param("start", Primitive::U8, "Start index to ask")?
        .param("stop", Primitive::U8, "Stop index to ask (inclusive)")?
        .doc("Add a one-byte start/stop header")?
        .build()?;

    let add_two_byte_range_header = lib
        .define_method("add_two_byte_range_header", class.clone())?
        .param(
            "variation",
            shared.variation_enum.clone(),
            "Variation to ask for",
        )?
        .param("start", Primitive::U16, "Start index to ask")?
        .param("stop", Primitive::U16, "Stop index to ask (inclusive)")?
        .doc("Add a two-byte start/stop header")?
        .build()?;

    let add_all_objects_header = lib
        .define_method("add_all_objects_header", class.clone())?
        .param(
            "variation",
            shared.variation_enum.clone(),
            "Variation to ask for",
        )?
        .doc("Add an all objects header")?
        .build()?;

    let add_one_byte_limited_count_header = lib
        .define_method("add_one_byte_limited_count_header", class.clone())?
        .param(
            "variation",
            shared.variation_enum.clone(),
            "Variation to ask for",
        )?
        .param("count", Primitive::U8, "Maximum number of events")?
        .doc("Add a one-byte limited count header")?
        .build()?;

    let add_two_byte_limited_count_header = lib
        .define_method("add_two_byte_limited_count_header", class.clone())?
        .param(
            "variation",
            shared.variation_enum.clone(),
            "Variation to ask for",
        )?
        .param("count", Primitive::U16, "Maximum number of events")?
        .doc("Add a two-byte limited count header")?
        .build()?;

    Ok(vec![
        add_one_byte_range_header,
        add_two_byte_range_header,
        add_all_objects_header,
        add_one_byte_limited_count_header,
        add_two_byte_limited_count_header,
    ])
}
