use crate::decode::PhysDecodeLevel;
use std::io::ErrorKind;
use std::net::SocketAddr;

use crate::udp::layer::UdpLayer;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Source or destination at the physical layer from which a link frame was read/written
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum PhysAddr {
    /// The default type used for everything other than UDP
    None,
    /// Frames carried over UDP come to/from a SocketAddr
    Udp(SocketAddr),
}

/// encapsulates all possible physical layers as an enum
pub(crate) enum PhysLayer {
    Tcp(tokio::net::TcpStream),
    Udp(UdpLayer),
    /// TLS type is boxed because its size is huge
    #[cfg(feature = "enable-tls")]
    Tls(Box<tokio_rustls::TlsStream<tokio::net::TcpStream>>),
    #[cfg(feature = "serial")]
    Serial(tokio_serial::SerialStream),
    #[cfg(test)]
    Mock(sfio_tokio_mock_io::Mock),
}

impl std::fmt::Debug for PhysLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PhysLayer::Tcp(_) => f.write_str("Tcp"),
            PhysLayer::Udp(_) => f.write_str("Udp"),
            #[cfg(feature = "enable-tls")]
            PhysLayer::Tls(_) => f.write_str("Tls"),
            #[cfg(feature = "serial")]
            PhysLayer::Serial(_) => f.write_str("Serial"),
            #[cfg(test)]
            PhysLayer::Mock(_) => f.write_str("Mock"),
        }
    }
}

impl PhysLayer {
    pub(crate) async fn read(
        &mut self,
        buffer: &mut [u8],
        level: PhysDecodeLevel,
    ) -> Result<(usize, PhysAddr), std::io::Error> {
        let (length, addr) = match self {
            Self::Tcp(x) => {
                let count = x.read(buffer).await?;
                (count, PhysAddr::None)
            }
            Self::Udp(x) => x.read(buffer).await?,
            #[cfg(feature = "enable-tls")]
            Self::Tls(x) => {
                let count = x.read(buffer).await?;
                (count, PhysAddr::None)
            }
            #[cfg(feature = "serial")]
            Self::Serial(x) => {
                let count = x.read(buffer).await?;
                (count, PhysAddr::None)
            }
            #[cfg(test)]
            Self::Mock(x) => {
                let count = x.read(buffer).await?;
                (count, PhysAddr::None)
            }
        };

        if length == 0 {
            return Err(std::io::Error::new(
                ErrorKind::UnexpectedEof,
                "read return 0",
            ));
        }

        if level.enabled() {
            if let Some(x) = buffer.get(0..length) {
                tracing::info!("PHYS RX - {}", PhysDisplay::new(level, x))
            }
        }

        Ok((length, addr))
    }

    pub(crate) async fn write(
        &mut self,
        data: &[u8],
        addr: PhysAddr,
        level: PhysDecodeLevel,
    ) -> Result<(), std::io::Error> {
        if level.enabled() {
            tracing::info!("PHYS TX - {}", PhysDisplay::new(level, data));
        }

        match self {
            Self::Tcp(x) => x.write_all(data).await,
            Self::Udp(x) => x.write_all(data, addr).await,
            #[cfg(feature = "enable-tls")]
            Self::Tls(x) => write_all_and_flush(x, data).await,
            #[cfg(feature = "serial")]
            Self::Serial(x) => x.write_all(data).await,
            #[cfg(test)]
            Self::Mock(x) => x.write_all(data).await,
        }
    }
}

/// Write all of the data and then flush the stream
///
/// tokio-rustls returns from a write once rustls has accepted the plaintext, even if the
/// resulting ciphertext could not yet be written to the socket. Nothing sends the buffered
/// ciphertext until the next write or flush, so TLS writes must be flushed.
#[cfg(feature = "enable-tls")]
async fn write_all_and_flush<S>(stream: &mut S, data: &[u8]) -> Result<(), std::io::Error>
where
    S: tokio::io::AsyncWrite + Unpin,
{
    stream.write_all(data).await?;
    stream.flush().await
}

pub(crate) struct PhysDisplay<'a> {
    level: PhysDecodeLevel,
    data: &'a [u8],
}

impl<'a> PhysDisplay<'a> {
    pub(crate) fn new(level: PhysDecodeLevel, data: &'a [u8]) -> Self {
        PhysDisplay { level, data }
    }
}

