//! Behavior shared by the TCP and TLS client spawn functions of the master and the outstation

use std::time::Duration;

use tokio::net::TcpListener;
use tokio::sync::mpsc;

use crate::app::{ConnectStrategy, Listener, MaybeAsync};
use crate::decode::PhysDecodeLevel;
use crate::link::{EndpointAddress, LinkErrorMode};
use crate::master::MasterChannelConfig;
use crate::outstation::database::EventBufferConfig;
use crate::outstation::{
    DefaultControlHandler, OutstationApplication, OutstationConfig, OutstationInformation,
};
use crate::tcp::{ClientState, ConnectOptions, EndpointList};
use crate::util::phys::{PhysAddr, PhysLayer};
use crate::util::test_support::{
    MASTER_REPLY, MASTER_REQUEST, NULL_UNSOLICITED_HEADER, OUTSTATION_REPLY, OUTSTATION_REQUEST,
};

const MASTER: u16 = 1;
const OUTSTATION: u16 = 1024;

const TIMEOUT: Duration = Duration::from_secs(5);

struct StateListener(mpsc::UnboundedSender<ClientState>);

impl Listener<ClientState> for StateListener {
    fn update(&mut self, value: ClientState) -> MaybeAsync<()> {
        // the test may have finished and dropped the receiver
        let _: Result<(), _> = self.0.send(value);
        MaybeAsync::ready(())
    }
}

struct States(mpsc::UnboundedReceiver<ClientState>);

impl States {
    fn new() -> (Box<dyn Listener<ClientState>>, Self) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Box::new(StateListener(tx)), Self(rx))
    }

    async fn expect(&mut self, expected: ClientState) {
        let state = tokio::time::timeout(TIMEOUT, self.0.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(state, expected);
    }
}

struct Peer {
    listener: TcpListener,
}

impl Peer {
    async fn new() -> Self {
        Self {
            listener: TcpListener::bind("127.0.0.1:0").await.unwrap(),
        }
    }

    fn endpoints(&self) -> EndpointList {
        EndpointList::new(self.listener.local_addr().unwrap().to_string(), &[])
    }

    async fn accept(&self) -> tokio::net::TcpStream {
        let (stream, _) = tokio::time::timeout(TIMEOUT, self.listener.accept())
            .await
            .unwrap()
            .unwrap();
        stream
    }
}

async fn read<const N: usize>(phys: &mut PhysLayer) -> [u8; N] {
    let mut buffer = [0; N];
    let mut count = 0;
    while count < N {
        let (n, _) = tokio::time::timeout(
            TIMEOUT,
            phys.read(&mut buffer[count..], PhysDecodeLevel::Nothing),
        )
        .await
        .unwrap()
        .unwrap();
        count = count.saturating_add(n);
    }
    buffer
}

async fn exchange(phys: &mut PhysLayer, request: &[u8], expected: &[u8]) {
    phys.write(request, PhysAddr::None, PhysDecodeLevel::Nothing)
        .await
        .unwrap();
    assert_eq!(read::<10>(phys).await, expected);
}

/// read the null unsolicited response an outstation sends when a connection opens
async fn read_null_unsolicited(phys: &mut PhysLayer) {
    let frame = read::<17>(phys).await;
    assert_eq!(frame[..10], NULL_UNSOLICITED_HEADER);
}

fn strategy() -> ConnectStrategy {
    ConnectStrategy::new(
        Duration::from_secs(1),
        Duration::from_secs(10),
        Duration::from_secs(1),
    )
}

fn master_config() -> MasterChannelConfig {
    MasterChannelConfig::new(EndpointAddress::try_new(MASTER).unwrap())
}

fn outstation_config() -> OutstationConfig {
    OutstationConfig::new(
        EndpointAddress::try_new(OUTSTATION).unwrap(),
        EndpointAddress::try_new(MASTER).unwrap(),
        EventBufferConfig::all_types(0),
    )
}

