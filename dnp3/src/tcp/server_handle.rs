// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use std::net::SocketAddr;

/// Handle to a running TCP or TLS server. Dropping the handle shuts down the server.
pub struct ServerHandle {
    pub(crate) addr: Option<SocketAddr>,
    pub(crate) signal: crate::util::shutdown::ShutdownSignal,
}

impl std::fmt::Debug for ServerHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerHandle")
            .field("addr", &self.addr)
            .finish_non_exhaustive()
    }
}

impl ServerHandle {
    /// Returns the local address to which this server is bound.
    ///
    /// This can be useful, for example, when binding to port 0 to figure out which port was actually bound.
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.addr
    }

    /// Stop accepting connections and shut down the server
    ///
    /// The server stops when its accept loop next observes the request. Calling it again, or
    /// after the server has stopped, does nothing. Dropping the handle has the same effect.
    pub fn shutdown(&self) {
        self.signal.shutdown();
    }
}
