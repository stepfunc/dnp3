//! [DNP3](http://www.dnp.org/) protocol using [Tokio](https://docs.rs/tokio) and `async/await`.
//!
//! # Features
//!
//! * Panic-free, zero-copy, zero-allocation parsing
//! * Focus on maximal correctness and compliance to the specification
//! * Automatic TCP connection management with configurable reconnect strategy
//! * Scalable performance using Tokio's multi-threaded executor
//! * Future and callback-based API modes

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
// missing_debug_implementations,
// missing_docs,
trivial_casts,
trivial_numeric_casts,
unused_import_braces,
unused_qualifications,
clippy::all
)]
#![forbid(
    unsafe_code,
    intra_doc_link_resolution_failure,
    safe_packed_borrows,
    while_true,
    bare_trait_objects
)]

#[cfg(test)]
extern crate tokio_test;

#[cfg(test)]
#[macro_use]
extern crate assert_matches;

/// application layer
pub mod app;
/// master API
pub mod master;

pub(crate) mod link;
#[cfg_attr(test, allow(dead_code))]
pub(crate) mod transport;
pub(crate) mod util;
