pub use bytes::*;
pub use enums::*;
pub use header::*;
pub use retry::*;
pub use sequence::*;
pub use timeout::*;
pub use types::*;
pub use variations::Variation;

/// Types used for making binary and analog output control requests
pub mod control {
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
/// publicly exported enumerations defined by the standard
#[rustfmt::skip]
mod enums;
/// extension impls for generated types
mod extensions;
/// application layer header types
mod header;
/// measurement types, e.g. Binary, Analog, Counter, etc
pub mod measurement;
/// application layer parser
pub(crate) mod parse;
/// retry strategies
mod retry;
/// application layer sequence number
mod sequence;
/// types for handling timeouts
mod timeout;
/// types used in various other application layer objects
mod types;
/// public variations
#[rustfmt::skip]
pub(crate) mod variations;

pub(crate) mod format;
/// errors associated with parsing the application layer
mod parse_error;

pub use parse_error::*;

#[rustfmt::skip]
pub(crate) mod gen {
    pub(crate) mod all;
    pub(crate) mod conversion;
    pub(crate) mod count;
    pub(crate) mod prefixed;
    pub(crate) mod ranged;
}
