// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Behavior shared by the TCP and TLS client spawn functions of the master and the outstation

use std::time::Duration;

use tokio::net::TcpListener;

use crate::app::ConnectStrategy;
use crate::decode::PhysDecodeLevel;
use crate::link::{EndpointAddress, LinkErrorMode};
use crate::master::{MasterChannel, MasterChannelConfig};
use crate::outstation::{DefaultControlHandler, OutstationConfig};
use crate::phys::{PhysAddr, PhysLayer};
use crate::tcp::{ClientState, ConnectOptions, EndpointList};
use crate::util::test_support::{
    NullApplication, NullInformation, Timeline, MASTER_REPLY, MASTER_REQUEST,
    NULL_UNSOLICITED_HEADER, OUTSTATION_REPLY, OUTSTATION_REQUEST,
};

const MASTER: u16 = 1;
const OUTSTATION: u16 = 1024;

const TIMEOUT: Duration = Duration::from_secs(5);

/// Expect the next state to be `state`, reported within [`TIMEOUT`]
async fn expect(states: &mut Timeline<ClientState>, state: ClientState) {
    tokio::time::timeout(TIMEOUT, states.expect_state(state))
        .await
        .unwrap();
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
    crate::util::test_support::outstation_config(OUTSTATION, MASTER)
}

/// Spawn a master TCP client that reports its states
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

/// Spawn a master TLS client that reports its states
#[cfg(feature = "enable-tls")]
fn spawn_tls_master(
    endpoints: EndpointList,
    strategy: ConnectStrategy,
) -> (MasterChannel, Timeline<ClientState>) {
    let (listener, timeline) = Timeline::new();
    let channel = crate::tcp::tls::spawn_master_tls_client_2(
        LinkErrorMode::Close,
        master_config(),
        endpoints,
        strategy,
        ConnectOptions::default(),
        listener,
        crate::util::test_support::tls::client_config(crate::tcp::tls::MinTlsVersion::V12),
    );
    (channel, timeline)
}

#[tokio::test]
async fn master_tcp_client_runs_a_master_session_over_tcp() {
    let peer = Peer::new().await;
    let (mut channel, mut states) = spawn_master(peer.endpoints(), strategy());
    expect(&mut states, ClientState::Disabled).await;
    channel.enable().await.unwrap();
    expect(&mut states, ClientState::Connecting).await;
    let mut phys = PhysLayer::tcp(peer.accept().await);
    expect(&mut states, ClientState::Connected).await;
    exchange(&mut phys, &OUTSTATION_REQUEST, &MASTER_REPLY).await;
    drop(channel);
    expect(&mut states, ClientState::Shutdown).await;
}

#[tokio::test]
async fn outstation_tcp_client_runs_an_outstation_session_over_tcp() {
    let peer = Peer::new().await;
    let (listener, mut states) = Timeline::new();
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
    expect(&mut states, ClientState::Disabled).await;
    handle.enable().await.unwrap();
    expect(&mut states, ClientState::Connecting).await;
    let mut phys = PhysLayer::tcp(peer.accept().await);
    expect(&mut states, ClientState::Connected).await;
    read_null_unsolicited(&mut phys).await;
    exchange(&mut phys, &MASTER_REQUEST, &OUTSTATION_REPLY).await;
    drop(handle);
    expect(&mut states, ClientState::Shutdown).await;
}

/// The master address an outstation client's connection handler sets, replacing [`MASTER`]
const NEW_MASTER: u16 = 2;

/// Connects to one endpoint, setting the master address to [`NEW_MASTER`]
struct NewMasterHandler(std::net::SocketAddr);

impl crate::tcp::ClientConnectionHandler for NewMasterHandler {
    fn endpoint_span_name(&self) -> String {
        self.0.to_string()
    }

    fn disconnected(&mut self, _: std::net::SocketAddr, _: Option<&str>) -> Duration {
        Duration::from_secs(1)
    }

    fn next(&mut self) -> Result<crate::tcp::ConnectionInfo, Duration> {
        let mut info = crate::tcp::ConnectionInfo::new(crate::tcp::Endpoint::address(self.0));
        info.set_master_address(EndpointAddress::raw(NEW_MASTER));
        Ok(info)
    }
}

/// The link header of unconfirmed user data from the outstation to `master` with the length field
/// `length` (the header fields plus the user data)
fn header_to(master: u16, length: u8) -> [u8; 10] {
    use crate::link::header::{AnyAddress, Header};
    crate::link::format::header_bytes(
        Header::unconfirmed_user_data(
            false,
            AnyAddress::from(master),
            AnyAddress::from(OUTSTATION),
        ),
        length,
    )
}

