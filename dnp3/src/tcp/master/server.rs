use crate::app::{Shutdown, Timeout};
use crate::decode::PhysDecodeLevel;
use crate::link;
use crate::link::reader::LinkModes;
use crate::link::LinkErrorMode;
use crate::master::task::MasterTask;
use crate::master::{MasterChannel, MasterChannelConfig, MasterChannelType};
use crate::util::phys::PhysLayer;
use crate::util::session::{Enabled, Session};
use std::future::Future;

use crate::tcp::accept::{Acceptor, Listen};
use crate::tcp::ServerHandle;
use crate::util::shutdown::ShutdownListener;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::num::NonZeroUsize;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinSet;
use tracing::Instrument;

/// Configuration that controls how the server performs remote link identification
#[derive(Copy, Clone, Debug)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct LinkIdConfig {
    max_tasks: NonZeroUsize,
    timeout: Timeout,
    decode_level: PhysDecodeLevel,
}

const DEFAULT_MAX_LINK_ID_TASKS: NonZeroUsize = NonZeroUsize::new(16).unwrap();

impl LinkIdConfig {
    /// Initialize with default values:
    ///
    /// * max_tasks = 16
    /// * timeout = 5 seconds
    /// * decode_level = Nothing
    pub fn new() -> Self {
        Self {
            max_tasks: DEFAULT_MAX_LINK_ID_TASKS,
            timeout: Timeout::default(),
            decode_level: PhysDecodeLevel::Nothing,
        }
    }

    /// Set the maximum number of simultaneous tasks used to perform link identification
    ///
    /// New connections are until a task is available
    pub fn max_tasks(self, max_tasks: NonZeroUsize) -> Self {
        Self { max_tasks, ..self }
    }

    /// Set the maximum number of time period to wait before receiving a link frame from the outstation
    pub fn timeout(self, timeout: Timeout) -> Self {
        Self { timeout, ..self }
    }

    /// Set the decode level to use when reading the link header used for identification
    pub fn decode_level(self, decode_level: PhysDecodeLevel) -> Self {
        Self {
            decode_level,
            ..self
        }
    }
}

impl Default for LinkIdConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Spawn a TCP server that accepts connections from outstations
///
/// The behavior of each connection is controlled by callbacks to a user-defined
/// implementation of the [`ConnectionHandler`] trait.
///
/// `addr` - local address on which the server will accept connection
/// `link_id_config` - Configuration used when identifying outstations based on transmitted link-frames
/// `handler` - Callbacks used to accept and start communication sessions
pub async fn spawn_master_tcp_server<C: ConnectionHandler>(
    local_addr: SocketAddr,
    link_id_config: LinkIdConfig,
    handler: C,
) -> std::io::Result<ServerHandle> {
    let listener = TcpListener::bind(local_addr).await?;

    let assigned_addr = listener.local_addr().ok();

    let (token, shutdown_listener) = crate::util::shutdown::shutdown_token();

    let accept_task = AcceptTask::new(listener, shutdown_listener, link_id_config, handler);

    let task = async move {
        tracing::info!("accepting connections");
        let Shutdown = accept_task.run().await;
        tracing::info!("shutdown");
    }
    .instrument(tracing::info_span!("master-tcp-server", addr = ?local_addr));

    tokio::spawn(task);

    let handle = ServerHandle {
        addr: assigned_addr,
        _token: token,
    };

    Ok(handle)
}

/// Error type indicating the connection should be rejected
#[derive(Copy, Clone, Debug)]
pub struct Reject;

/// Information returned by user code to configure an accepted connection
#[derive(Copy, Clone, Debug)]
pub struct AcceptConfig {
    /// Link error mode that will be used
    pub error_mode: LinkErrorMode,
    /// Configuration for the channel
    pub config: MasterChannelConfig,
}

/// Determines what action will be taken when a TCP connection is accepted from an outstation
#[derive(Copy, Clone, Debug)]
pub enum AcceptAction {
    /// Request that server attempt to identify the outstation by reading a link-layer header from the physical
    /// layer within a timeout.
    ///
    /// This header is typically the beginning of an unsolicited fragment from the outstation.
    GetLinkIdentity,
    /// Accept the connection, providing configuration information needed to create a [`MasterChannel`]
    Accept(AcceptConfig),
}

