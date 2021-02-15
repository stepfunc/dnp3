use crate::app::retry::ExponentialBackOff;
use crate::link::LinkErrorMode;
use crate::master::handle::{Listener, MasterConfig, MasterHandle};
use crate::master::session::{MasterSession, RunError};
use crate::master::ClientState;
use crate::tokio::net::TcpStream;
use crate::transport::TransportReader;
use crate::transport::TransportWriter;
use crate::util::phys::PhysLayer;
use crate::util::task::Shutdown;
use std::collections::VecDeque;
use std::future::Future;
use std::net::SocketAddr;
use std::time::Duration;
use tracing::Instrument;

/// List of IP endpoints
///
/// You can write IP addresses or DNS names and the port to connect to. e.g. `"127.0.0.1:20000"` or `"dnp3.myorg.com:20000"`.
///
/// The main endpoint is always favoured. When a successful connection is established, it resets the next endpoint
/// to try, meaning if the connection is lost, the main endpoint will be retried first, then the other endpoints
/// in the order they were defined.
#[derive(Clone, Debug)]
pub struct EndpointList {
    endpoints: Vec<String>,
    pending_endpoints: VecDeque<SocketAddr>,
    current_endpoint: usize,
}

impl EndpointList {
    /// Create a list with a single endpoint
    pub fn single(addr: String) -> Self {
        Self::new(addr, &[])
    }

    /// Create a list with a main endpoint followed by fail-overs
    pub fn new(addr: String, fail_overs: &[String]) -> Self {
        let mut endpoints = vec![addr];
        endpoints.extend_from_slice(fail_overs);
        Self {
            endpoints,
            pending_endpoints: VecDeque::new(),
            current_endpoint: 0,
        }
    }

    /// Add an IP endpoint
    pub fn add(&mut self, addr: String) {
        self.endpoints.push(addr);
    }

    /// Returns the first (main) address of the list
    pub fn main_addr(&self) -> &str {
        self.endpoints.first().unwrap()
    }

    fn reset(&mut self) {
        self.pending_endpoints.clear();
        self.current_endpoint = 0;
    }

    async fn next_address(&mut self) -> Option<SocketAddr> {
        if let Some(endpoint) = self.pending_endpoints.pop_front() {
            return Some(endpoint);
        }

        let start_idx = self.current_endpoint;

        loop {
            let endpoint_idx = self.current_endpoint;

            // Increment the current endpoint
            self.current_endpoint = (self.current_endpoint + 1) % self.endpoints.len();

            // Resolve the name
            if let Ok(endpoints) =
                crate::tokio::net::lookup_host(&self.endpoints[endpoint_idx]).await
            {
                self.pending_endpoints = endpoints.collect();

                if let Some(endpoint) = self.pending_endpoints.pop_front() {
                    return Some(endpoint);
                }
            } else {
                tracing::warn!("unable to resolve \"{}\"", &self.endpoints[endpoint_idx]);
            }

            // We tried every possibility, but haven't found anything
            if start_idx == self.current_endpoint {
                return None;
            }
        }
    }
}

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
        self.run_impl().await.ok();
    }

    async fn run_impl(&mut self) -> Result<(), Shutdown> {
        loop {
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
                        self.session.delay_for(delay).await?;
                    }
                    Ok(socket) => {
                        let mut io = PhysLayer::Tcp(socket);
                        tracing::info!("connected to {}", endpoint);
                        self.endpoints.reset();
                        self.back_off.on_success();
                        self.listener.update(ClientState::Connected);
                        match self
                            .session
                            .run(&mut io, &mut self.writer, &mut self.reader)
                            .await
                        {
                            RunError::Shutdown => {
                                self.listener.update(ClientState::Shutdown);
                                return Err(Shutdown);
                            }
                            RunError::Link(err) => {
                                tracing::warn!("connection lost - {}", err);
                                if let Some(delay) = self.reconnect_delay {
                                    tracing::warn!("waiting {} ms to reconnect", delay.as_millis());
                                    self.listener
                                        .update(ClientState::WaitAfterDisconnect(delay));
                                    self.session.delay_for(delay).await?;
                                }
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
                self.listener
                    .update(ClientState::WaitAfterFailedConnect(delay));
                self.session.delay_for(delay).await?;
            }
        }
    }
}
