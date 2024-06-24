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
    #[cfg(feature = "tls")]
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
            #[cfg(feature = "tls")]
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
            #[cfg(feature = "tls")]
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
            #[cfg(feature = "tls")]
            Self::Tls(x) => x.write_all(data).await,
            #[cfg(feature = "serial")]
            Self::Serial(x) => x.write_all(data).await,
            #[cfg(test)]
            Self::Mock(x) => x.write_all(data).await,
        }
    }
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

impl<'a> std::fmt::Display for PhysDisplay<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} bytes", self.data.len())?;
        if self.level.data_enabled() {
            crate::util::decode::format_bytes(f, self.data)?;
        }
        Ok(())
    }
}
