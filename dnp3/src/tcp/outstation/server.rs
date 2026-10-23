// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::{Listener, Shutdown};
use crate::link::reader::LinkModes;
use crate::link::LinkErrorMode;
use crate::outstation::task::OutstationTask;
use crate::outstation::{
    ConnectionState, ControlHandler, OutstationApplication, OutstationConfig, OutstationHandle,
    OutstationInformation,
};
use crate::tcp::accept::{Acceptor, Listen};
use crate::tcp::server_task::{NewSession, ServerTask as OutstationServerTask};
use crate::tcp::{AddressFilter, FilterError, ServerHandle};
use crate::util::channel::Sender;
use crate::util::phys::{PhysAddr, PhysLayer, Socket};
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
    async fn handle<S: Socket>(&self, socket: S, _addr: SocketAddr) -> Option<PhysLayer> {
        match self {
            Self::Tcp => Some(socket.into_phys()),
            #[cfg(feature = "enable-tls")]
            Self::Tls(config) => config
                .handle_connection(socket)
                .await
                .inspect_err(|err| err.log(_addr))
                .ok(),
        }
    }
}

impl Server {
    /// create a TCP server builder object that will eventually be bound
    /// to the specified address
    pub fn new_tcp_server(link_error_mode: LinkErrorMode, address: SocketAddr) -> Self {
        Self {
            link_modes: LinkModes::stream(link_error_mode),
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

    async fn process_connection<S: Socket>(&mut self, stream: S, addr: SocketAddr) {
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
        expect_attempt_gaps, Fault, FaultyListener, Peers, LOOPBACK, RESET, UNKNOWN,
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
        let rx = add_outstation(&mut server, filter).unwrap();
        (server, rx)
    }

    /// add an outstation that reports its connection state, and spawn it
    fn add_outstation(
        server: &mut Server,
        filter: AddressFilter,
    ) -> Result<mpsc::UnboundedReceiver<ConnectionState>, FilterError> {
        let (tx, rx) = mpsc::unbounded_channel();
        let (_handle, future) = server.add_outstation_no_spawn(
            outstation_config(),
            Box::new(NullApplication),
            Box::new(NullInformation),
            crate::outstation::DefaultControlHandler::create(),
            Box::new(ConnectionListener(tx)),
            filter,
        )?;
        tokio::spawn(future);
        Ok(rx)
    }

    #[cfg(feature = "enable-tls")]
    mod tls {
        use super::*;
        use crate::tcp::tls::MinTlsVersion;
        use crate::util::test_support::tls::{rustls_client, server_config};
        use crate::util::test_support::{
            Timeline, MASTER_REQUEST, NULL_UNSOLICITED_HEADER, OUTSTATION_REPLY,
        };
        use std::sync::Arc;
        use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
        use tokio::net::{TcpListener, TcpStream};
        use tokio_rustls::rustls::pki_types::{IpAddr, ServerName};

        const fn secs(x: u64) -> Duration {
            Duration::from_secs(x)
        }

        /// a TLS server with one outstation, registered with `filter`, that reports its connection
        /// state
        fn tls_server(
            config: OutstationConfig,
            filter: AddressFilter,
        ) -> (Server, Timeline<ConnectionState>) {
            let mut server = Server::new_tls_server(
                LinkErrorMode::Close,
                "127.0.0.1:0".parse().unwrap(),
                server_config(MinTlsVersion::V12),
            );
            let (listener, timeline) = Timeline::new();
            let (_handle, future) = server
                .add_outstation_no_spawn(
                    config,
                    Box::new(NullApplication),
                    Box::new(NullInformation),
                    crate::outstation::DefaultControlHandler::create(),
                    listener,
                    filter,
                )
                .unwrap();
            tokio::spawn(future);
            (server, timeline)
        }

        /// run the server on a loopback listener, returning the handle that keeps it running, the
        /// address to connect to and the server's task
        async fn run_on_loopback(
            server: Server,
        ) -> (ServerHandle, SocketAddr, tokio::task::JoinHandle<Shutdown>) {
            let socket = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = socket.local_addr().unwrap();
            let (handle, task) = server.create_task_with(socket, None);
            (handle, addr, tokio::spawn(task.run()))
        }

        /// a TLS server on an OS-assigned port with one outstation that reports its connection state
        async fn spawn_tls_server() -> (ServerHandle, SocketAddr, Timeline<ConnectionState>) {
            let (handle, addr, timeline, _task) = spawn_tls_server_with(AddressFilter::Any).await;
            (handle, addr, timeline)
        }

        /// a TLS server on an OS-assigned port with one outstation, registered with `filter`, that
        /// reports its connection state, and the server's task
        async fn spawn_tls_server_with(
            filter: AddressFilter,
        ) -> (
            ServerHandle,
            SocketAddr,
            Timeline<ConnectionState>,
            tokio::task::JoinHandle<Shutdown>,
        ) {
            let (server, timeline) = tls_server(outstation_config(), filter);
            let (handle, addr, task) = run_on_loopback(server).await;
            (handle, addr, timeline, task)
        }

        fn server_name() -> ServerName<'static> {
            ServerName::IpAddress(IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED.into()))
        }

        /// complete a TLS handshake with the server as a master holding the expected certificate
        async fn connect_tls(addr: SocketAddr) -> tokio_rustls::client::TlsStream<TcpStream> {
            handshake(TcpStream::connect(addr).await.unwrap()).await
        }

        /// complete a TLS handshake over `socket` as a master holding the expected certificate
        async fn handshake<S: AsyncRead + AsyncWrite + Unpin>(
            socket: S,
        ) -> tokio_rustls::client::TlsStream<S> {
            tokio_rustls::TlsConnector::from(Arc::new(rustls_client(MinTlsVersion::V12.into())))
                .connect(server_name(), socket)
                .await
                .unwrap()
        }

        /// the first flight of a master's TLS handshake
        fn client_hello() -> Vec<u8> {
            let mut connection = tokio_rustls::rustls::ClientConnection::new(
                Arc::new(rustls_client(MinTlsVersion::V12.into())),
                server_name(),
            )
            .unwrap();
            let mut hello = Vec::new();
            connection.write_tls(&mut hello).unwrap();
            hello
        }

        /// the content type of a TLS record that carries handshake messages
        const HANDSHAKE_RECORD: u8 = 0x16;

        /// whether an error means the server closed the connection
        fn is_closed(err: &std::io::Error) -> bool {
            matches!(
                err.kind(),
                std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::ConnectionAborted
                    | std::io::ErrorKind::BrokenPipe
            )
        }

        /// wait for the server to close a connection, discarding anything it sends first
        async fn assert_closed<S: AsyncRead + Unpin>(socket: &mut S) {
            let mut buffer = [0; 64];
            while let Ok(1..) = socket.read(&mut buffer).await {}
        }

        #[tokio::test(start_paused = true)]
        async fn tls_server_closes_a_stalled_handshake_after_the_timeout_and_accepts_the_next() {
            watchdog(async {
                let (server, mut timeline) = tls_server(outstation_config(), AddressFilter::Any);
                let (_handle, mut peers) = run_in_memory(server);
                // a peer that connects and never sends its ClientHello
                let (mut stalled, stalled_addr) = peers.connect(LOOPBACK);
                assert_closed(&mut stalled).await;
                assert_eq!(timeline.now(), secs(5));
                timeline.assert_quiet();
                // the timeout ran from the accept
                assert_eq!(peers.accepts(stalled_addr).await.elapsed(), secs(5));

                let (socket, _) = peers.connect(LOOPBACK);
                let _session = handshake(socket).await;
                timeline.expect_state(ConnectionState::Connected).await;
            })
            .await;
        }

        #[tokio::test]
        async fn tls_server_runs_an_outstation_session() {
            watchdog(async {
                let config = crate::util::test_support::outstation_config(1024, 1);
                let (server, mut timeline) = tls_server(config, AddressFilter::Any);
                let (_handle, addr, _task) = run_on_loopback(server).await;
                let mut session = connect_tls(addr).await;
                timeline.expect_state(ConnectionState::Connected).await;

                // the outstation's null unsolicited response, then its answer to a request
                let mut null_unsolicited = [0; 17];
                session.read_exact(&mut null_unsolicited).await.unwrap();
                assert_eq!(null_unsolicited[..10], NULL_UNSOLICITED_HEADER);
                session.write_all(&MASTER_REQUEST).await.unwrap();
                session.flush().await.unwrap();
                let mut reply = [0; 10];
                session.read_exact(&mut reply).await.unwrap();
                assert_eq!(reply, OUTSTATION_REPLY);
            })
            .await;
        }

        #[tokio::test]
        async fn dropping_handle_during_a_stalled_tls_handshake_stops_server() {
            watchdog(async {
                let (handle, addr, mut timeline, task) =
                    spawn_tls_server_with(AddressFilter::Any).await;
                // before the server can start the handshake, so before its timeout starts
                let started = std::time::Instant::now();
                // a peer that sends its ClientHello and never continues the handshake
                let mut stalled = TcpStream::connect(addr).await.unwrap();
                stalled.write_all(&client_hello()).await.unwrap();
                // the server answered, so its handshake is waiting for the peer
                let mut record_type = [0; 1];
                stalled.read_exact(&mut record_type).await.unwrap();
                assert_eq!(record_type, [HANDSHAKE_RECORD]);

                drop(handle);
                let Shutdown = task.await.unwrap();
                assert!(started.elapsed() < crate::tcp::tls::TLS_HANDSHAKE_TIMEOUT);
                assert_closed(&mut stalled).await;
                timeline.assert_quiet();
            })
            .await;
        }

        #[tokio::test]
        async fn an_unmatched_tls_peer_is_closed_before_any_handshake() {
            watchdog(async {
                let filter = AddressFilter::Exact("192.0.2.1".parse().unwrap());
                let (_handle, addr, mut timeline, _task) = spawn_tls_server_with(filter).await;
                let mut peer = TcpStream::connect(addr).await.unwrap();
                // the server may already have closed the connection
                if let Err(err) = peer.write_all(&client_hello()).await {
                    assert!(is_closed(&err), "{err:?}");
                }

                // no ServerHello comes back before the connection closes
                let mut buffer = [0; 64];
                match peer.read(&mut buffer).await {
                    Ok(count) => assert_eq!(count, 0),
                    Err(err) => assert!(is_closed(&err), "{err:?}"),
                }
                timeline.assert_quiet();
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

    fn exact(addr: &str) -> AddressFilter {
        AddressFilter::Exact(addr.parse().unwrap())
    }

    /// run the server on an in-memory listener, returning the handle that keeps it running and
    /// the peers that connect to it
    fn run_in_memory(server: Server) -> (ServerHandle, Peers) {
        let (listener, peers) = FaultyListener::new(&[]);
        let (handle, task) = server.create_task_with(listener, None);
        tokio::spawn(task.run());
        (handle, peers)
    }

    #[tokio::test]
    async fn a_connection_goes_to_the_outstation_whose_filter_matches() {
        watchdog(async {
            let mut server =
                Server::new_tcp_server(LinkErrorMode::Close, "127.0.0.1:0".parse().unwrap());
            // registered first, so routing that ignored the filters would pick it
            let mut other = add_outstation(&mut server, exact("192.0.2.1")).unwrap();
            let mut local = add_outstation(&mut server, exact("127.0.0.1")).unwrap();
            let (_handle, mut peers) = run_in_memory(server);

            let _client = peers.connect(LOOPBACK);
            assert_eq!(local.recv().await, Some(ConnectionState::Connected));
            assert_eq!(other.try_recv(), Err(mpsc::error::TryRecvError::Empty));
        })
        .await;
    }

    #[tokio::test]
    async fn a_connection_no_filter_matches_is_closed() {
        watchdog(async {
            let (server, mut states) = server_with_filter(exact("192.0.2.1"));
            let (_handle, mut peers) = run_in_memory(server);

            let (mut client, _) = peers.connect(LOOPBACK);
            let mut buffer = [0; 32];
            assert_eq!(
                tokio::io::AsyncReadExt::read(&mut client, &mut buffer)
                    .await
                    .unwrap(),
                0
            );
            assert_eq!(states.try_recv(), Err(mpsc::error::TryRecvError::Empty));
        })
        .await;
    }

    #[tokio::test]
    async fn a_filter_is_checked_against_every_registered_one() {
        // each conflicts with exactly one of the registered filters, in order
        let conflicting = ["127.0.0.1", "192.0.2.1", "10.0.0.7"];
        for addr in conflicting {
            let mut server =
                Server::new_tcp_server(LinkErrorMode::Close, "127.0.0.1:0".parse().unwrap());
            add_outstation(&mut server, exact("127.0.0.1")).unwrap();
            add_outstation(&mut server, exact("192.0.2.1")).unwrap();
            add_outstation(
                &mut server,
                AddressFilter::WildcardIpv4("10.0.0.*".parse().unwrap()),
            )
            .unwrap();
            assert_eq!(
                add_outstation(
                    &mut server,
                    AddressFilter::AnyOf([addr.parse().unwrap()].into())
                )
                .err(),
                Some(FilterError::Conflict),
                "{addr}"
            );
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
            let (listener, mut peers) = FaultyListener::new(&[UNKNOWN, RESET, UNKNOWN]);
            let _client = peers.connect(LOOPBACK);
            let (server, mut states) = server_with_outstation();
            let (_handle, task) = server.create_task_with(listener, None);
            tokio::spawn(task.run());

            expect_attempt_gaps(
                &mut peers.attempts,
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
            let (listener, mut peers) = FaultyListener::new(&[UNKNOWN]);
            let (server, _states) = server_with_outstation();
            let (handle, task) = server.create_task_with(listener, None);
            let task = tokio::spawn(task.run());

            let failed_at = peers.attempts.recv().await.unwrap();
            tokio::time::advance(Duration::from_millis(500)).await;
            assert!(!task.is_finished());

            drop(handle);
            task.await.unwrap();
            assert!(failed_at.elapsed() < ACCEPT_ERROR_DELAY);
            assert_eq!(peers.attempts.recv().await, None);
        })
        .await;
    }

    #[tokio::test]
    async fn dropping_handle_during_connection_errors_stops_server() {
        static RESETS: [Fault; 1000] = [RESET; 1000];
        watchdog(async {
            let (listener, mut peers) = FaultyListener::new(&RESETS);
            let (server, _states) = server_with_outstation();
            let (handle, task) = server.create_task_with(listener, None);
            let task = tokio::spawn(task.run());

            peers.attempts.recv().await.unwrap();
            drop(handle);
            task.await.unwrap();
            let mut count = 1;
            while peers.attempts.recv().await.is_some() {
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

    /// How the server hands connections to an outstation, ends them and reports them, over
    /// in-memory connections. The clock is paused and only advances once every task is idle, so
    /// after a sleep the server and the outstation have processed everything sent before it.
    mod sessions {
        use super::*;
        use crate::outstation::{DefaultControlHandler, Feature};
        use crate::tcp::accept::test_util::assert_closed;
        use crate::util::test_support::{Timeline, MASTER_REQUEST, OUTSTATION_REPLY};
        use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};
        use tokio::task::JoinHandle;

        const START: Duration = Duration::ZERO;
        const SETTLE: Duration = Duration::from_millis(1);

        struct Fixture {
            server: ServerHandle,
            server_task: JoinHandle<Shutdown>,
            outstation: OutstationHandle,
            outstation_task: JoinHandle<()>,
            states: Timeline<ConnectionState>,
            peers: Peers,
        }

        /// a server in `LinkErrorMode::Close` with outstation 1024, which sends nothing until it is
        /// asked, running on an in-memory listener
        fn spawn() -> Fixture {
            let mut config = crate::util::test_support::outstation_config(1024, 1);
            config.features.unsolicited = Feature::Disabled;
            let mut server =
                Server::new_tcp_server(LinkErrorMode::Close, "127.0.0.1:0".parse().unwrap());
            let (listener, states) = Timeline::new();
            let (outstation, future) = server
                .add_outstation_no_spawn(
                    config,
                    Box::new(NullApplication),
                    Box::new(NullInformation),
                    DefaultControlHandler::create(),
                    listener,
                    AddressFilter::Any,
                )
                .unwrap();
            let outstation_task = tokio::spawn(future);
            let (accept, peers) = FaultyListener::new(&[]);
            let (server, task) = server.create_task_with(accept, None);
            Fixture {
                server,
                server_task: tokio::spawn(task.run()),
                outstation,
                outstation_task,
                states,
                peers,
            }
        }

        async fn settle() {
            tokio::time::sleep(SETTLE).await;
        }

        /// the outstation answers a link status request on `peer`
        async fn answers(peer: &mut DuplexStream) {
            peer.write_all(&MASTER_REQUEST).await.unwrap();
            let mut reply = [0; 10];
            peer.read_exact(&mut reply).await.unwrap();
            assert_eq!(reply, OUTSTATION_REPLY);
        }

        #[tokio::test(start_paused = true)]
        async fn a_new_connection_replaces_the_session() {
            watchdog(async {
                let mut fixture = spawn();
                let (mut first, _) = fixture.peers.connect(LOOPBACK);
                answers(&mut first).await;
                fixture
                    .states
                    .expect(START, ConnectionState::Connected)
                    .await;

                let (mut second, _) = fixture.peers.connect(LOOPBACK);
                answers(&mut second).await;
                fixture
                    .states
                    .expect(START, ConnectionState::Disconnected)
                    .await;
                fixture
                    .states
                    .expect(START, ConnectionState::Connected)
                    .await;
                assert_closed(&mut first).await;

                settle().await;
                fixture.states.assert_quiet();
                answers(&mut second).await;
            })
            .await;
        }

        #[tokio::test(start_paused = true)]
        async fn a_corrupt_frame_ends_the_session_and_the_next_connection_is_served() {
            watchdog(async {
                let mut fixture = spawn();
                let (mut first, _) = fixture.peers.connect(LOOPBACK);
                fixture
                    .states
                    .expect(START, ConnectionState::Connected)
                    .await;

                let mut corrupt = MASTER_REQUEST;
                corrupt[9] ^= 0xFF;
                first.write_all(&corrupt).await.unwrap();
                assert_closed(&mut first).await;
                fixture
                    .states
                    .expect(START, ConnectionState::Disconnected)
                    .await;

                let (mut second, _) = fixture.peers.connect(LOOPBACK);
                answers(&mut second).await;
                fixture
                    .states
                    .expect(START, ConnectionState::Connected)
                    .await;
            })
            .await;
        }

        #[tokio::test(start_paused = true)]
        async fn disabling_closes_the_session_and_connections_until_enabled() {
            watchdog(async {
                let mut fixture = spawn();
                let (mut first, _) = fixture.peers.connect(LOOPBACK);
                fixture
                    .states
                    .expect(START, ConnectionState::Connected)
                    .await;

                fixture.outstation.disable().await.unwrap();
                assert_closed(&mut first).await;
                fixture
                    .states
                    .expect(START, ConnectionState::Disconnected)
                    .await;

                // a connection while disabled is closed without starting a session
                let (mut second, _) = fixture.peers.connect(LOOPBACK);
                settle().await;
                fixture.states.assert_quiet();
                assert_closed(&mut second).await;

                fixture.outstation.enable().await.unwrap();
                settle().await;
                let (mut third, _) = fixture.peers.connect(LOOPBACK);
                answers(&mut third).await;
                fixture
                    .states
                    .expect(Duration::from_millis(2), ConnectionState::Connected)
                    .await;
            })
            .await;
        }

        #[tokio::test(start_paused = true)]
        async fn shutting_down_an_idle_outstation_ends_its_task_and_closes_later_connections() {
            watchdog(async {
                let mut fixture = spawn();
                fixture.outstation.shutdown().await.unwrap();
                fixture.outstation_task.await.unwrap();

                let (mut peer, _) = fixture.peers.connect(LOOPBACK);
                assert_closed(&mut peer).await;
                fixture.states.assert_quiet();
                // the server keeps running
                assert!(!fixture.server_task.is_finished());
            })
            .await;
        }

        #[tokio::test(start_paused = true)]
        async fn dropping_the_server_handle_ends_an_idle_outstation() {
            watchdog(async {
                let mut fixture = spawn();
                drop(fixture.server);
                let Shutdown = fixture.server_task.await.unwrap();
                fixture.outstation_task.await.unwrap();
                assert_eq!(fixture.states.now(), START);
                fixture.states.assert_quiet();
            })
            .await;
        }
    }
}
