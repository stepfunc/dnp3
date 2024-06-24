use oo_bindgen::model::*;

const NOTHING: &str = "nothing";

pub(crate) struct DecodeLevels {
    pub(crate) app: EnumHandle,
    pub(crate) transport: EnumHandle,
    pub(crate) link: EnumHandle,
    pub(crate) phys: EnumHandle,
}

pub fn define_levels(lib: &mut LibraryBuilder) -> BackTraced<DecodeLevels> {
    let app_decode_level_enum = lib
        .define_enum("app_decode_level")?
        .push(NOTHING, "Decode nothing")?
        .push("header", " Decode the header-only")?
        .push("object_headers", "Decode the header and the object headers")?
        .push(
            "object_values",
            "Decode the header, the object headers, and the object values",
        )?
        .doc("Controls how transmitted and received application-layer fragments are decoded at the INFO log level")?
        .build()?;

    let transport_decode_level_enum = lib
        .define_enum("transport_decode_level")?
        .push(NOTHING, "Decode nothing")?
        .push("header", " Decode the header")?
        .push("payload", "Decode the header and the raw payload as hexadecimal")?
        .doc("Controls how transmitted and received transport segments are decoded at the INFO log level")?
        .build()?;

    let link_decode_level_enum = lib
        .define_enum("link_decode_level")?
        .push(NOTHING, "Decode nothing")?
        .push("header", " Decode the header")?
        .push(
            "payload",
            "Decode the header and the raw payload as hexadecimal",
        )?
        .doc("Controls how transmitted and received link frames are decoded at the INFO log level")?
        .build()?;

    let phys_decode_level_enum = lib
        .define_enum("phys_decode_level")?
        .push(NOTHING, "Log nothing")?
        .push(
            "length",
            "Log only the length of data that is sent and received",
        )?
        .push(
            "data",
            "Log the length and the actual data that is sent and received",
        )?
        .doc("Controls how data transmitted at the physical layer (TCP, serial, etc) is logged")?
        .build()?;

    Ok(DecodeLevels {
        app: app_decode_level_enum,
        transport: transport_decode_level_enum,
        link: link_decode_level_enum,
        phys: phys_decode_level_enum,
    })
}

pub fn define_decode_level_struct(
    lib: &mut LibraryBuilder,
    levels: &DecodeLevels,
) -> BackTraced<UniversalStructHandle> {
    let application_field = Name::create("application")?;
    let transport_field = Name::create("transport")?;
    let link_field = Name::create("link")?;
    let physical_field = Name::create("physical")?;

    let decode_level_struct = lib.declare_universal_struct("decode_level")?;
    let decode_level_struct = lib.define_universal_struct(decode_level_struct)?
        .add(&application_field, levels.app.clone(), "Controls application fragment decoding")?
        .add(&transport_field, levels.transport.clone(), "Controls transport segment layer decoding")?
        .add(&link_field, levels.link.clone(), "Controls link frame decoding")?
        .add(&physical_field, levels.phys.clone(), "Controls the logging of physical layer read/write")?
        .doc("Controls the decoding of transmitted and received data at the application, transport, link, and physical layers")?
        .end_fields()?
        .begin_initializer("init", InitializerType::Normal, "Initialize log levels to defaults")?
        .default_variant(&application_field, NOTHING)?
        .default_variant(&transport_field, NOTHING)?
        .default_variant(&link_field, NOTHING)?
        .default_variant(&physical_field, NOTHING)?
        .end_initializer()?
        .begin_initializer("nothing", InitializerType::Static, "Initialize log levels to nothing")?
        .default_variant(&application_field, NOTHING)?
        .default_variant(&transport_field, NOTHING)?
        .default_variant(&link_field, NOTHING)?
        .default_variant(&physical_field, NOTHING)?
        .end_initializer()?
        .build()?;

    Ok(decode_level_struct)
}
