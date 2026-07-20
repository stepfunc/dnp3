use std::net::SocketAddr;
use std::time::Duration;

use super::association::{Association, AssociationMap};
use super::handler::{AssociationHandle, MasterChannel, MasterChannelConfig, MasterChannelType};
use super::poll::PollHandle;
use super::request::ReadRequest;
use super::task::MasterTask as InnerMasterTask;
use super::{
    AssociationConfig, AssociationError, AssociationHandler, AssociationInformation, ReadHandler,
};
use crate::app::parse::options::ParseOptions;
use crate::app::{Listener, Timeout};
use crate::link::reader::LinkModes;
use crate::link::{EndpointAddress, LinkErrorMode, LinkReadMode};
use crate::tcp::client::ClientTask;
use crate::tcp::{ClientConnectionHandler, ClientState, PostConnectionHandler};
use crate::transport::FragmentAddr;
use crate::udp::layer::UdpFactory;
use crate::udp::task::UdpTask;
use crate::util::channel::Receiver;
use crate::util::phys::PhysAddr;
use crate::util::session::{Enabled, Session};
use tokio::time::Instant;

/// Builder for configuring a stream-based master task (TCP, TLS, serial)
/// before binding it to a transport.
///
/// Created via [`MasterBuilder::new`]. The returned [`MasterChannel`] can be
/// used for post-run operations, but async methods on it (and on any
/// [`AssociationHandle`] obtained from [`add_association`](Self::add_association))
/// require the task to be running.
pub struct MasterBuilder {
    channel: MasterChannel,
    rx: Receiver<super::messages::Message>,
    config: MasterChannelConfig,
    enabled: Enabled,
    associations: AssociationMap,
}

/// Builder for configuring a UDP master task before binding it to a transport.
///
/// Created via [`UdpMasterBuilder::new`]. The returned [`MasterChannel`] can be
/// used for post-run operations, but async methods on it (and on any
/// [`AssociationHandle`] obtained from [`add_association`](Self::add_association))
/// require the task to be running.
pub struct UdpMasterBuilder {
    channel: MasterChannel,
    rx: Receiver<super::messages::Message>,
    config: MasterChannelConfig,
    enabled: Enabled,
    associations: AssociationMap,
}

/// Builder for configuring an association and its polls before the master task
/// starts.
///
/// Created by [`MasterBuilder::add_association`] or
/// [`UdpMasterBuilder::add_association`] after the association has been
/// registered. Its lifetime binds it to the parent master builder, preventing
/// it from being registered with a different master or transport.
#[must_use = "call .into_handle() if runtime access to the association is needed"]
pub struct AssociationBuilder<'a> {
    handle: AssociationHandle,
    association: &'a mut Association,
}

/// A master task bound to a transport. Call [`run`](Self::run) to execute
/// the event loop.
///
/// Created by binding a [`MasterBuilder`] or [`UdpMasterBuilder`] to a transport.
///
/// No tracing span is attached automatically. This is by design so that the caller retains full
/// control over instrumentation and may wrap the [`run`](Self::run) future in whatever span it
/// chooses before spawning it.
#[must_use = "a MasterTask does nothing unless you call .run()"]
pub struct MasterTask {
    inner: MasterTaskType,
}

enum MasterTaskType {
    Tcp(TcpTask),
    #[cfg(feature = "enable-tls")]
    Tls(TlsTask),
    #[cfg(feature = "serial")]
    Serial(SerialTask),
    Udp(UdpMasterTask),
}

struct TcpTask {
    inner: InnerMasterTask,
    connect_handler: Box<dyn ClientConnectionHandler>,
    listener: Box<dyn Listener<ClientState>>,
}

#[cfg(feature = "enable-tls")]
struct TlsTask {
    inner: InnerMasterTask,
    connect_handler: Box<dyn ClientConnectionHandler>,
    tls_config: crate::tcp::tls::TlsClientConfig,
    listener: Box<dyn Listener<ClientState>>,
}

#[cfg(feature = "serial")]
struct SerialTask {
    inner: InnerMasterTask,
    path: String,
    settings: crate::serial::SerialSettings,
    retry_strategy: crate::app::RetryStrategy,
    listener: Box<dyn Listener<crate::serial::PortState>>,
}

