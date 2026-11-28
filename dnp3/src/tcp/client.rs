// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::Listener;
use crate::app::Shutdown;
use crate::link::error::LinkError;
use crate::phys::PhysLayer;
use crate::session::connection::{run_connection, Connection, OpenError};
use crate::session::Session;
use crate::tcp::{
    ClientConnectionHandler, ClientState, ConnectionInfo, EndpointInner, PostConnectionHandler,
};
use crate::util::shutdown::ShutdownListener;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpSocket, TcpStream};

#[cfg(test)]
pub(crate) use tests::BLACK_HOLE;
#[cfg(test)]
use tests::{connect, lookup_host};
#[cfg(not(test))]
use tokio::net::lookup_host;

#[cfg(not(test))]
fn connect(
    socket: TcpSocket,
    addr: SocketAddr,
) -> impl std::future::Future<Output = std::io::Result<TcpStream>> {
    socket.connect(addr)
}

pub(crate) struct ClientTask {
    session: Session,
    connect_handler: Box<dyn ClientConnectionHandler>,
    post_connection: PostConnectionHandler,
    listener: Box<dyn Listener<ClientState>>,
}

impl ClientTask {
    pub(crate) fn new(
        session: Session,
        connect_handler: Box<dyn ClientConnectionHandler>,
        post_connection: PostConnectionHandler,
        listener: Box<dyn Listener<ClientState>>,
    ) -> Self {
        Self {
            connect_handler,
            post_connection,
            session,
            listener,
        }
    }

    pub(crate) async fn run(mut self) {
        run_connection(&mut self).await;
        let Self {
            session,
            mut listener,
            ..
        } = self;
        session.finish().await;
        listener.update(ClientState::Shutdown).get().await;
    }

    async fn connect(&mut self) -> Result<(PhysLayer, SocketAddr, Option<Arc<String>>), OpenError> {
        let mut shutdown = self.session.shutdown_listener();
        loop {
            // the handler can keep supplying endpoints that fail without waiting
            if shutdown.is_shutdown() {
                return Err(OpenError::Shutdown);
            }
            let conn_info = self.connect_handler.next().map_err(OpenError::Retry)?;
            let settings = conn_info.settings;
            let connected = self
                .connect_to_endpoint(conn_info, &mut shutdown)
                .await
                .map_err(|Shutdown| OpenError::Shutdown)?;
            if let Some((phys, addr, hostname)) = connected {
                if let Some(master_address) = settings.master_address {
                    self.session.change_master_address(master_address);
                }

                self.connect_handler
                    .connected(addr, hostname.as_ref().map(|x| x.as_str()));
                return Ok((phys, addr, hostname));
            }
        }
    }

    /// Connect to one endpoint, or fail if shutdown is requested while resolving or connecting
    async fn connect_to_endpoint(
        &mut self,
        info: ConnectionInfo,
        shutdown: &mut ShutdownListener,
    ) -> Result<Option<(PhysLayer, SocketAddr, Option<Arc<String>>)>, Shutdown> {
        let hostname = match info.endpoint.inner {
            EndpointInner::Address(addr) => {
                let phys = self
                    .connect_to_addr(addr, info.timeout, info.local_endpoint, None, shutdown)
                    .await?;
                return Ok(phys.map(|x| (x, addr, None)));
            }
            EndpointInner::Hostname(hostname) => hostname,
        };

        let resolved = tokio::select! {
            _ = shutdown.wait() => return Err(Shutdown),
            resolved = lookup_host(hostname.as_str()) => resolved,
        };
        let addrs = match resolved {
            Ok(addrs) => addrs,
            Err(err) => {
                tracing::warn!("Unable to resolve: '{hostname}', err: {err}");
                self.connect_handler.resolution_failed(hostname.as_str());
                return Ok(None);
            }
        };

        for addr in addrs {
            let phys = self
                .connect_to_addr(
                    addr,
                    info.timeout,
                    info.local_endpoint,
                    Some(hostname.as_str()),
                    shutdown,
                )
                .await?;
            if let Some(x) = phys {
                return Ok(Some((x, addr, Some(hostname.clone()))));
            }
        }
        Ok(None)
    }

