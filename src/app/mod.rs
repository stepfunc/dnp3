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
/// application layer sequence number
pub mod sequence;
/// types for handling timeouts
pub mod timeout;
/// types used in various other application layer objects
pub mod types;

/// generated implementations of enums and variations
#[rustfmt::skip]
pub mod gen {
    /// generated protocol-defined enumerations
    pub mod enums;
    pub mod variations {
        /// publicly exported fixed-size variations
        pub mod fixed;
        /// enumeration of all variations
        pub mod variation;

        pub(crate) mod all;
        pub(crate) mod count;
        pub(crate) mod prefixed;
        pub(crate) mod ranged;
    }
    pub(crate) mod conversion;
}

pub(crate) mod format;