struct UdpMasterTask {
    inner: InnerMasterTask,
    local_endpoint: SocketAddr,
    retry_delay: Timeout,
}

fn build_inner_task(
    enabled: Enabled,
    associations: AssociationMap,
    link_modes: LinkModes,
    config: MasterChannelConfig,
    rx: Receiver<super::messages::Message>,
) -> InnerMasterTask {
    InnerMasterTask::with_associations(
        enabled,
        associations,
        link_modes,
        ParseOptions::get_static(),
        config,
        rx,
    )
}

impl MasterBuilder {
    /// Create a new master builder and its associated [`MasterChannel`].
    ///
    /// The channel can be cloned and stored for later use. Async operations
    /// on the channel require the task to be running.
    pub fn new(config: MasterChannelConfig) -> (Self, MasterChannel) {
        let (tx, rx) = crate::util::channel::request_channel();
        let channel = MasterChannel::new(tx, MasterChannelType::Stream);
        let builder = Self {
            channel: channel.clone(),
            rx,
            config,
            enabled: Enabled::No,
            associations: AssociationMap::new(),
        };
        (builder, channel)
    }

    /// Enable communications. This is a synchronous direct mutation of the
    /// master's initial state, unlike [`MasterChannel::enable`] which sends
    /// a message to an already-running task.
    pub fn enable(&mut self) {
        self.enabled = Enabled::Yes;
    }

    /// Register an association and return a scoped builder for adding polls
    /// before the master task starts.
    pub fn add_association(
        &mut self,
        address: EndpointAddress,
        config: AssociationConfig,
        read_handler: Box<dyn ReadHandler>,
        assoc_handler: Box<dyn AssociationHandler>,
        assoc_information: Box<dyn AssociationInformation>,
    ) -> Result<AssociationBuilder<'_>, AssociationError> {
        let addr = FragmentAddr {
            link: address,
            phys: PhysAddr::None,
        };
        let handle = AssociationHandle::new(address, self.channel.clone());
        let association = self
            .associations
            .register_and_get(Association::new_deferred(
                addr,
                config,
                read_handler,
                assoc_handler,
                assoc_information,
            ))?;
        Ok(AssociationBuilder {
            handle,
            association,
        })
    }

    /// Bind to a TCP transport, consuming the builder.
    ///
    /// * `link_error_mode` controls how link-layer errors are handled
    /// * `connection_handler` controls connection and reconnection behavior
    /// * `listener` receives connection state updates
    pub fn into_tcp(
        self,
        link_error_mode: LinkErrorMode,
        connection_handler: Box<dyn ClientConnectionHandler>,
        listener: Box<dyn Listener<ClientState>>,
    ) -> MasterTask {
        MasterTask {
            inner: MasterTaskType::Tcp(TcpTask {
                inner: build_inner_task(
                    self.enabled,
                    self.associations,
                    LinkModes::stream(link_error_mode),
                    self.config,
                    self.rx,
                ),
                connect_handler: connection_handler,
                listener,
            }),
        }
    }

    /// Bind to a TLS transport, consuming the builder.
    ///
    /// * `link_error_mode` controls how link-layer errors are handled
    /// * `connection_handler` controls connection and reconnection behavior
    /// * `listener` receives connection state updates
    /// * `tls_config` provides the TLS client configuration
    #[cfg(feature = "enable-tls")]
    pub fn into_tls(
        self,
        link_error_mode: LinkErrorMode,
        connection_handler: Box<dyn ClientConnectionHandler>,
        listener: Box<dyn Listener<ClientState>>,
        tls_config: crate::tcp::tls::TlsClientConfig,
    ) -> MasterTask {
        MasterTask {
            inner: MasterTaskType::Tls(TlsTask {
                inner: build_inner_task(
                    self.enabled,
                    self.associations,
                    LinkModes::stream(link_error_mode),
                    self.config,
                    self.rx,
                ),
                connect_handler: connection_handler,
                tls_config,
                listener,
            }),
        }
    }

    /// Bind to a serial transport, consuming the builder.
    ///
    /// * `path` is the serial port path (e.g. "/dev/ttyS0")
    /// * `serial_settings` configures baud rate, parity, etc.
    /// * `retry_delay` is the delay before retrying after a failed open or link error
    /// * `listener` receives port state updates
    #[cfg(feature = "serial")]
    pub fn into_serial(
        self,
        path: &str,
        serial_settings: crate::serial::SerialSettings,
        retry_delay: Duration,
        listener: Box<dyn Listener<crate::serial::PortState>>,
    ) -> MasterTask {
        MasterTask {
            inner: MasterTaskType::Serial(SerialTask {
                inner: build_inner_task(
                    self.enabled,
                    self.associations,
                    LinkModes::serial(),
                    self.config,
                    self.rx,
                ),
                path: path.to_string(),
                settings: serial_settings,
                retry_strategy: crate::app::RetryStrategy::new(retry_delay, retry_delay),
                listener,
            }),
        }
    }
}

