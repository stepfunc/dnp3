use crate::app::{ConnectStrategy, ExponentialBackOff, RetryStrategy};
use crate::tcp::EndpointList;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

/// An endpoint as either an IP address or a hostname to resolve
#[derive(Clone, Debug)]
pub struct Endpoint {
    pub(crate) inner: EndpointInner,
}

/// An endpoint as either an IP address or a hostname to resolve
#[derive(Clone, Debug)]
pub(crate) enum EndpointInner {
    /// Socket address, e.g. 192.168.1.42:20000
    Address(SocketAddr),
    /// resolve using a hostname
    Hostname(Arc<String>),
}

impl Endpoint {
    /// Create an endpoint which will be resolved as a hostname
    pub fn hostname(name: String) -> Self {
        Self {
            inner: EndpointInner::Hostname(Arc::new(name)),
        }
    }

    /// Create an endpoint which will be resolved as a hostname
    pub fn address(addr: SocketAddr) -> Self {
        Self {
            inner: EndpointInner::Address(addr),
        }
    }
}

impl From<String> for Endpoint {
    fn from(s: String) -> Self {
        if let Ok(addr) = s.parse() {
            Self::address(addr)
        } else {
            Self::hostname(s)
        }
    }
}

/// Controls how TCP and TLS clients connect to endpoints
pub(crate) trait ConnectorHandler: Send {
    /// Endpoint that will be logged in conjunction with a tracing span
    fn main_endpoint(&self) -> Endpoint;

    /// Return the next endpoint to which a connection will be attempted
    fn next(&mut self) -> Result<Endpoint, Duration>;

    /// Notification that connection operation failed
    fn connect_failed(&mut self, addr: SocketAddr, hostname: Option<&str>);

    /// Notification that a connection attempt succeeded
    fn connected(&mut self, addr: SocketAddr, hostname: Option<&str>);

    /// Notification that a previously successful connection failed. The task will sleep for the specified
    /// duration before attempting another connection
    fn disconnected(&mut self, addr: SocketAddr, hostname: Option<&str>) -> Duration;

    /// Notification that DNS resolution failed. The task will sleep for the specified
    /// duration before attempting another connection.
    fn resolution_failed(&mut self, host_name: &str);
}

pub(crate) struct SimpleConnectHandler {
    next: usize,
    endpoints: Vec<Endpoint>,
    backoff: ExponentialBackOff,
    reconnect_delay: Duration,
}

impl SimpleConnectHandler {
    pub(crate) fn create(
        list: EndpointList,
        connect_strategy: ConnectStrategy,
    ) -> Box<dyn ConnectorHandler> {
        Box::new(Self {
            next: 0,
            endpoints: list.endpoints(),
            backoff: ExponentialBackOff::new(RetryStrategy::new(
                connect_strategy.min_connect_delay,
                connect_strategy.max_connect_delay,
            )),
            reconnect_delay: connect_strategy.reconnect_delay,
        })
    }
}

impl ConnectorHandler for SimpleConnectHandler {
    fn main_endpoint(&self) -> Endpoint {
        self.endpoints[0].clone()
    }

    fn next(&mut self) -> Result<Endpoint, Duration> {
        match self.endpoints.get(self.next) {
            None => {
                self.next = 0;
                Err(self.backoff.on_failure())
            }
            Some(x) => {
                self.next += 1;
                Ok(x.clone())
            }
        }
    }

    fn connect_failed(&mut self, _: SocketAddr, _: Option<&str>) {}

    fn connected(&mut self, _: SocketAddr, _: Option<&str>) {
        self.next = 0;
        self.backoff.on_success();
    }

    fn disconnected(&mut self, _: SocketAddr, _: Option<&str>) -> Duration {
        self.reconnect_delay
    }

    fn resolution_failed(&mut self, _: &str) {}
}
