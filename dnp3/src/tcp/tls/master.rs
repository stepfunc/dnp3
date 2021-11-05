use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use tracing::Instrument;

use crate::app::{ConnectStrategy, ExponentialBackOff, Listener};
use crate::app::{RetryStrategy, Shutdown};
use crate::link::LinkErrorMode;
use crate::master::session::{MasterSession, RunError, StateChange};
use crate::master::{MasterChannel, MasterChannelConfig};
use crate::tcp::ClientState;
use crate::tcp::EndpointList;
use crate::tcp::tls::TlsConfig;
use crate::tokio::net::TcpStream;
use crate::transport::TransportReader;
use crate::transport::TransportWriter;
use crate::util::phys::PhysLayer;

/// Spawn a task onto the `Tokio` runtime. The task runs until the returned handle, and any
/// `AssociationHandle` created from it, are dropped.
///
/// **Note**: This function may only be called from within the runtime itself, and panics otherwise.
/// It is preferable to use this method instead of `create(..)` when using `[tokio::main]`.
pub fn spawn_master_tls_client(
    link_error_mode: LinkErrorMode,
    config: MasterChannelConfig,
    endpoints: EndpointList,
    tls_config: TlsConfig,
    connect_strategy: ConnectStrategy,
    listener: Box<dyn Listener<ClientState>>,
) -> MasterChannel {
    let (future, handle) = create_master_tls_client(
        link_error_mode,
        config,
        endpoints,
        tls_config,
        connect_strategy,
        listener,
    );
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
pub fn create_master_tls_client(
    link_error_mode: LinkErrorMode,
    config: MasterChannelConfig,
    endpoints: EndpointList,
    tls_config: TlsConfig,
    connect_strategy: ConnectStrategy,
    listener: Box<dyn Listener<ClientState>>,
) -> (impl Future<Output = ()> + 'static, MasterChannel) {
    let main_addr = endpoints.main_addr().to_string();
    let (mut task, handle) = MasterTask::new(
        link_error_mode,
        endpoints,
        config,
        tls_config,
        connect_strategy,
        listener,
    );
    let future = async move {
        task.run()
            .instrument(tracing::info_span!("DNP3-Master-TCP", "endpoint" = ?main_addr))
            .await;
    };
    (future, handle)
}

struct MasterTask {
    endpoints: EndpointList,
    back_off: ExponentialBackOff,
    reconnect_delay: Duration,
    session: MasterSession,
    reader: TransportReader,
    writer: TransportWriter,
    tls_config: TlsConfig,
    listener: Box<dyn Listener<ClientState>>,
}

impl MasterTask {
    fn new(
        link_error_mode: LinkErrorMode,
        endpoints: EndpointList,
        config: MasterChannelConfig,
        tls_config: TlsConfig,
        connect_strategy: ConnectStrategy,
        listener: Box<dyn Listener<ClientState>>,
    ) -> (Self, MasterChannel) {
        let (tx, rx) = crate::util::channel::request_channel();
        let session = MasterSession::new(
            false,
            config.decode_level,
            config.response_timeout,
            config.tx_buffer_size,
            rx,
        );
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
            session,
            reader,
            writer,
            tls_config,
            listener,
        };
        (task, MasterChannel::new(tx))
    }

    async fn run(&mut self) {
        let _ = self.run_impl().await;
        self.session.shutdown().await;
        self.listener.update(ClientState::Shutdown);
    }

    async fn run_impl(&mut self) -> Result<(), Shutdown> {
        loop {
            self.listener.update(ClientState::Disabled);
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
                    self.on_connection_failure(delay).await
                }
                Ok(socket) => {
                    // Establish SSL session
                    let config = self.tls_config.to_client_config();
                    let connector = tokio_rustls::TlsConnector::from(Arc::new(config));
                    let dnsname = self.tls_config.dns_name();

                    match connector.connect(dnsname, socket).await {
                        Err(err) => {
                            let delay = self.back_off.on_failure();
                            tracing::warn!(
                                "failed to establish TLS session with {}: {} - waiting {} ms to retry",
                                endpoint,
                                err,
                                delay.as_millis()
                            );
                            self.on_connection_failure(delay).await
                        }
                        Ok(stream) => {
                            tracing::info!("connected to {}", endpoint);
                            self.endpoints.reset();
                            self.back_off.on_success();
                            self.listener.update(ClientState::Connected);
                            self.run_socket(stream).await
                        }
                    }
                }
            }
        } else {
            let delay = self.back_off.on_failure();
            tracing::warn!(
                "Name resolution failure - waiting {} ms to retry",
                delay.as_millis()
            );
            self.on_connection_failure(delay).await
        }
    }

    async fn run_socket(&mut self, socket: tokio_rustls::client::TlsStream<TcpStream>) -> Result<(), StateChange> {
        let mut io = PhysLayer::Tls(tokio_rustls::TlsStream::from(socket));
        match self
            .session
            .run(&mut io, &mut self.writer, &mut self.reader)
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
                    self.listener
                        .update(ClientState::WaitAfterDisconnect(self.reconnect_delay));
                    self.session.wait_for_retry(self.reconnect_delay).await?;
                }
                Ok(())
            }
        }
    }

    async fn on_connection_failure(&mut self, delay: Duration) -> Result<(), StateChange> {
        self.listener
            .update(ClientState::WaitAfterFailedConnect(delay));
        self.session.wait_for_retry(delay).await
    }
}