impl UdpMasterBuilder {
    /// Create a new UDP master builder and its associated [`MasterChannel`].
    ///
    /// The channel can be cloned and stored for later use. Async operations
    /// on the channel require the task to be running.
    pub fn new(config: MasterChannelConfig) -> (Self, MasterChannel) {
        let (tx, rx) = crate::util::channel::request_channel();
        let channel = MasterChannel::new(tx, MasterChannelType::Udp);
        let builder = Self {
            channel: channel.clone(),
            rx,
            config,
            enabled: Enabled::No,
            associations: AssociationMap::new(),
        };
        (builder, channel)
    }

    /// Enable communications. This is a synchronous direct mutation of the
    /// master's initial state, unlike [`MasterChannel::enable`] which sends
    /// a message to an already-running task.
    pub fn enable(&mut self) {
        self.enabled = Enabled::Yes;
    }

    /// Register a UDP association and return a scoped builder for adding polls
    /// before the master task starts.
    ///
    /// * `address` is the DNP3 link-layer address of the outstation
    /// * `destination` is the IP address and port of the outstation
    pub fn add_association(
        &mut self,
        address: EndpointAddress,
        destination: SocketAddr,
        config: AssociationConfig,
        read_handler: Box<dyn ReadHandler>,
        assoc_handler: Box<dyn AssociationHandler>,
        assoc_information: Box<dyn AssociationInformation>,
    ) -> Result<AssociationBuilder<'_>, AssociationError> {
        let addr = FragmentAddr {
            link: address,
            phys: PhysAddr::Udp(destination),
        };
        let handle = AssociationHandle::new(address, self.channel.clone());
        let association = self
            .associations
            .register_and_get(Association::new_deferred(
                addr,
                config,
                read_handler,
                assoc_handler,
                assoc_information,
            ))?;
        Ok(AssociationBuilder {
            handle,
            association,
        })
    }

    /// Bind to a UDP transport, consuming the builder.
    ///
    /// * `local_endpoint` is the local IP address and port to bind to
    /// * `read_mode` controls how link-layer frames are read from datagrams
    /// * `retry_delay` is the delay before retrying after a failed bind
    pub fn into_udp(
        self,
        local_endpoint: SocketAddr,
        read_mode: LinkReadMode,
        retry_delay: Timeout,
    ) -> MasterTask {
        let link_modes = LinkModes {
            error_mode: LinkErrorMode::Discard,
            read_mode,
        };
        MasterTask {
            inner: MasterTaskType::Udp(UdpMasterTask {
                inner: build_inner_task(
                    self.enabled,
                    self.associations,
                    link_modes,
                    self.config,
                    self.rx,
                ),
                local_endpoint,
                retry_delay,
            }),
        }
    }
}

impl AssociationBuilder<'_> {
    /// Add a periodic poll to the association.
    ///
    /// Returns a [`PollHandle`] that can be used for post-run operations.
    /// Async methods on the handle require the task to be running.
    pub fn add_poll(&mut self, request: ReadRequest, period: Duration) -> PollHandle {
        let id = self.association.add_poll(request, period);
        PollHandle::new(self.handle.clone(), id)
    }

    /// Convert this scoped builder into an [`AssociationHandle`].
    ///
    /// This releases the mutable borrow of the parent master builder. Async
    /// methods on the returned handle require the master task to be running.
    pub fn into_handle(self) -> AssociationHandle {
        self.handle
    }
}

