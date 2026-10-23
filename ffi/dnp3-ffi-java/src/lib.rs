// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

#![allow(
    clippy::unused_unit,
    clippy::useless_conversion,
    clippy::redundant_closure,
    clippy::needless_borrow,
    clippy::needless_lifetimes,
    clippy::needless_return,
    clippy::not_unsafe_ptr_arg_deref,
    clippy::let_unit_value,
    clippy::uninlined_format_args,
    unused_variables,
    dead_code,
    static_mut_refs
)]
// ^ these lints don't matter in the generated code

include!(concat!(env!("OUT_DIR"), "/jni.rs"));
