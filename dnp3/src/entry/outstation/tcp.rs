use crate::entry::outstation::AddressFilter;
use crate::outstation::task::{IOType, OutstationHandle, OutstationTask};

use tracing::Instrument;

struct Outstation {
    filter: Box<dyn AddressFilter>,
    handle: OutstationHandle,
}

pub struct TCPServer {
    local: std::net::SocketAddr,
    listener: crate::tokio::net::TcpListener,
    outstations: Vec<Outstation>,
}

impl TCPServer {
    pub async fn bind(address: std::net::SocketAddr) -> Result<Self, crate::tokio::io::Error> {
        let listener = crate::tokio::net::TcpListener::bind(address).await?;
        Ok(Self {
            local: address,
            listener,
            outstations: Vec::new(),
        })
    }

    pub fn add_outstation(
        &mut self,
        mut task: OutstationTask,
        handle: OutstationHandle,
        filter: Box<dyn AddressFilter>,
    ) -> impl std::future::Future<Output = ()> {
        let outstation = Outstation { filter, handle };
        self.outstations.push(outstation);

        let endpoint = self.local;
        async move {
            task.run()
                .instrument(tracing::info_span!("TCPServer", "local" = ?endpoint))
                .await
        }
    }

    pub async fn build(mut self) {
        let local = self.local;
        self.run()
            .instrument(tracing::info_span!("TCPServer", "listen" = ?local))
            .await
    }

    async fn run(&mut self) {
        tracing::info!("accepting connections");

        let mut connection_id: u64 = 0;
        loop {
            match self.listener.accept().await {
                Ok((stream, addr)) => {
                    let id = connection_id;
                    connection_id = connection_id.wrapping_add(1);

                    tracing::info!("accepted connection from: {}", addr);

                    let best = self
                        .outstations
                        .iter_mut()
                        .filter(|x| x.filter.matches(&addr).value.is_some())
                        .max_by_key(|x| x.filter.matches(&addr));

                    match best {
                        None => {
                            tracing::warn!("no matching outstation for: {}", addr)
                        }
                        Some(x) => {
                            let _ = x.handle.new_io(id, IOType::TCPStream(stream)).await;
                        }
                    }
                }
                Err(err) => {
                    tracing::error!("{}", err);
                    return;
                }
            }
        }
    }
}
