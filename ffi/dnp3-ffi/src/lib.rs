#![allow(clippy::missing_safety_doc, clippy::default_constructed_unit_structs)]

pub(crate) use crate::tracing::*;
/// these use statements allow the code in the FFI to not have to known the real locations
/// but instead just use `crate::<name>` when invoking an implementation
pub use bytes::*;
pub use command::*;
pub use connect::*;
pub use handler::*;
pub use master::*;
pub use outstation::*;
pub use request::*;
pub use runtime::*;
pub(crate) use tcp::*;
pub use write_dead_band_request::*;

pub(crate) mod attr;

mod bytes;
mod command;
mod connect;
mod decoding;
mod handler;
mod master;
mod outstation;
mod request;
mod runtime;
mod tcp;
mod tracing;
mod write_dead_band_request;

#[allow(
    dead_code,
    clippy::derive_partial_eq_without_eq,
    clippy::useless_conversion,
    clippy::extra_unused_lifetimes
)]
pub mod ffi;

lazy_static::lazy_static! {
    static ref VERSION: std::ffi::CString = std::ffi::CString::new(dnp3::VERSION).unwrap();
}

fn version() -> &'static std::ffi::CStr {
    &VERSION
}

impl From<crate::TracingInitError> for std::os::raw::c_int {
    fn from(_: crate::TracingInitError) -> Self {
        crate::ffi::ParamError::LoggingAlreadyConfigured.into()
    }
}

impl From<crate::runtime::RuntimeError> for std::os::raw::c_int {
    fn from(err: crate::runtime::RuntimeError) -> Self {
        let err: crate::ffi::ParamError = err.into();
        err.into()
    }
}

impl From<dnp3::app::Shutdown> for crate::ffi::ParamError {
    fn from(_: dnp3::app::Shutdown) -> Self {
        crate::ffi::ParamError::MasterAlreadyShutdown
    }
}

impl From<RuntimeError> for crate::ffi::ParamError {
    fn from(err: crate::runtime::RuntimeError) -> Self {
        match err {
            crate::runtime::RuntimeError::RuntimeDestroyed => {
                crate::ffi::ParamError::RuntimeDestroyed
            }
            crate::runtime::RuntimeError::CannotBlockWithinAsync => {
                crate::ffi::ParamError::RuntimeCannotBlockWithinAsync
            }
            crate::runtime::RuntimeError::FailedToCreateRuntime => {
                crate::ffi::ParamError::RuntimeCreationFailure
            }
        }
    }
}
