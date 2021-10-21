use oo_bindgen::class::ClassHandle;
use oo_bindgen::*;

use crate::shared::SharedDefinitions;
use oo_bindgen::types::BasicType;

pub fn define(
    lib: &mut LibraryBuilder,
    shared: &SharedDefinitions,
) -> Result<ClassHandle, BindingError> {
    let request = lib.declare_class("Request")?;

    let request_new_fn = lib
        .define_function("request_new")
        .returns(request.clone(), "Handle to the created request")?
        .doc("Create a new request")?
        .build()?;

    let request_new_class_fn = lib
        .define_function("request_new_class")
        .param("class0", BasicType::Bool, "Ask for class 0 (static data)")?
        .param("class1", BasicType::Bool, "Ask for class 1 events")?
        .param("class2", BasicType::Bool, "Ask for class 2 events")?
        .param("class3", BasicType::Bool, "Ask for class 3 events")?
        .returns(
          request.clone(),
            "Handle to the created request",
        )?
        .doc(
            doc("Create a new request asking for classes")
            .details("An identical request can be created manually with {class:Request.AddAllObjectsHeader()} and variations {enum:Variation.Group60Var1}, {enum:Variation.Group60Var2}, {enum:Variation.Group60Var3} and {enum:Variation.Group60Var4}.")
        )?
        .build()?;

    let request_destroy_fn = lib
        .define_function("request_destroy")
        .param(
            "request",
          request.clone(),
            "Request to destroy",
        )?
        .returns_nothing()?
        .doc("Destroy a request created with {class:Request.[constructor]} or {class:Request.ClassRequest()}.")?
        .build()?;

    let request_add_one_byte_header_fn = lib
        .define_function("request_add_one_byte_header")
        .param("request", request.clone(), "Request to modify")?
        .param(
            "variation",
            shared.variation_enum.clone(),
            "Variation to ask for",
        )?
        .param("start", BasicType::U8, "Start index to ask")?
        .param("stop", BasicType::U8, "Stop index to ask (inclusive)")?
        .returns_nothing()?
        .doc("Add a one-byte start/stop variation interrogation")?
        .build()?;

    let request_add_two_byte_header_fn = lib
        .define_function("request_add_two_byte_header")
        .param("request", request.clone(), "Request to modify")?
        .param(
            "variation",
            shared.variation_enum.clone(),
            "Variation to ask for",
        )?
        .param("start", BasicType::U16, "Start index to ask")?
        .param("stop", BasicType::U16, "Stop index to ask (inclusive)")?
        .returns_nothing()?
        .doc("Add a two-byte start/stop variation interrogation")?
        .build()?;

    let request_add_all_objects_header_fn = lib
        .define_function("request_add_all_objects_header")
        .param("request", request.clone(), "Request to modify")?
        .param(
            "variation",
            shared.variation_enum.clone(),
            "Variation to ask for",
        )?
        .returns_nothing()?
        .doc("Add an all objects variation interrogation")?
        .build()?;

    let request = lib
        .define_class(&request)?
        .constructor(&request_new_fn)?
        .destructor(&request_destroy_fn)?
        .static_method("ClassRequest", &request_new_class_fn)?
        .method("AddOneByteHeader", &request_add_one_byte_header_fn)?
        .method("AddTwoByteHeader", &request_add_two_byte_header_fn)?
        .method("AddAllObjectsHeader", &request_add_all_objects_header_fn)?
        .doc(
            doc("Custom request")
            .details("Whenever a method takes a request as a parameter, the request is internally copied. Therefore, it is possible to reuse the same requests over and over.")
        )?
        .build()?;

    Ok(request)
}
