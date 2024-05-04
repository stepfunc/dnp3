use crate::app::{ReadError, Shutdown, Timeout};
use crate::decode::PhysDecodeLevel;
use crate::link;
use crate::link::reader::LinkModes;
use crate::link::LinkErrorMode;
use crate::master::task::MasterTask;
use crate::master::{MasterChannel, MasterChannelConfig, MasterChannelType};
use crate::util::channel::{Receiver, Sender};
use crate::util::phys::PhysLayer;
use crate::util::session::{Enabled, Session};
use std::future::Future;

use crate::tcp::ServerHandle;
use crate::util::future::forever;
use crate::util::shutdown::ShutdownListener;
use scursor::ReadCursor;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::num::NonZeroUsize;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
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

impl LinkIdConfig {
    /// Initialize with default values:
    ///
    /// * max_tasks = 16
    /// * timeout = 5 seconds
    /// * decode_level = Nothing
    pub fn new() -> Self {
        Self {
            max_tasks: NonZeroUsize::new(16).unwrap(),
            timeout: Timeout::from_secs(5).unwrap(),
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

    let (tx, rx) = crate::util::channel::request_channel();

    let (token, shutdown_listener) = crate::util::shutdown::shutdown_token();

    let accept_task = AcceptTask {
        conn_id: 0,
        pending_link_id_tasks: 0,
        link_id_config,
        listener,
        shutdown_listener,
        handler,
        id_task_results: rx,
        id_task_sender: tx,
    };

    let task = async move {
        tracing::info!("accepting connections");
        let _ = accept_task.run().await;
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

/// Callbacks to user code that determine how the server processes connections
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

struct AcceptTask<C: ConnectionHandler> {
    conn_id: u64,
    pending_link_id_tasks: usize,
    link_id_config: LinkIdConfig,
    listener: TcpListener,
    shutdown_listener: ShutdownListener,
    handler: C,
    id_task_results: Receiver<LinkIdResult>,
    id_task_sender: Sender<LinkIdResult>,
}

enum TaskEvent {
    Accept(TcpStream, SocketAddr),
    LinkId(LinkIdResult),
}

impl<C: ConnectionHandler> AcceptTask<C> {
    async fn run(mut self) -> Result<(), Shutdown> {
        loop {
            let event = match self.next_event().await {
                Ok(res) => res?,
                Err(err) => {
                    tracing::warn!("I/O error, exiting: {err}");
                    return Err(Shutdown);
                }
            };

            self.process_event(event).await;
        }
    }

    async fn process_event(&mut self, event: TaskEvent) {
        match event {
            TaskEvent::Accept(stream, addr) => {
                tracing::info!("accepted connection from {addr:?}");
                crate::tcp::configure_server(&stream);
                self.handle_accept(stream, addr).await;
            }
            TaskEvent::LinkId(res) => {
                self.pending_link_id_tasks -= 1;
                match res {
                    Ok((phys, addr, id)) => self.handle_link_identity(phys, addr, id).await,
                    Err(err) => {
                        tracing::warn!("unable to identify remote link: {err}");
                    }
                }
            }
        }
    }

    fn next_conn_id(&mut self) -> u64 {
        let ret = self.conn_id;
        self.conn_id += 1;
        ret
    }

    async fn next_event(&mut self) -> std::io::Result<Result<TaskEvent, Shutdown>> {
        let can_accept = self.pending_link_id_tasks < self.link_id_config.max_tasks.get();

        let accept_connection = async {
            if can_accept {
                self.listener.accept().await
            } else {
                forever().await
            }
        };

        tokio::select! {
            res = accept_connection  => {
                let (stream, addr) = res?;
               Ok(Ok(TaskEvent::Accept(stream, addr)))
            }
            res = self.id_task_results.receive() => {
                // unwrap is fine b/c both the sending and receiving sides of the channel are owned by this struct
                let id = res.expect("bad channel logic");
                Ok(Ok(TaskEvent::LinkId(id)))
            }
            _ = self.shutdown_listener.listen() => {
                Ok(Err(Shutdown))
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
                self.spawn_session(
                    phys,
                    addr,
                    x.config,
                    x.error_mode,
                    id.header_bytes.as_slice(),
                    Some(id),
                )
                .await;
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
        let phys = PhysLayer::Tcp(stream);

        match self.handler.accept(addr).await {
            Err(Reject) => {
                tracing::info!("rejected connection from {addr}");
            }
            Ok(AcceptAction::Accept(x)) => {
                self.spawn_session(phys, addr, x.config, x.error_mode, &[], None)
                    .await;
            }
            Ok(AcceptAction::GetLinkIdentity) => {
                // spawn a task to identify the remote link layer
                tokio::spawn(identify_link(
                    phys,
                    self.link_id_config.decode_level,
                    addr,
                    self.link_id_config.timeout.into(),
                    self.id_task_sender.clone(),
                ));
                self.pending_link_id_tasks += 1;
            }
        }
    }

    async fn spawn_session(
        &mut self,
        phys: PhysLayer,
        addr: SocketAddr,
        config: MasterChannelConfig,
        error_mode: LinkErrorMode,
        seed_data: &[u8],
        link_id: Option<LinkIdentity>,
    ) {
        let (tx, rx) = crate::util::channel::request_channel();
        let mut task = MasterTask::new(Enabled::No, LinkModes::stream(error_mode), config, rx);
        if let Err(err) = task.seed_link(seed_data) {
            tracing::error!("unable to seed link layer: {err:?}");
            return;
        }

        let conn_id = self.next_conn_id();

        let task = SessionTask {
            phys,
            session: Session::master(task),
        };

        let future = async move {
            let _ = task.run().await;
        }
        .instrument(tracing::info_span!("master-session", remote = ?addr, conn = conn_id));

        tokio::spawn(future);

        let channel = MasterChannel::new(tx, MasterChannelType::Stream);

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

impl SessionTask {
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

async fn identify_link(
    mut phys: PhysLayer,
    link_id_decode_level: PhysDecodeLevel,
    addr: SocketAddr,
    timeout: Duration,
    mut reply_to: Sender<LinkIdResult>,
) {
    let result = identify_or_timeout(&mut phys, link_id_decode_level, timeout).await;
    let reply = result.map(|x| (phys, addr, x));
    let _ = reply_to.send(reply).await;
}

async fn identify_or_timeout(
    layer: &mut PhysLayer,
    decode_level: PhysDecodeLevel,
    timeout: Duration,
) -> std::io::Result<LinkIdentity> {
    match tokio::time::timeout(timeout, read_link_identity(layer, decode_level)).await {
        Ok(Ok(id)) => Ok(id),
        Ok(Err(err)) => Err(std::io::Error::new(ErrorKind::Other, err)),
        Err(_) => Err(std::io::Error::new(
            ErrorKind::Other,
            "No link header within timeout",
        )),
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
            let remaining = &mut header[count..link::constant::LINK_HEADER_LENGTH];
            let (num, _) = layer.read(remaining, decode_level).await?;
            count += num;
            if count == link::constant::LINK_HEADER_LENGTH {
                return Ok(header);
            }
        }
    }

    fn read_addr(header: &HeaderBytes) -> Result<(u16, u16), ReadError> {
        let mut cursor = ReadCursor::new(header);
        // just skip over the 0x0564 | LENGTH | CTRL
        cursor.read_bytes(4)?;
        let destination = cursor.read_u16_le()?;
        let source = cursor.read_u16_le()?;
        Ok((destination, source))
    }

    let header_bytes = read_header(layer, decode_level).await?;

    let (destination, source) = read_addr(&header_bytes)
        .map_err(|_| std::io::Error::new(ErrorKind::Other, "Bad read logic"))?;

    Ok(LinkIdentity {
        source,
        destination,
        header_bytes,
    })
}
