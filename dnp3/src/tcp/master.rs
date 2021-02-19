use std::future::Future;
use std::time::Duration;

use tracing::Instrument;

use crate::app::ExponentialBackOff;
use crate::link::LinkErrorMode;
use crate::master::session::{MasterSession, RunError};
use crate::master::{Listener, MasterConfig, MasterHandle};
use crate::tcp::ClientState;
use crate::tcp::EndpointList;
use crate::tokio::net::TcpStream;
use crate::transport::TransportReader;
use crate::transport::TransportWriter;
use crate::util::phys::PhysLayer;
use crate::util::task::Shutdown;

/// Spawn a task onto the `Tokio` runtime. The task runs until the returned handle, and any
/// `AssociationHandle` created from it, are dropped.
///
/// **Note**: This function may only be called from within the runtime itself, and panics otherwise.
/// It is preferable to use this method instead of `create(..)` when using `[tokio::main]`.
pub fn spawn_master_tcp_client(
    link_error_mode: LinkErrorMode,
    config: MasterConfig,
    endpoints: EndpointList,
    listener: Listener<ClientState>,
) -> MasterHandle {
    let (future, handle) = create_master_tcp_client(link_error_mode, config, endpoints, listener);
    crate::tokio::spawn(future);
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
    link_error_mode: LinkErrorMode,
    config: MasterConfig,
    endpoints: EndpointList,
    listener: Listener<ClientState>,
) -> (impl Future<Output = ()> + 'static, MasterHandle) {
    let main_addr = endpoints.main_addr().to_string();
    let (mut task, handle) = MasterTask::new(link_error_mode, endpoints, config, listener);
    let future = async move {
        task.run()
            .instrument(tracing::info_span!("DNP3-Master-TCP", "endpoint" = ?main_addr))
            .await
    };
    (future, handle)
}

struct MasterTask {
    endpoints: EndpointList,
    back_off: ExponentialBackOff,
    reconnect_delay: Option<Duration>,
    session: MasterSession,
    reader: TransportReader,
    writer: TransportWriter,
    listener: Listener<ClientState>,
}

impl MasterTask {
    fn new(
        link_error_mode: LinkErrorMode,
        endpoints: EndpointList,
        config: MasterConfig,
        listener: Listener<ClientState>,
    ) -> (Self, MasterHandle) {
        let (tx, rx) = crate::tokio::sync::mpsc::channel(100); // TODO
        let session = MasterSession::new(
            config.decode_level,
            config.response_timeout,
            config.tx_buffer_size,
            rx,
        );
        let (reader, writer) = crate::transport::create_master_transport_layer(
            link_error_mode,
            config.address,
            config.rx_buffer_size,
        );
        let task = Self {
            endpoints,
            back_off: ExponentialBackOff::new(config.reconnection_strategy.retry_strategy),
            reconnect_delay: config.reconnection_strategy.reconnect_delay,
            session,
            reader,
            writer,
            listener,
        };
        (task, MasterHandle::new(tx))
    }

    async fn run(&mut self) {
        loop {
            if self.run_one_connection().await.is_err() {
                return;
            }
        }
    }

    async fn run_one_connection(&mut self) -> Result<(), Shutdown> {
        if let Some(endpoint) = self.endpoints.next_address().await {
            self.listener.update(ClientState::Connecting);
            match TcpStream::connect(endpoint).await {
                Err(err) => {
                    let delay = self.back_off.on_failure();
                    tracing::warn!(
                        "failed to connect to {}: {} - waiting {} ms to retry",
                        endpoint,
                        err,
                        delay.as_millis()
                    );
                    self.listener
                        .update(ClientState::WaitAfterFailedConnect(delay));
                    self.session.delay_for(delay).await
                }
                Ok(socket) => {
                    tracing::info!("connected to {}", endpoint);
                    self.endpoints.reset();
                    self.back_off.on_success();
                    self.listener.update(ClientState::Connected);
                    self.run_socket(socket).await
                }
            }
        } else {
            let delay = self.back_off.on_failure();
            tracing::warn!(
                "Name resolution failure - waiting {} ms to retry",
                delay.as_millis()
            );
            self.listener
                .update(ClientState::WaitAfterFailedConnect(delay));
            self.session.delay_for(delay).await
        }
    }

    async fn run_socket(&mut self, socket: TcpStream) -> Result<(), Shutdown> {
        let mut io = PhysLayer::Tcp(socket);
        match self
            .session
            .run(&mut io, &mut self.writer, &mut self.reader)
            .await
        {
            RunError::Shutdown => {
                self.listener.update(ClientState::Shutdown);
                Err(Shutdown)
            }
            RunError::Link(err) => {
                tracing::warn!("connection lost - {}", err);
                if let Some(delay) = self.reconnect_delay {
                    tracing::warn!("waiting {} ms to reconnect", delay.as_millis());
                    self.listener
                        .update(ClientState::WaitAfterDisconnect(delay));
                    self.session.delay_for(delay).await?;
                }
                Ok(())
            }
        }
    }
}