/// READ requests of class 0 data with sequence numbers 1 and 2
const CLASS_0_READ_SEQ_1: [u8; 5] = [0xC1, 0x01, 0x3C, 0x01, 0x06];
const CLASS_0_READ_SEQ_2: [u8; 5] = [0xC2, 0x01, 0x3C, 0x01, 0x06];

/// A frame from `master` carrying a whole application fragment
fn request(master: u16, fragment: &[u8]) -> Vec<u8> {
    use crate::link::header::{AnyAddress, Header};
    use crate::link::test_data::{data_frame, segment};
    data_frame(
        Header::unconfirmed_user_data(true, AnyAddress::from(OUTSTATION), AnyAddress::from(master)),
        segment(true, true, 0),
        fragment,
    )
}

#[tokio::test]
async fn outstation_tcp_client_uses_the_master_address_its_connection_handler_sets() {
    crate::util::test_support::watchdog(async {
        let peer = Peer::new().await;
        let (listener, mut states) = Timeline::new();
        let mut handle = crate::tcp::spawn_outstation_tcp_client_2(
            LinkErrorMode::Close,
            outstation_config(),
            Box::new(NewMasterHandler(peer.listener.local_addr().unwrap())),
            Box::new(NullApplication),
            Box::new(NullInformation),
            DefaultControlHandler::create(),
            listener,
        );
        expect(&mut states, ClientState::Disabled).await;
        handle.enable().await.unwrap();
        let mut phys = PhysLayer::tcp(peer.accept().await);

        // the null unsolicited response goes to the new master
        let null_unsolicited = read::<17>(&mut phys).await;
        assert_eq!(null_unsolicited[..10], header_to(NEW_MASTER, 10));

        // the new master confirms it; then a READ from the configured master is discarded, and the
        // new master's is answered
        let uns_seq = null_unsolicited[11] & 0x0F;
        let requests = [
            request(NEW_MASTER, &[0xD0 | uns_seq, 0x00]),
            request(MASTER, &CLASS_0_READ_SEQ_1),
            request(NEW_MASTER, &CLASS_0_READ_SEQ_2),
        ]
        .concat();
        phys.write(&requests, PhysAddr::None, PhysDecodeLevel::Nothing)
            .await
            .unwrap();
        let response = read::<17>(&mut phys).await;
        assert_eq!(response[..10], header_to(NEW_MASTER, 10));
        // after the transport header: the application control (FIR, FIN, seq 2) and RESPONSE
        assert_eq!(response[11..13], [0xC2, 0x81]);
    })
    .await;
}

#[cfg(feature = "enable-tls")]
mod tls {
    use super::*;
    use crate::outstation::OutstationHandle;
    use crate::phys::PhysIo;
    use crate::tcp::tls::MinTlsVersion;
    use crate::util::test_support::tls::{client_config, server_config};

    async fn accept_tls(peer: &Peer) -> PhysLayer {
        server_config(MinTlsVersion::V12)
            .handle_connection(peer.accept().await)
            .await
            .unwrap()
    }

    /// Spawn an outstation TLS client that reports its states
    fn spawn_tls_outstation(endpoints: EndpointList) -> (OutstationHandle, Timeline<ClientState>) {
        let (listener, timeline) = Timeline::new();
        let handle = crate::tcp::tls::spawn_outstation_tls_client(
            LinkErrorMode::Close,
            endpoints,
            strategy(),
            ConnectOptions::default(),
            outstation_config(),
            Box::new(NullApplication),
            Box::new(NullInformation),
            DefaultControlHandler::create(),
            listener,
            client_config(MinTlsVersion::V12),
        );
        (handle, timeline)
    }

    #[tokio::test]
    async fn master_tls_client_runs_a_master_session_over_tls() {
        let peer = Peer::new().await;
        let (mut channel, mut states) = spawn_tls_master(peer.endpoints(), strategy());
        expect(&mut states, ClientState::Disabled).await;
        channel.enable().await.unwrap();
        expect(&mut states, ClientState::Connecting).await;
        let mut phys = accept_tls(&peer).await;
        assert!(matches!(phys.io(), PhysIo::Tls(_)));
        expect(&mut states, ClientState::Connected).await;
        exchange(&mut phys, &OUTSTATION_REQUEST, &MASTER_REPLY).await;
        drop(channel);
        expect(&mut states, ClientState::Shutdown).await;
    }

