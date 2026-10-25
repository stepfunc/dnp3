// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::phys::PhysLayer;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpStream;

pub(crate) enum PostConnectionHandler {
    Tcp,
    #[cfg(feature = "enable-tls")]
    Tls(crate::tcp::tls::TlsClientConfig),
}

impl PostConnectionHandler {
    pub(crate) async fn post_connect(
        &self,
        socket: TcpStream,
        _endpoint: &SocketAddr,
    ) -> Option<PhysLayer> {
        match self {
            Self::Tcp => Some(PhysLayer::tcp(socket)),
            #[cfg(feature = "enable-tls")]
            Self::Tls(config) => config.handle_connection(socket, _endpoint).await.ok(),
        }
    }
}

/// Options that control how TCP connections are established
#[derive(Copy, Clone, Debug, Default)]
pub struct ConnectOptions {
    pub(crate) local_endpoint: Option<SocketAddr>,
    pub(crate) timeout: Option<Duration>,
}

impl ConnectOptions {
    /// Set the local address to which the socket is bound. If not specified, then any available
    /// adapter may be used with an OS-assigned port.
    pub fn set_local_endpoint(&mut self, address: SocketAddr) {
        self.local_endpoint = Some(address);
    }

    /// Set a timeout for the TCP connection that might be less than the default for the OS
    pub fn set_connect_timeout(&mut self, timeout: Duration) {
        self.timeout = Some(timeout);
    }
}
