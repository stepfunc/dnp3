use crate::app::{Listener, Shutdown};
use crate::link::reader::LinkModes;
use crate::link::{LinkErrorMode, LinkReadMode};
use crate::outstation::task::OutstationTask;
use crate::outstation::{
    ConnectionState, ControlHandler, OutstationApplication, OutstationConfig, OutstationHandle,
    OutstationInformation,
};
use crate::tcp::server_task::{NewSession, ServerTask};
use crate::tcp::{AddressFilter, FilterError, ServerHandle};
use crate::util::channel::Sender;
use crate::util::phys::{PhysAddr, PhysLayer};
use crate::util::session::{Enabled, Session};
use crate::util::shutdown::ShutdownListener;
use std::net::SocketAddr;
use tracing::Instrument;

struct OutstationInfo {
    filter: AddressFilter,
    handle: OutstationHandle,
    /// how we notify the outstation adapter task to switch to new socket
    sender: Sender<NewSession>,
}

/// A builder for creating a TCP server with one or more outstation instances
/// associated with it
pub struct Server {
    link_modes: LinkModes,
    connection_id: u64,
    address: SocketAddr,
    outstations: Vec<OutstationInfo>,
    connection_handler: ServerConnectionHandler,
}

enum ServerConnectionHandler {
    Tcp,
    #[cfg(feature = "tls")]
    Tls(crate::tcp::tls::TlsServerConfig),
}

impl ServerConnectionHandler {
    async fn handle(&mut self, socket: tokio::net::TcpStream) -> Result<PhysLayer, String> {
        match self {
            Self::Tcp => Ok(PhysLayer::Tcp(socket)),
            #[cfg(feature = "tls")]
            Self::Tls(config) => config.handle_connection(socket).await,
        }
    }
}

impl Server {
    /// create a TCP server builder object that will eventually be bound
    /// to the specified address
    pub fn new_tcp_server(link_error_mode: LinkErrorMode, address: SocketAddr) -> Self {
        Self {
            link_modes: LinkModes {
                error_mode: link_error_mode,
                read_mode: LinkReadMode::Stream,
            },
            connection_id: 0,
            address,
            outstations: Vec::new(),
            connection_handler: ServerConnectionHandler::Tcp,
        }
    }

    /// create a TLS server builder object that will eventually be bound to the specified address
    #[cfg(feature = "tls")]
    pub fn new_tls_server(
        link_error_mode: LinkErrorMode,
        address: SocketAddr,
        tls_config: crate::tcp::tls::TlsServerConfig,
    ) -> Self {
        Self {
            link_modes: LinkModes::stream(link_error_mode),
            connection_id: 0,
            address,
            outstations: Vec::new(),
            connection_handler: ServerConnectionHandler::Tls(tls_config),
        }
    }

    /// associate an outstation with the TcpServer, but do not spawn it
    pub fn add_outstation_no_spawn(
        &mut self,
        config: OutstationConfig,
        application: Box<dyn OutstationApplication>,
        information: Box<dyn OutstationInformation>,
        control_handler: Box<dyn ControlHandler>,
        listener: Box<dyn Listener<ConnectionState>>,
        filter: AddressFilter,
    ) -> Result<(OutstationHandle, impl std::future::Future<Output = ()>), FilterError> {
        for item in self.outstations.iter() {
            if filter.conflicts_with(&item.filter) {
                return Err(FilterError::Conflict);
            }
        }

        let (task, handle) = OutstationTask::create(
            Enabled::Yes,
            self.link_modes,
            config,
            PhysAddr::None,
            application,
            information,
            control_handler,
        );

        let (mut adapter, tx) = ServerTask::create(Session::outstation(task), listener);

        let outstation = OutstationInfo {
            filter,
            handle: handle.clone(),
            sender: tx,
        };
        self.outstations.push(outstation);

        let endpoint = self.address;
        let address = config.outstation_address.raw_value();
        let future = async move {
            let _ = adapter.run()
                .instrument(
                    tracing::info_span!("dnp3-outstation-tcp", "listen" = ?endpoint, "addr" = address),
                )
                .await;
        };
        Ok((handle, future))
    }

