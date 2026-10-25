// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

#![doc = include_str!("../README.md")]
#![cfg_attr(
    test,
    expect(dead_code, reason = "some crate items are only used outside of tests")
)]
// deny explicit panic constructs, indexing, unchecked arithmetic and as conversions; clippy.toml
// allows unwrap, expect, panic and indexing in test code, and test modules that need arithmetic
// say so
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::panic_in_result_fn,
    clippy::unwrap_in_result,
    clippy::string_slice,
    clippy::exit,
    clippy::mem_forget,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::dbg_macro,
    clippy::infinite_loop,
    clippy::large_futures,
    clippy::large_stack_arrays,
    clippy::iter_over_hash_type,
    clippy::float_cmp,
    clippy::lossy_float_literal,
    clippy::assertions_on_result_states,
    clippy::let_underscore_untyped,
    clippy::let_underscore_future,
    let_underscore_drop,
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    missing_debug_implementations
)]

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

pub(crate) mod phys;
pub(crate) mod session;
pub(crate) mod transport;
pub(crate) mod util;

#[cfg(all(test, dnp3_no_panic))]
mod no_panic_check;
