// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

mod address_filter;
mod client;
mod server;

pub use address_filter::*;
pub use client::*;
pub use server::*;

/// wraps a session so that it can switch communication sessions
pub(crate) mod server_task;
