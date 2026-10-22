use crate::app::{Listener, Shutdown};
use crate::link::reader::LinkModes;
use crate::link::{LinkErrorMode, LinkReadMode};
use crate::outstation::task::OutstationTask;
use crate::outstation::{
    ConnectionState, ControlHandler, OutstationApplication, OutstationConfig, OutstationHandle,
    OutstationInformation,
};
use crate::tcp::accept::{Acceptor, Listen};
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

impl std::fmt::Debug for Server {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Server")
            .field("address", &self.address)
            .finish_non_exhaustive()
    }
}

struct AcceptTask<L = tokio::net::TcpListener> {
    server: Server,
    listener: L,
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

#[cfg(feature = "unstable")]
impl std::fmt::Debug for TcpServerTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TcpServerTask").finish_non_exhaustive()
    }
}

enum ServerConnectionHandler {
    Tcp,
    #[cfg(feature = "enable-tls")]
    Tls(crate::tcp::tls::TlsServerConfig),
}

impl ServerConnectionHandler {
    async fn handle(&self, socket: tokio::net::TcpStream, _addr: SocketAddr) -> Option<PhysLayer> {
        match self {
            Self::Tcp => Some(PhysLayer::tcp(socket)),
            #[cfg(feature = "enable-tls")]
            Self::Tls(config) => match config.handle_connection(socket).await {
                Ok(phys) => Some(phys),
                Err(crate::tcp::tls::HandshakeError::Timeout) => {
                    tracing::warn!(
                        "error from {}: TLS handshake didn't complete within {} seconds",
                        _addr,
                        crate::tcp::tls::TLS_HANDSHAKE_TIMEOUT.as_secs()
                    );
                    None
                }
                Err(crate::tcp::tls::HandshakeError::Failed(err)) => {
                    tracing::warn!(
                        "error from {}: failed to establish TLS session: {}",
                        _addr,
                        err
                    );
                    None
                }
            },
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
            let _: Result<(), crate::app::Shutdown> = adapter.run().await;
        };
        Ok((handle, future))
    }

    /// Associate an outstation with the TCP server and return a concrete task without spawning it.
    ///
    /// The caller is responsible for running the returned [`crate::outstation::OutstationTask`]
    /// independently from the TCP server task.
    #[cfg(feature = "unstable")]
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
        self.create_task_with(listener, addr)
    }

    fn create_task_with<L: Listen>(
        self,
        listener: L,
        addr: Option<SocketAddr>,
    ) -> (ServerHandle, AcceptTask<L>) {
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
        let listener = tokio::net::TcpListener::bind(self.address).await?;
        Ok(self.spawn(listener))
    }

    // Spawning outside the `bind` future keeps the server task from being compiled again in every
    // crate that awaits `bind`.
    fn spawn(self, listener: tokio::net::TcpListener) -> ServerHandle {
        let configured_address = self.address;
        let (handle, task) = self.create_task(listener);
        let local = handle.local_addr().unwrap_or(configured_address);
        tokio::spawn(
            task.run()
                .instrument(tracing::info_span!("tcp-server", "listen" = ?local)),
        );
        handle
    }

    async fn run<L: Listen>(&mut self, listener: L, mut shutdown_rx: ShutdownListener) -> Shutdown {
        tracing::info!("accepting connections");

        let mut acceptor = Acceptor::new(listener);
        loop {
            tokio::select! {
                _ = self.accept_one(&mut acceptor) => {}
                _ = shutdown_rx.listen() => break,
            }
        }

        tracing::info!("shutting down outstations");

        for x in self.outstations.iter_mut() {
            // best effort to shut down outstations before exiting
            let _: Result<(), crate::app::Shutdown> = x.handle.shutdown().await;
        }

        tracing::info!("shutdown");

        Shutdown
    }

    async fn accept_one<L: Listen>(&mut self, acceptor: &mut Acceptor<L>) {
        let (stream, addr) = acceptor.accept().await;
        crate::tcp::configure_server(&stream);
        self.process_connection(stream, addr).await;
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
            Some(x) => {
                if let Some(phys) = self.connection_handler.handle(stream, addr).await {
                    if x.sender.send(NewSession::new(id, phys)).await.is_err() {
                        tracing::warn!("outstation is shut down, closing connection from {}", addr);
                    }
                }
            }
        }
    }
}

