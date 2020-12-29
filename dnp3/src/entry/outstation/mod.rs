use crate::outstation::task::{IOType, OutstationHandle, OutstationTask};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Priority {
    value: Option<usize>,
}

pub trait AddressFilter: Send {
    fn matches(&self, address: &std::net::SocketAddr) -> Option<usize>;
}

#[derive(Copy, Clone)]
pub struct AnyAddress;

impl AddressFilter for AnyAddress {
    fn matches(&self, _: &std::net::SocketAddr) -> Option<usize> {
        Some(0)
    }
}

impl AnyAddress {
    pub fn create() -> Box<dyn AddressFilter> {
        Box::new(AnyAddress)
    }
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
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    tracing::info!("accepted connection from: {}", addr);

                    let best = self
                        .outstations
                        .iter_mut()
                        .max_by_key(|x| x.filter.matches(&addr));

                    match best {
                        None => {
                            tracing::warn!("no matching outstation for: {}", addr)
                        }
                        Some(x) => {
                            let _ = x.handle.new_io(IOType::TCPStream(stream)).await;
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
