use crate::app::parse::options::ParseOptions;
use crate::app::{Listener, Shutdown};
use crate::link::reader::LinkModes;
use crate::link::{LinkErrorMode, LinkReadMode};
use crate::outstation::task::OutstationTask;
use crate::outstation::{
    ConnectionState, ControlHandler, OutstationApplication, OutstationConfig, OutstationHandle,
    OutstationInformation,
};
use crate::tcp::server_task::{NewSession, ServerTask as OutstationServerTask};
use crate::tcp::{AddressFilter, FilterError, ServerHandle};
use crate::util::channel::Sender;
use crate::util::phys::{PhysAddr, PhysLayer};
use crate::util::session::Enabled;
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

struct AcceptTask {
    server: Server,
    listener: tokio::net::TcpListener,
    shutdown_rx: ShutdownListener,
}

/// A TCP server task that accepts connections and routes them to outstations.
///
/// It is the caller's responsibility to run this task on a Tokio runtime. Each outstation must be
/// run independently using the task returned by [`Server::add_outstation_task`] or the future
/// returned by [`Server::add_outstation_no_spawn`]. No tracing span is attached automatically, so
/// the caller may instrument the [`run`](Self::run) future as desired.
#[cfg(feature = "unstable")]
#[must_use = "a TcpServerTask does nothing unless you call .run()"]
pub struct TcpServerTask {
    inner: AcceptTask,
}

enum ServerConnectionHandler {
    Tcp,
    #[cfg(feature = "enable-tls")]
    Tls(crate::tcp::tls::TlsServerConfig),
}

