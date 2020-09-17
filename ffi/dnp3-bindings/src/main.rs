use std::path::Path;

fn main() {
    let builder_settings = ci_script::BindingBuilderSettings {
        ffi_name: "dnp3_ffi",
        destination_path: Path::new("ffi/bindings"),
        library: &dnp3_schema::build_lib().unwrap(),
    };

    ci_script::run(builder_settings);
}
