// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use std::path::Path;

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    let builder_settings = oo_bindgen::cli::BindingBuilderSettings {
        ffi_target_name: "dnp3-ffi-c",
        jni_target_name: "dnp3-ffi-java",
        ffi_name: "dnp3_ffi",
        ffi_path: Path::new("ffi/dnp3-ffi-c").into(),
        java_group_id: "io.stepfunc",
        destination_path: Path::new("ffi/bindings").into(),
        library: std::rc::Rc::new(dnp3_schema::build_lib().unwrap()),
    };

    oo_bindgen::cli::run(builder_settings);
}
