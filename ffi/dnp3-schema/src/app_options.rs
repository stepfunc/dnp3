use oo_bindgen::model::Primitive::Bool;
use oo_bindgen::model::{doc, BackTraced, LibraryBuilder};

pub(crate) fn define(lib: &mut LibraryBuilder) -> BackTraced<()> {
    let parse_zero_length_strings = lib
        .define_function("parse_zero_length_strings")?
        .param("enabled", Bool, "If true, enable the parsing of zero length strings")?
        .doc(
            doc("Enable or disable the parsing of zero-length strings, e.g., Group 110/111 Variation 0")
                .details("This is disabled by default for security reasons as enabling it can allow resource exhaustion attacks.")             
        )?
        .build_static("parse_zero_length_strings")?;

    lib.define_static_class("app_layer_options")?
        .static_method(parse_zero_length_strings)?
        .doc("Global application layer configuration settings")?
        .build()?;

    Ok(())
}
