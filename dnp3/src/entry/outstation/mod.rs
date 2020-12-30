use crate::outstation::task::{IOType, OutstationHandle, OutstationTask};
use tracing::Instrument;

pub mod filters;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Match {
    value: Option<u32>,
}

impl Match {
    pub fn yes(value: u32) -> Self {
        Self { value: Some(value) }
    }

    pub fn no() -> Self {
        Self { value: None }
    }
}

pub trait AddressFilter: Send {
    fn matches(&self, address: &std::net::SocketAddr) -> Match;
}

struct Outstation {
    filter: Box<dyn AddressFilter>,
    handle: OutstationHandle,
    _join_handle: crate::tokio::task::JoinHandle<()>,
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

    pub fn add(
        &mut self,
        mut task: OutstationTask,
        handle: OutstationHandle,
        filter: Box<dyn AddressFilter>,
    ) {
        let endpoint = self.local;
        let join_handle = crate::tokio::spawn(async move {
            task.run()
                .instrument(tracing::info_span!("TCPServer", "local" = ?endpoint))
                .await
        });

        let outstation = Outstation {
            filter,
            handle,
            _join_handle: join_handle,
        };
        self.outstations.push(outstation);
    }

    pub async fn run(&mut self) {
        let local = self.local;
        self.run_inner()
            .instrument(tracing::info_span!("TCPServer", "listen" = ?local))
            .await
    }

    async fn run_inner(&mut self) {
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