    async fn connect_to_addr(
        &mut self,
        addr: SocketAddr,
        timeout: Option<Duration>,
        local_endpoint: Option<SocketAddr>,
        hostname: Option<&str>,
        shutdown: &mut ShutdownListener,
    ) -> Result<Option<PhysLayer>, Shutdown> {
        let result = if addr.is_ipv4() {
            TcpSocket::new_v4()
        } else {
            TcpSocket::new_v6()
        };

        let socket = match result {
            Ok(x) => x,
            Err(err) => {
                tracing::warn!("unable to create socket: {}", err);
                return Ok(None);
            }
        };

        if let Some(local) = local_endpoint {
            if let Err(err) = socket.bind(local) {
                tracing::warn!("unable to bind socket to {}: {}", local, err);
                return Ok(None);
            }
        }

        match hostname {
            None => tracing::info!("connecting to {addr}"),
            Some(host) => tracing::info!("connecting to {addr} ({host})"),
        }
        self.connect_handler.connecting(addr, hostname);

        // covers the connect and any TLS handshake; a shutdown here is not a failed connect
        let phys = tokio::select! {
            _ = shutdown.wait() => return Err(Shutdown),
            phys = Self::connect_socket(&self.post_connection, socket, addr, timeout) => phys,
        };
        if phys.is_none() {
            self.connect_handler.connect_failed(addr, hostname);
        }
        Ok(phys)
    }

    /// Not a method: holding `&self` across its awaits would need the task to be `Sync`
    async fn connect_socket(
        post_connection: &PostConnectionHandler,
        socket: TcpSocket,
        addr: SocketAddr,
        timeout: Option<Duration>,
    ) -> Option<PhysLayer> {
        let fut = connect(socket, addr);
        let result = match timeout {
            None => fut.await,
            Some(timeout) => match tokio::time::timeout(timeout, fut).await {
                Ok(x) => x,
                Err(_) => {
                    tracing::warn!(
                        "unable to connect to {} within timeout of {:?}",
                        addr,
                        timeout
                    );
                    return None;
                }
            },
        };

        let stream = match result {
            Ok(x) => x,
            Err(err) => {
                tracing::warn!("failed to connect to {}: {}", addr, err);
                return None;
            }
        };

        crate::tcp::configure_client(&stream);

        let phys = post_connection.post_connect(stream, &addr).await?;

        tracing::info!("connected to {}", addr);

        Some(phys)
    }
}

impl Connection for ClientTask {
    type Peer = (SocketAddr, Option<Arc<String>>);

    fn session(&mut self) -> &mut Session {
        &mut self.session
    }

    async fn disabled(&mut self) {
        self.listener.update(ClientState::Disabled).get().await;
    }

    async fn open(&mut self) -> Result<(PhysLayer, Self::Peer), OpenError> {
        self.listener.update(ClientState::Connecting).get().await;

        match self.connect().await {
            Ok((phys, addr, hostname)) => {
                self.listener.update(ClientState::Connected).get().await;
                Ok((phys, (addr, hostname)))
            }
            Err(OpenError::Shutdown) => Err(OpenError::Shutdown),
            Err(OpenError::Retry(delay)) => {
                tracing::info!("waiting {} ms to retry connection", delay.as_millis());
                self.listener
                    .update(ClientState::WaitAfterFailedConnect(delay))
                    .get()
                    .await;
                Err(OpenError::Retry(delay))
            }
        }
    }

