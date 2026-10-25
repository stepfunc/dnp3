// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use std::net::SocketAddr;

/// Source or destination at the physical layer from which a link frame was read/written
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum PhysAddr {
    /// The default type used for everything other than UDP
    None,
    /// Frames carried over UDP come to/from a SocketAddr
    Udp(SocketAddr),
}