struct NullApplication;
impl OutstationApplication for NullApplication {}

struct NullInformation;
impl OutstationInformation for NullInformation {}

#[tokio::test]
async fn master_tcp_client_runs_a_master_session_over_tcp() {
    let peer = Peer::new().await;
    let (listener, mut states) = States::new();
    let mut channel = crate::tcp::spawn_master_tcp_client_2(
        LinkErrorMode::Close,
        master_config(),
        peer.endpoints(),
        strategy(),
        ConnectOptions::default(),
        listener,
    );
    states.expect(ClientState::Disabled).await;
    channel.enable().await.unwrap();
    states.expect(ClientState::Connecting).await;
    let mut phys = PhysLayer::tcp(peer.accept().await);
    states.expect(ClientState::Connected).await;
    exchange(&mut phys, &OUTSTATION_REQUEST, &MASTER_REPLY).await;
    drop(channel);
    states.expect(ClientState::Shutdown).await;
}

#[tokio::test]
async fn outstation_tcp_client_runs_an_outstation_session_over_tcp() {
    let peer = Peer::new().await;
    let (listener, mut states) = States::new();
    let mut handle = crate::tcp::spawn_outstation_tcp_client(
        LinkErrorMode::Close,
        peer.endpoints(),
        strategy(),
        ConnectOptions::default(),
        outstation_config(),
        Box::new(NullApplication),
        Box::new(NullInformation),
        DefaultControlHandler::create(),
        listener,
    );
    states.expect(ClientState::Disabled).await;
    handle.enable().await.unwrap();
    states.expect(ClientState::Connecting).await;
    let mut phys = PhysLayer::tcp(peer.accept().await);
    states.expect(ClientState::Connected).await;
    read_null_unsolicited(&mut phys).await;
    exchange(&mut phys, &MASTER_REQUEST, &OUTSTATION_REPLY).await;
    drop(handle);
    states.expect(ClientState::Shutdown).await;
}

#[cfg(feature = "enable-tls")]
mod tls {
    use super::*;
    use crate::tcp::tls::{MinTlsVersion, TlsClientConfig, TlsServerConfig};
    use crate::util::phys::PhysIo;
    use std::path::PathBuf;

