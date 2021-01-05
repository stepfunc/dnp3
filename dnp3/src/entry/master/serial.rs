use crate::app::retry::ExponentialBackOff;
use crate::entry::master::ClientState;
use crate::master::handle::{Listener, MasterConfiguration, MasterHandle};
use crate::master::session::{MasterSession, RunError};
use crate::transport::TransportReader;
use crate::transport::TransportWriter;
use crate::util::task::Shutdown;
use std::future::Future;
use std::path::PathBuf;
use std::time::Duration;
pub use tokio_serial::DataBits;
pub use tokio_serial::FlowControl;
pub use tokio_serial::Parity;
use tokio_serial::Serial;
/// Serial port settings
pub use tokio_serial::SerialPortSettings;
pub use tokio_serial::StopBits;
use tracing::Instrument;

/// Spawn a task onto the `Tokio` runtime. The task runs until the returned handle, and any
/// `AssociationHandle` created from it, are dropped.
///
/// **Note**: This function may only be called from within the runtime itself, and panics otherwise.
/// It is preferable to use this method instead of `create(..)` when using `[tokio::main]`.
pub fn spawn_master_serial_client(
    config: MasterConfiguration,
    path: &str,
    serial_settings: SerialPortSettings,
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
    config: MasterConfiguration,
    path: &str,
    serial_settings: SerialPortSettings,
    listener: Listener<ClientState>,
) -> (impl Future<Output = ()> + 'static, MasterHandle) {
    let string_path = path.to_owned();
    let path = PathBuf::from(path);
    let (mut task, handle) = MasterTask::new(path, serial_settings, config, listener);
    let future = async move {
        task.run()
            .instrument(tracing::info_span!("MasterSerial", "port" = ?string_path))
            .await
    };
    (future, handle)
}

struct MasterTask {
    path: PathBuf,
    serial_settings: SerialPortSettings,
    back_off: ExponentialBackOff,
    reconnect_delay: Option<Duration>,
    session: MasterSession,
    reader: TransportReader,
    writer: TransportWriter,
    listener: Listener<ClientState>,
}

impl MasterTask {
    fn new(
        path: PathBuf,
        serial_settings: SerialPortSettings,
        config: MasterConfiguration,
        listener: Listener<ClientState>,
    ) -> (Self, MasterHandle) {
        let (tx, rx) = crate::tokio::sync::mpsc::channel(100); // TODO
        let session = MasterSession::new(
            config.level,
            config.response_timeout,
            config.tx_buffer_size,
            rx,
        );
        let (reader, writer) = crate::transport::create_master_transport_layer(
            config.address,
            config.rx_buffer_size,
            config.bubble_framing_errors,
        );
        let task = Self {
            path,
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

    async fn run(&mut self) {
        self.run_impl().await.ok();
    }

    async fn run_impl(&mut self) -> Result<(), Shutdown> {
        loop {
            self.listener.update(ClientState::Connecting);
            match Serial::from_path(self.path.as_path(), &self.serial_settings) {
                Err(err) => {
                    let delay = self.back_off.on_failure();
                    tracing::warn!("{} - waiting {} ms to retry", err, delay.as_millis());
                    self.listener
                        .update(ClientState::WaitAfterFailedConnect(delay));
                    self.session.delay_for(delay).await?;
                }
                Ok(mut socket) => {
                    tracing::info!("connected");
                    self.back_off.on_success();
                    self.listener.update(ClientState::Connected);
                    match self
                        .session
                        .run(&mut socket, &mut self.writer, &mut self.reader)
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
        }
    }
}
