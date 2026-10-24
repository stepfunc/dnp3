// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

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

    /// Set the local address to which the socket is bound.
    ///
    /// If not specified, the OS will select the network adapter based on the routing table
    /// for the destination address, and assign an ephemeral port.
    ///
    /// # Use Cases
    ///
    /// This is primarily useful for enforcing network segmentation in multi-homed systems,
    /// such as OT gateways that bridge device and enterprise networks. By explicitly binding
    /// to a specific adapter, you ensure traffic goes out the correct interface regardless
    /// of routing table configuration, providing defense against misconfiguration.
    ///
    /// # Port Selection
    ///
    /// Typically you should specify port 0 to let the OS assign an ephemeral port while
    /// forcing a specific network adapter. Using a specific non-zero port is rarely needed
    /// for client connections and may cause bind failures if the port is already in use.
    pub fn set_local_endpoint(&mut self, local: SocketAddr) {
        self.local_endpoint = Some(local);
    }

    /// Change the master address for this and all subsequent connections.
    ///
    /// The master address change is persistent and remains active across reconnections
    /// until explicitly changed again. This is useful when an outstation needs to communicate
    /// with different masters at different endpoints, such as during failover scenarios or
    /// when connecting to a backup master.
    ///
    /// By default, the outstation uses the master address specified in its configuration.
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
    #[expect(
        unused_variables,
        reason = "the default implementation ignores the notification"
    )]
    fn connecting(&mut self, addr: SocketAddr, hostname: Option<&str>) {}

    /// Notification that connection operation failed
    #[expect(
        unused_variables,
        reason = "the default implementation ignores the notification"
    )]
    fn connect_failed(&mut self, addr: SocketAddr, hostname: Option<&str>) {}

    /// Notification that a connection attempt succeeded
    #[expect(
        unused_variables,
        reason = "the default implementation ignores the notification"
    )]
    fn connected(&mut self, addr: SocketAddr, hostname: Option<&str>) {}

    /// Notification that DNS resolution failed. The task will sleep for the specified
    /// duration before attempting another connection.
    #[expect(
        unused_variables,
        reason = "the default implementation ignores the notification"
    )]
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
        match self.endpoints.first().map(|x| &x.inner) {
            Some(EndpointInner::Address(x)) => x.to_string(),
            Some(EndpointInner::Hostname(x)) => x.to_string(),
            None => String::new(),
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
                self.next = self.next.saturating_add(1);
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

#[cfg(test)]
mod tests {
    use super::*;

    const fn secs(x: u64) -> Duration {
        Duration::from_secs(x)
    }

    /// What a connection attempt targets: an address, or a hostname to resolve
    #[derive(Clone, Debug, PartialEq)]
    enum Target {
        Address(SocketAddr),
        Hostname(String),
    }

    /// The parts of a [`ConnectionInfo`] the handler fills in
    #[derive(Clone, Debug, PartialEq)]
    struct Attempt {
        target: Target,
        timeout: Option<Duration>,
        local_endpoint: Option<SocketAddr>,
    }

    fn addr(s: &str) -> SocketAddr {
        s.parse().unwrap()
    }

    /// The next attempt, or the delay before the list is tried again
    fn next(handler: &mut dyn ClientConnectionHandler) -> Result<Attempt, Duration> {
        handler.next().map(|info| Attempt {
            target: match info.endpoint.inner {
                EndpointInner::Address(x) => Target::Address(x),
                EndpointInner::Hostname(x) => Target::Hostname(x.to_string()),
            },
            timeout: info.timeout,
            local_endpoint: info.local_endpoint,
        })
    }

    /// Property: a `SimpleConnectHandler` gives the endpoints of its list in order, each with the
    /// configured connect timeout and local endpoint, then a delay before it starts over that
    /// begins at the minimum and doubles, saturating, up to the maximum. A connection returns it
    /// to the first endpoint and the minimum delay, and a disconnection always gives the
    /// reconnect delay.
    ///
    /// The oracle is an independent model of what the handler documents. It takes the minimum
    /// and maximum of the two configured delays, with the 1 ms floor of `RetryStrategy`, and
    /// doubles with its own saturating arithmetic, so it also checks `ExponentialBackOff`.
    ///
    /// The generator draws 1 to 4 endpoints, each an address or a hostname; delays of up to 5 s,
    /// zero, any number of seconds or `Duration::MAX`, in either order and both zero more often;
    /// a connect timeout and a local endpoint half the time each; and up to 63 operations, mostly
    /// asking for the next attempt. It is built to reach a list tried several times over, a delay
    /// held at the maximum, doubling that overflows, and connections in the middle of the list
    /// and after a delay. Not covered: what the client does with an attempt (resolving hostnames,
    /// connecting and the connect timeout), which the examples in `tcp/client.rs` pin.
    mod model {
        use super::*;

        use proptest::prelude::*;

        #[derive(Clone, Debug)]
        enum Op {
            Next,
            Connected,
            Disconnected,
        }

        fn op() -> impl Strategy<Value = Op> {
            prop_oneof![
                6 => Just(Op::Next),
                1 => Just(Op::Connected),
                1 => Just(Op::Disconnected),
            ]
        }

        /// short delays, zero, and delays so long that doubling them overflows
        fn delay() -> impl Strategy<Value = Duration> {
            prop_oneof![
                4 => (0u64..=5_000).prop_map(Duration::from_millis),
                1 => Just(Duration::ZERO),
                1 => any::<u64>().prop_map(Duration::from_secs),
                1 => Just(Duration::MAX),
            ]
        }

        /// the two bounds of the connect delay, in either order, and both zero more often than
        /// independent choices would make them
        fn connect_delays() -> impl Strategy<Value = (Duration, Duration)> {
            prop_oneof![
                4 => (delay(), delay()),
                1 => Just((Duration::ZERO, Duration::ZERO)),
            ]
        }

        /// the endpoint at `position` in the list: an address, or a hostname to resolve
        fn target(position: u8, hostname: bool) -> Target {
            if hostname {
                Target::Hostname(format!("outstation{position}.test:20000"))
            } else {
                Target::Address(SocketAddr::from(([192, 0, 2, position], 20000)))
            }
        }

        fn endpoint_string(target: &Target) -> String {
            match target {
                Target::Address(x) => x.to_string(),
                Target::Hostname(x) => x.clone(),
            }
        }

        /// What the handler documents: it tries the endpoints in order, then waits a delay that
        /// starts at the minimum and doubles up to the maximum before starting over; a connection
        /// returns it to the main endpoint and the minimum delay
        struct Model {
            attempts: Vec<Attempt>,
            next: usize,
            min_delay: Duration,
            max_delay: Duration,
            last_delay: Option<Duration>,
        }

        impl Model {
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "the model counts at most four endpoints"
            )]
            fn next(&mut self) -> Result<Attempt, Duration> {
                if let Some(attempt) = self.attempts.get(self.next) {
                    self.next += 1;
                    return Ok(attempt.clone());
                }
                self.next = 0;
                let delay = match self.last_delay {
                    None => self.min_delay,
                    Some(last) => last.saturating_mul(2).min(self.max_delay),
                };
                self.last_delay = Some(delay);
                Err(delay)
            }

            fn connected(&mut self) {
                self.next = 0;
                self.last_delay = None;
            }
        }

        proptest! {
            #![proptest_config(crate::util::test_support::proptest_config(256))]

            #[test]
            fn tries_endpoints_in_order_and_backs_off_like_the_model(
                hostnames in prop::collection::vec(any::<bool>(), 1..=4),
                (first_delay, second_delay) in connect_delays(),
                reconnect_delay in delay(),
                timeout in prop::option::of(1u64..=60),
                local_endpoint in any::<bool>(),
                ops in prop::collection::vec(op(), 0..64),
            ) {
                let targets: Vec<Target> = (1..)
                    .zip(&hostnames)
                    .map(|(position, hostname)| target(position, *hostname))
                    .collect();
                let mut list = EndpointList::single(endpoint_string(&targets[0]));
                for target in &targets[1..] {
                    list.add(endpoint_string(target));
                }
                let mut options = ConnectOptions::default();
                if let Some(timeout) = timeout {
                    options.set_connect_timeout(secs(timeout));
                }
                let local_endpoint = local_endpoint.then(|| addr("192.0.2.100:0"));
                if let Some(local_endpoint) = local_endpoint {
                    options.set_local_endpoint(local_endpoint);
                }
                let mut handler = SimpleConnectHandler::create(
                    list,
                    options,
                    ConnectStrategy::new(first_delay, second_delay, reconnect_delay),
                );

                let one_millisecond = Duration::from_millis(1);
                let mut model = Model {
                    attempts: targets
                        .into_iter()
                        .map(|target| Attempt {
                            target,
                            timeout: timeout.map(secs),
                            local_endpoint,
                        })
                        .collect(),
                    next: 0,
                    min_delay: first_delay.min(second_delay).max(one_millisecond),
                    max_delay: first_delay.max(second_delay).max(one_millisecond),
                    last_delay: None,
                };

                let connected_to = addr("192.0.2.1:20000");
                for op in ops {
                    match op {
                        Op::Next => prop_assert_eq!(next(handler.as_mut()), model.next()),
                        Op::Connected => {
                            handler.connected(connected_to, None);
                            model.connected();
                        }
                        Op::Disconnected => prop_assert_eq!(
                            handler.disconnected(connected_to, None),
                            reconnect_delay
                        ),
                    }
                }
            }
        }
    }
}
