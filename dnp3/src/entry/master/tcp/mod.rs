use crate::app::parse::DecodeLogLevel;
use crate::app::timeout::Timeout;
use crate::master::error::Shutdown;
use crate::master::handle::{Listener, MasterHandle};
use crate::master::runner::{RunError, Runner};
use crate::transport::{TransportReader, TransportType, TransportWriter};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::macros::support::Future;
use tokio::net::TcpStream;

#[derive(Copy, Clone)]
pub struct ReconnectStrategy {
    min_delay: Duration,
    max_delay: Duration,
}

impl ReconnectStrategy {
    pub fn new(min_delay: Duration, max_delay: Duration) -> Self {
        Self {
            min_delay,
            max_delay,
        }
    }
}

/// state of TCP client connection
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ClientState {
    Connecting,
    Connected,
    WaitAfterFailedConnect(Duration),
    WaitAfterDisconnect(Duration),
    Shutdown,
}

struct ExponentialBackOff {
    strategy: ReconnectStrategy,
    last: Option<Duration>,
}

impl ExponentialBackOff {
    fn new(strategy: ReconnectStrategy) -> Self {
        Self {
            strategy,
            last: None,
        }
    }

    fn on_connect_success(&mut self) {
        self.last = None;
    }

    fn on_connect_failure(&mut self) -> Duration {
        match self.last {
            Some(x) => {
                let next = x
                    .checked_mul(2)
                    .unwrap_or(self.strategy.max_delay)
                    .min(self.strategy.max_delay);
                self.last = Some(next);
                next
            }
            None => {
                self.last = Some(self.strategy.min_delay);
                self.strategy.min_delay
            }
        }
    }

    fn min_delay(&self) -> Duration {
        self.strategy.min_delay
    }
}

impl Default for ReconnectStrategy {
    fn default() -> Self {
        Self::new(Duration::from_secs(1), Duration::from_secs(10))
    }
}

pub(crate) struct MasterTask {
    endpoint: SocketAddr,
    back_off: ExponentialBackOff,
    runner: Runner,
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
    address: u16,
    level: DecodeLogLevel,
    strategy: ReconnectStrategy,
    timeout: Timeout,
    endpoint: SocketAddr,
    listener: Listener<ClientState>,
) -> MasterHandle {
    let (mut task, handle) = MasterTask::new(address, level, strategy, timeout, endpoint, listener);
    tokio::spawn(async move { task.run().await });
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
    address: u16,
    level: DecodeLogLevel,
    strategy: ReconnectStrategy,
    response_timeout: Timeout,
    endpoint: SocketAddr,
    listener: Listener<ClientState>,
) -> (impl Future<Output = ()> + 'static, MasterHandle) {
    let (mut task, handle) = MasterTask::new(
        address,
        level,
        strategy,
        response_timeout,
        endpoint,
        listener,
    );
    (async move { task.run().await }, handle)
}

impl MasterTask {
    fn new(
        address: u16,
        level: DecodeLogLevel,
        strategy: ReconnectStrategy,
        response_timeout: Timeout,
        endpoint: SocketAddr,
        listener: Listener<ClientState>,
    ) -> (Self, MasterHandle) {
        let (tx, rx) = tokio::sync::mpsc::channel(100); // TODO
        let runner = Runner::new(level, response_timeout, rx);
        let (reader, writer) =
            crate::transport::create_transport_layer(TransportType::Master, address);
        let task = Self {
            endpoint,
            back_off: ExponentialBackOff::new(strategy),
            runner,
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
                    let delay = self.back_off.on_connect_failure();
                    log::warn!("{} - waiting {} ms to retry", err, delay.as_millis());
                    self.listener
                        .update(ClientState::WaitAfterFailedConnect(delay));
                    self.runner.delay_for(delay).await?;
                }
                Ok(mut socket) => {
                    log::info!("connected to: {}", self.endpoint);
                    self.back_off.on_connect_success();
                    self.listener.update(ClientState::Connected);
                    match self
                        .runner
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
                            self.runner.delay_for(delay).await?;
                        }
                    }
                }
            }
        }
    }
}
