#![allow(clippy::clippy::missing_safety_doc, clippy::useless_conversion)]

/// these use statements allow the code in the FFI to not have to known the real locations
/// but instead just use crate::<name> when invoking an implementation
pub use command::*;
pub use handler::*;
pub use logging::*;
pub use master::*;
pub use outstation::*;
pub use request::*;
pub use runtime::*;

mod command;
mod handler;
mod logging;
mod master;
mod outstation;
mod request;
mod runtime;

pub mod ffi;
