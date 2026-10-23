// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! The C library (`dnp3_ffi`) used by the C, C++ and .NET bindings. The C ABI lives in
//! `dnp3-ffi`; it's a separate rlib so this cdylib-only crate can be built with LTO.

extern crate dnp3_ffi_impl as _;
