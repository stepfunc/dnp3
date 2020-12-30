use crate::outstation::task::{IOType, OutstationHandle, OutstationTask};

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
    outstations: Vec<Outstation>,
}

impl Default for TCPServer {
    fn default() -> Self {
        Self {
            outstations: Vec::new(),
        }
    }
}

impl TCPServer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bind(
        &mut self,
        mut task: OutstationTask,
        handle: OutstationHandle,
        filter: Box<dyn AddressFilter>,
    ) {
        let join_handle = crate::tokio::spawn(async move { task.run().await });
        let outstation = Outstation {
            filter,
            handle,
            _join_handle: join_handle,
        };
        self.outstations.push(outstation);
    }

    pub async fn run(&mut self, mut listener: crate::tokio::net::TcpListener) {
        let mut connection_id: u64 = 0;
        loop {
            match listener.accept().await {
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
