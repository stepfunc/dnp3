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
    let mut builder = LibraryBuilder::new("dnp3rs", Version::parse(dnp3::VERSION).unwrap());
    builder.c_ffi_prefix("dnp3")?;
    builder.description("Safe and fast DNP3 library")?;
    builder.license(
        [
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
    )?;

    // Shared stuff
    let shared_def = shared::define(&mut builder)?;
    // master and outstation APIs
    master::define(&mut builder, &shared_def)?;
    outstation::define(&mut builder, &shared_def)?;

    builder.build()
}
