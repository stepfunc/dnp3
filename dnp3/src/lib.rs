//! [DNP3](http://www.dnp.org/) protocol using [Tokio](https://docs.rs/tokio) and `async/await`.
//!
//! # Features
//!
//! * Panic-free, zero-copy, zero-allocation parsing
//! * Focus on maximal correctness and compliance to the specification
//! * Automatic TCP connection management with configurable reconnect strategy
//! * Scalable performance using Tokio's multi-threaded executor
//! * Future and callback-based API modes
//!
//! # Master example
//!
//! ```no_run
//! use dnp3::prelude::master::*;
//!
//! use std::net::SocketAddr;
//! use std::str::FromStr;
//! use std::time::Duration;
//!
//! // example of using the master API asynchronously from within the Tokio runtime
//! #[tokio::main(flavor = "multi_thread")]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!
//!     // spawn the master onto another task
//!     let mut master = spawn_master_tcp_client(
//!         MasterConfiguration::new(
//!             EndpointAddress::from(1)?,
//!             DecodeLogLevel::ObjectValues,
//!             ReconnectStrategy::default(),
//!             Timeout::from_secs(1)?,
//!         ),
//!         EndpointList::single("127.0.0.1:20000".to_owned()),
//!         Listener::None,
//!     );
//!
//!     let mut association = master.add_association(EndpointAddress::from(1024)?, Configuration::default(), NullHandler::boxed()).await?;
//!     association.add_poll(
//!         EventClasses::all().to_classes().to_request(),
//!         Duration::from_secs(5),
//!     ).await;
//!
//!     // In a real application, use the handle to make requests. Measurement data
//!     // comes back via the handler specified when creating the association. See
//!     // the provided examples for more control.
//!     tokio::time::sleep(Duration::from_secs(60)).await;
//!     Ok(())
//! }
//! ```
//!

#![deny(
dead_code,
arithmetic_overflow,
invalid_type_param_default,
//missing_fragment_specifier,
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
    // intra_doc_link_resolution_failure, broken_intra_doc_links
    safe_packed_borrows,
    while_true,
    bare_trait_objects
)]
// TODO - remove before release
#![cfg_attr(test, allow(dead_code))]

#[cfg(test)]
#[macro_use]
extern crate assert_matches;

/// application layer types shared by both the master and outstation APIs
pub mod app;
/// entry points for creating and spawning async tasks
pub mod entry;
/// types, enums, and traits specific to masters
pub mod master;
/// types, enums, and traits specific to outstations
pub mod outstation;
/// preludes for master and outstation
pub mod prelude;

pub(crate) mod link;
#[cfg_attr(test, allow(dead_code))]
pub(crate) mod transport;
pub(crate) mod util;

pub(crate) mod tokio;
