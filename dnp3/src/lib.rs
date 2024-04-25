#![doc = include_str!("../README.md")]
#![cfg_attr(test, allow(dead_code))]

#[cfg(test)]
#[macro_use]
extern crate assert_matches;

/// Current version of the library
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Application layer types shared by both the master and outstation APIs
pub mod app;
/// Types used to control decoding in the log
pub mod decode;
/// Types specific to the link-layer
pub mod link;
/// Types and traits specific to masters
pub mod master;
/// Types and traits specific to outstations
pub mod outstation;
/// Entry points and types for serial
#[cfg(feature = "serial")]
pub mod serial;
/// Entry points and types for TCP
pub mod tcp;
/// Entry points and types for UDP
pub mod udp;

pub(crate) mod transport;
pub(crate) mod util;
