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

use scursor::ReadCursor;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use tracing::Instrument;

/// Spawn a TCP server that accept connections from outstations
///
///
pub async fn spawn_master_tcp_server(
    addr: SocketAddr,
    handler: Box<dyn ConnectionHandler>,
) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr).await?;

    let (tx, rx) = crate::util::channel::request_channel();

    let accept_task = AcceptTask {
        conn_id: 0,
        listener,
        handler,
        id_task_results: rx,
        id_task_sender: tx,
    };

    let task = async move {
        let _ = accept_task
            .run()
            .instrument(tracing::info_span!("master-tcp-server", addr = ?addr))
            .await;
    };

    tokio::spawn(task);

    Ok(())
}

/// Determines what action will be taken when a TCP connection is accepted
#[derive(Copy, Clone, Debug)]
pub enum AcceptAction {
    /// Reject the connection, the socket will be closed
    Reject,
    /// Read a link-layer frame to determine the identity of the connecting device
    ///
    /// This is typically the outstation sending an unsolicited message
    GetLinkIdentity {
        /// How long the server should wait to receive a link-layer header
        /// before dropping the connection
        timeout: Timeout,
    },
    /// Accept the connection
    Accept {
        /// Configuration for the channel
        config: MasterChannelConfig,
        /// Link error mode that will be used
        error_mode: LinkErrorMode,
    },
}

/// Callbacks to user code that determine how the server processes connections
pub trait ConnectionHandler: Send {
    /// Filter the connection solely based on the remote address
    fn accept_tcp_connection(&mut self, addr: SocketAddr) -> AcceptAction;

    /// Start a communication session that was previously accepted
    ///
    /// The user may add associations to the channel and then enable it
    fn start(&mut self, channel: MasterChannel, addr: SocketAddr);
}

type IdentifyLinkResult = std::io::Result<(PhysLayer, LinkIdentity)>;

struct AcceptTask {
    conn_id: u64,
    listener: TcpListener,
    handler: Box<dyn ConnectionHandler>,
    id_task_results: Receiver<IdentifyLinkResult>,
    id_task_sender: Sender<IdentifyLinkResult>,
}

impl AcceptTask {
    async fn run(mut self) -> std::io::Result<()> {
        loop {
            self.accept_one().await?;
        }
    }

    fn next_conn_id(&mut self) -> u64 {
        let ret = self.conn_id;
        self.conn_id += 1;
        ret
    }

    async fn accept_one(&mut self) -> std::io::Result<()> {
        let (stream, addr) = self.listener.accept().await?;

        let phys = PhysLayer::Tcp(stream);

        match self.handler.accept_tcp_connection(addr) {
            AcceptAction::Reject => {
                tracing::info!("rejected connection from {addr}");
                return Ok(());
            }
            AcceptAction::Accept { config, error_mode } => {
                self.spawn_session(phys, addr, config, error_mode);
            }
            AcceptAction::GetLinkIdentity { timeout } => {
                // spawn a task to identify the remote link layer
                tokio::spawn(identify_link(
                    phys,
                    timeout.into(),
                    self.id_task_sender.clone(),
                ));
            }
        };

        Ok(())
    }

    fn spawn_session(
        &mut self,
        phys: PhysLayer,
        addr: SocketAddr,
        config: MasterChannelConfig,
        error_mode: LinkErrorMode,
    ) -> u64 {
        let conn_id = self.next_conn_id();
        let (tx, rx) = crate::util::channel::request_channel();
        let task = MasterTask::new(Enabled::No, LinkModes::stream(error_mode), config, rx);

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

        self.handler.start(channel, addr);

        conn_id
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

#[derive(Copy, Clone, Debug)]
struct LinkIdentity {
    source: u16,
    destination: u16,
    header_bytes: [u8; link::constant::LINK_HEADER_LENGTH],
}

type HeaderBytes = [u8; link::constant::LINK_HEADER_LENGTH];

async fn identify_link(
    mut phys: PhysLayer,
    timeout: Duration,
    mut reply_to: Sender<IdentifyLinkResult>,
) {
    let result = identify_or_timeout(&mut phys, timeout).await;
    let _ = reply_to.send(result.map(|x| (phys, x))).await;
}

async fn identify_or_timeout(
    layer: &mut PhysLayer,
    timeout: Duration,
) -> std::io::Result<LinkIdentity> {
    match tokio::time::timeout(timeout, read_link_identity(layer)).await {
        Ok(Ok(id)) => Ok(id),
        Ok(Err(err)) => Err(std::io::Error::new(ErrorKind::Other, err)),
        Err(_) => Err(std::io::Error::new(
            ErrorKind::Other,
            "No link header within timeout",
        )),
    }
}

async fn read_link_identity(layer: &mut PhysLayer) -> std::io::Result<LinkIdentity> {
    async fn read_header(layer: &mut PhysLayer) -> std::io::Result<HeaderBytes> {
        let mut header = [0; link::constant::LINK_HEADER_LENGTH];
        let mut count = 0;
        loop {
            let remaining = &mut header[count..link::constant::LINK_HEADER_LENGTH];
            let (num, _) = layer.read(remaining, PhysDecodeLevel::Nothing).await?;
            count += num;
            if count == link::constant::LINK_HEADER_LENGTH {
                return Ok(header);
            }
        }
    }

    fn read_addr(header: &HeaderBytes) -> Result<(u16, u16), ReadError> {
        // just skip over the 0x0564 | LENGTH | CTRL
        let mut cursor = ReadCursor::new(header);
        cursor.read_bytes(5)?;
        let destination = cursor.read_u16_le()?;
        let source = cursor.read_u16_le()?;
        Ok((destination, source))
    }

    let header_bytes = read_header(layer).await?;

    let (destination, source) = read_addr(&header_bytes)
        .map_err(|_| std::io::Error::new(ErrorKind::Other, "Bad read logic"))?;

    Ok(LinkIdentity {
        source,
        destination,
        header_bytes,
    })
}
