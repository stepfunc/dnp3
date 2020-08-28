fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let lib = dnp3_schema::build_lib().unwrap();
    rust_oo_bindgen::RustCodegen::new(&lib).generate().unwrap();
}