impl MasterTask {
    /// Run the master event loop.
    ///
    /// This method consumes the task and runs until the [`MasterChannel`] (and
    /// all associated handles) are dropped.
    pub async fn run(mut self) {
        self.inner.start(Instant::now());
        match self.inner {
            MasterTaskType::Tcp(task) => task.run().await,
            #[cfg(feature = "enable-tls")]
            MasterTaskType::Tls(task) => task.run().await,
            #[cfg(feature = "serial")]
            MasterTaskType::Serial(task) => task.run().await,
            MasterTaskType::Udp(task) => task.run().await,
        }
    }
}

impl MasterTaskType {
    fn start(&mut self, now: Instant) {
        match self {
            MasterTaskType::Tcp(task) => task.inner.start(now),
            #[cfg(feature = "enable-tls")]
            MasterTaskType::Tls(task) => task.inner.start(now),
            #[cfg(feature = "serial")]
            MasterTaskType::Serial(task) => task.inner.start(now),
            MasterTaskType::Udp(task) => task.inner.start(now),
        }
    }
}

impl TcpTask {
    async fn run(self) {
        let session = Session::master(self.inner);
        let mut client = ClientTask::new(
            session,
            self.connect_handler,
            PostConnectionHandler::Tcp,
            self.listener,
        );
        client.run().await;
    }
}

#[cfg(feature = "enable-tls")]
impl TlsTask {
    async fn run(self) {
        let session = Session::master(self.inner);
        let mut client = ClientTask::new(
            session,
            self.connect_handler,
            PostConnectionHandler::Tls(self.tls_config),
            self.listener,
        );
        client.run().await;
    }
}

#[cfg(feature = "serial")]
impl SerialTask {
    async fn run(self) {
        let session = Session::master(self.inner);
        let mut serial = crate::serial::task::SerialTask::new(
            &self.path,
            self.settings,
            session,
            self.retry_strategy,
            self.listener,
        );
        serial.run().await;
    }
}

impl UdpMasterTask {
    async fn run(self) {
        let local_endpoint = self.local_endpoint;
        let session = Session::master(self.inner);
        let task = UdpTask {
            session,
            factory: UdpFactory::bound(local_endpoint),
            retry_delay: self.retry_delay,
        };
        let _ = task.run().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::MaybeAsync;
    use crate::master::messages::{AssociationMsgType, Message};
    use crate::master::poll::PollMsg;
    use std::marker::PhantomData;

    struct ChannelListener<T> {
        tx: tokio::sync::mpsc::UnboundedSender<T>,
    }

    impl<T: Send + Sync + 'static> Listener<T> for ChannelListener<T> {
        fn update(&mut self, value: T) -> MaybeAsync<()> {
            let _ = self.tx.send(value);
            MaybeAsync::ready(())
        }
    }