    async fn failed(&mut self, (addr, hostname): Self::Peer, err: LinkError) -> Duration {
        tracing::warn!("connection to {addr} failed: {err}");
        let hostname = hostname.as_ref().map(|x| x.as_str());
        let reconnect_delay = self.connect_handler.disconnected(addr, hostname);

        self.listener
            .update(ClientState::WaitAfterDisconnect(reconnect_delay))
            .get()
            .await;

        if !reconnect_delay.is_zero() {
            tracing::warn!("waiting {reconnect_delay:?} to reconnect");
        }
        reconnect_delay
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::NullListener;
    use crate::link::reader::LinkModes;
    use crate::link::EndpointAddress;
    use crate::master::{MasterChannel, MasterChannelConfig, MasterChannelType};
    use crate::tcp::{wire_master_client, Endpoint};
    use crate::util::test_support::{watchdog, Timeline};
    use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

    /// a hostname the test resolver answers with the addresses after the prefix, separated by
    /// commas, so a test controls the resolution without DNS
    const CANNED: &str = "canned:";

    /// [`tokio::net::lookup_host`], except for a hostname starting with [`CANNED`]
    pub(super) async fn lookup_host(host: &str) -> std::io::Result<std::vec::IntoIter<SocketAddr>> {
        let addrs: Vec<SocketAddr> = match host.strip_prefix(CANNED) {
            Some(list) => list
                .split(',')
                .filter(|x| !x.is_empty())
                .map(|x| x.parse().unwrap())
                .collect(),
            None => tokio::net::lookup_host(host).await?.collect(),
        };
        Ok(addrs.into_iter())
    }

    /// an address the test connect never reaches, so only a timeout ends an attempt to it
    pub(crate) const BLACK_HOLE: SocketAddr = SocketAddr::new(
        std::net::IpAddr::V4(std::net::Ipv4Addr::new(192, 0, 2, 1)),
        20000,
    );

    /// [`TcpSocket::connect`], except for [`BLACK_HOLE`]
    pub(super) async fn connect(socket: TcpSocket, addr: SocketAddr) -> std::io::Result<TcpStream> {
        if addr == BLACK_HOLE {
            std::future::pending().await
        } else {
            socket.connect(addr).await
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    enum Event {
        Connecting(SocketAddr, Option<String>),
        ConnectFailed(SocketAddr, Option<String>),
        Connected(SocketAddr, Option<String>),
        ResolutionFailed(String),
    }

    struct RecordingHandler {
        attempts: std::collections::VecDeque<ConnectionInfo>,
        events: UnboundedSender<Event>,
    }

    impl RecordingHandler {
        fn record(&self, event: Event) {
            drop(self.events.send(event));
        }
    }

    impl ClientConnectionHandler for RecordingHandler {
        fn endpoint_span_name(&self) -> String {
            "test".to_string()
        }

        fn disconnected(&mut self, _: SocketAddr, _: Option<&str>) -> Duration {
            Duration::from_secs(60)
        }

        fn next(&mut self) -> Result<ConnectionInfo, Duration> {
            // make each attempt once, then ask the task to sleep
            self.attempts.pop_front().ok_or(Duration::from_secs(60))
        }

        fn connecting(&mut self, addr: SocketAddr, hostname: Option<&str>) {
            self.record(Event::Connecting(addr, hostname.map(String::from)));
        }

        fn connect_failed(&mut self, addr: SocketAddr, hostname: Option<&str>) {
            self.record(Event::ConnectFailed(addr, hostname.map(String::from)));
        }

        fn connected(&mut self, addr: SocketAddr, hostname: Option<&str>) {
            self.record(Event::Connected(addr, hostname.map(String::from)));
        }

        fn resolution_failed(&mut self, hostname: &str) {
            self.record(Event::ResolutionFailed(hostname.to_string()));
        }
    }

    fn client_task(
        addr: SocketAddr,
        post_connection: PostConnectionHandler,
    ) -> (ClientTask, UnboundedReceiver<Event>, MasterChannel) {
        client_task_with(
            vec![ConnectionInfo::new(Endpoint::address(addr))],
            post_connection,
        )
    }

    /// a task that makes the attempts once each
    fn client_task_with(
        attempts: Vec<ConnectionInfo>,
        post_connection: PostConnectionHandler,
    ) -> (ClientTask, UnboundedReceiver<Event>, MasterChannel) {
        client_task_listening(attempts, post_connection, NullListener::create())
    }

    /// a task that makes the attempts once each and reports its state to `listener`, with the
    /// channel that keeps it from shutting down
    fn client_task_listening(
        attempts: Vec<ConnectionInfo>,
        post_connection: PostConnectionHandler,
        listener: Box<dyn Listener<ClientState>>,
    ) -> (ClientTask, UnboundedReceiver<Event>, MasterChannel) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let handler = RecordingHandler {
            attempts: attempts.into(),
            events: tx,
        };
        let (task, channel) = wire_master_client(
            LinkModes::test(),
            MasterChannelType::Stream,
            Box::new(handler),
            MasterChannelConfig::new(EndpointAddress::raw(1)),
            post_connection,
            listener,
        );
        (task, rx, channel)
    }

    fn events(rx: &mut UnboundedReceiver<Event>) -> Vec<Event> {
        let mut events = Vec::new();
        while let Ok(x) = rx.try_recv() {
            events.push(x);
        }
        events
    }

    /// a port that refuses connections, reserved while the returned socket lives
    fn refusing_port() -> (TcpSocket, SocketAddr) {
        // a bound socket that never listens
        let reserved = TcpSocket::new_v4().unwrap();
        reserved.bind("127.0.0.1:0".parse().unwrap()).unwrap();
        let addr = reserved.local_addr().unwrap();
        (reserved, addr)
    }

    /// the address and hostname of a successful connection
    fn connected_to(
        result: Result<(PhysLayer, SocketAddr, Option<Arc<String>>), OpenError>,
    ) -> Result<(SocketAddr, Option<String>), OpenError> {
        result.map(|(_, addr, hostname)| (addr, hostname.map(|x| x.to_string())))
    }

    // catches reporting a failed connect for one that shutdown abandoned
    #[tokio::test(start_paused = true)]
    async fn shutdown_during_a_connect_reports_no_failure() {
        watchdog(async {
            let (mut task, mut rx, channel) = client_task(BLACK_HOLE, PostConnectionHandler::Tcp);
            let shutdown = async {
                tokio::task::yield_now().await;
                channel.shutdown();
            };
            let (result, ()) = tokio::join!(task.connect(), shutdown);
            assert_eq!(result.err(), Some(OpenError::Shutdown));
            assert_eq!(events(&mut rx), [Event::Connecting(BLACK_HOLE, None)]);
        })
        .await;
    }

    #[tokio::test]
    async fn refused_connection_notifies_connect_failed() {
        let (reserved, addr) = refusing_port();
        let (mut task, mut rx, _channel) = client_task(addr, PostConnectionHandler::Tcp);

        assert_eq!(
            task.connect().await.err(),
            Some(OpenError::Retry(Duration::from_secs(60)))
        );
        assert_eq!(
            events(&mut rx),
            [
                Event::Connecting(addr, None),
                Event::ConnectFailed(addr, None)
            ]
        );
        drop(reserved);
    }

    #[tokio::test]
    async fn successful_connection_notifies_connected() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (mut task, mut rx, _channel) = client_task(addr, PostConnectionHandler::Tcp);

        assert_eq!(task.connect().await.ok().map(|(_, x, _)| x), Some(addr));
        assert_eq!(
            events(&mut rx),
            [Event::Connecting(addr, None), Event::Connected(addr, None)]
        );
    }

    #[cfg(feature = "enable-tls")]
    #[tokio::test]
    async fn failed_tls_handshake_notifies_connect_failed() {
        let config =
            crate::util::test_support::tls::client_config(crate::tcp::tls::MinTlsVersion::V12);

        // a server that closes the connection without answering the ClientHello
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            drop(socket);
        });