/// Callbacks to user code that determine how a master acting as a TCP or TLS server processes connections
pub trait ConnectionHandler: Send + 'static {
    /// Filter the connection solely based on the remote address
    fn accept(
        &mut self,
        addr: SocketAddr,
    ) -> impl Future<Output = Result<AcceptAction, Reject>> + Send;

    /// Start a communication session that was previously accepted using only the socket address
    ///
    /// The user may add associations to the channel and then enable it
    fn start(
        &mut self,
        channel: MasterChannel,
        addr: SocketAddr,
    ) -> impl Future<Output = ()> + Send;

    /// Filter the connection solely based on the remote address
    fn accept_link_id(
        &mut self,
        addr: SocketAddr,
        source: u16,
        destination: u16,
    ) -> impl Future<Output = Result<AcceptConfig, Reject>> + Send;

    /// Start a communication session that was previously accepted using link identity information.
    ///
    /// The user may add associations to the channel and then enable it
    fn start_with_link_id(
        &mut self,
        channel: MasterChannel,
        addr: SocketAddr,
        source: u16,
        destination: u16,
    ) -> impl Future<Output = ()> + Send;
}

type LinkIdResult = std::io::Result<(PhysLayer, SocketAddr, LinkIdentity)>;

struct AcceptTask<C: ConnectionHandler, L: Listen> {
    conn_id: u64,
    link_id_config: LinkIdConfig,
    acceptor: Acceptor<L>,
    shutdown_listener: ShutdownListener,
    handler: C,
    link_id_tasks: JoinSet<LinkIdResult>,
}

enum TaskEvent {
    Accept(TcpStream, SocketAddr),
    LinkId(LinkIdResult),
}

impl<C: ConnectionHandler, L: Listen> AcceptTask<C, L> {
    fn new(
        listener: L,
        shutdown_listener: ShutdownListener,
        link_id_config: LinkIdConfig,
        handler: C,
    ) -> Self {
        Self {
            conn_id: 0,
            link_id_config,
            acceptor: Acceptor::new(listener),
            shutdown_listener,
            handler,
            link_id_tasks: JoinSet::new(),
        }
    }

    async fn run(mut self) -> Shutdown {
        loop {
            match self.next_event().await {
                Ok(event) => self.process_event(event).await,
                Err(Shutdown) => return Shutdown,
            }
        }
    }

    async fn process_event(&mut self, event: TaskEvent) {
        match event {
            TaskEvent::Accept(stream, addr) => {
                tracing::info!("accepted connection from {addr:?}");
                crate::tcp::configure_server(&stream);
                self.handle_accept(stream, addr).await;
            }
            TaskEvent::LinkId(res) => match res {
                Ok((phys, addr, id)) => self.handle_link_identity(phys, addr, id).await,
                Err(err) => {
                    tracing::warn!("unable to identify remote link: {err}");
                }
            },
        }
    }

    fn next_conn_id(&mut self) -> u64 {
        let ret = self.conn_id;
        self.conn_id = self.conn_id.wrapping_add(1);
        ret
    }

    async fn next_event(&mut self) -> Result<TaskEvent, Shutdown> {
        let can_accept = self.link_id_tasks.len() < self.link_id_config.max_tasks.get();

        let accept_connection = async {
            if can_accept {
                self.acceptor.accept().await
            } else {
                std::future::pending().await
            }
        };

        tokio::select! {
            (stream, addr) = accept_connection  => {
                Ok(TaskEvent::Accept(stream, addr))
            }
            Some(res) = self.link_id_tasks.join_next(), if !self.link_id_tasks.is_empty() => {
                // a task that panicked or was cancelled is a failure to identify the link
                let res = res.unwrap_or_else(|err| Err(std::io::Error::other(err)));
                Ok(TaskEvent::LinkId(res))
            }
            _ = self.shutdown_listener.listen() => {
                Err(Shutdown)
            }
        }
    }

