use crate::util::phys::{PhysAddr, PhysLayer};
use std::io::ErrorKind;
use std::net::SocketAddr;

enum UdpType {
    /// socket is only bound, not connected, communicates 1 to many
    Bound(tokio::net::UdpSocket),
    /// socket is bound AND connected, communicates 1 to 1
    Connected(tokio::net::UdpSocket),
}

enum UdpFactoryType {
    Bound {
        local: SocketAddr,
    },
    Connected {
        local: SocketAddr,
        remote: SocketAddr,
    },
}

pub(crate) struct UdpFactory(UdpFactoryType);

impl UdpFactory {
    pub(crate) fn bound(local: SocketAddr) -> Self {
        Self(UdpFactoryType::Bound { local })
    }

    pub(crate) fn connected(local: SocketAddr, remote: SocketAddr) -> Self {
        Self(UdpFactoryType::Connected { local, remote })
    }
}

impl UdpFactory {
    pub(crate) async fn open(&self) -> std::io::Result<PhysLayer> {
        match self.0 {
            UdpFactoryType::Bound { local } => {
                let layer = UdpLayer::bind(local).await?;
                Ok(PhysLayer::Udp(layer))
            }
            UdpFactoryType::Connected { local, remote } => {
                let layer = UdpLayer::connect(local, remote).await?;
                Ok(PhysLayer::Udp(layer))
            }
        }
    }
}

impl UdpLayer {
    async fn bind(local: SocketAddr) -> std::io::Result<Self> {
        let socket = tokio::net::UdpSocket::bind(local).await?;
        Ok(UdpLayer {
            inner: UdpType::Bound(socket),
        })
    }

    async fn connect(local: SocketAddr, remote: SocketAddr) -> std::io::Result<Self> {
        let socket = tokio::net::UdpSocket::bind(local).await?;
        socket.connect(remote).await?;
        Ok(UdpLayer {
            inner: UdpType::Connected(socket),
        })
    }
}

pub(crate) struct UdpLayer {
    inner: UdpType,
}

impl UdpType {
    pub(crate) async fn read(
        &mut self,
        buffer: &mut [u8],
    ) -> Result<(usize, PhysAddr), std::io::Error> {
        match self {
            Self::Bound(x) => x
                .recv_from(buffer)
                .await
                .map(|(x, s)| (x, PhysAddr::Udp(s))),
            Self::Connected(x) => x.recv(buffer).await.map(|x| (x, PhysAddr::None)),
        }
    }

    pub(crate) async fn write(
        &mut self,
        data: &[u8],
        addr: PhysAddr,
    ) -> Result<usize, std::io::Error> {
        match self {
            UdpType::Bound(s) => {
                let addr = match addr {
                    PhysAddr::None => {
                        return Err(std::io::Error::new(
                            ErrorKind::Other,
                            "No destination specified for un-connected UDP socket",
                        ))
                    }
                    PhysAddr::Udp(x) => x,
                };
                s.send_to(data, addr).await
            }
            UdpType::Connected(s) => s.send(data).await,
        }
    }
}

impl UdpLayer {
    pub(crate) async fn read(
        &mut self,
        buffer: &mut [u8],
    ) -> Result<(usize, PhysAddr), std::io::Error> {
        let (count, source) = loop {
            match self.inner.read(buffer).await {
                Ok(x) => break x,
                Err(err) => {
                    if err.kind() == ErrorKind::ConnectionReset {
                        tracing::warn!("Trapped ICMP UDP reset: {}", err);
                    } else {
                        return Err(err);
                    }
                }
            }
        };

        if count == buffer.len() {
            tracing::error!("UDP under-read w/ buffer size == {count}");
        }
        Ok((count, source))
    }

    pub(crate) async fn write_all(
        &mut self,
        data: &[u8],
        addr: PhysAddr,
    ) -> Result<(), std::io::Error> {
        let count = self.inner.write(data, addr).await?;
        if count < data.len() {
            tracing::error!("UDP under-write");
        }
        Ok(())
    }
}