        let (mut task, mut rx, _channel) = client_task(addr, PostConnectionHandler::Tls(config));

        assert_eq!(
            task.connect().await.err(),
            Some(OpenError::Retry(Duration::from_secs(60)))
        );
        server.await.unwrap();
        assert_eq!(
            events(&mut rx),
            [
                Event::Connecting(addr, None),
                Event::ConnectFailed(addr, None)
            ]
        );
    }

    fn hostname(host: &str) -> ConnectionInfo {
        ConnectionInfo::new(Endpoint::hostname(host.to_string()))
    }

    #[tokio::test(start_paused = true)]
    async fn connect_timeout_moves_on_to_the_next_endpoint() {
        watchdog(async {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let mut black_hole = ConnectionInfo::new(Endpoint::address(BLACK_HOLE));
            black_hole.set_timeout(Duration::from_secs(3));
            let attempts = vec![black_hole, ConnectionInfo::new(Endpoint::address(addr))];
            let (mut task, mut rx, _channel) =
                client_task_with(attempts, PostConnectionHandler::Tcp);

            let start = tokio::time::Instant::now();
            assert_eq!(connected_to(task.connect().await), Ok((addr, None)));
            assert_eq!(start.elapsed(), Duration::from_secs(3));
            assert_eq!(
                events(&mut rx),
                [
                    Event::Connecting(BLACK_HOLE, None),
                    Event::ConnectFailed(BLACK_HOLE, None),
                    Event::Connecting(addr, None),
                    Event::Connected(addr, None),
                ]
            );
        })
        .await;
    }

    #[tokio::test]
    async fn hostname_tries_each_resolved_address_in_order() {
        watchdog(async {
            let (_reserved, refused) = refusing_port();
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let host = format!("{CANNED}{refused},{addr}");
            let (mut task, mut rx, _channel) =
                client_task_with(vec![hostname(&host)], PostConnectionHandler::Tcp);

            assert_eq!(
                connected_to(task.connect().await),
                Ok((addr, Some(host.clone())))
            );
            let host = Some(host);
            assert_eq!(
                events(&mut rx),
                [
                    Event::Connecting(refused, host.clone()),
                    Event::ConnectFailed(refused, host.clone()),
                    Event::Connecting(addr, host.clone()),
                    Event::Connected(addr, host),
                ]
            );
        })
        .await;
    }

    /// A listener on 127.0.0.1, and a socket that reserves the same port on ::1 without
    /// listening, so no other listener can answer a connection to localhost on that port
    ///
    /// There is no reservation if the host has no IPv6 loopback, which localhost can't then reach.
    async fn localhost_listener() -> (tokio::net::TcpListener, Option<TcpSocket>) {
        for _ in 0..100 {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let Ok(reserved) = TcpSocket::new_v6() else {
                return (listener, None);
            };
            match reserved.bind((std::net::Ipv6Addr::LOCALHOST, port).into()) {
                Ok(()) => return (listener, Some(reserved)),
                // the port is taken on ::1, so pick another
                Err(err) if err.kind() == std::io::ErrorKind::AddrInUse => {}
                Err(_) => return (listener, None),
            }
        }
        panic!("no port was free on both loopback addresses");
    }

    #[tokio::test]
    async fn localhost_reaches_a_loopback_listener() {
        watchdog(async {
            let (listener, _reserved) = localhost_listener().await;
            let addr = listener.local_addr().unwrap();
            let host = format!("localhost:{}", addr.port());
            let (mut task, mut rx, _channel) =
                client_task_with(vec![hostname(&host)], PostConnectionHandler::Tcp);

            assert_eq!(
                connected_to(task.connect().await),
                Ok((addr, Some(host.clone())))
            );
            let host = Some(host);
            let events = events(&mut rx);
            // localhost may also resolve to ::1, which refuses the connection, and try it first
            let (events, last) = events
                .split_at_checked(events.len().saturating_sub(2))
                .unwrap();
            assert_eq!(
                last,
                [
                    Event::Connecting(addr, host.clone()),
                    Event::Connected(addr, host.clone())
                ]
            );
            let (pairs, rest) = events.as_chunks::<2>();
            assert!(rest.is_empty(), "unexpected events {rest:?}");
            for pair in pairs {
                let [Event::Connecting(x, h1), Event::ConnectFailed(y, h2)] = pair else {
                    panic!("unexpected events {pair:?}");
                };
                assert_eq!((x, h1, h2), (y, &host, &host));
                assert_ne!(*x, addr);
            }
        })
        .await;
    }

    #[tokio::test]
    async fn failed_resolution_notifies_resolution_failed() {
        watchdog(async {
            // without a port, the lookup fails before any DNS query
            let (mut task, mut rx, _channel) =
                client_task_with(vec![hostname("no-port")], PostConnectionHandler::Tcp);

            assert_eq!(
                connected_to(task.connect().await),
                Err(OpenError::Retry(Duration::from_secs(60)))
            );
            assert_eq!(
                events(&mut rx),
                [Event::ResolutionFailed("no-port".to_string())]
            );
        })
        .await;
    }

    // A failed local bind and a hostname that resolves to no addresses aren't connection attempts:
    // the handler hears nothing of them, and the task moves on to the next endpoint

    #[tokio::test]
    async fn hostname_resolving_to_no_addresses_notifies_nothing() {
        watchdog(async {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let attempts = vec![
                hostname(CANNED),
                ConnectionInfo::new(Endpoint::address(addr)),
            ];
            let (mut task, mut rx, _channel) =
                client_task_with(attempts, PostConnectionHandler::Tcp);

            assert_eq!(connected_to(task.connect().await), Ok((addr, None)));
            assert_eq!(
                events(&mut rx),
                [Event::Connecting(addr, None), Event::Connected(addr, None)]
            );
        })
        .await;
    }

    #[tokio::test]
    async fn failed_local_bind_notifies_nothing() {
        watchdog(async {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            // the listener holds the local endpoint, so binding to it fails
            let mut bind_fails = ConnectionInfo::new(Endpoint::address(addr));
            bind_fails.set_local_endpoint(addr);
            let attempts = vec![bind_fails, ConnectionInfo::new(Endpoint::address(addr))];
            let (mut task, mut rx, _channel) =
                client_task_with(attempts, PostConnectionHandler::Tcp);

            assert_eq!(connected_to(task.connect().await), Ok((addr, None)));
            assert_eq!(
                events(&mut rx),
                [Event::Connecting(addr, None), Event::Connected(addr, None)]
            );
        })
        .await;
    }

    #[tokio::test]
    async fn failed_local_bind_waits_for_the_next_attempt() {
        watchdog(async {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            // the listener holds the local endpoint, so binding to it fails
            let mut bind_fails = ConnectionInfo::new(Endpoint::address(addr));
            bind_fails.set_local_endpoint(addr);
            let (states, mut timeline) = Timeline::new();
            let (mut task, mut rx, _channel) =
                client_task_listening(vec![bind_fails], PostConnectionHandler::Tcp, states);

            assert_eq!(
                task.open().await.err(),
                Some(OpenError::Retry(Duration::from_secs(60)))
            );
            timeline.expect_state(ClientState::Connecting).await;
            timeline
                .expect_state(ClientState::WaitAfterFailedConnect(Duration::from_secs(60)))
                .await;
            timeline.assert_quiet();
            assert_eq!(events(&mut rx), []);
        })
        .await;
    }

    // every 127.0.0.0/8 address is on the loopback interface on Linux, but not on all platforms
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn connects_from_the_local_endpoint() {
        watchdog(async {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let local = std::net::Ipv4Addr::new(127, 0, 0, 2);
            let mut info = ConnectionInfo::new(Endpoint::address(addr));
            info.set_local_endpoint((local, 0).into());
            let (mut task, mut rx, _channel) =
                client_task_with(vec![info], PostConnectionHandler::Tcp);

            assert_eq!(connected_to(task.connect().await), Ok((addr, None)));
            let (_stream, peer) = listener.accept().await.unwrap();
            assert_eq!(peer.ip(), local);
            assert_eq!(
                events(&mut rx),
                [Event::Connecting(addr, None), Event::Connected(addr, None)]
            );
        })
        .await;
    }
}
