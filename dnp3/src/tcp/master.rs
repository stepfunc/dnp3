use std::future::Future;
use std::net::SocketAddr;
use std::time::Duration;

use tracing::Instrument;

use crate::app::{ConnectStrategy, ExponentialBackOff, Listener};
use crate::app::{RetryStrategy, Shutdown};
use crate::link::LinkErrorMode;
use crate::master::session::{MasterSession, RunError, StateChange};
use crate::master::{MasterChannel, MasterChannelConfig};
use crate::tcp::ClientState;
use crate::tcp::EndpointList;
use crate::transport::TransportReader;
use crate::transport::TransportWriter;
use crate::util::phys::PhysLayer;

use tokio::net::TcpStream;

/// Spawn a task onto the `Tokio` runtime. The task runs until the returned handle, and any
/// `AssociationHandle` created from it, are dropped.
///
/// **Note**: This function may only be called from within the runtime itself, and panics otherwise.
/// It is preferable to use this method instead of `create(..)` when using `[tokio::main]`.
pub fn spawn_master_tcp_client(
    link_error_mode: LinkErrorMode,
    config: MasterChannelConfig,
    endpoints: EndpointList,
    connect_strategy: ConnectStrategy,
    listener: Box<dyn Listener<ClientState>>,
) -> MasterChannel {
    let (future, handle) = create_master_tcp_client(
        link_error_mode,
        config,
        endpoints,
        connect_strategy,
        listener,
    );
    tokio::spawn(future);
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
    config: MasterChannelConfig,
    endpoints: EndpointList,
    connect_strategy: ConnectStrategy,
    listener: Box<dyn Listener<ClientState>>,
) -> (impl Future<Output = ()> + 'static, MasterChannel) {
    let main_addr = endpoints.main_addr().to_string();
    let (mut task, handle) = MasterTask::new(
        link_error_mode,
        endpoints,
        config,
        connect_strategy,
        MasterTaskConnectionHandler::Tcp,
        listener,
    );
    let future = async move {
        task.run()
            .instrument(tracing::info_span!("dnp3-master-tcp", "endpoint" = ?main_addr))
            .await;
    };
    (future, handle)
}

pub(crate) enum MasterTaskConnectionHandler {
    Tcp,
    #[cfg(feature = "tls")]
    Tls(crate::tcp::tls::TlsClientConfig),
}

impl MasterTaskConnectionHandler {
    async fn handle(
        &mut self,
        socket: TcpStream,
        endpoint: &SocketAddr,
    ) -> Result<PhysLayer, String> {
        match self {
            Self::Tcp => Ok(PhysLayer::Tcp(socket)),
            #[cfg(feature = "tls")]
            Self::Tls(config) => config.handle_connection(socket, endpoint).await,
        }
    }
}

pub(crate) struct MasterTask {
    endpoints: EndpointList,
    back_off: ExponentialBackOff,
    reconnect_delay: Duration,
    connection_handler: MasterTaskConnectionHandler,
    session: MasterSession,
    reader: TransportReader,
    writer: TransportWriter,
    listener: Box<dyn Listener<ClientState>>,
}

impl MasterTask {
    pub(crate) fn new(
        link_error_mode: LinkErrorMode,
        endpoints: EndpointList,
        config: MasterChannelConfig,
        connect_strategy: ConnectStrategy,
        connection_handler: MasterTaskConnectionHandler,
        listener: Box<dyn Listener<ClientState>>,
    ) -> (Self, MasterChannel) {
        let (tx, rx) = crate::util::channel::request_channel();
        let session = MasterSession::new(false, config.decode_level, config.tx_buffer_size, rx);
        let (reader, writer) = crate::transport::create_master_transport_layer(
            link_error_mode,
            config.master_address,
            config.rx_buffer_size,
        );
        let task = Self {
            endpoints,
            back_off: ExponentialBackOff::new(RetryStrategy::new(
                connect_strategy.min_connect_delay,
                connect_strategy.max_connect_delay,
            )),
            reconnect_delay: connect_strategy.reconnect_delay,
            connection_handler,
            session,
            reader,
            writer,
            listener,
        };
        (task, MasterChannel::new(tx))
    }

    pub(crate) async fn run(&mut self) {
        let _ = self.run_impl().await;
        self.session.shutdown().await;
        self.listener.update(ClientState::Shutdown).get().await;
    }

    async fn run_impl(&mut self) -> Result<(), Shutdown> {
        loop {
            self.listener.update(ClientState::Disabled).get().await;
            self.session.wait_for_enabled().await?;
            if let Err(StateChange::Shutdown) = self.run_connection().await {
                return Err(Shutdown);
            }
        }
    }

    async fn run_connection(&mut self) -> Result<(), StateChange> {
        loop {
            self.run_one_connection().await?;
        }
    }

    async fn run_one_connection(&mut self) -> Result<(), StateChange> {
        if let Some(endpoint) = self.endpoints.next_address().await {
            self.listener.update(ClientState::Connecting).get().await;
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
                        .update(ClientState::WaitAfterFailedConnect(delay))
                        .get()
                        .await;
                    self.session.wait_for_retry(delay).await
                }
                Ok(socket) => match self.connection_handler.handle(socket, &endpoint).await {
                    Err(err) => {
                        let delay = self.back_off.on_failure();
                        tracing::warn!("{} - waiting {} ms to retry", err, delay.as_millis());
                        self.on_connection_failure(delay).await
                    }
                    Ok(phys) => {
                        tracing::info!("connected to {}", endpoint);
                        self.endpoints.reset();
                        self.back_off.on_success();
                        self.listener.update(ClientState::Connected).get().await;
                        self.run_phys(phys).await
                    }
                },
            }
        } else {
            let delay = self.back_off.on_failure();
            tracing::warn!(
                "name resolution failure - waiting {} ms to retry",
                delay.as_millis()
            );
            self.on_connection_failure(delay).await
        }
    }

    async fn run_phys(&mut self, mut phys: PhysLayer) -> Result<(), StateChange> {
        match self
            .session
            .run(&mut phys, &mut self.writer, &mut self.reader)
            .await
        {
            RunError::State(s) => Err(s),
            RunError::Link(err) => {
                tracing::warn!("connection lost - {}", err);
                if self.reconnect_delay > Duration::from_secs(0) {
                    tracing::warn!(
                        "waiting {} ms to reconnect",
                        self.reconnect_delay.as_millis()
                    );
                    self.on_connection_failure(self.reconnect_delay).await?;
                }
                Ok(())
            }
        }
    }

    async fn on_connection_failure(&mut self, delay: Duration) -> Result<(), StateChange> {
        self.listener
            .update(ClientState::WaitAfterFailedConnect(delay))
            .get()
            .await;
        self.session.wait_for_retry(delay).await
    }
}
