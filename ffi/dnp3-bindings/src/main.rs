use std::path::Path;

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    let builder_settings = oo_bindgen::cli::BindingBuilderSettings {
        ffi_target_name: "dnp3-ffi",
        jni_target_name: "dnp3-ffi-java",
        ffi_name: "dnp3_ffi",
        ffi_path: Path::new("ffi/dnp3-ffi").into(),
        java_group_id: "io.stepfunc",
        destination_path: Path::new("ffi/bindings").into(),
        library: std::rc::Rc::new(dnp3_schema::build_lib().unwrap()),
    };

    oo_bindgen::cli::run(builder_settings);
}
