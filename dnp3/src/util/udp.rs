use crate::util::phys::PhysAddr;

pub(crate) struct UdpLayer {
    socket: tokio::net::UdpSocket,
}

impl UdpLayer {
    pub(crate) async fn read(
        &mut self,
        buffer: &mut [u8],
    ) -> Result<(usize, PhysAddr), std::io::Error> {
        let (count, source) = self.socket.recv_from(buffer).await?;
        if count == buffer.len() {
            tracing::error!("UDP under-read w/ buffer size == {count}");
        }
        Ok((count, PhysAddr::Udp(source)))
    }

    pub(crate) async fn write_all(
        &mut self,
        data: &[u8],
        addr: PhysAddr,
    ) -> Result<(), std::io::Error> {
        let addr = match addr {
            PhysAddr::None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "No UDP destination specified",
                ))
            }
            PhysAddr::Udp(x) => x,
        };
        let count = self.socket.send_to(data, addr).await?;
        if count < data.len() {
            tracing::error!("UDP under-write");
        }
        Ok(())
    }
}
