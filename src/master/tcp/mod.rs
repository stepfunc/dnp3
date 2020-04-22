use crate::master::session::SessionMap;

use crate::app::parse::parser::ParseLogLevel;
use crate::master::runner::{MasterHandle, RunError, Runner, Shutdown};
use crate::transport::{ReaderType, WriterType};
use std::net::SocketAddr;
use std::time::Duration;
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

pub struct MasterTask {
    endpoint: SocketAddr,
    back_off: ExponentialBackOff,
    runner: Runner,
    reader: ReaderType,
    writer: WriterType,
}

impl MasterTask {
    pub fn spawn(
        address: u16,
        level: ParseLogLevel,
        strategy: ReconnectStrategy,
        timeout: Duration,
        endpoint: SocketAddr,
        sessions: SessionMap,
    ) -> MasterHandle {
        let (mut task, handle) =
            MasterTask::new(address, level, strategy, timeout, endpoint, sessions);
        tokio::spawn(async move { task.run().await });
        handle
    }

    pub fn new(
        address: u16,
        level: ParseLogLevel,
        strategy: ReconnectStrategy,
        response_timeout: Duration,
        endpoint: SocketAddr,
        sessions: SessionMap,
    ) -> (Self, MasterHandle) {
        let (tx, rx) = tokio::sync::mpsc::channel(100); // TODO
        let runner = Runner::new(level, response_timeout, sessions, rx);
        let (reader, writer) = crate::transport::create_transport_layer(true, address);
        let task = Self {
            endpoint,
            back_off: ExponentialBackOff::new(strategy),
            runner,
            reader,
            writer,
        };
        (task, MasterHandle::new(tx))
    }

    pub async fn run(&mut self) -> Result<(), Shutdown> {
        loop {
            match TcpStream::connect(self.endpoint).await {
                Err(err) => {
                    let delay = self.back_off.on_connect_failure();
                    log::warn!("{} - waiting {} ms to retry", err, delay.as_millis());
                    self.runner.delay_for(delay).await?;
                }
                Ok(mut socket) => {
                    self.back_off.on_connect_success();
                    match self
                        .runner
                        .run(&mut socket, &mut self.writer, &mut self.reader)
                        .await
                    {
                        RunError::Shutdown => return Err(Shutdown),
                        RunError::Link(err) => {
                            let delay = self.back_off.min_delay();
                            log::warn!("{} - waiting { }ms to reconnect", err, delay.as_millis());
                            self.runner.reset();
                            self.reader.reset();
                            self.writer.reset();
                            self.runner.delay_for(delay).await?;
                        }
                    }
                }
            }
        }
    }
}
