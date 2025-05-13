use crate::app::{Listener, Shutdown};
use crate::tcp::{
    ClientState, ConnectOptions, ConnectorHandler, Endpoint, EndpointInner, PostConnectionHandler,
};
use crate::util::phys::PhysLayer;
use crate::util::session::{RunError, Session, StopReason};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpSocket;

pub(crate) struct ClientTask {
    session: Session,
    connect_handler: Box<dyn ConnectorHandler>,
    connect_options: ConnectOptions,
    post_connection: PostConnectionHandler,
    listener: Box<dyn Listener<ClientState>>,
}

impl ClientTask {
    pub(crate) fn new(
        session: Session,
        connect_handler: Box<dyn ConnectorHandler>,
        connect_options: ConnectOptions,
        post_connection: PostConnectionHandler,
        listener: Box<dyn Listener<ClientState>>,
    ) -> Self {
        Self {
            connect_handler,
            connect_options,
            post_connection,
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

        match self.connect().await {
            Ok((phys, addr, hostname)) => {
                self.listener.update(ClientState::Connected).get().await;
                self.run_phys(phys, addr, hostname).await
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

    async fn connect(&mut self) -> Result<(PhysLayer, SocketAddr, Option<Arc<String>>), Duration> {
        loop {
            let endpoint = self.connect_handler.next()?;
            if let Some((phys, addr, hostname)) = self.connect_to_endpoint(endpoint).await {
                self.connect_handler
                    .connected(addr, hostname.as_ref().map(|x| x.as_str()));
                return Ok((phys, addr, hostname));
            }
        }
    }

    async fn connect_to_endpoint(
        &mut self,
        endpoint: Endpoint,
    ) -> Option<(PhysLayer, SocketAddr, Option<Arc<String>>)> {
        match endpoint.inner {
            EndpointInner::Address(addr) => {
                self.connect_to_addr(addr).await.map(|x| (x, addr, None))
            }
            EndpointInner::Hostname(hostname) => {
                match tokio::net::lookup_host(hostname.as_str()).await {
                    Ok(addrs) => {
                        for addr in addrs {
                            if let Some(x) = self.connect_to_addr(addr).await {
                                return Some((x, addr, Some(hostname.clone())));
                            }
                        }
                        None
                    }
                    Err(err) => {
                        tracing::warn!("Unable to resolve: '{hostname}', err: {err}");
                        self.connect_handler.resolution_failed(hostname.as_str());
                        None
                    }
                }
            }
        }
    }

    async fn connect_to_addr(&mut self, addr: SocketAddr) -> Option<PhysLayer> {
        let result = if addr.is_ipv4() {
            TcpSocket::new_v4()
        } else {
            TcpSocket::new_v6()
        };

        let socket = match result {
            Ok(x) => x,
            Err(err) => {
                tracing::warn!("unable to create socket: {}", err);
                return None;
            }
        };

        if let Some(local) = self.connect_options.local_endpoint {
            if let Err(err) = socket.bind(local) {
                tracing::warn!("unable to bind socket to {}: {}", local, err);
                return None;
            }
        }

        let fut = socket.connect(addr);
        let result = match self.connect_options.timeout {
            None => fut.await,
            Some(timeout) => match tokio::time::timeout(timeout, fut).await {
                Ok(x) => x,
                Err(_) => {
                    tracing::warn!(
                        "unable to connect to {} within timeout of {:?}",
                        addr,
                        timeout
                    );
                    return None;
                }
            },
        };

        let stream = match result {
            Ok(x) => x,
            Err(err) => {
                tracing::warn!("failed to connect to {}: {}", addr, err);
                return None;
            }
        };

        crate::tcp::configure_client(&stream);

        let phys = match self.post_connection.post_connect(stream, &addr).await {
            Some(x) => x,
            None => {
                return None;
            }
        };

        tracing::info!("connected to {}", addr);

        Some(phys)
    }

    async fn run_phys(
        &mut self,
        mut phys: PhysLayer,
        addr: SocketAddr,
        hostname: Option<Arc<String>>,
    ) -> Result<(), StopReason> {
        match self.session.run(&mut phys).await {
            RunError::Stop(s) => Err(s),
            RunError::Link(err) => {
                tracing::warn!("connection lost - {}", err);
                let hostname = hostname.as_ref().map(|x| x.as_str());
                let reconnect_delay = self.connect_handler.disconnected(addr, hostname);

                self.listener
                    .update(ClientState::WaitAfterDisconnect(reconnect_delay))
                    .get()
                    .await;

                if !reconnect_delay.is_zero() {
                    tracing::warn!("waiting {reconnect_delay:?} to reconnect");
                    self.session.wait_for_retry(reconnect_delay).await?;
                }

                Ok(())
            }
        }
    }
}
