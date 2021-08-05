use crate::decode::PhysDecodeLevel;
use crate::tokio::io::{AsyncReadExt, AsyncWriteExt};

// encapsulates all possible physical layers as an enum
pub(crate) enum PhysLayer {
    Tcp(crate::tokio::net::TcpStream),
    Tls(tokio_rustls::TlsStream<crate::tokio::net::TcpStream>),
    Serial(tokio_serial::SerialStream),
    #[cfg(test)]
    Mock(tokio_mock::mock::test::io::MockIO),
}

impl std::fmt::Debug for PhysLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PhysLayer::Tcp(_) => f.write_str("Tcp"),
            PhysLayer::Tls(_) => f.write_str("TLS"),
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
    ) -> Result<usize, std::io::Error> {
        let length = match self {
            Self::Tcp(x) => x.read(buffer).await?,
            Self::Tls(x) => x.read(buffer).await?,
            Self::Serial(x) => x.read(buffer).await?,
            #[cfg(test)]
            Self::Mock(x) => x.read(buffer).await?,
        };

        if level.enabled() {
            if let Some(x) = buffer.get(0..length) {
                tracing::info!("PHYS RX - {}", PhysDisplay::new(level, x))
            }
        }

        Ok(length)
    }

    pub(crate) async fn write(
        &mut self,
        data: &[u8],
        level: PhysDecodeLevel,
    ) -> Result<(), std::io::Error> {
        if level.enabled() {
            tracing::info!("PHYS TX - {}", PhysDisplay::new(level, data));
        }

        match self {
            Self::Tcp(x) => x.write_all(data).await,
            Self::Tls(x) => x.write_all(data).await,
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
