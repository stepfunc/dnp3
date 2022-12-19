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

    let request = lib
        .define_class(&request)?
        .constructor(constructor)?
        .destructor(destructor)?
        .doc(doc(
            "Define a custom request to WRITE analog input dead-bands",
        ))?
        .build()?;

    Ok(request)
}
