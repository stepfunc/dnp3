use crate::entry::outstation::AddressFilter;
use crate::outstation::task::{IOType, OutstationHandle, OutstationTask};

use crate::outstation::config::OutstationConfig;
use crate::outstation::database::DatabaseConfig;
use crate::outstation::traits::{ControlHandler, OutstationApplication, OutstationInformation};
use crate::util::task::Shutdown;
use tracing::Instrument;

struct Outstation {
    filter: Box<dyn AddressFilter>,
    handle: OutstationHandle,
}

pub struct TCPServer {
    connection_id: u64,
    local: std::net::SocketAddr,
    listener: crate::tokio::net::TcpListener,
    outstations: Vec<Outstation>,
}

/// Handle to a running server. Dropping the handle, shuts down the server.
pub struct ServerHandle {
    _tx: crate::tokio::sync::oneshot::Sender<()>,
}

impl TCPServer {
    pub async fn bind(address: std::net::SocketAddr) -> Result<Self, crate::tokio::io::Error> {
        let listener = crate::tokio::net::TcpListener::bind(address).await?;
        Ok(Self {
            connection_id: 0,
            local: address,
            listener,
            outstations: Vec::new(),
        })
    }

    pub fn add_outstation(
        &mut self,
        config: OutstationConfig,
        database: DatabaseConfig,
        application: Box<dyn OutstationApplication>,
        information: Box<dyn OutstationInformation>,
        control_handler: Box<dyn ControlHandler>,
        filter: Box<dyn AddressFilter>,
    ) -> (OutstationHandle, impl std::future::Future<Output = ()>) {
        let (mut task, handle) =
            OutstationTask::create(config, database, application, information, control_handler);

        let outstation = Outstation {
            filter,
            handle: handle.clone(),
        };
        self.outstations.push(outstation);

        let endpoint = self.local;
        let address = config.outstation_address.raw_value();
        let future = async move {
            task.run()
                .instrument(
                    tracing::info_span!("Outstation", "listen" = ?endpoint, "addr" = address),
                )
                .await
        };
        (handle, future)
    }

    pub fn build(mut self) -> (ServerHandle, impl std::future::Future<Output = Shutdown>) {
        let (tx, rx) = crate::tokio::sync::oneshot::channel();

        let task = async move {
            let local = self.local;
            self.run(rx)
                .instrument(tracing::info_span!("TCPServer", "listen" = ?local))
                .await
        };

        let handle = ServerHandle { _tx: tx };

        (handle, task)
    }

    async fn run(&mut self, rx: crate::tokio::sync::oneshot::Receiver<()>) -> Shutdown {
        tracing::info!("accepting connections");

        crate::tokio::select! {
             _ = self.accept_loop() => {

             }
             _ = rx => {

             }
        }

        tracing::info!("shutting down outstations");

        for x in self.outstations.iter_mut() {
            // best effort to shutdown outstations before exiting
            let _ = x.handle.shutdown().await;
        }

        tracing::info!("shutdown");

        Shutdown
    }

    async fn accept_loop(&mut self) -> Result<(), Shutdown> {
        loop {
            self.accept_one().await?;
        }
    }

    async fn accept_one(&mut self) -> Result<(), Shutdown> {
        match self.listener.accept().await {
            Ok((stream, addr)) => {
                self.process_connection(stream, addr).await;
                Ok(())
            }
            Err(err) => {
                tracing::error!("{}", err);
                Err(Shutdown)
            }
        }
    }

    async fn process_connection(
        &mut self,
        stream: crate::tokio::net::TcpStream,
        addr: std::net::SocketAddr,
    ) {
        let id = self.connection_id;
        self.connection_id = self.connection_id.wrapping_add(1);

        tracing::info!("accepted connection {} from: {}", id, addr);

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
}
