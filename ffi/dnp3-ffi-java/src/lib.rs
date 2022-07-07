#![allow(
    clippy::unused_unit,
    clippy::useless_conversion,
    clippy::redundant_closure,
    clippy::needless_borrow,
    clippy::needless_return,
    clippy::not_unsafe_ptr_arg_deref,
    clippy::let_unit_value,
    unused_variables,
    dead_code
)]
// ^ these lints don't matter in the generated code

include!(concat!(env!("OUT_DIR"), "/jni.rs"));
