pub use app_enums::*;
pub use bytes::*;
pub use header::*;
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

mod bytes;

mod control_types;
#[rustfmt::skip]
mod app_enums;
mod control_enums;
mod extensions;
mod header;
/// measurement types, e.g. Binary, Analog, Counter, etc
pub mod measurement;
/// application layer parser
pub(crate) mod parse;
mod retry;
mod sequence;
mod shutdown;
mod timeout;
mod types;

#[rustfmt::skip]
pub(crate) mod variations;

pub(crate) mod format;
/// errors associated with parsing the application layer
mod parse_error;

#[rustfmt::skip]
pub(crate) mod gen {
    pub(crate) mod all;
    pub(crate) mod conversion;
    pub(crate) mod count;
    pub(crate) mod prefixed;
    pub(crate) mod ranged;
}
