/// internal enum used all over the place to specify master or outstation
#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum EndpointType {
    Master,
    Outstation,
}

/// publicly exported enumerations defined by the standard
#[rustfmt::skip]
pub mod enums;
/// extension impls for generated types
mod extensions;
/// measurement flags (aka quality) and display implementations
pub mod flags;
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
pub mod types;
/// public variations
#[rustfmt::skip]
pub mod variations;

pub(crate) mod format;

#[rustfmt::skip]
pub(crate) mod gen {
    pub(crate) mod all;
    pub(crate) mod conversion;
    pub(crate) mod count;
    pub(crate) mod prefixed;
    pub(crate) mod ranged;
}
