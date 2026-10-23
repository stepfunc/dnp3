// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use oo_bindgen::model::Primitive::Bool;
use oo_bindgen::model::{doc, BackTraced, LibraryBuilder};

pub(crate) fn define(lib: &mut LibraryBuilder) -> BackTraced<()> {
    let parse_zero_length_strings = lib
        .define_function("parse_zero_length_strings")?
        .param("enabled", Bool, "If true, enable the parsing of zero length strings")?
        .doc(
            doc("Enable or disable the parsing of zero-length strings, e.g., Group 110/111 Variation 0")
                .details("This is disabled by default for security reasons as enabling it can allow resource exhaustion attacks.")
                .details("This global option is a work-around to preserve API compatability until a future 2.0 release")
        )?
        .build_static("parse_zero_length_strings")?;

    lib.define_static_class("app_layer_options")?
        .static_method(parse_zero_length_strings)?
        .doc("Global application layer configuration settings")?
        .build()?;

    Ok(())
}
