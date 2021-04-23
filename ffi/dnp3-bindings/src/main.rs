use std::path::Path;

fn main() {
    let builder_settings = ci_script::BindingBuilderSettings {
        ffi_name: "dnp3_ffi",
        ffi_path: Path::new("ffi/dnp3-ffi"),
        java_group_id: "io.stepfunc",
        destination_path: Path::new("ffi/bindings"),
        library: &dnp3_schema::build_lib().unwrap(),
    };

    ci_script::run(builder_settings);
}
