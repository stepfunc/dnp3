#![allow(clippy::missing_safety_doc, clippy::useless_conversion)]
#![allow(clippy::derive_partial_eq_without_eq)]
#![allow(dead_code)]

pub(crate) use crate::tracing::*;
/// these use statements allow the code in the FFI to not have to known the real locations
/// but instead just use crate::<name> when invoking an implementation
pub use command::*;
pub use decoding::*;
pub use handler::*;
pub use master::*;
pub use outstation::*;
pub use request::*;
pub use runtime::*;
pub(crate) use tcp::*;

mod command;
mod decoding;
mod handler;
mod master;
mod outstation;
mod request;
mod runtime;
mod tcp;
mod tracing;

pub mod ffi;

impl From<crate::TracingInitError> for std::os::raw::c_int {
    fn from(_: crate::TracingInitError) -> Self {
        crate::ffi::ParamError::LoggingAlreadyConfigured.into()
    }
}

lazy_static::lazy_static! {
    static ref VERSION: std::ffi::CString = std::ffi::CString::new(dnp3::VERSION).unwrap();
}

fn version() -> &'static std::ffi::CStr {
    &VERSION
}
