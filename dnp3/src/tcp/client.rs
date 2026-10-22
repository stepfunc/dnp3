use crate::app::Listener;
use crate::link::error::LinkError;
use crate::tcp::{
    ClientConnectionHandler, ClientState, ConnectionInfo, EndpointInner, PostConnectionHandler,
};
use crate::util::phys::PhysLayer;
use crate::util::session::{run_connection, Connection, Session};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpSocket;

pub(crate) struct ClientTask {
    session: Session,
    connect_handler: Box<dyn ClientConnectionHandler>,
    post_connection: PostConnectionHandler,
    listener: Box<dyn Listener<ClientState>>,
}

impl ClientTask {
    pub(crate) fn new(
        session: Session,
        connect_handler: Box<dyn ClientConnectionHandler>,
        post_connection: PostConnectionHandler,
        listener: Box<dyn Listener<ClientState>>,
    ) -> Self {
        Self {
            connect_handler,
            post_connection,
            session,
            listener,
        }
    }

    pub(crate) async fn run(&mut self) {
        run_connection(self).await;
        self.listener.update(ClientState::Shutdown).get().await;
    }

    async fn connect(&mut self) -> Result<(PhysLayer, SocketAddr, Option<Arc<String>>), Duration> {
        loop {
            let conn_info = self.connect_handler.next()?;
            let settings = conn_info.settings;
            if let Some((phys, addr, hostname)) = self.connect_to_endpoint(conn_info).await {
                if let Some(master_address) = settings.master_address {
                    self.session.change_master_address(master_address);
                }

                self.connect_handler
                    .connected(addr, hostname.as_ref().map(|x| x.as_str()));
                return Ok((phys, addr, hostname));
            }
        }
    }