    async fn handle_link_identity(&mut self, phys: PhysLayer, addr: SocketAddr, id: LinkIdentity) {
        match self
            .handler
            .accept_link_id(addr, id.source, id.destination)
            .await
        {
            Ok(x) => {
                self.spawn_session(phys, addr, x, Some(id)).await;
            }
            Err(Reject) => {
                tracing::warn!(
                    "Dropping connection from {addr:?} with source = {} and destination = {}",
                    id.source,
                    id.destination
                );
            }
        }
    }

    async fn handle_accept(&mut self, stream: TcpStream, addr: SocketAddr) {
        let phys = PhysLayer::tcp(stream);

        match self.handler.accept(addr).await {
            Err(Reject) => {
                tracing::info!("rejected connection from {addr}");
            }
            Ok(AcceptAction::Accept(x)) => {
                self.spawn_session(phys, addr, x, None).await;
            }
            Ok(AcceptAction::GetLinkIdentity) => {
                spawn_identify_link(&mut self.link_id_tasks, &self.link_id_config, phys, addr);
            }
        }
    }

    async fn spawn_session(
        &mut self,
        phys: PhysLayer,
        addr: SocketAddr,
        accept: AcceptConfig,
        link_id: Option<LinkIdentity>,
    ) {
        let Some((task, channel)) = SessionTask::create(phys, accept, link_id.as_ref()) else {
            return;
        };

        task.spawn(addr, self.next_conn_id());

        match link_id {
            Some(id) => {
                self.handler
                    .start_with_link_id(channel, addr, id.source, id.destination)
                    .await
            }
            None => self.handler.start(channel, addr).await,
        }
    }
}

struct SessionTask {
    phys: PhysLayer,
    session: Session,
}

// Creating and spawning the session outside the generic `AcceptTask` keeps the session future
// from being compiled again in every crate that instantiates `spawn_master_tcp_server`.
impl SessionTask {
    fn create(
        phys: PhysLayer,
        accept: AcceptConfig,
        link_id: Option<&LinkIdentity>,
    ) -> Option<(Self, MasterChannel)> {
        let (tx, rx) = crate::util::channel::request_channel();
        let mut task = MasterTask::new(
            Enabled::No,
            LinkModes::stream(accept.error_mode),
            accept.config,
            rx,
        );
        let seed_data: &[u8] = link_id.map_or(&[], |id| &id.header_bytes);
        if let Err(err) = task.seed_link(seed_data) {
            tracing::error!("unable to seed link layer: {err:?}");
            return None;
        }

        let task = Self {
            phys,
            session: Session::master(task),
        };
        Some((task, MasterChannel::new(tx, MasterChannelType::Stream)))
    }

    fn spawn(self, addr: SocketAddr, conn_id: u64) {
        let future = async move {
            let _: Result<(), crate::app::Shutdown> = self.run().await;
        }
        .instrument(tracing::info_span!("master-session", remote = ?addr, conn = conn_id));

        tokio::spawn(future);
    }

    async fn run(mut self) -> Result<(), Shutdown> {
        self.session.wait_for_enabled().await?;
        let err = self.session.run(&mut self.phys).await;
        tracing::info!("closing: {err}");
        Ok(())
    }
}

type HeaderBytes = [u8; link::constant::LINK_HEADER_LENGTH];

#[derive(Copy, Clone, Debug)]
struct LinkIdentity {
    source: u16,
    destination: u16,
    header_bytes: HeaderBytes,
}

/// Spawn a task to identify the remote link layer, outside the generic `AcceptTask` for the same
/// reason as [`SessionTask`]
fn spawn_identify_link(
    tasks: &mut JoinSet<LinkIdResult>,
    config: &LinkIdConfig,
    phys: PhysLayer,
    addr: SocketAddr,
) {
    tasks.spawn(identify_link(
        phys,
        config.decode_level,
        addr,
        config.timeout.into(),
    ));
}

async fn identify_link(
    mut phys: PhysLayer,
    link_id_decode_level: PhysDecodeLevel,
    addr: SocketAddr,
    timeout: Duration,
) -> LinkIdResult {
    let result = identify_or_timeout(&mut phys, link_id_decode_level, timeout).await;
    result.map(|x| (phys, addr, x))
}

