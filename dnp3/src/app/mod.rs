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

/// publicly exported enumerations defined by the standard
#[rustfmt::skip]
mod enums;
/// extension impls for generated types
mod extensions;
/// measurement flags (aka quality) and display implementations
mod flags;
/// application layer header types
pub mod header;
/// measurement types, e.g. Binary, Analog, Counter, etc
pub mod measurement;
/// application layer parser
pub mod parse;
/// retry strategies
pub mod retry;
/// application layer sequence number
pub mod sequence;
/// types for handling timeouts
pub mod timeout;
/// types used in various other application layer objects
mod types;
/// public variations
#[rustfmt::skip]
pub mod variations;

pub use enums::*;
pub use flags::*;
pub use types::*;

pub(crate) mod format;

#[rustfmt::skip]
pub(crate) mod gen {
    pub(crate) mod all;
    pub(crate) mod conversion;
    pub(crate) mod count;
    pub(crate) mod prefixed;
    pub(crate) mod ranged;
}
