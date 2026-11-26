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
    const NAME: &str = "read_request";
    let read_request = lib.declare_class(NAME)?;

    let class_scan_fn = lib
        .define_function("read_request_new_class_scan")?
        .param("class0", Primitive::Bool, "Ask for class 0 (static data)")?
        .param("class1", Primitive::Bool, "Ask for class 1 events")?
        .param("class2", Primitive::Bool, "Ask for class 2 events")?
        .param("class3", Primitive::Bool, "Ask for class 3 events")?
        .returns(read_request.clone(), "Handle to the created request")?
        .doc(
            doc("Create a new read request asking for classes")
                .details("The request asks for events in classes 1, 2 and 3, then class 0."),
        )?
        .build_static("class_scan")?;

    let specific_attribute_fn = lib
        .define_function("read_request_new_specific_attribute")?
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
        .returns(read_request.clone(), "Handle to the created request")?
        .doc("Create a new read request asking for a specific device attribute (group 0)")?
        .build_static("specific_attribute")?;

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

    let destructor = lib.define_destructor(read_request.clone(), "Destroy a read request")?;

    let constructors = define_single_header_constructors(lib, shared, &read_request, NAME)?;
    let header_methods = define_header_methods(lib, shared, &read_request)?;

    let mut builder = lib
        .define_class(&read_request)?
        .destructor(destructor)?
        .static_method(class_scan_fn)?
        .static_method(specific_attribute_fn)?;
    for method in constructors {
        builder = builder.static_method(method)?;
    }
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
    const NAME: &str = "request";
    let request = lib.declare_class(NAME)?;

    let constructor = lib
        .define_constructor(request.clone())?
        .doc("Create a new request with no headers")?
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

    let destructor = lib
        .define_destructor(
            request.clone(),
            "Destroy a request created with {class:request.[constructor]} or {class:request.class_request()}."
        )?;

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

    let constructors = define_single_header_constructors(lib, shared, &request, NAME)?;
    let header_methods = define_header_methods(lib, shared, &request)?;

    let mut builder = lib
        .define_class(&request)?
        .constructor(constructor)?
        .destructor(destructor)?
        .static_method(request_new_class_fn)?;
    for method in constructors {
        builder = builder.static_method(method)?;
    }
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

/// The static methods that create a request of a single header, for both request classes
fn define_single_header_constructors(
    lib: &mut LibraryBuilder,
    shared: &SharedDefinitions,
    class: &ClassDeclarationHandle,
    name: &str,
) -> BackTraced<Vec<StaticMethod<Unvalidated>>> {
    let more = |method: &str| {
        format!("More headers can be added to it with {{class:{name}.{method}()}} and the other add methods.")
    };

    let all_objects = lib
        .define_function(format!("{name}_new_all_objects"))?
        .param(
            "variation",
            shared.variation_enum.clone(),
            "Variation to ask for",
        )?
        .returns(class.clone(), "Handle to the created request")?
        .doc(
            doc("Create a new request asking for all objects of a particular variation.")
                .details(more("add_all_objects_header")),
        )?
        .build_static("all_objects")?;

    let one_byte_range = lib
        .define_function(format!("{name}_new_one_byte_range"))?
        .param(
            "variation",
            shared.variation_enum.clone(),
            "Variation to ask for",
        )?
        .param("start", Primitive::U8, "Start index to ask")?
        .param("stop", Primitive::U8, "Stop index to ask (inclusive)")?
        .returns(class.clone(), "Handle to the created request")?
        .doc(
            doc("Create a new request asking for range of objects (using 8-bit start/stop).")
                .details(more("add_one_byte_range_header")),
        )?
        .build_static("one_byte_range")?;

    let two_byte_range = lib
        .define_function(format!("{name}_new_two_byte_range"))?
        .param(
            "variation",
            shared.variation_enum.clone(),
            "Variation to ask for",
        )?
        .param("start", Primitive::U16, "Start index to ask")?
        .param("stop", Primitive::U16, "Stop index to ask (inclusive)")?
        .returns(class.clone(), "Handle to the created request")?
        .doc(
            doc("Create a new request asking for range of objects (using 16-bit start/stop).")
                .details(more("add_two_byte_range_header")),
        )?
        .build_static("two_byte_range")?;

    let one_byte_limited_count = lib
        .define_function(format!("{name}_new_one_byte_limited_count"))?
        .param(
            "variation",
            shared.variation_enum.clone(),
            "Variation to ask for",
        )?
        .param("count", Primitive::U8, "Maximum number of events")?
        .returns(class.clone(), "Handle to the created request")?
        .doc(
            doc("Create a new request asking for a limited count of objects (using an 8-bit count).")
                .details(more("add_one_byte_limited_count_header")),
        )?
        .build_static("one_byte_limited_count")?;

    let two_byte_limited_count = lib
        .define_function(format!("{name}_new_two_byte_limited_count"))?
        .param(
            "variation",
            shared.variation_enum.clone(),
            "Variation to ask for",
        )?
        .param("count", Primitive::U16, "Maximum number of events")?
        .returns(class.clone(), "Handle to the created request")?
        .doc(
            doc("Create a new request asking for a limited count of objects (using a 16-bit count).")
                .details(more("add_two_byte_limited_count_header")),
        )?
        .build_static("two_byte_limited_count")?;

    Ok(vec![
        all_objects,
        one_byte_range,
        two_byte_range,
        one_byte_limited_count,
        two_byte_limited_count,
    ])
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
