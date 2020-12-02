use crate::app::retry::ExponentialBackOff;
use crate::master::error::Shutdown;
use crate::master::handle::{Listener, MasterConfiguration, MasterHandle};
use crate::master::session::{MasterSession, RunError};
use crate::tokio::net::TcpStream;
use crate::transport::reader::TransportReader;
use crate::transport::TransportWriter;
use std::future::Future;
use std::net::SocketAddr;
use std::time::Duration;

/// state of TCP client connection
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ClientState {
    Connecting,
    Connected,
    WaitAfterFailedConnect(Duration),
    WaitAfterDisconnect(Duration),
    Shutdown,
}

pub(crate) struct MasterTask {
    endpoint: SocketAddr,
    back_off: ExponentialBackOff,
    session: MasterSession,
    reader: TransportReader,
    writer: TransportWriter,
    listener: Listener<ClientState>,
}

/// Spawn a task onto the `Tokio` runtime. The task runs until the returned handle, and any
/// `AssociationHandle` created from it, are dropped.
///
/// **Note**: This function may only be called from within the runtime itself, and panics otherwise.
/// It is preferable to use this method instead of `create(..)` when using `[tokio::main]`.
pub fn spawn_master_tcp_client(
    config: MasterConfiguration,
    endpoint: SocketAddr,
    listener: Listener<ClientState>,
) -> MasterHandle {
    let (mut task, handle) = MasterTask::new(config, endpoint, listener);
    crate::tokio::spawn(async move { task.run().await });
    handle
}

/// Create a Future, which can be spawned onto a runtime, along with a controlling handle.
///
/// Once spawned or otherwise executed using the `run` method, the task runs until the handle
/// and any `AssociationHandle` created from it are dropped.
///
/// **Note**: This function is required instead of `spawn` when using a runtime to directly spawn
/// tasks instead of within the context of a runtime, e.g. in applications that cannot use
/// `[tokio::main]` such as C language bindings.
pub fn create_master_tcp_client(
    config: MasterConfiguration,
    endpoint: SocketAddr,
    listener: Listener<ClientState>,
) -> (impl Future<Output = ()> + 'static, MasterHandle) {
    let (mut task, handle) = MasterTask::new(config, endpoint, listener);
    (async move { task.run().await }, handle)
}

impl MasterTask {
    fn new(
        config: MasterConfiguration,
        endpoint: SocketAddr,
        listener: Listener<ClientState>,
    ) -> (Self, MasterHandle) {
        let (tx, rx) = crate::tokio::sync::mpsc::channel(100); // TODO
        let session = MasterSession::new(
            config.level,
            config.response_timeout,
            config.tx_buffer_size,
            rx,
        );
        let (reader, writer) =
            crate::transport::create_master_transport_layer(config.address, config.rx_buffer_size);
        let task = Self {
            endpoint,
            back_off: ExponentialBackOff::new(config.reconnection_strategy),
            session,
            reader,
            writer,
            listener,
        };
        (task, MasterHandle::new(tx))
    }

    async fn run(&mut self) {
        self.run_impl().await.ok();
    }

    async fn run_impl(&mut self) -> Result<(), Shutdown> {
        loop {
            self.listener.update(ClientState::Connecting);
            match TcpStream::connect(self.endpoint).await {
                Err(err) => {
                    let delay = self.back_off.on_failure();
                    log::warn!("{} - waiting {} ms to retry", err, delay.as_millis());
                    self.listener
                        .update(ClientState::WaitAfterFailedConnect(delay));
                    self.session.delay_for(delay).await?;
                }
                Ok(mut socket) => {
                    log::info!("connected to: {}", self.endpoint);
                    self.back_off.on_success();
                    self.listener.update(ClientState::Connected);
                    match self
                        .session
                        .run(&mut socket, &mut self.writer, &mut self.reader)
                        .await
                    {
                        RunError::Shutdown => {
                            self.listener.update(ClientState::Shutdown);
                            return Err(Shutdown);
                        }
                        RunError::Link(err) => {
                            let delay = self.back_off.min_delay();
                            log::warn!("{} - waiting {} ms to reconnect", err, delay.as_millis());
                            self.listener
                                .update(ClientState::WaitAfterDisconnect(delay));
                            self.session.delay_for(delay).await?;
                        }
                    }
                }
            }
        }
    }
}