    fn certs() -> PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../certs/self_signed")
    }

    pub(super) fn client_config() -> TlsClientConfig {
        let certs = certs();
        TlsClientConfig::self_signed(
            &certs.join("entity2_cert.pem"),
            &certs.join("entity1_cert.pem"),
            &certs.join("entity1_key.pem"),
            None,
            MinTlsVersion::V12,
        )
        .unwrap()
    }

    pub(super) fn server_config() -> TlsServerConfig {
        let certs = certs();
        TlsServerConfig::self_signed(
            &certs.join("entity1_cert.pem"),
            &certs.join("entity2_cert.pem"),
            &certs.join("entity2_key.pem"),
            None,
            MinTlsVersion::V12,
        )
        .unwrap()
    }

    async fn accept_tls(peer: &Peer) -> PhysLayer {
        server_config()
            .handle_connection(peer.accept().await)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn master_tls_client_runs_a_master_session_over_tls() {
        let peer = Peer::new().await;
        let (listener, mut states) = States::new();
        let mut channel = crate::tcp::tls::spawn_master_tls_client_2(
            LinkErrorMode::Close,
            master_config(),
            peer.endpoints(),
            strategy(),
            ConnectOptions::default(),
            listener,
            client_config(),
        );
        states.expect(ClientState::Disabled).await;
        channel.enable().await.unwrap();
        states.expect(ClientState::Connecting).await;
        let mut phys = accept_tls(&peer).await;
        assert!(matches!(phys.io(), PhysIo::Tls(_)));
        states.expect(ClientState::Connected).await;
        exchange(&mut phys, &OUTSTATION_REQUEST, &MASTER_REPLY).await;
        drop(channel);
        states.expect(ClientState::Shutdown).await;
    }

    #[tokio::test]
    async fn outstation_tls_client_runs_an_outstation_session_over_tls() {
        let peer = Peer::new().await;
        let (listener, mut states) = States::new();
        let mut handle = crate::tcp::tls::spawn_outstation_tls_client(
            LinkErrorMode::Close,
            peer.endpoints(),
            strategy(),
            ConnectOptions::default(),
            outstation_config(),
            Box::new(NullApplication),
            Box::new(NullInformation),
            DefaultControlHandler::create(),
            listener,
            client_config(),
        );
        states.expect(ClientState::Disabled).await;
        handle.enable().await.unwrap();
        states.expect(ClientState::Connecting).await;
        let mut phys = accept_tls(&peer).await;
        states.expect(ClientState::Connected).await;
        read_null_unsolicited(&mut phys).await;
        exchange(&mut phys, &MASTER_REQUEST, &OUTSTATION_REPLY).await;
        drop(handle);
        states.expect(ClientState::Shutdown).await;
    }

    #[tokio::test]
    async fn master_tls_client_does_not_connect_to_a_plain_tcp_peer() {
        let peer = Peer::new().await;
        let (listener, mut states) = States::new();
        let mut channel = crate::tcp::tls::spawn_master_tls_client_2(
            LinkErrorMode::Close,
            master_config(),
            peer.endpoints(),
            strategy(),
            ConnectOptions::default(),
            listener,
            client_config(),
        );
        states.expect(ClientState::Disabled).await;
        channel.enable().await.unwrap();
        states.expect(ClientState::Connecting).await;
        // the peer closes the connection instead of answering the ClientHello
        drop(peer.accept().await);
        states
            .expect(ClientState::WaitAfterFailedConnect(Duration::from_secs(1)))
            .await;
    }

    #[tokio::test]
    async fn outstation_tls_client_does_not_connect_to_a_plain_tcp_peer() {
        let peer = Peer::new().await;
        let (listener, mut states) = States::new();
        let mut handle = crate::tcp::tls::spawn_outstation_tls_client(
            LinkErrorMode::Close,
            peer.endpoints(),
            strategy(),
            ConnectOptions::default(),
            outstation_config(),
            Box::new(NullApplication),
            Box::new(NullInformation),
            DefaultControlHandler::create(),
            listener,
            client_config(),
        );
        states.expect(ClientState::Disabled).await;
        handle.enable().await.unwrap();
        states.expect(ClientState::Connecting).await;
        // the peer closes the connection instead of answering the ClientHello
        drop(peer.accept().await);
        states
            .expect(ClientState::WaitAfterFailedConnect(Duration::from_secs(1)))
            .await;
    }
}

/// The connect, retry, disable and shutdown sequence of a TCP or TLS client
///
/// The tests with a paused clock run a master with no associations, so the client's own waits
/// (and the TLS handshake timeout) are the only timers while a task waits on a socket, and the
/// clock jumps only to the end of them.
mod lifecycle {
    use super::*;
    use crate::master::MasterChannel;
    use crate::util::test_support::{watchdog, Timeline};
    use std::io::Read;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpSocket;

    const fn secs(x: u64) -> Duration {
        Duration::from_secs(x)
    }

    /// A socket bound to a local port without listening, so connections to it are refused
    fn refusing_port() -> (TcpSocket, EndpointList) {
        let socket = TcpSocket::new_v4().unwrap();
        socket.bind("127.0.0.1:0".parse().unwrap()).unwrap();
        let endpoints = EndpointList::new(socket.local_addr().unwrap().to_string(), &[]);
        (socket, endpoints)
    }

    fn spawn_master(
        endpoints: EndpointList,
        strategy: ConnectStrategy,
    ) -> (MasterChannel, Timeline<ClientState>) {
        let (listener, timeline) = Timeline::new();
        let channel = crate::tcp::spawn_master_tcp_client_2(
            LinkErrorMode::Close,
            master_config(),
            endpoints,
            strategy,
            ConnectOptions::default(),
            listener,
        );
        (channel, timeline)
    }

