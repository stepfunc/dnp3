use std::time::Duration;
use tracing::Instrument;

use crate::app::{ConnectStrategy, Listener};
use crate::app::{RetryStrategy, Shutdown};
use crate::link::LinkErrorMode;
use crate::master::session::MasterSession;
use crate::master::{MasterChannel, MasterChannelConfig};
use crate::shared::{RunError, StopReason};
use crate::tcp::EndpointList;
use crate::tcp::{ClientState, ConnectOptions, Connector, PostConnectionHandler};
use crate::transport::TransportReader;
use crate::transport::TransportWriter;
use crate::util::phys::PhysLayer;

/// Spawn a task onto the `Tokio` runtime. The task runs until the returned handle, and any
/// `AssociationHandle` created from it, are dropped.
///
/// **Note**: This function may only be called from within the runtime itself, and panics otherwise.
/// Use Runtime::enter() if required.
pub fn spawn_master_tcp_client(
    link_error_mode: LinkErrorMode,
    config: MasterChannelConfig,
    endpoints: EndpointList,
    connect_strategy: ConnectStrategy,
    listener: Box<dyn Listener<ClientState>>,
) -> MasterChannel {
    spawn_master_tcp_client_2(
        link_error_mode,
        config,
        endpoints,
        connect_strategy,
        ConnectOptions::default(),
        listener,
    )
}

/// Just like [spawn_master_tcp_client], but this variant was added later to also accept and
/// apply [ConnectOptions].
pub fn spawn_master_tcp_client_2(
    link_error_mode: LinkErrorMode,
    config: MasterChannelConfig,
    endpoints: EndpointList,
    connect_strategy: ConnectStrategy,
    connect_options: ConnectOptions,
    listener: Box<dyn Listener<ClientState>>,
) -> MasterChannel {
    let main_addr = endpoints.main_addr().to_string();
    let (mut task, handle) = MasterTask::new(
        link_error_mode,
        endpoints,
        config,
        connect_strategy,
        connect_options,
        PostConnectionHandler::Tcp,
        listener,
    );
    let future = async move {
        task.run()
            .instrument(tracing::info_span!("dnp3-master-tcp", "endpoint" = ?main_addr))
            .await;
    };
    tokio::spawn(future);
    handle
}

pub(crate) struct MasterTask {
    connector: Connector,
    reconnect_delay: Duration,
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
        connect_options: ConnectOptions,
        connection_handler: PostConnectionHandler,
        listener: Box<dyn Listener<ClientState>>,
    ) -> (Self, MasterChannel) {
        let (tx, rx) = crate::util::channel::request_channel();
        let session = MasterSession::new(false, config.decode_level, config.tx_buffer_size, rx);
        let (reader, writer) = crate::transport::create_master_transport_layer(
            link_error_mode,
            config.master_address,
            config.rx_buffer_size,
        );

        let retry_strategy = RetryStrategy::new(
            connect_strategy.min_connect_delay,
            connect_strategy.max_connect_delay,
        );

        let task = Self {
            connector: Connector::new(
                endpoints,
                connect_options,
                retry_strategy,
                connection_handler,
            ),
            reconnect_delay: connect_strategy.reconnect_delay,
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
            if let Err(StopReason::Shutdown) = self.run_connection().await {
                return Err(Shutdown);
            }
        }
    }

    async fn run_connection(&mut self) -> Result<(), StopReason> {
        loop {
            self.run_one_connection().await?;
        }
    }

    async fn run_one_connection(&mut self) -> Result<(), StopReason> {
        self.listener.update(ClientState::Connecting).get().await;
        match self.connector.connect().await {
            Ok(phys) => {
                self.listener.update(ClientState::Connected).get().await;
                self.run_phys(phys).await
            }
            Err(delay) => {
                tracing::info!("waiting {} ms to retry connection", delay.as_millis());
                self.listener
                    .update(ClientState::WaitAfterFailedConnect(delay))
                    .get()
                    .await;
                self.session.wait_for_retry(delay).await
            }
        }
    }

    async fn run_phys(&mut self, mut phys: PhysLayer) -> Result<(), StopReason> {
        match self
            .session
            .run(&mut phys, &mut self.writer, &mut self.reader)
            .await
        {
            RunError::Stop(s) => Err(s),
            RunError::Link(err) => {
                tracing::warn!("connection lost - {}", err);

                self.listener
                    .update(ClientState::WaitAfterDisconnect(self.reconnect_delay))
                    .get()
                    .await;

                if !self.reconnect_delay.is_zero() {
                    tracing::warn!(
                        "waiting {} ms to reconnect",
                        self.reconnect_delay.as_millis()
                    );

                    self.session.wait_for_retry(self.reconnect_delay).await?;
                }

                Ok(())
            }
        }
    }
}
