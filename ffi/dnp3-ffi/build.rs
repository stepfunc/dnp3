use std::env;
use std::io::Write;
use std::path::Path;
use std::process::exit;

fn write_tracing_ffi() {
    let mut file =
        std::fs::File::create(Path::new(&env::var_os("OUT_DIR").unwrap()).join("tracing.rs"))
            .unwrap();
    file.write_all(sfio_tracing_ffi::get_impl_file().as_bytes())
        .unwrap();
}

fn write_tokio_ffi() {
    let mut file =
        std::fs::File::create(Path::new(&env::var_os("OUT_DIR").unwrap()).join("runtime.rs"))
            .unwrap();
    file.write_all(sfio_tokio_ffi::get_impl_file().as_bytes())
        .unwrap();
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    write_tracing_ffi();
    write_tokio_ffi();

    match dnp3_schema::build_lib() {
        Ok(lib) => {
            oo_bindgen::backend::rust::generate_ffi(&lib).unwrap();
        }
        Err(err) => {
            eprintln!("DNP3 model error: {err}");
            exit(-1);
        }
    };
}
