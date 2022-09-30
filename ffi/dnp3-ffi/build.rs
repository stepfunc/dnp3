use std::env;
use std::io::Write;
use std::path::Path;
use std::process::exit;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let mut file =
        std::fs::File::create(Path::new(&env::var_os("OUT_DIR").unwrap()).join("tracing.rs"))
            .unwrap();
    file.write_all(tracing_ffi_schema::get_impl_file().as_bytes())
        .unwrap();

    match dnp3_schema::build_lib() {
        Ok(lib) => {
            rust_oo_bindgen::RustCodegen::new(&lib).generate().unwrap();
        }
        Err(err) => {
            eprintln!("DNP3 model error: {}", err);
            exit(-1);
        }
    };
}
