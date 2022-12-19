use oo_bindgen::model::*;

pub fn define(lib: &mut LibraryBuilder) -> BackTraced<ClassHandle> {
    let request = lib.declare_class("write_dead_band_request")?;

    let constructor = lib
        .define_constructor(request.clone())?
        .doc("Create a new request to write analog input dead-bands")?
        .build()?;

    let destructor = lib.define_destructor(
        request.clone(),
        "Destroy a request created with {class:write_dead_band_request.[constructor]}",
    )?;

    let add_g32v1_u8 = lib
        .define_method("add_g32v1_u8", request.clone())?
        .doc("Add a GV34Var1 16-bit dead-band with 8-bit indexing  to the request")?
        .param(
            "index",
            Primitive::U8,
            "Index of the analog input to which the dead-band applies",
        )?
        .param("dead_band", Primitive::U16, "Value of the dead-band")?
        .build()?;

    let request = lib
        .define_class(&request)?
        .constructor(constructor)?
        .destructor(destructor)?
        .method(add_g32v1_u8)?
        .doc(doc(
            "Define a custom request to WRITE analog input dead-bands",
        ))?
        .build()?;

    Ok(request)
}
