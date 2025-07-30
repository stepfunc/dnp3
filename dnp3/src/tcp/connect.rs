use crate::util::phys::PhysLayer;
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
        &mut self,
        socket: TcpStream,
        _endpoint: &SocketAddr,
    ) -> Option<PhysLayer> {
        match self {
            Self::Tcp => Some(PhysLayer::Tcp(socket)),
            #[cfg(feature = "enable-tls")]
            Self::Tls(config) => config.handle_connection(socket, _endpoint).await,
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
