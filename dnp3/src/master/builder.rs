use std::net::SocketAddr;
use std::time::Duration;

use tracing::Instrument;

use super::association::{Association, AssociationMap};
use super::handler::{AssociationHandle, MasterChannel, MasterChannelConfig, MasterChannelType};
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

/// A master task bound to a transport. Call [`run`](Self::run) to execute
/// the event loop.
///
/// Created by [`MasterBuilder::into_tcp`], [`MasterBuilder::into_tls`],
/// [`MasterBuilder::into_serial`], or [`UdpMasterBuilder::into_udp`].
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

    /// Register an association with the master before the task is running.
    ///
    /// Returns an [`AssociationHandle`] that can be used for post-run operations.
    /// Async methods on the handle require the task to be running.
    pub fn add_association(
        &mut self,
        address: EndpointAddress,
        config: AssociationConfig,
        read_handler: Box<dyn ReadHandler>,
        assoc_handler: Box<dyn AssociationHandler>,
        assoc_information: Box<dyn AssociationInformation>,
    ) -> Result<AssociationHandle, AssociationError> {
        let addr = FragmentAddr {
            link: address,
            phys: PhysAddr::None,
        };
        self.associations.register(Association::new(
            addr,
            config,
            read_handler,
            assoc_handler,
            assoc_information,
        ))?;
        Ok(AssociationHandle::new(address, self.channel.clone()))
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

    /// Register a UDP association with the master before the task is running.
    ///
    /// * `address` is the DNP3 link-layer address of the outstation
    /// * `destination` is the IP address and port of the outstation
    ///
    /// Returns an [`AssociationHandle`] that can be used for post-run operations.
    /// Async methods on the handle require the task to be running.
    pub fn add_association(
        &mut self,
        address: EndpointAddress,
        destination: SocketAddr,
        config: AssociationConfig,
        read_handler: Box<dyn ReadHandler>,
        assoc_handler: Box<dyn AssociationHandler>,
        assoc_information: Box<dyn AssociationInformation>,
    ) -> Result<AssociationHandle, AssociationError> {
        let addr = FragmentAddr {
            link: address,
            phys: PhysAddr::Udp(destination),
        };
        self.associations.register(Association::new(
            addr,
            config,
            read_handler,
            assoc_handler,
            assoc_information,
        ))?;
        Ok(AssociationHandle::new(address, self.channel.clone()))
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

impl MasterTask {
    /// Run the master event loop.
    ///
    /// This method consumes the task and runs until the [`MasterChannel`] (and
    /// all associated handles) are dropped.
    pub async fn run(self) {
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

impl TcpTask {
    async fn run(self) {
        let name = self.connect_handler.endpoint_span_name();
        let session = Session::master(self.inner);
        let mut client = ClientTask::new(
            session,
            self.connect_handler,
            PostConnectionHandler::Tcp,
            self.listener,
        );
        client
            .run()
            .instrument(tracing::info_span!("dnp3-master-tcp-client", "endpoint" = ?name))
            .await;
    }
}

#[cfg(feature = "enable-tls")]
impl TlsTask {
    async fn run(self) {
        let name = self.connect_handler.endpoint_span_name();
        let session = Session::master(self.inner);
        let mut client = ClientTask::new(
            session,
            self.connect_handler,
            PostConnectionHandler::Tls(self.tls_config),
            self.listener,
        );
        client
            .run()
            .instrument(tracing::info_span!("dnp3-master-tls-client", "endpoint" = ?name))
            .await;
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
        serial
            .run()
            .instrument(tracing::info_span!("dnp3-master-serial", "port" = ?self.path))
            .await;
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
        let _ = task
            .run()
            .instrument(tracing::info_span!("dnp3-master-udp", "endpoint" = ?local_endpoint))
            .await;
    }
}
