// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

pub(crate) mod layer;
mod master;
mod outstation;
pub(crate) mod task;

#[cfg(test)]
mod lifecycle_tests;

pub use master::*;
pub use outstation::*;

/// Describes how the UDP socket reads and writes datagrams from remote endpoint(s)
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UdpSocketMode {
    /// The UDP endpoint will only communicate with the specified remote endpoint
    OneToOne,
    /// The UDP endpoint will accept packets any remote endpoint.
    ///
    /// When this mode is used with an outstation, the outstation will respond to the address from which the request was sent.
    /// It will use supplied remote endpoint only for sending unsolicited responses.
    OneToMany,
}
