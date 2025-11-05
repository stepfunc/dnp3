use crate::app::{ConnectStrategy, ExponentialBackOff, RetryStrategy};
use crate::link::EndpointAddress;
use crate::tcp::{ConnectOptions, EndpointList};
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
    /// resolve using a hostname, e.g. www.google.com:443
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

#[derive(Default, Copy, Clone, Debug)]
pub(crate) struct SessionSettings {
    pub(crate) master_address: Option<EndpointAddress>,
}

/// Information about the next connection attempt
#[derive(Clone, Debug)]
pub struct ConnectionInfo {
    pub(crate) endpoint: Endpoint,
    pub(crate) timeout: Option<Duration>,
    pub(crate) local_endpoint: Option<SocketAddr>,
    pub(crate) settings: SessionSettings,
}

impl ConnectionInfo {
    /// Instantiate with just the endpoint defined and all other parameters
    /// set to their default values.
    pub fn new(endpoint: Endpoint) -> Self {
        Self {
            endpoint,
            timeout: None,
            local_endpoint: None,
            settings: Default::default(),
        }
    }

    /// Sets a user-level connection timeout. This field defaults to [`None`]
    /// meaning that OS's default timeout mechanism is used.
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = Some(timeout);
    }

    /// Set the local address to which the socket is bound. If not specified, then any available
    /// adapter may be used with an OS-assigned port.
    pub fn set_local_endpoint(&mut self, local: SocketAddr) {
        self.local_endpoint = Some(local);
    }

    /// Use a different master address for this endpoint
    pub fn set_master_address(&mut self, address: EndpointAddress) {
        self.settings.master_address = Some(address);
    }
}

/// Provides fine-grained control over how TCP and TLS clients connect to endpoints
pub trait ClientConnectionHandler: Send {
    /// When a client communication session is first created, this function is called once
    /// to generate a string used in a `tracing::span`, e.g.:
    ///
    /// { endpoint = <endpoint_span_name> }
    ///
    /// This string is typically the SocketAddr or hostname of the main endpoint, but
    /// can be overridden by the user to be anything e.g., a unique name / UUID / etc.
    fn endpoint_span_name(&self) -> String;

    /// Notification that a previously successful connection failed. The task will sleep for the specified
    /// duration before attempting another connection
    fn disconnected(&mut self, addr: SocketAddr, hostname: Option<&str>) -> Duration;

    /// Return the next endpoint to which a connection will be attempted or indicate that
    /// the connecting task should sleep for a period of time.
    fn next(&mut self) -> Result<ConnectionInfo, Duration>;

    /// Notification that a connection attempt is being made
    #[allow(unused)]
    fn connecting(&mut self, addr: SocketAddr, hostname: Option<&str>) {}

    /// Notification that connection operation failed
    #[allow(unused)]
    fn connect_failed(&mut self, addr: SocketAddr, hostname: Option<&str>) {}

    /// Notification that a connection attempt succeeded
    #[allow(unused)]
    fn connected(&mut self, addr: SocketAddr, hostname: Option<&str>) {}

    /// Notification that DNS resolution failed. The task will sleep for the specified
    /// duration before attempting another connection.
    #[allow(unused)]
    fn resolution_failed(&mut self, host_name: &str) {}
}

pub(crate) struct SimpleConnectHandler {
    next: usize,
    endpoints: Vec<Endpoint>,
    options: ConnectOptions,
    backoff: ExponentialBackOff,
    reconnect_delay: Duration,
}

impl SimpleConnectHandler {
    pub(crate) fn create(
        list: EndpointList,
        options: ConnectOptions,
        connect_strategy: ConnectStrategy,
    ) -> Box<dyn ClientConnectionHandler> {
        Box::new(Self {
            next: 0,
            endpoints: list.endpoints(),
            options,
            backoff: ExponentialBackOff::new(RetryStrategy::new(
                connect_strategy.min_connect_delay,
                connect_strategy.max_connect_delay,
            )),
            reconnect_delay: connect_strategy.reconnect_delay,
        })
    }
}

impl ClientConnectionHandler for SimpleConnectHandler {
    fn endpoint_span_name(&self) -> String {
        match &self.endpoints[0].inner {
            EndpointInner::Address(x) => x.to_string(),
            EndpointInner::Hostname(x) => x.to_string(),
        }
    }

    fn disconnected(&mut self, _: SocketAddr, _: Option<&str>) -> Duration {
        self.reconnect_delay
    }

    fn next(&mut self) -> Result<ConnectionInfo, Duration> {
        match self.endpoints.get(self.next) {
            None => {
                self.next = 0;
                Err(self.backoff.on_failure())
            }
            Some(x) => {
                self.next += 1;
                let mut info = ConnectionInfo::new(x.clone());
                if let Some(x) = self.options.timeout {
                    info.set_timeout(x);
                }
                if let Some(x) = self.options.local_endpoint {
                    info.set_local_endpoint(x);
                }
                Ok(info)
            }
        }
    }

    fn connected(&mut self, _: SocketAddr, _: Option<&str>) {
        self.next = 0;
        self.backoff.on_success();
    }
}