    #[tokio::test]
    async fn outstation_tls_client_runs_an_outstation_session_over_tls() {
        let peer = Peer::new().await;
        let (mut handle, mut states) = spawn_tls_outstation(peer.endpoints());
        expect(&mut states, ClientState::Disabled).await;
        handle.enable().await.unwrap();
        expect(&mut states, ClientState::Connecting).await;
        let mut phys = accept_tls(&peer).await;
        expect(&mut states, ClientState::Connected).await;
        read_null_unsolicited(&mut phys).await;
        exchange(&mut phys, &MASTER_REQUEST, &OUTSTATION_REPLY).await;
        drop(handle);
        expect(&mut states, ClientState::Shutdown).await;
    }

    #[tokio::test]
    async fn master_tls_client_does_not_connect_to_a_plain_tcp_peer() {
        let peer = Peer::new().await;
        let (mut channel, mut states) = spawn_tls_master(peer.endpoints(), strategy());
        expect(&mut states, ClientState::Disabled).await;
        channel.enable().await.unwrap();
        expect(&mut states, ClientState::Connecting).await;
        // the peer closes the connection instead of answering the ClientHello
        drop(peer.accept().await);
        expect(
            &mut states,
            ClientState::WaitAfterFailedConnect(Duration::from_secs(1)),
        )
        .await;
    }

    #[tokio::test]
    async fn outstation_tls_client_does_not_connect_to_a_plain_tcp_peer() {
        let peer = Peer::new().await;
        let (mut handle, mut states) = spawn_tls_outstation(peer.endpoints());
        expect(&mut states, ClientState::Disabled).await;
        handle.enable().await.unwrap();
        expect(&mut states, ClientState::Connecting).await;
        // the peer closes the connection instead of answering the ClientHello
        drop(peer.accept().await);
        expect(
            &mut states,
            ClientState::WaitAfterFailedConnect(Duration::from_secs(1)),
        )
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
    use crate::util::test_support::watchdog;
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
        let (mut channel, mut states) = spawn_master(peer.endpoints(), strategy());
        expect(&mut states, ClientState::Disabled).await;
        channel.enable().await.unwrap();
        expect(&mut states, ClientState::Connecting).await;
        channel.disable().await.unwrap();
        expect(&mut states, ClientState::Connected).await;
        expect(&mut states, ClientState::Disabled).await;
        let mut stream = peer.accept().await;
        tokio::time::timeout(TIMEOUT, assert_closed(&mut stream))
            .await
            .unwrap();
    }

    #[cfg(feature = "enable-tls")]
    mod tls {
        use super::*;
        use crate::tcp::tls::MinTlsVersion;
        use crate::util::test_support::tls::server_config;

        /// The peer never answers the ClientHello, so the handshake times out after 5 seconds
        #[tokio::test(start_paused = true)]
        async fn disable_during_a_tls_handshake_takes_effect_once_it_fails() {
            watchdog(async {
                let peer = Peer::new().await;
                let (mut channel, mut timeline) = spawn_tls_master(peer.endpoints(), strategy());
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
                let (mut channel, mut timeline) = spawn_tls_master(peer.endpoints(), strategy());
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
        ) -> (MasterChannel, Timeline<ClientState>, tokio::net::TcpStream) {
            let (mut channel, mut states) = spawn_tls_master(peer.endpoints(), strategy());
            expect(&mut states, ClientState::Disabled).await;
            channel.enable().await.unwrap();
            expect(&mut states, ClientState::Connecting).await;
            let stream = peer.accept().await;
            (channel, states, stream)
        }

        #[tokio::test]
        async fn disable_during_a_tls_handshake_that_completes_closes_the_session() {
            let peer = Peer::new().await;
            let (mut channel, mut states, stream) = connecting_tls_master(&peer).await;
            channel.disable().await.unwrap();
            let mut phys = server_config(MinTlsVersion::V12)
                .handle_connection(stream)
                .await
                .unwrap();
            expect(&mut states, ClientState::Connected).await;
            expect(&mut states, ClientState::Disabled).await;
            assert_tls_closed(&mut phys).await;
        }

        #[tokio::test]
        async fn shutdown_during_a_tls_handshake_that_completes_closes_the_session() {
            let peer = Peer::new().await;
            let (channel, mut states, stream) = connecting_tls_master(&peer).await;
            drop(channel);
            let mut phys = server_config(MinTlsVersion::V12)
                .handle_connection(stream)
                .await
                .unwrap();
            expect(&mut states, ClientState::Connected).await;
            expect(&mut states, ClientState::Shutdown).await;
            assert_tls_closed(&mut phys).await;
        }
    }
}
