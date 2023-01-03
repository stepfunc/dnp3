use oo_bindgen::model::*;

pub(crate) fn define(lib: &mut LibraryBuilder) -> BackTraced<ClassHandle> {
    let request = lib.declare_class("write_dead_band_request")?;

    let constructor = lib
        .define_constructor(request.clone())?
        .doc("A builder class to create one or more headers of analog input dead-bands")?
        .build()?;

    let destructor = lib.define_destructor(
        request.clone(),
        "Destroy a request created with {class:write_dead_band_request.[constructor]}",
    )?;

    let add_g34v1_u8 = define_add_method(lib, request.clone(), Primitive::U8, Primitive::U16)?;
    let add_g34v2_u8 = define_add_method(lib, request.clone(), Primitive::U8, Primitive::U32)?;
    let add_g34v3_u8 = define_add_method(lib, request.clone(), Primitive::U8, Primitive::Float)?;
    let add_g34v1_u16 = define_add_method(lib, request.clone(), Primitive::U16, Primitive::U16)?;
    let add_g34v2_u16 = define_add_method(lib, request.clone(), Primitive::U16, Primitive::U32)?;
    let add_g34v3_u16 = define_add_method(lib, request.clone(), Primitive::U16, Primitive::Float)?;

    let finish_header = lib
        .define_method("finish_header", request.clone())?
        .doc(
            doc("If a header is currently being written, then this will complete the header so that no new objects may be added to it")
                .details("This happens automatically if you change the type or index when adding dead-band values. This method allows you to fragment the same type across multiple object headers.")
        )?
        .build()?;

    let request = lib
        .define_class(&request)?
        .constructor(constructor)?
        .destructor(destructor)?
        .method(add_g34v1_u8)?
        .method(add_g34v2_u8)?
        .method(add_g34v3_u8)?
        .method(add_g34v1_u16)?
        .method(add_g34v2_u16)?
        .method(add_g34v3_u16)?
        .method(finish_header)?
        .doc(doc(
            "Define a custom request to WRITE analog input dead-bands",
        ))?
        .build()?;

    Ok(request)
}

fn define_add_method(
    lib: &mut LibraryBuilder,
    request: ClassDeclarationHandle,
    index_type: Primitive,
    dead_band_type: Primitive,
) -> BackTraced<Method<Unvalidated>> {
    let variation = match dead_band_type {
        Primitive::U16 => "g34v1",
        Primitive::U32 => "g34v2",
        Primitive::Float => "g34v3",
        _ => unimplemented!(),
    };

    let var_doc = match dead_band_type {
        Primitive::U16 => "unsigned 16-bit",
        Primitive::U32 => "unsigned 32-bit",
        Primitive::Float => "single-precision floating point",
        _ => unimplemented!(),
    };

    let index = match index_type {
        Primitive::U8 => "u8",
        Primitive::U16 => "u16",
        _ => unimplemented!(),
    };

    let num_index_bits = match index_type {
        Primitive::U8 => "8",
        Primitive::U16 => "16",
        _ => unimplemented!(),
    };

    let desc = format!("Add a {variation} ({var_doc}) dead-band with {num_index_bits}-bit indexing  to the request");

    let method = lib
        .define_method(format!("add_{variation}_{index}"), request)?
        .doc(
            doc(desc)
                .details("If this variation and index are the same as the current header, then it will be added to it. Otherwise, this call we create a new header of this type.d")
        )?
        .param(
            "index",
            index_type,
            "Index of the analog input to which the dead-band applies",
        )?
        .param("dead_band", dead_band_type, "Value of the dead-band")?
        .build()?;

    Ok(method)
}