    fn channel_listener<T: Send + Sync + 'static>() -> (
        Box<dyn Listener<T>>,
        tokio::sync::mpsc::UnboundedReceiver<T>,
    ) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        (Box::new(ChannelListener { tx }), rx)
    }

    struct NeverConnect {
        _not_constructible: PhantomData<()>,
    }

    impl NeverConnect {
        fn create() -> Box<dyn ClientConnectionHandler> {
            Box::new(Self {
                _not_constructible: PhantomData,
            })
        }
    }

    impl ClientConnectionHandler for NeverConnect {
        fn endpoint_span_name(&self) -> String {
            "test".to_string()
        }

        fn disconnected(&mut self, _: SocketAddr, _: Option<&str>) -> Duration {
            Duration::from_secs(60)
        }

        fn next(&mut self) -> Result<crate::tcp::ConnectionInfo, Duration> {
            Err(Duration::from_secs(60))
        }
    }

    struct NullReadHandler;
    impl ReadHandler for NullReadHandler {}

    struct NullAssociationHandler;
    impl AssociationHandler for NullAssociationHandler {}

    struct NullAssociationInformation;
    impl AssociationInformation for NullAssociationInformation {}

    fn endpoint(value: u16) -> EndpointAddress {
        EndpointAddress::try_new(value).unwrap()
    }

    fn add_stream_association(
        builder: &mut MasterBuilder,
        address: EndpointAddress,
    ) -> Result<AssociationBuilder<'_>, AssociationError> {
        builder.add_association(
            address,
            AssociationConfig::quiet(),
            Box::new(NullReadHandler),
            Box::new(NullAssociationHandler),
            Box::new(NullAssociationInformation),
        )
    }

    #[tokio::test]
    async fn preconfigured_poll_handle_targets_parent_builder() {
        let (mut builder, _channel) = MasterBuilder::new(MasterChannelConfig::new(endpoint(1)));
        let address = endpoint(1024);

        let mut association = add_stream_association(&mut builder, address).unwrap();
        let mut poll = association.add_poll(
            ReadRequest::ClassScan(crate::master::Classes::all()),
            Duration::from_secs(5),
        );
        let handle = association.into_handle();

        assert_eq!(handle.address(), address);
        poll.demand().await.unwrap();

        match builder.rx.receive().await.unwrap() {
            Message::Association(msg) => {
                assert_eq!(msg.address, address);
                assert!(matches!(
                    msg.details,
                    AssociationMsgType::Poll(PollMsg::Demand(0))
                ));
            }
            _ => panic!("expected an association message"),
        }
    }

    #[test]
    fn duplicate_address_fails_before_a_scoped_builder_is_returned() {
        let (mut builder, _channel) = MasterBuilder::new(MasterChannelConfig::new(endpoint(1)));
        let address = endpoint(1024);

        let association = add_stream_association(&mut builder, address).unwrap();
        let _handle = association.into_handle();

        assert!(matches!(
            add_stream_association(&mut builder, address),
            Err(AssociationError::DuplicateAddress(x)) if x == address
        ));
    }

    #[tokio::test]
    async fn enabled_tcp_task_does_not_report_disabled() {
        let (mut builder, channel) = MasterBuilder::new(MasterChannelConfig::new(endpoint(1)));
        builder.enable();
        let (listener, mut states) = channel_listener();
        let task = builder.into_tcp(LinkErrorMode::Close, NeverConnect::create(), listener);

        let join = tokio::spawn(task.run());
        assert_eq!(states.recv().await, Some(ClientState::Connecting));

        drop(channel);
        join.await.unwrap();
    }

    #[tokio::test]
    async fn disabled_tcp_task_reports_disabled() {
        let (builder, channel) = MasterBuilder::new(MasterChannelConfig::new(endpoint(1)));
        let (listener, mut states) = channel_listener();
        let task = builder.into_tcp(LinkErrorMode::Close, NeverConnect::create(), listener);

        let join = tokio::spawn(task.run());
        assert_eq!(states.recv().await, Some(ClientState::Disabled));

        drop(channel);
        join.await.unwrap();
    }

    #[cfg(feature = "serial")]
    #[tokio::test]
    async fn enabled_serial_task_does_not_report_disabled() {
        let (mut builder, channel) = MasterBuilder::new(MasterChannelConfig::new(endpoint(1)));
        builder.enable();
        let (listener, mut states) = channel_listener();
        let task = builder.into_serial(
            "/path/that/does/not/exist/dnp3-test",
            crate::serial::SerialSettings::default(),
            Duration::from_secs(60),
            listener,
        );

        let join = tokio::spawn(task.run());
        assert!(matches!(
            states.recv().await,
            Some(crate::serial::PortState::Wait(_))
        ));

        drop(channel);
        join.await.unwrap();
    }

    #[cfg(feature = "serial")]
    #[tokio::test]
    async fn disabled_serial_task_reports_disabled() {
        let (builder, channel) = MasterBuilder::new(MasterChannelConfig::new(endpoint(1)));
        let (listener, mut states) = channel_listener();
        let task = builder.into_serial(
            "/path/that/does/not/exist/dnp3-test",
            crate::serial::SerialSettings::default(),
            Duration::from_secs(60),
            listener,
        );

        let join = tokio::spawn(task.run());
        assert_eq!(
            states.recv().await,
            Some(crate::serial::PortState::Disabled)
        );

        drop(channel);
        join.await.unwrap();
    }
}