impl<L: Listen> AcceptTask<L> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::MaybeAsync;
    #[cfg(feature = "unstable")]
    use crate::app::NullListener;
    use crate::tcp::accept::test_util::{
        expect_attempt_gaps, Fault, FaultyListener, RESET, UNKNOWN,
    };
    use crate::tcp::accept::ACCEPT_ERROR_DELAY;
    use crate::util::test_support::{watchdog, NullApplication, NullInformation};
    use std::time::Duration;
    use tokio::sync::mpsc;

    fn outstation_config() -> OutstationConfig {
        crate::util::test_support::outstation_config(10, 1)
    }

    struct ConnectionListener(mpsc::UnboundedSender<ConnectionState>);
    impl Listener<ConnectionState> for ConnectionListener {
        fn update(&mut self, value: ConnectionState) -> MaybeAsync<()> {
            self.0.send(value).unwrap();
            MaybeAsync::ready(())
        }
    }

    /// a server with one outstation that reports its connection state
    fn server_with_outstation() -> (Server, mpsc::UnboundedReceiver<ConnectionState>) {
        server_with_filter(AddressFilter::Any)
    }

    fn server_with_filter(
        filter: AddressFilter,
    ) -> (Server, mpsc::UnboundedReceiver<ConnectionState>) {
        let mut server =
            Server::new_tcp_server(LinkErrorMode::Close, "127.0.0.1:0".parse().unwrap());
        let (tx, rx) = mpsc::unbounded_channel();
        let (_handle, future) = server
            .add_outstation_no_spawn(
                outstation_config(),
                Box::new(NullApplication),
                Box::new(NullInformation),
                crate::outstation::DefaultControlHandler::create(),
                Box::new(ConnectionListener(tx)),
                filter,
            )
            .unwrap();
        tokio::spawn(future);
        (server, rx)
    }

    #[cfg(feature = "enable-tls")]
    mod tls {
        use super::*;
        use crate::tcp::tls::MinTlsVersion;
        use crate::util::test_support::tls::{cert, server_config};
        use crate::util::test_support::Timeline;
        use std::sync::Arc;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::{TcpListener, TcpStream};
        use tokio_rustls::rustls::pki_types::{IpAddr, ServerName};

        const fn secs(x: u64) -> Duration {
            Duration::from_secs(x)
        }

        /// a TLS server on an OS-assigned port with one outstation that reports its connection state
        async fn spawn_tls_server() -> (ServerHandle, SocketAddr, Timeline<ConnectionState>) {
            let mut server = Server::new_tls_server(
                LinkErrorMode::Close,
                "127.0.0.1:0".parse().unwrap(),
                server_config(),
            );
            let (listener, timeline) = Timeline::new();
            let (_handle, future) = server
                .add_outstation_no_spawn(
                    outstation_config(),
                    Box::new(NullApplication),
                    Box::new(NullInformation),
                    crate::outstation::DefaultControlHandler::create(),
                    listener,
                    AddressFilter::Any,
                )
                .unwrap();
            tokio::spawn(future);
            let socket = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = socket.local_addr().unwrap();
            let (handle, task) = server.create_task_with(socket, None);
            tokio::spawn(task.run());
            (handle, addr, timeline)
        }

        /// complete a TLS handshake with the server as a master holding the expected certificate
        async fn connect_tls(addr: SocketAddr) -> tokio_rustls::client::TlsStream<TcpStream> {
            let config = sfio_rustls_config::client::self_signed(
                MinTlsVersion::V12.into(),
                &cert("entity2_cert.pem"),
                &cert("entity1_cert.pem"),
                &cert("entity1_key.pem"),
                None,
            )
            .unwrap();
            let socket = TcpStream::connect(addr).await.unwrap();
            tokio_rustls::TlsConnector::from(Arc::new(config))
                .connect(
                    ServerName::IpAddress(IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED.into())),
                    socket,
                )
                .await
                .unwrap()
        }

        /// wait for the server to close a connection, discarding anything it sends first
        async fn assert_closed(socket: &mut TcpStream) {
            let mut buffer = [0; 64];
            while let Ok(1..) = socket.read(&mut buffer).await {}
        }

        #[tokio::test(start_paused = true)]
        async fn tls_server_closes_a_stalled_handshake_after_the_timeout_and_accepts_the_next() {
            watchdog(async {
                let (_handle, addr, mut timeline) = spawn_tls_server().await;
                // a peer that connects and never sends its ClientHello
                let mut stalled = TcpStream::connect(addr).await.unwrap();
                assert_closed(&mut stalled).await;
                assert_eq!(timeline.now(), secs(5));
                timeline.assert_quiet();

                // a paused clock advances whenever the runtime waits on a socket, so it could expire
                // the next handshake's timeout before the handshake completes
                tokio::time::resume();
                let _session = connect_tls(addr).await;
                timeline.expect_state(ConnectionState::Connected).await;
            })
            .await;
        }

        #[tokio::test]
        async fn tls_server_closes_a_failed_handshake_at_once_and_accepts_the_next() {
            watchdog(async {
                let (_handle, addr, mut timeline) = spawn_tls_server().await;
                // a peer that sends a DNP3 frame instead of a ClientHello
                let mut plain = TcpStream::connect(addr).await.unwrap();
                plain
                    .write_all(&crate::util::test_support::MASTER_REQUEST)
                    .await
                    .unwrap();
                assert_closed(&mut plain).await;
                assert!(timeline.now() < secs(5));
                timeline.assert_quiet();

                let _session = connect_tls(addr).await;
                timeline.expect_state(ConnectionState::Connected).await;
            })
            .await;
        }
    }

    #[tokio::test]
    async fn ipv4_filters_match_ipv4_peers_on_a_dual_stack_listener() {
        let filters = [
            AddressFilter::Exact("127.0.0.1".parse().unwrap()),
            AddressFilter::AnyOf(["127.0.0.1".parse().unwrap()].into()),
            AddressFilter::WildcardIpv4("127.0.0.*".parse().unwrap()),
        ];
        for filter in filters {
            watchdog(async {
                let Ok(listener) = tokio::net::TcpListener::bind("[::]:0").await else {
                    return;
                };
                let port = listener.local_addr().unwrap().port();
                let Ok(_client) = tokio::net::TcpStream::connect(("127.0.0.1", port)).await else {
                    // the platform doesn't accept IPv4 on an IPv6 socket
                    return;
                };
                let (server, mut states) = server_with_filter(filter);
                let (_handle, task) = server.create_task_with(listener, None);
                tokio::spawn(task.run());

                assert_eq!(states.recv().await, Some(ConnectionState::Connected));
            })
            .await;
        }
    }

    #[tokio::test]
    async fn bind_serves_connections_on_the_bound_address() {
        watchdog(async {
            let (server, mut states) = server_with_outstation();
            let handle = server.bind().await.unwrap();
            let addr = handle.local_addr().unwrap();
            assert_ne!(addr.port(), 0);

            let _client = tokio::net::TcpStream::connect(addr).await.unwrap();
            assert_eq!(states.recv().await, Some(ConnectionState::Connected));
        })
        .await;
    }

    #[tokio::test]
    async fn bind_reports_an_address_in_use() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let server = Server::new_tcp_server(LinkErrorMode::Close, listener.local_addr().unwrap());

        let err = server.bind().await.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::AddrInUse);
    }

    #[tokio::test(start_paused = true)]
    async fn accepts_after_accept_errors() {
        watchdog(async {
            let (listener, mut attempts) = FaultyListener::new(&[UNKNOWN, RESET, UNKNOWN]).await;
            let _client = tokio::net::TcpStream::connect(listener.local_addr())
                .await
                .unwrap();
            let (server, mut states) = server_with_outstation();
            let (_handle, task) = server.create_task_with(listener, None);
            tokio::spawn(task.run());

            expect_attempt_gaps(
                &mut attempts,
                &[ACCEPT_ERROR_DELAY, Duration::ZERO, ACCEPT_ERROR_DELAY],
            )
            .await;
            assert_eq!(states.recv().await, Some(ConnectionState::Connected));
        })
        .await;
    }

    #[tokio::test(start_paused = true)]
    async fn dropping_handle_during_accept_error_delay_stops_server() {
        watchdog(async {
            let (listener, mut attempts) = FaultyListener::new(&[UNKNOWN]).await;
            let (server, _states) = server_with_outstation();
            let (handle, task) = server.create_task_with(listener, None);
            let task = tokio::spawn(task.run());

            let failed_at = attempts.recv().await.unwrap();
            tokio::time::advance(Duration::from_millis(500)).await;
            assert!(!task.is_finished());

            drop(handle);
            task.await.unwrap();
            assert!(failed_at.elapsed() < ACCEPT_ERROR_DELAY);
            assert_eq!(attempts.recv().await, None);
        })
        .await;
    }

    #[tokio::test]
    async fn dropping_handle_during_connection_errors_stops_server() {
        static RESETS: [Fault; 1000] = [RESET; 1000];
        watchdog(async {
            let (listener, mut attempts) = FaultyListener::new(&RESETS).await;
            let (server, _states) = server_with_outstation();
            let (handle, task) = server.create_task_with(listener, None);
            let task = tokio::spawn(task.run());

            attempts.recv().await.unwrap();
            drop(handle);
            task.await.unwrap();
            let mut count = 1;
            while attempts.recv().await.is_some() {
                count += 1;
            }
            assert!(count < RESETS.len(), "{count}");
        })
        .await;
    }

    #[cfg(feature = "unstable")]
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

    #[cfg(feature = "unstable")]
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