async fn identify_or_timeout(
    layer: &mut PhysLayer,
    decode_level: PhysDecodeLevel,
    timeout: Duration,
) -> std::io::Result<LinkIdentity> {
    match tokio::time::timeout(timeout, read_link_identity(layer, decode_level)).await {
        Ok(Ok(id)) => Ok(id),
        Ok(Err(err)) => Err(std::io::Error::other(err)),
        Err(_) => Err(std::io::Error::other("No link header within timeout")),
    }
}

async fn read_link_identity(
    layer: &mut PhysLayer,
    decode_level: PhysDecodeLevel,
) -> std::io::Result<LinkIdentity> {
    async fn read_header(
        layer: &mut PhysLayer,
        decode_level: PhysDecodeLevel,
    ) -> std::io::Result<HeaderBytes> {
        let mut header = [0; link::constant::LINK_HEADER_LENGTH];
        let mut count = 0;
        loop {
            let remaining = header
                .get_mut(count..)
                .ok_or_else(|| std::io::Error::other("link header overrun"))?;
            let (num, _) = layer.read(remaining, decode_level).await?;
            count = count.saturating_add(num);
            if count == link::constant::LINK_HEADER_LENGTH {
                return Ok(header);
            }
        }
    }

    let header_bytes = read_header(layer, decode_level).await?;

    let (header, _) = link::parser::parse_header_bytes(header_bytes)
        .map_err(|err| std::io::Error::new(ErrorKind::InvalidData, err.to_string()))?;

    Ok(LinkIdentity {
        source: header.source.value(),
        destination: header.destination.value(),
        header_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link::EndpointAddress;
    use crate::tcp::accept::test_util::{
        expect_attempt_gaps, Fault, FaultyListener, RESET, UNKNOWN,
    };
    use crate::tcp::accept::ACCEPT_ERROR_DELAY;
    use crate::util::test_support::watchdog;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::sync::mpsc;

    /// request link status from outstation 1024 to master 1
    const LINK_STATUS_REQUEST: [u8; 10] =
        [0x05, 0x64, 0x05, 0x49, 0x01, 0x00, 0x00, 0x04, 0xD2, 0x36];
    /// link status from master 1 to outstation 1024
    const LINK_STATUS_RESPONSE: [u8; 10] =
        [0x05, 0x64, 0x05, 0x8B, 0x00, 0x04, 0x01, 0x00, 0x2E, 0xF7];

    #[derive(Debug, PartialEq)]
    enum Event {
        Accept,
        AcceptLinkId(u16, u16),
        Start,
        StartWithLinkId(u16, u16),
    }

    struct Handler {
        identify: bool,
        events: mpsc::UnboundedSender<Event>,
        channels: mpsc::UnboundedSender<MasterChannel>,
    }

    fn accept_config() -> AcceptConfig {
        AcceptConfig {
            error_mode: LinkErrorMode::Close,
            config: MasterChannelConfig::new(EndpointAddress::try_new(1).unwrap()),
        }
    }

    impl ConnectionHandler for Handler {
        async fn accept(&mut self, _: SocketAddr) -> Result<AcceptAction, Reject> {
            self.events.send(Event::Accept).unwrap();
            if self.identify {
                Ok(AcceptAction::GetLinkIdentity)
            } else {
                Ok(AcceptAction::Accept(accept_config()))
            }
        }

        async fn start(&mut self, channel: MasterChannel, _: SocketAddr) {
            self.events.send(Event::Start).unwrap();
            self.channels.send(channel).unwrap();
        }

        async fn accept_link_id(
            &mut self,
            _: SocketAddr,
            source: u16,
            destination: u16,
        ) -> Result<AcceptConfig, Reject> {
            self.events
                .send(Event::AcceptLinkId(source, destination))
                .unwrap();
            Ok(accept_config())
        }

        async fn start_with_link_id(
            &mut self,
            channel: MasterChannel,
            _: SocketAddr,
            source: u16,
            destination: u16,
        ) {
            self.events
                .send(Event::StartWithLinkId(source, destination))
                .unwrap();
            self.channels.send(channel).unwrap();
        }
    }

    struct Fixture {
        _server: ServerHandle,
        stream: TcpStream,
        events: mpsc::UnboundedReceiver<Event>,
        channels: mpsc::UnboundedReceiver<MasterChannel>,
        channel: Option<MasterChannel>,
    }

    impl Fixture {
        async fn new(identify: bool) -> Self {
            let (events_tx, events) = mpsc::unbounded_channel();
            let (channels_tx, channels) = mpsc::unbounded_channel();
            let handler = Handler {
                identify,
                events: events_tx,
                channels: channels_tx,
            };
            let server = spawn_master_tcp_server(
                "127.0.0.1:0".parse().unwrap(),
                LinkIdConfig::new(),
                handler,
            )
            .await
            .unwrap();
            let stream = TcpStream::connect(server.local_addr().unwrap())
                .await
                .unwrap();
            Self {
                _server: server,
                stream,
                events,
                channels,
                channel: None,
            }
        }

        async fn expect_event(&mut self, expected: Event) {
            let event = tokio::time::timeout(Duration::from_secs(5), self.events.recv())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(event, expected);
        }

        async fn enable_channel(&mut self) {
            let mut channel = tokio::time::timeout(Duration::from_secs(5), self.channels.recv())
                .await
                .unwrap()
                .unwrap();
            channel.enable().await.unwrap();
            self.channel = Some(channel);
        }

        async fn expect_bytes(&mut self, expected: &[u8]) {
            let mut buffer = vec![0; expected.len()];
            tokio::time::timeout(Duration::from_secs(5), self.stream.read_exact(&mut buffer))
                .await
                .unwrap()
                .unwrap();
            assert_eq!(buffer, expected);
        }

        /// the server closes the connection without reporting a link identity
        async fn expect_closed_without_identity(&mut self) {
            let mut buffer = [0; 1];
            let closed = async {
                tokio::select! {
                    Some(event) = self.events.recv() => panic!("unexpected event: {event:?}"),
                    res = self.stream.read(&mut buffer) => res.unwrap(),
                }
            };
            let count = tokio::time::timeout(Duration::from_secs(5), closed)
                .await
                .unwrap();
            assert_eq!(count, 0);
            assert_eq!(
                self.events.try_recv(),
                Err(mpsc::error::TryRecvError::Empty)
            );
        }
    }

    #[tokio::test]
    async fn accepted_session_starts_without_link_identity() {
        let mut fixture = Fixture::new(false).await;
        fixture.expect_event(Event::Accept).await;
        fixture.expect_event(Event::Start).await;
        fixture.enable_channel().await;
        fixture
            .stream
            .write_all(&LINK_STATUS_REQUEST)
            .await
            .unwrap();
        fixture.expect_bytes(&LINK_STATUS_RESPONSE).await;
    }

    #[tokio::test]
    async fn identified_session_is_seeded_with_the_identifying_header() {
        let mut fixture = Fixture::new(true).await;
        fixture.expect_event(Event::Accept).await;
        fixture
            .stream
            .write_all(&LINK_STATUS_REQUEST)
            .await
            .unwrap();
        fixture.expect_event(Event::AcceptLinkId(1024, 1)).await;
        fixture.expect_event(Event::StartWithLinkId(1024, 1)).await;
        fixture.enable_channel().await;
        // the session answers the header that was read to identify it
        fixture.expect_bytes(&LINK_STATUS_RESPONSE).await;
    }

    async fn rejects_identification(header: [u8; 10]) {
        let mut fixture = Fixture::new(true).await;
        fixture.expect_event(Event::Accept).await;
        fixture.stream.write_all(&header).await.unwrap();
        fixture.expect_closed_without_identity().await;
    }

    #[tokio::test]
    async fn header_with_bad_start1_is_not_identified() {
        let mut header = LINK_STATUS_REQUEST;
        header[0] = 0x06;
        rejects_identification(header).await;
    }

    #[tokio::test]
    async fn header_with_bad_start2_is_not_identified() {
        let mut header = LINK_STATUS_REQUEST;
        header[1] = 0x65;
        rejects_identification(header).await;
    }

    #[tokio::test]
    async fn header_with_bad_crc_is_not_identified() {
        let mut header = LINK_STATUS_REQUEST;
        header[9] ^= 0xFF;
        rejects_identification(header).await;
    }

    #[tokio::test]
    async fn header_with_bad_length_is_not_identified() {
        // length 4 with a matching CRC
        rejects_identification([0x05, 0x64, 0x04, 0xC0, 0x01, 0x00, 0x00, 0x04, 0x0E, 0x94]).await;
    }

    struct FaultyServer {
        handle: ServerHandle,
        task: tokio::task::JoinHandle<Shutdown>,
        events: mpsc::UnboundedReceiver<Event>,
        _channels: mpsc::UnboundedReceiver<MasterChannel>,
    }

    impl FaultyServer {
        fn spawn(listener: FaultyListener, identify: bool) -> Self {
            let (events_tx, events) = mpsc::unbounded_channel();
            let (channels_tx, channels) = mpsc::unbounded_channel();
            let handler = Handler {
                identify,
                events: events_tx,
                channels: channels_tx,
            };
            let (token, shutdown_listener) = crate::util::shutdown::shutdown_token();
            let task = AcceptTask::new(listener, shutdown_listener, LinkIdConfig::new(), handler);
            Self {
                handle: ServerHandle {
                    addr: None,
                    _token: token,
                },
                task: tokio::spawn(task.run()),
                events,
                _channels: channels,
            }
        }
    }

    #[tokio::test(start_paused = true)]
    async fn accepts_after_accept_errors() {
        watchdog(async {
            let (listener, mut attempts) = FaultyListener::new(&[UNKNOWN, RESET, UNKNOWN]).await;
            let _client = TcpStream::connect(listener.local_addr()).await.unwrap();
            let mut server = FaultyServer::spawn(listener, false);

            expect_attempt_gaps(
                &mut attempts,
                &[ACCEPT_ERROR_DELAY, Duration::ZERO, ACCEPT_ERROR_DELAY],
            )
            .await;
            assert_eq!(server.events.recv().await, Some(Event::Accept));
            assert_eq!(server.events.recv().await, Some(Event::Start));
        })
        .await;
    }

    #[tokio::test(start_paused = true)]
    async fn link_identity_during_accept_error_delay_does_not_cut_it_short() {
        watchdog(async {
            let (listener, mut attempts) = FaultyListener::new(&[Fault::None, UNKNOWN]).await;
            let mut client = TcpStream::connect(listener.local_addr()).await.unwrap();
            let mut server = FaultyServer::spawn(listener, true);
            // Waiting for I/O lets a paused clock jump to the next timer. This one keeps each
            // jump to a millisecond, so the link header arrives long before the retry is due.
            let ticker = tokio::spawn(async {
                for _ in 0..2000 {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
            });

            attempts.recv().await.unwrap();
            assert_eq!(server.events.recv().await, Some(Event::Accept));
            let failed_at = attempts.recv().await.unwrap();

            // identifying the accepted connection ends the select while the server waits to retry
            client.write_all(&LINK_STATUS_REQUEST).await.unwrap();
            assert_eq!(
                server.events.recv().await,
                Some(Event::AcceptLinkId(1024, 1))
            );
            assert_eq!(
                server.events.recv().await,
                Some(Event::StartWithLinkId(1024, 1))
            );
            assert!(failed_at.elapsed() < Duration::from_millis(100));

            let retried_at = attempts.recv().await.unwrap();
            assert_eq!(retried_at.duration_since(failed_at), ACCEPT_ERROR_DELAY);
            ticker.abort();
        })
        .await;
    }

    #[tokio::test(start_paused = true)]
    async fn dropping_handle_during_accept_error_delay_stops_server() {
        watchdog(async {
            let (listener, mut attempts) = FaultyListener::new(&[UNKNOWN]).await;
            let server = FaultyServer::spawn(listener, false);

            let failed_at = attempts.recv().await.unwrap();
            tokio::time::advance(Duration::from_millis(500)).await;
            assert!(!server.task.is_finished());

            drop(server.handle);
            server.task.await.unwrap();
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
            let server = FaultyServer::spawn(listener, false);

            attempts.recv().await.unwrap();
            drop(server.handle);
            server.task.await.unwrap();
            let mut count = 1;
            while attempts.recv().await.is_some() {
                count += 1;
            }
            assert!(count < RESETS.len(), "{count}");
        })
        .await;
    }
}
