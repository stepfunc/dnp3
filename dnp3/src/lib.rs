//! [DNP3](http://www.dnp.org/) protocol using [Tokio](https://docs.rs/tokio) and `async/await`.
//!
//! # Features
//!
//! * Panic-free, zero-copy, zero-allocation parsing
//! * Fully automated level 2 outstation conformance tests
//! * Automatic TCP connection management with configurable reconnect strategy
//! * Scalable performance using Tokio's multi-threaded executor
//!
//! # License
//!
//! This crate is made available under a non-commercial / non-production license.
//!
//! Please inquire about commercial licensing here:
//!
//! <https://stepfunc.io/contact/>

#![deny(
dead_code,
arithmetic_overflow,
invalid_type_param_default,
missing_fragment_specifier,
mutable_transmutes,
no_mangle_const_items,
overflowing_literals,
patterns_in_fns_without_body,
pub_use_of_private_extern_crate,
unknown_crate_types,
const_err,
order_dependent_trait_objects,
illegal_floating_point_literal_pattern,
improper_ctypes,
late_bound_lifetime_arguments,
non_camel_case_types,
non_shorthand_field_patterns,
non_snake_case,
non_upper_case_globals,
no_mangle_generic_items,
private_in_public,
stable_features,
type_alias_bounds,
tyvar_behind_raw_pointer,
unconditional_recursion,
unused_comparisons,
unreachable_pub,
anonymous_parameters,
missing_copy_implementations,
//missing_debug_implementations,
missing_docs,
trivial_casts,
trivial_numeric_casts,
unused_import_braces,
unused_qualifications,
clippy::all
)]
#![forbid(
    unsafe_code,
    rustdoc::broken_intra_doc_links,
    unaligned_references,
    while_true,
    bare_trait_objects
)]
#![cfg_attr(test, allow(dead_code))]

#[cfg(test)]
#[macro_use]
extern crate assert_matches;

/// Current version of the library
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// application layer types shared by both the master and outstation APIs
pub mod app;
/// types used to control decoding in the log
pub mod decode;
/// Types specific to the link-layer
pub mod link;
/// Types and traits specific to masters
pub mod master;
/// Types and traits specific to outstations
pub mod outstation;
/// Entry points and types for serial
pub mod serial;
/// Entry points and types for TCP
pub mod tcp;

pub(crate) mod tokio;
pub(crate) mod transport;
pub(crate) mod util;
