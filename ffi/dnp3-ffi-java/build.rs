use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=lib.rs");

    // normally you'd never want to write files here, but this crate isn't used as a dependency
    let out_path: PathBuf = Path::new(&std::env::var_os("OUT_DIR").unwrap()).join("jni.rs");

    let config = oo_bindgen::backend::java::JniBindgenConfig {
        group_id: "io.stepfunc",
        ffi_name: "dnp3_ffi",
    };

    match dnp3_schema::build_lib() {
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(-1);
        }
        Ok(lib) => {
            oo_bindgen::backend::java::generate_jni(&out_path, &lib, &config).unwrap();
        }
    }
}
