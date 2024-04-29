use std::net::SocketAddr;

/// Handle to a running TCP or TLS server. Dropping the handle shuts down the server.
pub struct ServerHandle {
    pub(crate) addr: Option<SocketAddr>,
    pub(crate) _token: crate::util::shutdown::ShutdownToken,
}

impl ServerHandle {
    /// Returns the local address to which this server is bound.
    ///
    /// This can be useful, for example, when binding to port 0 to figure out which port was actually bound.
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.addr
    }
}
