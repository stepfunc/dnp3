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

const MASTER: u16 = 1;
const OUTSTATION: u16 = 1024;

/// request link status from outstation 1024 to master 1, and the master's reply
const OUTSTATION_REQUEST: [u8; 10] = [0x05, 0x64, 0x05, 0x49, 0x01, 0x00, 0x00, 0x04, 0xD2, 0x36];
const MASTER_REPLY: [u8; 10] = [0x05, 0x64, 0x05, 0x8B, 0x00, 0x04, 0x01, 0x00, 0x2E, 0xF7];

/// request link status from master 1 to outstation 1024, and the outstation's reply
const MASTER_REQUEST: [u8; 10] = [0x05, 0x64, 0x05, 0xC9, 0x00, 0x04, 0x01, 0x00, 0x98, 0x81];
const OUTSTATION_REPLY: [u8; 10] = [0x05, 0x64, 0x05, 0x0B, 0x01, 0x00, 0x00, 0x04, 0x64, 0x40];

/// the link header of unconfirmed user data from outstation 1024 to master 1 holding 5 bytes
const NULL_UNSOLICITED_HEADER: [u8; 10] =
    [0x05, 0x64, 0x0A, 0x44, 0x01, 0x00, 0x00, 0x04, 0x67, 0x88];

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
    let mut phys = PhysLayer::Tcp(peer.accept().await);
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
    let mut phys = PhysLayer::Tcp(peer.accept().await);
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
    use std::path::PathBuf;

    fn certs() -> PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../certs/self_signed")
    }

    fn client_config() -> TlsClientConfig {
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

    fn server_config() -> TlsServerConfig {
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
        assert!(matches!(phys, PhysLayer::Tls(_)));
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
