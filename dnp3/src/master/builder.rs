use std::time::Duration;

use tracing::Instrument;

use super::association::{Association, AssociationMap};
use super::handler::{AssociationHandle, MasterChannel, MasterChannelConfig, MasterChannelType};
use super::task::MasterTask;
use super::{
    AssociationConfig, AssociationError, AssociationHandler, AssociationInformation, ReadHandler,
};
use crate::app::parse::options::ParseOptions;
use crate::app::Listener;
use crate::link::reader::LinkModes;
use crate::link::{EndpointAddress, LinkErrorMode};
use crate::tcp::client::ClientTask;
use crate::tcp::{ClientConnectionHandler, ClientState, PostConnectionHandler};
use crate::transport::FragmentAddr;
use crate::util::channel::Receiver;
use crate::util::phys::PhysAddr;
use crate::util::session::{Enabled, Session};

/// Builder for configuring a master task before binding it to a transport.
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

/// A master task bound to a TCP transport. Call [`run`](Self::run) to execute
/// the event loop.
#[must_use = "a MasterTcpTask does nothing unless you call .run()"]
pub struct MasterTcpTask {
    inner: MasterTask,
    connect_handler: Box<dyn ClientConnectionHandler>,
    listener: Box<dyn Listener<ClientState>>,
}

/// A master task bound to a serial transport. Call [`run`](Self::run) to execute
/// the event loop.
#[cfg(feature = "serial")]
#[must_use = "a MasterSerialTask does nothing unless you call .run()"]
pub struct MasterSerialTask {
    inner: MasterTask,
    path: String,
    settings: crate::serial::SerialSettings,
    retry_strategy: crate::app::RetryStrategy,
    listener: Box<dyn Listener<crate::serial::PortState>>,
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
    ) -> MasterTcpTask {
        let inner = MasterTask::with_associations(
            self.enabled,
            self.associations,
            LinkModes::stream(link_error_mode),
            ParseOptions::get_static(),
            self.config,
            self.rx,
        );
        MasterTcpTask {
            inner,
            connect_handler: connection_handler,
            listener,
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
    ) -> MasterSerialTask {
        let inner = MasterTask::with_associations(
            self.enabled,
            self.associations,
            LinkModes::serial(),
            ParseOptions::get_static(),
            self.config,
            self.rx,
        );
        MasterSerialTask {
            inner,
            path: path.to_string(),
            settings: serial_settings,
            retry_strategy: crate::app::RetryStrategy::new(retry_delay, retry_delay),
            listener,
        }
    }
}

impl MasterTcpTask {
    /// Run the master TCP client event loop.
    ///
    /// This method consumes the task and runs until the [`MasterChannel`] (and
    /// all associated handles) are dropped.
    pub async fn run(self) {
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

#[cfg(feature = "serial")]
impl MasterSerialTask {
    /// Run the master serial event loop.
    ///
    /// This method consumes the task and runs until the [`MasterChannel`] (and
    /// all associated handles) are dropped.
    pub async fn run(self) {
        let log_path = self.path.clone();
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
            .instrument(tracing::info_span!("dnp3-master-serial", "port" = ?log_path))
            .await;
    }
}