    /// associate an outstation with the TcpServer and spawn it
    ///
    /// Must be called from within the Tokio runtime
    pub fn add_outstation(
        &mut self,
        config: OutstationConfig,
        application: Box<dyn OutstationApplication>,
        information: Box<dyn OutstationInformation>,
        control_handler: Box<dyn ControlHandler>,
        listener: Box<dyn Listener<ConnectionState>>,
        filter: AddressFilter,
    ) -> Result<OutstationHandle, FilterError> {
        let (handle, future) = self.add_outstation_no_spawn(
            config,
            application,
            information,
            control_handler,
            listener,
            filter,
        )?;
        tokio::spawn(future);
        Ok(handle)
    }

    /// Consume the `TcpServer` builder object, bind it to pre-specified port, and return a (ServerHandle, Future)
    /// tuple.
    ///
    /// This may be called outside the Tokio runtime and allows for manual spawning
    pub async fn bind_no_spawn(
        mut self,
    ) -> Result<(ServerHandle, impl std::future::Future<Output = Shutdown>), tokio::io::Error> {
        let listener = tokio::net::TcpListener::bind(self.address).await?;

        let addr = listener.local_addr().ok();

        let (token, shutdown_rx) = crate::util::shutdown::shutdown_token();

        let task = async move {
            let local = self.address;
            self.run(listener, shutdown_rx)
                .instrument(tracing::info_span!("tcp-server", "listen" = ?local))
                .await
        };

        let handle = ServerHandle {
            addr,
            _token: token,
        };

        Ok((handle, task))
    }

    /// Consume the `TcpServer` builder object, bind it to pre-specified port, and spawn the server
    /// task onto the Tokio runtime. Returns a ServerHandle that will shut down the server and all
    /// associated outstations when dropped.
    ///
    ///
    /// This must be called from within the Tokio runtime
    pub async fn bind(self) -> Result<ServerHandle, tokio::io::Error> {
        let (handle, future) = self.bind_no_spawn().await?;
        tokio::spawn(future);
        Ok(handle)
    }

    async fn run(
        &mut self,
        listener: tokio::net::TcpListener,
        mut shutdown_rx: ShutdownListener,
    ) -> Shutdown {
        tracing::info!("accepting connections");

        tokio::select! {
             _ = self.accept_loop(listener) => {
                // if the accept loop shuts down we exit
             }
             _ = shutdown_rx.listen() => {
                // if we get the message or shutdown we exit
             }
        }

        tracing::info!("shutting down outstations");

        for x in self.outstations.iter_mut() {
            // best effort to shut down outstations before exiting
            let _ = x.handle.shutdown().await;
        }

        tracing::info!("shutdown");

        Shutdown
    }

    async fn accept_loop(&mut self, listener: tokio::net::TcpListener) -> Result<(), Shutdown> {
        loop {
            self.accept_one(&listener).await?;
        }
    }

    async fn accept_one(&mut self, listener: &tokio::net::TcpListener) -> Result<(), Shutdown> {
        match listener.accept().await {
            Ok((stream, addr)) => {
                crate::tcp::configure_server(&stream);
                self.process_connection(stream, addr).await;
                Ok(())
            }
            Err(err) => {
                tracing::error!("{}", err);
                Err(Shutdown)
            }
        }
    }

    async fn process_connection(&mut self, stream: tokio::net::TcpStream, addr: SocketAddr) {
        let id = self.connection_id;
        self.connection_id = self.connection_id.wrapping_add(1);

        tracing::info!("accepted connection {} from: {}", id, addr);

        let first_match = self
            .outstations
            .iter_mut()
            .find(|x| x.filter.matches(addr.ip()));

        match first_match {
            None => {
                tracing::warn!("no matching outstation for: {}", addr)
            }
            Some(x) => match self.connection_handler.handle(stream).await {
                Err(err) => {
                    tracing::warn!("error from {}: {}", addr, err);
                }
                Ok(phys) => {
                    let _ = x.sender.send(NewSession::new(id, phys)).await;
                }
            },
        }
    }
}
