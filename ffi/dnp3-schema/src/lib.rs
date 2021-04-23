use std::path::PathBuf;

use oo_bindgen::*;

mod constants;
mod database;
mod handler;
mod logging;
mod master;
mod outstation;
mod request;
mod runtime;
mod shared;
mod variation;

pub fn build_lib() -> Result<Library, BindingError> {
    let info = LibraryInfo {
        description: "Safe and fast DNP3 library".to_string(),
        project_url: "https://stepfunc.io/products/libraries/dnp3/".to_string(),
        repository: "stepfunc/dnp3".to_string(),
        license_name: "Custom license".to_string(),
        license_description: [
            "This library is provided under the terms of a non-commercial license.",
            "",
            "Please refer to the source repository for details:",
            "",
            "https://github.com/stepfunc/dnp3/blob/master/LICENSE.txt",
            "",
            "Please contact Step Function I/O if you are interested in commercial license:",
            "",
            "info@stepfunc.io",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect(),
        license_path: PathBuf::from("LICENSE.txt"),
        developers: vec![
            DeveloperInfo {
                name: "J. Adam Crain".to_string(),
                email: "adam@stepfunc.io".to_string(),
                organization: "Step Function I/O".to_string(),
                organization_url: "https://stepfunc.io/".to_string(),
            },
            DeveloperInfo {
                name: "Émile Grégoire".to_string(),
                email: "emile@stepfunc.io".to_string(),
                organization: "Step Function I/O".to_string(),
                organization_url: "https://stepfunc.io/".to_string(),
            },
        ],
    };
    let mut builder = LibraryBuilder::new("dnp3", Version::parse(dnp3::VERSION).unwrap(), info);
    builder.c_ffi_prefix("dnp3")?;

    // Shared stuff
    let shared_def = shared::define(&mut builder)?;
    // master and outstation APIs
    master::define(&mut builder, &shared_def)?;
    outstation::define(&mut builder, &shared_def)?;

    builder.build()
}