impl std::fmt::Display for PhysDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} bytes", self.data.len())?;
        if self.level.data_enabled() {
            crate::util::decode::format_bytes(f, self.data)?;
        }
        Ok(())
    }
}

#[cfg(all(test, feature = "enable-tls"))]
mod tls_tests {
    use super::write_all_and_flush;

    use std::future::Future;
    use std::net::Ipv4Addr;
    use std::path::PathBuf;
    use std::pin::{pin, Pin};
    use std::sync::Arc;
    use std::task::{Context, Poll, Wake, Waker};
    use std::time::Duration;

    use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};
    use tokio_rustls::rustls::pki_types::{IpAddr, ServerName};
    use tokio_rustls::{client, server, TlsAcceptor, TlsConnector};

    use crate::tcp::tls::MinTlsVersion;

    // small enough that a single frame's ciphertext can't fit
    const DUPLEX_CAPACITY: usize = 64;
    const FRAME: [u8; 292] = [0xAB; 292];

    struct NoopWaker;

    impl Wake for NoopWaker {
        fn wake(self: Arc<Self>) {}
    }

    fn poll_once<F: Future>(future: Pin<&mut F>) -> Poll<F::Output> {
        let waker = Waker::from(Arc::new(NoopWaker));
        future.poll(&mut Context::from_waker(&waker))
    }

    async fn with_timeout<F: Future>(future: F) -> F::Output {
        tokio::time::timeout(Duration::from_secs(5), future)
            .await
            .expect("timed out")
    }

    fn cert_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../certs/self_signed")
            .join(name)
    }

    async fn connect() -> (
        client::TlsStream<DuplexStream>,
        server::TlsStream<DuplexStream>,
    ) {
        let client_config = sfio_rustls_config::client::self_signed(
            MinTlsVersion::V12.into(),
            &cert_path("entity2_cert.pem"),
            &cert_path("entity1_cert.pem"),
            &cert_path("entity1_key.pem"),
            None,
        )
        .unwrap();

        let mut server_config = sfio_rustls_config::server::self_signed(
            MinTlsVersion::V12.into(),
            &cert_path("entity1_cert.pem"),
            &cert_path("entity2_cert.pem"),
            &cert_path("entity2_key.pem"),
            None,
        )
        .unwrap();
        // session tickets are written after the client handshake completes and would
        // fill the duplex, stalling the server handshake
        server_config.send_tls13_tickets = 0;

        let (client_io, server_io) = tokio::io::duplex(DUPLEX_CAPACITY);
        let connector = TlsConnector::from(Arc::new(client_config));
        let acceptor = TlsAcceptor::from(Arc::new(server_config));
        let server_name = ServerName::IpAddress(IpAddr::V4(Ipv4Addr::UNSPECIFIED.into()));

        let (client, server) = with_timeout(async {
            tokio::join!(
                connector.connect(server_name, client_io),
                acceptor.accept(server_io)
            )
        })
        .await;

        (client.unwrap(), server.unwrap())
    }

    #[tokio::test]
    async fn write_all_alone_can_leave_ciphertext_buffered() {
        let (mut client, _server) = connect().await;

        // the server isn't reading, yet the write completes immediately
        assert!(matches!(
            poll_once(pin!(client.write_all(&FRAME))),
            Poll::Ready(Ok(()))
        ));
        assert!(client.get_ref().1.wants_write());
    }

    #[tokio::test]
    async fn write_all_and_flush_completes_once_ciphertext_is_written() {
        let (mut client, mut server) = connect().await;

        let mut received = [0; FRAME.len()];
        {
            let mut write = pin!(write_all_and_flush(&mut client, &FRAME));
            assert!(poll_once(write.as_mut()).is_pending());

            let (write_result, read_result) =
                with_timeout(async { tokio::join!(write, server.read_exact(&mut received)) }).await;
            write_result.unwrap();
            read_result.unwrap();
        }

        assert_eq!(received, FRAME);
        assert!(!client.get_ref().1.wants_write());
    }

    #[tokio::test]
    async fn write_all_and_flush_fails_if_peer_closes_before_flush() {
        let (mut client, server) = connect().await;

        let mut write = pin!(write_all_and_flush(&mut client, &FRAME));
        assert!(poll_once(write.as_mut()).is_pending());

        drop(server);

        assert!(with_timeout(write).await.is_err());
    }
}
