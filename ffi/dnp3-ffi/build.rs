use std::process::exit;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

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
