// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Sans-I/O outstation protocol core and thin Tokio driver.
//!
//! [`OutstationCore`] owns all application-layer protocol state, response buffers,
//! handler callbacks, and its [`MonotonicClock`](crate::util::clock::MonotonicClock).
//! [`OutstationTask`](crate::outstation::session::task::OutstationTask) is a thin Tokio driver
//! that wires it to a [`Transport`](crate::transport::Transport),
//! [`PhysLayer`](crate::phys::PhysLayer), and
//! [`DatabaseHandle`](crate::outstation::database::DatabaseHandle).

mod broadcast;
mod context;
mod control;
mod core;
pub(crate) mod deferred;
mod non_read;
mod phase;
mod request;
mod response;
mod state;
pub(crate) mod task;
#[cfg(test)]
mod tests;
mod unsolicited;
mod write;

use self::core::{Outbound, Output, OutstationCore};
