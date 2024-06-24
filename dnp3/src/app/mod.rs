pub use app_enums::*;
pub use buffer_size::*;
pub use file::*;
pub use header::*;
pub use listener::*;
pub use maybe_async::MaybeAsync;
pub use parse_error::*;
pub use retry::*;
pub use sequence::*;
pub use shutdown::*;
pub use timeout::*;
pub use types::*;
pub use variations::Variation;

/// Types used for making binary and analog output control requests
pub mod control {
    pub use super::control_enums::*;
    pub use super::control_types::ControlCode;
    pub use super::variations::{Group12Var1, Group41Var1, Group41Var2, Group41Var3, Group41Var4};
}

/// internal enum used all over the place to specify master or outstation
#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum EndpointType {
    Master,
    Outstation,
}

impl EndpointType {
    pub(crate) fn dir_bit(&self) -> bool {
        *self == EndpointType::Master
    }
}

/// Types association with Device Attributes (Group 0)
pub mod attr;
mod control_types;
#[rustfmt::skip]
mod app_enums;
mod buffer_size;
mod control_enums;
mod extensions;

pub(crate) mod file;
mod header;
mod listener;
mod maybe_async;
/// Measurement types, e.g. Binary, Analog, Counter, etc
pub mod measurement;
pub(crate) mod parse;
mod retry;
mod sequence;
mod shutdown;
mod timeout;
mod types;

#[rustfmt::skip]
pub(crate) mod variations;

pub(crate) mod format;
mod parse_error;

#[rustfmt::skip]
pub(crate) mod gen {
    pub(crate) mod all;
    pub(crate) mod conversion;
    pub(crate) mod count;
    pub(crate) mod prefixed;
    pub(crate) mod ranged;
}
