use crate::app::Shutdown;
use crate::link::reader::LinkModes;
use crate::link::LinkErrorMode;
use crate::master::task::MasterTask;
use crate::master::{MasterChannel, MasterChannelConfig, MasterChannelType};
use crate::util::phys::PhysLayer;
use crate::util::session::{Enabled, Session};
use std::net::SocketAddr;
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

    let accept_task = AcceptTask {
        conn_id: 0,
        listener,
        handler,
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
    //WaitForLinkMessage(Timeout),
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
    fn accept(&mut self, addr: SocketAddr) -> AcceptAction;

    /// Start a communication session that was previously accepted
    ///
    /// The user may add associations to the channel and then enable it
    fn start(&mut self, channel: MasterChannel, addr: SocketAddr);
}

struct AcceptTask {
    conn_id: u64,
    listener: TcpListener,
    handler: Box<dyn ConnectionHandler>,
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

        let (config, error_mode) = match self.handler.accept(addr) {
            AcceptAction::Reject => {
                tracing::info!("rejected connection from {addr}");
                return Ok(());
            }
            AcceptAction::Accept { config, error_mode } => (config, error_mode),
        };

        let conn_id = self.next_conn_id();
        let (tx, rx) = crate::util::channel::request_channel();
        let task = MasterTask::new(Enabled::No, LinkModes::stream(error_mode), config, rx);

        let task = SessionTask {
            phys: PhysLayer::Tcp(stream),
            session: Session::master(task),
        };

        let future = async move {
            let _ = task.run().await;
        }
        .instrument(tracing::info_span!("master-session", remote = ?addr, conn = conn_id));

        tokio::spawn(future);

        let channel = MasterChannel::new(tx, MasterChannelType::Stream);

        self.handler.start(channel, addr);

        Ok(())
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
