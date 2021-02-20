use std::future::Future;
use std::time::Duration;

use tracing::Instrument;

use crate::app::ExponentialBackOff;
use crate::link::LinkErrorMode;
use crate::master::session::{MasterSession, RunError, StateChange};
use crate::master::*;
use crate::serial::SerialSettings;
use crate::tcp::ClientState;
use crate::transport::TransportReader;
use crate::transport::TransportWriter;
use crate::util::phys::PhysLayer;
use crate::util::task::Shutdown;

/// Spawn a task onto the `Tokio` runtime. The task runs until the returned handle, and any
/// `AssociationHandle` created from it, are dropped.
///
/// **Note**: This function may only be called from within the runtime itself, and panics otherwise.
/// It is preferable to use this method instead of `create(..)` when using `[tokio::main]`.
pub fn spawn_master_serial_client(
    config: MasterConfig,
    path: &str,
    serial_settings: SerialSettings,
    listener: Listener<ClientState>,
) -> MasterHandle {
    let (future, handle) = create_master_serial_client(config, path, serial_settings, listener);
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
pub fn create_master_serial_client(
    config: MasterConfig,
    path: &str,
    settings: SerialSettings,
    listener: Listener<ClientState>,
) -> (impl Future<Output = ()> + 'static, MasterHandle) {
    let log_path = path.to_owned();
    let (mut task, handle) = MasterTask::new(path, settings, config, listener);
    let future = async move {
        let _ = task
            .run()
            .instrument(tracing::info_span!("DNP3-Master-Serial", "port" = ?log_path))
            .await;
    };
    (future, handle)
}

struct MasterTask {
    path: String,
    serial_settings: SerialSettings,
    back_off: ExponentialBackOff,
    reconnect_delay: Option<Duration>,
    session: MasterSession,
    reader: TransportReader,
    writer: TransportWriter,
    listener: Listener<ClientState>,
}

impl MasterTask {
    fn new(
        path: &str,
        serial_settings: SerialSettings,
        config: MasterConfig,
        listener: Listener<ClientState>,
    ) -> (Self, MasterHandle) {
        let (tx, rx) = crate::tokio::sync::mpsc::channel(100); // TODO
        let session = MasterSession::new(
            false,
            config.decode_level,
            config.response_timeout,
            config.tx_buffer_size,
            rx,
        );
        let (reader, writer) = crate::transport::create_master_transport_layer(
            // serial ports always discard link parsing errors
            LinkErrorMode::Discard,
            config.address,
            config.rx_buffer_size,
        );
        let task = Self {
            path: path.to_string(),
            serial_settings,
            back_off: ExponentialBackOff::new(config.reconnection_strategy.retry_strategy),
            reconnect_delay: config.reconnection_strategy.reconnect_delay,
            session,
            reader,
            writer,
            listener,
        };
        (task, MasterHandle::new(tx))
    }

    async fn run(&mut self) -> Result<(), Shutdown> {
        loop {
            self.session.wait_for_enabled().await?;
            if let Err(StateChange::Shutdown) = self.run_enabled().await {
                self.listener.update(ClientState::Shutdown);
                return Err(Shutdown);
            }
        }
    }

    async fn run_enabled(&mut self) -> Result<(), StateChange> {
        loop {
            self.listener.update(ClientState::Connecting);
            match tokio_one_serial::open(self.path.as_str(), self.serial_settings) {
                Err(err) => {
                    let delay = self.back_off.on_failure();
                    tracing::warn!("{} - waiting {} ms to retry", err, delay.as_millis());
                    self.listener
                        .update(ClientState::WaitAfterFailedConnect(delay));
                    self.session.wait_for_retry(delay).await?;
                }
                Ok(serial) => {
                    let mut io = PhysLayer::Serial(serial);
                    tracing::info!("connected");
                    self.back_off.on_success();
                    self.listener.update(ClientState::Connected);
                    match self
                        .session
                        .run(&mut io, &mut self.writer, &mut self.reader)
                        .await
                    {
                        RunError::State(x) => {
                            return Err(x);
                        }
                        RunError::Link(err) => {
                            tracing::warn!("connection lost - {}", err);
                            if let Some(delay) = self.reconnect_delay {
                                tracing::warn!("waiting {} ms to reconnect", delay.as_millis());
                                self.listener
                                    .update(ClientState::WaitAfterDisconnect(delay));
                                self.session.wait_for_retry(delay).await?;
                            }
                        }
                    }
                }
            }
        }
    }
}
