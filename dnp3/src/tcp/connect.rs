use crate::app::{ExponentialBackOff, RetryStrategy};
use crate::tcp::EndpointList;
use crate::util::phys::PhysLayer;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::{TcpSocket, TcpStream};

pub(crate) enum PostConnectionHandler {
    Tcp,
    #[cfg(feature = "tls")]
    Tls(crate::tcp::tls::TlsClientConfig),
}

impl PostConnectionHandler {
    async fn post_connect(
        &mut self,
        socket: TcpStream,
        _endpoint: &SocketAddr,
    ) -> Option<PhysLayer> {
        match self {
            Self::Tcp => Some(PhysLayer::Tcp(socket)),
            #[cfg(feature = "tls")]
            Self::Tls(config) => config.handle_connection(socket, _endpoint).await,
        }
    }
}

/// Options that control how TCP connections are established
#[derive(Copy, Clone, Debug, Default)]
pub struct ConnectOptions {
    pub(crate) local_endpoint: Option<SocketAddr>,
    pub(crate) _timeout: Option<SocketAddr>,
}

impl ConnectOptions {
    /// Set the local address to which the socket is bound. If not specified, then any available
    /// ethernet adapter may be used with an OS assigned port.
    pub fn set_local_endpoint(&mut self, address: SocketAddr) {
        self.local_endpoint = Some(address);
    }
}

/// All of the state required to establish a TCP or TLS connection including the retry logic
pub(crate) struct Connector {
    endpoints: EndpointList,
    options: ConnectOptions,
    back_off: ExponentialBackOff,
    post_connect: PostConnectionHandler,
}

impl Connector {
    pub(crate) fn new(
        endpoints: EndpointList,
        options: ConnectOptions,
        retry: RetryStrategy,
        post_connect: PostConnectionHandler,
    ) -> Self {
        Self {
            endpoints,
            options,
            back_off: ExponentialBackOff::new(retry),
            post_connect,
        }
    }

    /// Attempt a single connection to the next address in the list
    pub(crate) async fn connect(&mut self) -> Result<PhysLayer, Duration> {
        let addr = match self.endpoints.next_address().await {
            Some(x) => x,
            None => {
                let delay = self.back_off.on_failure();
                tracing::warn!("name resolution failure");
                return Err(delay);
            }
        };

        let result = if addr.is_ipv4() {
            TcpSocket::new_v4()
        } else {
            TcpSocket::new_v6()
        };

        let socket = match result {
            Ok(x) => x,
            Err(err) => {
                let delay = self.back_off.on_failure();
                tracing::warn!("unable to create socket: {}", err);
                return Err(delay);
            }
        };

        if let Some(local) = self.options.local_endpoint {
            if let Err(err) = socket.bind(local) {
                let delay = self.back_off.on_failure();
                tracing::warn!("unable to bind socket to {}: {}", local, err);
                return Err(delay);
            }
        }

        let stream = match socket.connect(addr).await {
            Ok(x) => x,
            Err(err) => {
                let delay = self.back_off.on_failure();
                tracing::warn!("failed to connect to {}: {}", addr, err);
                return Err(delay);
            }
        };

        crate::tcp::configure_client(&stream);

        let phys = match self.post_connect.post_connect(stream, &addr).await {
            Some(x) => x,
            None => {
                let delay = self.back_off.on_failure();
                return Err(delay);
            }
        };

        tracing::info!("connected to {}", addr);
        self.endpoints.reset();
        self.back_off.on_success();

        Ok(phys)
    }
}