impl ServerConnectionHandler {
    async fn handle(&mut self, socket: tokio::net::TcpStream) -> Result<PhysLayer, String> {
        match self {
            Self::Tcp => Ok(PhysLayer::Tcp(socket)),
            #[cfg(feature = "enable-tls")]
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
    #[cfg(feature = "enable-tls")]
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
    ///
    /// The returned future is intentionally uninstrumented: unlike
    /// [`add_outstation`](Self::add_outstation), it attaches no tracing span. This is by design so
    /// that the caller retains full control over instrumentation and may wrap the future in
    /// whatever span (e.g. `dnp3-outstation-tcp`) it chooses before spawning it.
    pub fn add_outstation_no_spawn(
        &mut self,
        config: OutstationConfig,
        application: Box<dyn OutstationApplication>,
        information: Box<dyn OutstationInformation>,
        control_handler: Box<dyn ControlHandler>,
        listener: Box<dyn Listener<ConnectionState>>,
        filter: AddressFilter,
    ) -> Result<(OutstationHandle, impl std::future::Future<Output = ()>), FilterError> {
        let (handle, mut adapter) = self.register_outstation(
            config,
            application,
            information,
            control_handler,
            listener,
            filter,
        )?;

        let future = async move {
            let _ = adapter.run().await;
        };
        Ok((handle, future))
    }

    /// Associate an outstation with the TCP server and return a concrete task without spawning it.
    ///
    /// The caller is responsible for running the returned [`crate::outstation::OutstationTask`]
    /// independently from the TCP server task.
    #[cfg(feature = "unstable")]
    #[allow(clippy::too_many_arguments)]
    pub fn add_outstation_task(
        &mut self,
        config: OutstationConfig,
        application: Box<dyn OutstationApplication>,
        information: Box<dyn OutstationInformation>,
        control_handler: Box<dyn ControlHandler>,
        listener: Box<dyn Listener<ConnectionState>>,
        filter: AddressFilter,
    ) -> Result<(OutstationHandle, crate::outstation::OutstationTask), FilterError> {
        let (handle, adapter) = self.register_outstation(
            config,
            application,
            information,
            control_handler,
            listener,
            filter,
        )?;

        Ok((handle, crate::outstation::OutstationTask::tcp(adapter)))
    }

    #[allow(clippy::too_many_arguments)]
    fn register_outstation(
        &mut self,
        config: OutstationConfig,
        application: Box<dyn OutstationApplication>,
        information: Box<dyn OutstationInformation>,
        control_handler: Box<dyn ControlHandler>,
        listener: Box<dyn Listener<ConnectionState>>,
        filter: AddressFilter,
    ) -> Result<(OutstationHandle, OutstationServerTask), FilterError> {
        for item in self.outstations.iter() {
            if filter.conflicts_with(&item.filter) {
                return Err(FilterError::Conflict);
            }
        }

        let (task, handle) = OutstationTask::create(
            Enabled::Yes,
            self.link_modes,
            ParseOptions::get_static(),
            config,
            PhysAddr::None,
            application,
            information,
            control_handler,
        );

        let (adapter, tx) = OutstationServerTask::create(task, listener);

        let outstation = OutstationInfo {
            filter,
            handle: handle.clone(),
            sender: tx,
        };
        self.outstations.push(outstation);
        Ok((handle, adapter))
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
        let endpoint = self.address;
        let address = config.outstation_address.raw_value();
        let (handle, future) = self.add_outstation_no_spawn(
            config,
            application,
            information,
            control_handler,
            listener,
            filter,
        )?;
        tokio::spawn(future.instrument(
            tracing::info_span!("dnp3-outstation-tcp", "listen" = ?endpoint, "addr" = address),
        ));
        Ok(handle)
    }

    /// Consume the `TcpServer` builder object, bind it to pre-specified port, and return a (ServerHandle, Future)
    /// tuple.
    ///
    /// This may be called outside the Tokio runtime and allows for manual spawning
    ///
    /// The returned future is intentionally uninstrumented: unlike [`bind`](Self::bind), it
    /// attaches no tracing span. This is by design so that the caller retains full control over
    /// instrumentation and may wrap the future in whatever span (e.g. `tcp-server`) it chooses
    /// before spawning it.
    pub async fn bind_no_spawn(
        self,
    ) -> Result<(ServerHandle, impl std::future::Future<Output = Shutdown>), tokio::io::Error> {
        let listener = tokio::net::TcpListener::bind(self.address).await?;
        let (handle, task) = self.create_task(listener);
        Ok((handle, async move { task.run().await }))
    }

    /// Consume this server and create a task using an already-bound TCP listener.
    ///
    /// The returned task does nothing until [`TcpServerTask::run`] is called and may be run on any
    /// Tokio runtime.
    #[cfg(feature = "unstable")]
    pub fn into_task(self, listener: tokio::net::TcpListener) -> (ServerHandle, TcpServerTask) {
        let (handle, task) = self.create_task(listener);
        (handle, TcpServerTask { inner: task })
    }

    fn create_task(self, listener: tokio::net::TcpListener) -> (ServerHandle, AcceptTask) {
        let addr = listener.local_addr().ok();
        let (token, shutdown_rx) = crate::util::shutdown::shutdown_token();

        let task = AcceptTask {
            server: self,
            listener,
            shutdown_rx,
        };

        let handle = ServerHandle {
            addr,
            _token: token,
        };

        (handle, task)
    }

    /// Consume the `TcpServer` builder object, bind it to pre-specified port, and spawn the server
    /// task onto the Tokio runtime. Returns a ServerHandle that will shut down the server and all
    /// associated outstations when dropped.
    ///
    ///
    /// This must be called from within the Tokio runtime
    pub async fn bind(self) -> Result<ServerHandle, tokio::io::Error> {
        let configured_address = self.address;
        let (handle, future) = self.bind_no_spawn().await?;
        let local = handle.local_addr().unwrap_or(configured_address);
        tokio::spawn(future.instrument(tracing::info_span!("tcp-server", "listen" = ?local)));
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

impl AcceptTask {
    async fn run(mut self) -> Shutdown {
        self.server.run(self.listener, self.shutdown_rx).await
    }
}

#[cfg(feature = "unstable")]
impl TcpServerTask {
    /// Run until the associated [`ServerHandle`] is dropped or the accept loop terminates.
    pub async fn run(self) {
        self.inner.run().await;
    }
}

#[cfg(all(test, feature = "unstable"))]
mod tests {
    use super::*;
    use crate::app::NullListener;
    use crate::link::EndpointAddress;
    use crate::outstation::database::EventBufferConfig;

    struct NullApplication;
    impl OutstationApplication for NullApplication {}

    struct NullInformation;
    impl OutstationInformation for NullInformation {}

    fn outstation_config() -> OutstationConfig {
        OutstationConfig::new(
            EndpointAddress::try_new(10).unwrap(),
            EndpointAddress::try_new(1).unwrap(),
            EventBufferConfig::all_types(0),
        )
    }

    #[tokio::test]
    async fn into_task_uses_supplied_listener_and_stops_with_handle() {
        let configured_address = "127.0.0.1:0".parse().unwrap();
        let listener = tokio::net::TcpListener::bind(configured_address)
            .await
            .unwrap();
        let listener_address = listener.local_addr().unwrap();
        let server = Server::new_tcp_server(LinkErrorMode::Close, configured_address);

        let (handle, task) = server.into_task(listener);

        assert_eq!(handle.local_addr(), Some(listener_address));
        drop(handle);
        tokio::spawn(task.run()).await.unwrap();
    }

    #[tokio::test]
    async fn add_outstation_task_returns_independently_runnable_task() {
        let configured_address = "127.0.0.1:0".parse().unwrap();
        let mut server = Server::new_tcp_server(LinkErrorMode::Close, configured_address);

        let (handle, task) = server
            .add_outstation_task(
                outstation_config(),
                Box::new(NullApplication),
                Box::new(NullInformation),
                crate::outstation::DefaultControlHandler::create(),
                NullListener::create(),
                AddressFilter::Any,
            )
            .unwrap();

        drop(handle);
        drop(server);
        tokio::spawn(task.run()).await.unwrap();
    }
}
