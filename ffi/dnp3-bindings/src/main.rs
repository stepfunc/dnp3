use std::path::Path;

fn main() {
    let builder_settings = ci_script::BindingBuilderSettings {
        ffi_target_name: "dnp3-ffi",
        ffi_name: "dnp3_ffi",
        ffi_path: Path::new("ffi/dnp3-ffi").into(),
        java_group_id: "io.stepfunc",
        destination_path: Path::new("ffi/bindings").into(),
        library: std::rc::Rc::new(dnp3_schema::build_lib().unwrap()),
    };

    ci_script::run(builder_settings);
}
