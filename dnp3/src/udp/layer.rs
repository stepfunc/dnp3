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
        Ok(Self::bound(socket))
    }

    pub(crate) fn bound(socket: tokio::net::UdpSocket) -> Self {
        UdpLayer {
            inner: UdpType::Bound(socket),
        }
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
                        return Err(std::io::Error::other(
                            "No destination specified for un-connected UDP socket",
                        ));
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
                    if is_icmp_error(&err) {
                        tracing::warn!("Trapped ICMP error on UDP read: {}", err);
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
        let count = match self.inner.write(data, addr).await {
            Ok(count) => count,
            // the error reports an earlier datagram, so this one wasn't sent yet
            Err(err) if is_icmp_error(&err) => {
                tracing::warn!("Trapped ICMP error on UDP write: {}", err);
                self.inner.write(data, addr).await?
            }
            Err(err) => return Err(err),
        };
        if count < data.len() {
            tracing::error!("UDP under-write");
        }
        Ok(())
    }
}

/// An ICMP error, such as port unreachable, reports that an earlier datagram wasn't delivered.
/// Windows reports it as a reset, and Linux reports it as a refusal on a connected socket.
fn is_icmp_error(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        ErrorKind::ConnectionReset | ErrorKind::ConnectionRefused
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const LOCAL: &str = "127.0.0.1:0";

    /// Connect a layer to a port that is closed once the layer has its own port, so they can't
    /// be the same, and check with a plain socket that sending to it raises an ICMP error
    async fn connected_to_closed_port() -> (UdpLayer, SocketAddr) {
        let remote = tokio::net::UdpSocket::bind(LOCAL).await.unwrap();
        let address = remote.local_addr().unwrap();
        let layer = UdpLayer::connect(LOCAL.parse().unwrap(), address)
            .await
            .unwrap();
        drop(remote);

        let probe = tokio::net::UdpSocket::bind(LOCAL).await.unwrap();
        probe.connect(address).await.unwrap();
        probe.send(&[0]).await.unwrap();
        let mut buffer = [0; 16];
        let err = tokio::time::timeout(Duration::from_secs(1), probe.recv(&mut buffer))
            .await
            .unwrap()
            .unwrap_err();
        assert!(is_icmp_error(&err), "unexpected error: {err}");

        (layer, address)
    }

    #[tokio::test]
    async fn read_ignores_a_refused_datagram() {
        let (mut layer, _) = connected_to_closed_port().await;
        layer.write_all(&[1], PhysAddr::None).await.unwrap();

        let mut buffer = [0; 16];
        let result =
            tokio::time::timeout(Duration::from_millis(200), layer.read(&mut buffer)).await;
        assert!(result.is_err(), "read should wait for data, got {result:?}");
    }

    #[tokio::test]
    async fn write_after_a_refused_datagram_is_sent() {
        let (mut layer, address) = connected_to_closed_port().await;
        layer.write_all(&[1], PhysAddr::None).await.unwrap();
        // the probe above saw its error by now, so this one has arrived too
        let remote = tokio::net::UdpSocket::bind(address).await.unwrap();
        layer.write_all(&[2], PhysAddr::None).await.unwrap();

        let mut buffer = [0; 16];
        let count = tokio::time::timeout(Duration::from_secs(1), remote.recv(&mut buffer))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(buffer.get(..count), Some([2].as_slice()));
    }
}
