/// extension impls for generated types
mod extensions;
/// measurement flags (aka quality) and display implementations
pub mod flags;
pub(crate) mod format;
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
pub mod types;

#[rustfmt::skip]
pub mod gen {
    pub(crate) mod conversion;
    pub mod enums;
    pub mod variations {
        pub(crate) mod all;
        pub(crate) mod count;
        pub mod fixed;
        pub(crate) mod prefixed;
        pub(crate) mod ranged;
        pub mod variation;
    }
}