    async fn connect_to_endpoint(
        &mut self,
        info: ConnectionInfo,
    ) -> Option<(PhysLayer, SocketAddr, Option<Arc<String>>)> {
        match info.endpoint.inner {
            EndpointInner::Address(addr) => self
                .connect_to_addr(addr, info.timeout, info.local_endpoint, None)
                .await
                .map(|x| (x, addr, None)),
            EndpointInner::Hostname(hostname) => {
                match tokio::net::lookup_host(hostname.as_str()).await {
                    Ok(addrs) => {
                        for addr in addrs {
                            if let Some(x) = self
                                .connect_to_addr(
                                    addr,
                                    info.timeout,
                                    info.local_endpoint,
                                    Some(hostname.as_str()),
                                )
                                .await
                            {
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

    async fn connect_to_addr(
        &mut self,
        addr: SocketAddr,
        timeout: Option<Duration>,
        local_endpoint: Option<SocketAddr>,
        hostname: Option<&str>,
    ) -> Option<PhysLayer> {
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

        if let Some(local) = local_endpoint {
            if let Err(err) = socket.bind(local) {
                tracing::warn!("unable to bind socket to {}: {}", local, err);
                return None;
            }
        }

        match hostname {
            None => tracing::info!("connecting to {addr}"),
            Some(host) => tracing::info!("connecting to {addr} ({host})"),
        }
        self.connect_handler.connecting(addr, hostname);

        let phys = self.connect_socket(socket, addr, timeout).await;
        if phys.is_none() {
            self.connect_handler.connect_failed(addr, hostname);
        }
        phys
    }

    async fn connect_socket(
        &mut self,
        socket: TcpSocket,
        addr: SocketAddr,
        timeout: Option<Duration>,
    ) -> Option<PhysLayer> {
        let fut = socket.connect(addr);
        let result = match timeout {
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

        let phys = self.post_connection.post_connect(stream, &addr).await?;

        tracing::info!("connected to {}", addr);

        Some(phys)
    }
}

impl Connection for ClientTask {
    type Peer = (SocketAddr, Option<Arc<String>>);

    fn session(&mut self) -> &mut Session {
        &mut self.session
    }

    async fn disabled(&mut self) {
        self.listener.update(ClientState::Disabled).get().await;
    }

    async fn open(&mut self) -> Result<(PhysLayer, Self::Peer), Duration> {
        self.listener.update(ClientState::Connecting).get().await;

        match self.connect().await {
            Ok((phys, addr, hostname)) => {
                self.listener.update(ClientState::Connected).get().await;
                Ok((phys, (addr, hostname)))
            }
            Err(delay) => {
                tracing::info!("waiting {} ms to retry connection", delay.as_millis());
                self.listener
                    .update(ClientState::WaitAfterFailedConnect(delay))
                    .get()
                    .await;
                Err(delay)
            }
        }
    }

    async fn failed(&mut self, (addr, hostname): Self::Peer, err: LinkError) -> Duration {
        tracing::warn!("connection to {addr} failed: {err}");
        let hostname = hostname.as_ref().map(|x| x.as_str());
        let reconnect_delay = self.connect_handler.disconnected(addr, hostname);

        self.listener
            .update(ClientState::WaitAfterDisconnect(reconnect_delay))
            .get()
            .await;

        if !reconnect_delay.is_zero() {
            tracing::warn!("waiting {reconnect_delay:?} to reconnect");
        }
        reconnect_delay
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::NullListener;
    use crate::link::reader::LinkModes;
    use crate::link::EndpointAddress;
    use crate::master::{MasterChannelConfig, MasterChannelType};
    use crate::tcp::{wire_master_client, Endpoint};
    use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

    #[derive(Debug, PartialEq, Eq)]
    enum Event {
        Connecting(SocketAddr),
        ConnectFailed(SocketAddr),
        Connected(SocketAddr),
    }

    struct RecordingHandler {
        endpoint: Option<SocketAddr>,
        events: UnboundedSender<Event>,
    }

    impl ClientConnectionHandler for RecordingHandler {
        fn endpoint_span_name(&self) -> String {
            "test".to_string()
        }

        fn disconnected(&mut self, _: SocketAddr, _: Option<&str>) -> Duration {
            Duration::from_secs(60)
        }

        fn next(&mut self) -> Result<ConnectionInfo, Duration> {
            // attempt a single connection, then ask the task to sleep
            match self.endpoint.take() {
                Some(addr) => Ok(ConnectionInfo::new(Endpoint::address(addr))),
                None => Err(Duration::from_secs(60)),
            }
        }

        fn connecting(&mut self, addr: SocketAddr, _: Option<&str>) {
            drop(self.events.send(Event::Connecting(addr)));
        }

        fn connect_failed(&mut self, addr: SocketAddr, _: Option<&str>) {
            drop(self.events.send(Event::ConnectFailed(addr)));
        }

        fn connected(&mut self, addr: SocketAddr, _: Option<&str>) {
            drop(self.events.send(Event::Connected(addr)));
        }
    }

    fn client_task(
        addr: SocketAddr,
        post_connection: PostConnectionHandler,
    ) -> (ClientTask, UnboundedReceiver<Event>) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let handler = RecordingHandler {
            endpoint: Some(addr),
            events: tx,
        };
        let (task, _channel) = wire_master_client(
            LinkModes::test(),
            MasterChannelType::Stream,
            Box::new(handler),
            MasterChannelConfig::new(EndpointAddress::raw(1)),
            post_connection,
            NullListener::create(),
        );
        (task, rx)
    }

    fn events(rx: &mut UnboundedReceiver<Event>) -> Vec<Event> {
        let mut events = Vec::new();
        while let Ok(x) = rx.try_recv() {
            events.push(x);
        }
        events
    }

    #[tokio::test]
    async fn refused_connection_notifies_connect_failed() {
        // the port stays reserved by a bound socket that never listens, so connects are refused
        let reserved = TcpSocket::new_v4().unwrap();
        reserved.bind("127.0.0.1:0".parse().unwrap()).unwrap();
        let addr = reserved.local_addr().unwrap();
        let (mut task, mut rx) = client_task(addr, PostConnectionHandler::Tcp);

        assert_eq!(task.connect().await.err(), Some(Duration::from_secs(60)));
        assert_eq!(
            events(&mut rx),
            [Event::Connecting(addr), Event::ConnectFailed(addr)]
        );
        drop(reserved);
    }

    #[tokio::test]
    async fn successful_connection_notifies_connected() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (mut task, mut rx) = client_task(addr, PostConnectionHandler::Tcp);

        assert_eq!(task.connect().await.ok().map(|(_, x, _)| x), Some(addr));
        assert_eq!(
            events(&mut rx),
            [Event::Connecting(addr), Event::Connected(addr)]
        );
    }

    #[cfg(feature = "enable-tls")]
    #[tokio::test]
    async fn failed_tls_handshake_notifies_connect_failed() {
        let config = crate::util::test_support::tls::client_config();

        // a server that closes the connection without answering the ClientHello
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            drop(socket);
        });

        let (mut task, mut rx) = client_task(addr, PostConnectionHandler::Tls(config));

        assert_eq!(task.connect().await.err(), Some(Duration::from_secs(60)));
        server.await.unwrap();
        assert_eq!(
            events(&mut rx),
            [Event::Connecting(addr), Event::ConnectFailed(addr)]
        );
    }
}
