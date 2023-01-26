use crate::app::{ConnectStrategy, Listener, RetryStrategy, Shutdown};
use crate::tcp::{ClientState, ConnectOptions, Connector, EndpointList, PostConnectionHandler};
use crate::util::phys::PhysLayer;
use crate::util::session::{RunError, Session, StopReason};
use std::time::Duration;

pub(crate) struct ClientTask {
    session: Session,
    connector: Connector,
    reconnect_delay: Duration,
    listener: Box<dyn Listener<ClientState>>,
}

impl ClientTask {
    pub(crate) fn new(
        session: Session,
        endpoints: EndpointList,
        connect_strategy: ConnectStrategy,
        connect_options: ConnectOptions,
        connection_handler: PostConnectionHandler,
        listener: Box<dyn Listener<ClientState>>,
    ) -> Self {
        let retry_strategy = RetryStrategy::new(
            connect_strategy.min_connect_delay,
            connect_strategy.max_connect_delay,
        );

        Self {
            connector: Connector::new(
                endpoints,
                connect_options,
                retry_strategy,
                connection_handler,
            ),
            reconnect_delay: connect_strategy.reconnect_delay,
            session,
            listener,
        }
    }

    pub(crate) async fn run(&mut self) {
        let _ = self.run_impl().await;
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
        match self.session.run(&mut phys).await {
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