    /// Accept without a timeout, which would make a paused clock jump
    async fn accept(listener: &TcpListener) -> tokio::net::TcpStream {
        listener.accept().await.unwrap().0
    }

    async fn assert_closed(stream: &mut tokio::net::TcpStream) {
        let mut buffer = [0; 16];
        assert_eq!(stream.read(&mut buffer).await.unwrap(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn failed_connects_retry_with_growing_delays_until_a_session_resets_them() {
        watchdog(async {
            let (socket, endpoints) = refusing_port();
            let (mut channel, mut timeline) =
                spawn_master(endpoints, ConnectStrategy::new(secs(1), secs(4), secs(3)));
            timeline.expect(secs(0), ClientState::Disabled).await;
            channel.enable().await.unwrap();
            timeline.expect(secs(0), ClientState::Connecting).await;
            timeline
                .expect(secs(0), ClientState::WaitAfterFailedConnect(secs(1)))
                .await;
            timeline.expect(secs(1), ClientState::Connecting).await;
            timeline
                .expect(secs(1), ClientState::WaitAfterFailedConnect(secs(2)))
                .await;
            timeline.expect(secs(3), ClientState::Connecting).await;
            timeline
                .expect(secs(3), ClientState::WaitAfterFailedConnect(secs(4)))
                .await;
            timeline.expect(secs(7), ClientState::Connecting).await;
            timeline
                .expect(secs(7), ClientState::WaitAfterFailedConnect(secs(4)))
                .await;

            let listener = socket.listen(1024).unwrap();
            timeline.expect(secs(11), ClientState::Connecting).await;
            let stream = accept(&listener).await;
            timeline.expect(secs(11), ClientState::Connected).await;

            // refuse connections again, then end the session
            drop(listener);
            drop(stream);
            timeline
                .expect(secs(11), ClientState::WaitAfterDisconnect(secs(3)))
                .await;
            timeline.expect(secs(14), ClientState::Connecting).await;
            timeline
                .expect(secs(14), ClientState::WaitAfterFailedConnect(secs(1)))
                .await;
            timeline.expect(secs(15), ClientState::Connecting).await;
            timeline
                .expect(secs(15), ClientState::WaitAfterFailedConnect(secs(2)))
                .await;
            drop(channel);
            timeline.expect(secs(15), ClientState::Shutdown).await;
        })
        .await;
    }

    #[tokio::test(start_paused = true)]
    async fn link_error_keeps_the_connection_open_until_the_reconnect_delay_elapses() {
        watchdog(async {
            let peer = Peer::new().await;
            let (mut channel, mut timeline) = spawn_master(
                peer.endpoints(),
                ConnectStrategy::new(secs(1), secs(10), secs(3)),
            );
            timeline.expect(secs(0), ClientState::Disabled).await;
            channel.enable().await.unwrap();
            timeline.expect(secs(0), ClientState::Connecting).await;
            let mut stream = accept(&peer.listener).await;
            timeline.expect(secs(0), ClientState::Connected).await;

            // not a link frame, which ends the session in LinkErrorMode::Close
            stream.write_all(&[0; 10]).await.unwrap();
            timeline
                .expect(secs(0), ClientState::WaitAfterDisconnect(secs(3)))
                .await;
            let mut stream = stream.into_std().unwrap();
            let mut buffer = [0; 16];
            assert_eq!(
                stream.peek(&mut buffer).map_err(|err| err.kind()),
                Err(std::io::ErrorKind::WouldBlock)
            );

            timeline.expect(secs(3), ClientState::Connecting).await;
            // the client closed the old connection before connecting again
            stream.set_nonblocking(false).unwrap();
            stream.set_read_timeout(Some(TIMEOUT)).unwrap();
            assert_eq!(stream.read(&mut buffer).unwrap(), 0);
            timeline.expect(secs(3), ClientState::Connected).await;
            drop(channel);
            timeline.expect(secs(3), ClientState::Shutdown).await;
        })
        .await;
    }

    #[tokio::test(start_paused = true)]
    async fn zero_reconnect_delay_reconnects_at_once() {
        watchdog(async {
            let peer = Peer::new().await;
            let (mut channel, mut timeline) = spawn_master(
                peer.endpoints(),
                ConnectStrategy::new(secs(1), secs(10), Duration::ZERO),
            );
            timeline.expect(secs(0), ClientState::Disabled).await;
            channel.enable().await.unwrap();
            timeline.expect(secs(0), ClientState::Connecting).await;
            drop(accept(&peer.listener).await);
            timeline.expect(secs(0), ClientState::Connected).await;
            timeline
                .expect(secs(0), ClientState::WaitAfterDisconnect(Duration::ZERO))
                .await;
            timeline.expect(secs(0), ClientState::Connecting).await;
            let _stream = accept(&peer.listener).await;
            timeline.expect(secs(0), ClientState::Connected).await;
            drop(channel);
            timeline.expect(secs(0), ClientState::Shutdown).await;
        })
        .await;
    }

    #[tokio::test(start_paused = true)]
    async fn disable_during_a_connect_retry_wait_stops_at_once_and_keeps_the_back_off() {
        watchdog(async {
            let (_socket, endpoints) = refusing_port();
            let (mut channel, mut timeline) =
                spawn_master(endpoints, ConnectStrategy::new(secs(5), secs(20), secs(1)));
            timeline.expect(secs(0), ClientState::Disabled).await;
            channel.enable().await.unwrap();
            timeline.expect(secs(0), ClientState::Connecting).await;
            timeline
                .expect(secs(0), ClientState::WaitAfterFailedConnect(secs(5)))
                .await;
            channel.disable().await.unwrap();
            timeline.expect(secs(0), ClientState::Disabled).await;

            tokio::time::sleep(secs(30)).await;
            timeline.assert_quiet();
            channel.enable().await.unwrap();
            timeline.expect(secs(30), ClientState::Connecting).await;
            timeline
                .expect(secs(30), ClientState::WaitAfterFailedConnect(secs(10)))
                .await;
            drop(channel);
            timeline.expect(secs(30), ClientState::Shutdown).await;
        })
        .await;
    }

    #[tokio::test(start_paused = true)]
    async fn disable_during_a_reconnect_wait_stops_at_once() {
        watchdog(async {
            let peer = Peer::new().await;
            let (mut channel, mut timeline) = spawn_master(
                peer.endpoints(),
                ConnectStrategy::new(secs(1), secs(10), secs(5)),
            );
            timeline.expect(secs(0), ClientState::Disabled).await;
            channel.enable().await.unwrap();
            timeline.expect(secs(0), ClientState::Connecting).await;
            drop(accept(&peer.listener).await);
            timeline.expect(secs(0), ClientState::Connected).await;
            timeline
                .expect(secs(0), ClientState::WaitAfterDisconnect(secs(5)))
                .await;
            channel.disable().await.unwrap();
            timeline.expect(secs(0), ClientState::Disabled).await;

            tokio::time::sleep(secs(10)).await;
            timeline.assert_quiet();
            channel.enable().await.unwrap();
            timeline.expect(secs(10), ClientState::Connecting).await;
            let _stream = accept(&peer.listener).await;
            timeline.expect(secs(10), ClientState::Connected).await;
            drop(channel);
            timeline.expect(secs(10), ClientState::Shutdown).await;
        })
        .await;
    }

    #[tokio::test(start_paused = true)]
    async fn shutdown_during_a_connect_retry_wait_stops_at_once() {
        watchdog(async {
            let (_socket, endpoints) = refusing_port();
            let (mut channel, mut timeline) =
                spawn_master(endpoints, ConnectStrategy::new(secs(5), secs(20), secs(1)));
            timeline.expect(secs(0), ClientState::Disabled).await;
            channel.enable().await.unwrap();
            timeline.expect(secs(0), ClientState::Connecting).await;
            timeline
                .expect(secs(0), ClientState::WaitAfterFailedConnect(secs(5)))
                .await;
            drop(channel);
            timeline.expect(secs(0), ClientState::Shutdown).await;
        })
        .await;
    }

    #[tokio::test(start_paused = true)]
    async fn shutdown_during_a_reconnect_wait_stops_at_once() {
        watchdog(async {
            let peer = Peer::new().await;
            let (mut channel, mut timeline) = spawn_master(
                peer.endpoints(),
                ConnectStrategy::new(secs(1), secs(10), secs(5)),
            );
            timeline.expect(secs(0), ClientState::Disabled).await;
            channel.enable().await.unwrap();
            timeline.expect(secs(0), ClientState::Connecting).await;
            drop(accept(&peer.listener).await);
            timeline.expect(secs(0), ClientState::Connected).await;
            timeline
                .expect(secs(0), ClientState::WaitAfterDisconnect(secs(5)))
                .await;
            drop(channel);
            timeline.expect(secs(0), ClientState::Shutdown).await;
        })
        .await;
    }

    #[tokio::test(start_paused = true)]
    async fn disable_during_a_session_closes_the_connection_until_enabled() {
        watchdog(async {
            let peer = Peer::new().await;
            let (mut channel, mut timeline) = spawn_master(
                peer.endpoints(),
                ConnectStrategy::new(secs(1), secs(10), secs(5)),
            );
            timeline.expect(secs(0), ClientState::Disabled).await;
            channel.enable().await.unwrap();
            timeline.expect(secs(0), ClientState::Connecting).await;
            let mut stream = accept(&peer.listener).await;
            timeline.expect(secs(0), ClientState::Connected).await;
            channel.disable().await.unwrap();
            timeline.expect(secs(0), ClientState::Disabled).await;
            assert_closed(&mut stream).await;

            tokio::time::sleep(secs(10)).await;
            timeline.assert_quiet();
            channel.enable().await.unwrap();
            timeline.expect(secs(10), ClientState::Connecting).await;
            let mut stream = accept(&peer.listener).await;
            timeline.expect(secs(10), ClientState::Connected).await;
            drop(channel);
            timeline.expect(secs(10), ClientState::Shutdown).await;
            assert_closed(&mut stream).await;
        })
        .await;
    }

    /// Whether or not the connection was already established when the disable arrived, the
    /// client connects and then closes the session
    #[tokio::test]
    async fn disable_while_connecting_takes_effect_once_connected() {
        let peer = Peer::new().await;
        let (listener, mut states) = States::new();
        let mut channel = crate::tcp::spawn_master_tcp_client_2(
            LinkErrorMode::Close,
            master_config(),
            peer.endpoints(),
            strategy(),
            ConnectOptions::default(),
            listener,
        );
        states.expect(ClientState::Disabled).await;
        channel.enable().await.unwrap();
        states.expect(ClientState::Connecting).await;
        channel.disable().await.unwrap();
        states.expect(ClientState::Connected).await;
        states.expect(ClientState::Disabled).await;
        let mut stream = peer.accept().await;
        tokio::time::timeout(TIMEOUT, assert_closed(&mut stream))
            .await
            .unwrap();
    }

    #[cfg(feature = "enable-tls")]
    mod tls {
        use super::super::tls::{client_config, server_config};
        use super::*;

        fn spawn_tls_master(peer: &Peer) -> (MasterChannel, Timeline<ClientState>) {
            let (listener, timeline) = Timeline::new();
            let channel = crate::tcp::tls::spawn_master_tls_client_2(
                LinkErrorMode::Close,
                master_config(),
                peer.endpoints(),
                strategy(),
                ConnectOptions::default(),
                listener,
                client_config(),
            );
            (channel, timeline)
        }

        /// The peer never answers the ClientHello, so the handshake times out after 5 seconds
        #[tokio::test(start_paused = true)]
        async fn disable_during_a_tls_handshake_takes_effect_once_it_fails() {
            watchdog(async {
                let peer = Peer::new().await;
                let (mut channel, mut timeline) = spawn_tls_master(&peer);
                timeline.expect(secs(0), ClientState::Disabled).await;
                channel.enable().await.unwrap();
                timeline.expect(secs(0), ClientState::Connecting).await;
                channel.disable().await.unwrap();
                timeline
                    .expect(secs(5), ClientState::WaitAfterFailedConnect(secs(1)))
                    .await;
                timeline.expect(secs(5), ClientState::Disabled).await;
            })
            .await;
        }

        /// The peer never answers the ClientHello, so the handshake times out after 5 seconds
        #[tokio::test(start_paused = true)]
        async fn shutdown_during_a_tls_handshake_takes_effect_once_it_fails() {
            watchdog(async {
                let peer = Peer::new().await;
                let (mut channel, mut timeline) = spawn_tls_master(&peer);
                timeline.expect(secs(0), ClientState::Disabled).await;
                channel.enable().await.unwrap();
                timeline.expect(secs(0), ClientState::Connecting).await;
                drop(channel);
                timeline
                    .expect(secs(5), ClientState::WaitAfterFailedConnect(secs(1)))
                    .await;
                timeline.expect(secs(5), ClientState::Shutdown).await;
            })
            .await;
        }

        async fn assert_tls_closed(phys: &mut PhysLayer) {
            let mut buffer = [0; 16];
            let result =
                tokio::time::timeout(TIMEOUT, phys.read(&mut buffer, PhysDecodeLevel::Nothing))
                    .await
                    .unwrap();
            // the OS reports the peer closing the connection differently: Windows as an aborted
            // connection, others as the end of the stream
            let kind = result.map(|(count, _)| count).map_err(|err| err.kind());
            assert!(
                matches!(
                    kind,
                    Err(std::io::ErrorKind::UnexpectedEof
                        | std::io::ErrorKind::ConnectionAborted
                        | std::io::ErrorKind::ConnectionReset)
                ),
                "{kind:?}"
            );
        }

        /// Spawn a TLS master, enable it, and accept its TCP connection without answering the
        /// ClientHello yet
        async fn connecting_tls_master(
            peer: &Peer,
        ) -> (MasterChannel, States, tokio::net::TcpStream) {
            let (listener, mut states) = States::new();
            let mut channel = crate::tcp::tls::spawn_master_tls_client_2(
                LinkErrorMode::Close,
                master_config(),
                peer.endpoints(),
                strategy(),
                ConnectOptions::default(),
                listener,
                client_config(),
            );
            states.expect(ClientState::Disabled).await;
            channel.enable().await.unwrap();
            states.expect(ClientState::Connecting).await;
            let stream = peer.accept().await;
            (channel, states, stream)
        }

        #[tokio::test]
        async fn disable_during_a_tls_handshake_that_completes_closes_the_session() {
            let peer = Peer::new().await;
            let (mut channel, mut states, stream) = connecting_tls_master(&peer).await;
            channel.disable().await.unwrap();
            let mut phys = server_config().handle_connection(stream).await.unwrap();
            states.expect(ClientState::Connected).await;
            states.expect(ClientState::Disabled).await;
            assert_tls_closed(&mut phys).await;
        }

        #[tokio::test]
        async fn shutdown_during_a_tls_handshake_that_completes_closes_the_session() {
            let peer = Peer::new().await;
            let (channel, mut states, stream) = connecting_tls_master(&peer).await;
            drop(channel);
            let mut phys = server_config().handle_connection(stream).await.unwrap();
            states.expect(ClientState::Connected).await;
            states.expect(ClientState::Shutdown).await;
            assert_tls_closed(&mut phys).await;
        }
    }
}
