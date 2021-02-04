/// errors associated with parsing the application layer
pub mod error;

/// bytes handling helpers
pub mod bytes;

/// Controls how transmitted and received ASDUs are logged
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DecodeLogLevel {
    /// Log nothing
    Nothing,
    /// Log the header-only
    Header,
    /// Log the header and the object headers
    ObjectHeaders,
    /// Log the header, the object headers, and the object values
    ObjectValues,
}

impl DecodeLogLevel {
    pub(crate) fn enabled(&self) -> bool {
        *self != Self::Nothing
    }
}

pub(crate) mod bit;
pub(crate) mod count;
pub(crate) mod parser;
pub(crate) mod prefix;
pub(crate) mod range;
pub(crate) mod traits;
