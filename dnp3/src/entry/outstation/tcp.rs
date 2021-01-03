use crate::entry::outstation::{AddressFilter, FilterError};
use crate::outstation::task::{IOType, OutstationHandle, OutstationTask};

use crate::outstation::config::OutstationConfig;
use crate::outstation::database::DatabaseConfig;
use crate::outstation::traits::{ControlHandler, OutstationApplication, OutstationInformation};
use crate::util::task::Shutdown;
use tracing::Instrument;

struct Outstation {
    filter: AddressFilter,
    handle: OutstationHandle,
}

pub struct TCPServer {
    connection_id: u64,
    address: std::net::SocketAddr,
    outstations: Vec<Outstation>,
}

/// Handle to a running server. Dropping the handle, shuts down the server.
pub struct ServerHandle {
    _tx: crate::tokio::sync::oneshot::Sender<()>,
}

impl TCPServer {
    pub fn new(address: std::net::SocketAddr) -> Self {
        Self {
            connection_id: 0,
            address,
            outstations: Vec::new(),
        }
    }

    pub fn add_outstation(
        &mut self,
        config: OutstationConfig,
        database: DatabaseConfig,
        application: Box<dyn OutstationApplication>,
        information: Box<dyn OutstationInformation>,
        control_handler: Box<dyn ControlHandler>,
        filter: AddressFilter,
    ) -> Result<(OutstationHandle, impl std::future::Future<Output = ()>), FilterError> {
        for item in self.outstations.iter() {
            if filter.conflicts_with(&item.filter) {
                return Err(FilterError::Conflict);
            }
        }

        let (mut task, handle) =
            OutstationTask::create(config, database, application, information, control_handler);

        let outstation = Outstation {
            filter,
            handle: handle.clone(),
        };
        self.outstations.push(outstation);

        let endpoint = self.address;
        let address = config.outstation_address.raw_value();
        let future = async move {
            task.run()
                .instrument(
                    tracing::info_span!("Outstation", "listen" = ?endpoint, "addr" = address),
                )
                .await
        };
        Ok((handle, future))
    }

    pub async fn bind(
        mut self,
    ) -> Result<(ServerHandle, impl std::future::Future<Output = Shutdown>), crate::tokio::io::Error>
    {
        let listener = crate::tokio::net::TcpListener::bind(self.address).await?;

        let (tx, rx) = crate::tokio::sync::oneshot::channel();

        let task = async move {
            let local = self.address;
            self.run(listener, rx)
                .instrument(tracing::info_span!("TCPServer", "listen" = ?local))
                .await
        };

        let handle = ServerHandle { _tx: tx };

        Ok((handle, task))
    }

    async fn run(
        &mut self,
        listener: crate::tokio::net::TcpListener,
        rx: crate::tokio::sync::oneshot::Receiver<()>,
    ) -> Shutdown {
        tracing::info!("accepting connections");

        crate::tokio::select! {
             _ = self.accept_loop(listener) => {

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

    async fn accept_loop(
        &mut self,
        mut listener: crate::tokio::net::TcpListener,
    ) -> Result<(), Shutdown> {
        loop {
            self.accept_one(&mut listener).await?;
        }
    }

    async fn accept_one(
        &mut self,
        listener: &mut crate::tokio::net::TcpListener,
    ) -> Result<(), Shutdown> {
        match listener.accept().await {
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

        let first_match = self.outstations.iter_mut().find(|x| x.filter.matches(addr));

        match first_match {
            None => {
                tracing::warn!("no matching outstation for: {}", addr)
            }
            Some(x) => {
                let _ = x.handle.new_io(id, IOType::TCPStream(stream)).await;
            }
        }
    }
}
