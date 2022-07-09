use std::time::Duration;

use tracing::Instrument;

use crate::app::{Listener, Shutdown};
use crate::link::LinkErrorMode;
use crate::master::session::{MasterSession, RunError, StateChange};
use crate::master::*;
use crate::serial::{PortState, SerialSettings};
use crate::transport::TransportReader;
use crate::transport::TransportWriter;
use crate::util::phys::PhysLayer;

/// Spawn a master task onto the `Tokio` runtime. The task runs until the returned handle, and any
/// `AssociationHandle` created from it, are dropped.
///
/// **Note**: This function may only be called from within the runtime itself, and panics otherwise.
/// Use Runtime::enter() if required.
pub fn spawn_master_serial(
    config: MasterChannelConfig,
    path: &str,
    serial_settings: SerialSettings,
    retry_delay: Duration,
    listener: Box<dyn Listener<PortState>>,
) -> MasterChannel {
    let log_path = path.to_owned();
    let (mut task, handle) = MasterTask::new(path, serial_settings, config, retry_delay, listener);
    let future = async move {
        task.run()
            .instrument(tracing::info_span!("dnp3-master-serial", "port" = ?log_path))
            .await;
    };
    tokio::spawn(future);
    handle
}

struct MasterTask {
    path: String,
    serial_settings: SerialSettings,
    retry_delay: Duration,
    session: MasterSession,
    reader: TransportReader,
    writer: TransportWriter,
    listener: Box<dyn Listener<PortState>>,
}

impl MasterTask {
    fn new(
        path: &str,
        serial_settings: SerialSettings,
        config: MasterChannelConfig,
        retry_delay: Duration,
        listener: Box<dyn Listener<PortState>>,
    ) -> (Self, MasterChannel) {
        let (tx, rx) = crate::util::channel::request_channel();
        let session = MasterSession::new(false, config.decode_level, config.tx_buffer_size, rx);
        let (reader, writer) = crate::transport::create_master_transport_layer(
            // serial ports always discard link parsing errors
            LinkErrorMode::Discard,
            config.master_address,
            config.rx_buffer_size,
        );
        let task = Self {
            path: path.to_string(),
            serial_settings,
            retry_delay,
            session,
            reader,
            writer,
            listener,
        };
        (task, MasterChannel::new(tx))
    }

    async fn run(&mut self) {
        let _ = self.run_impl().await;
        self.session.shutdown().await;
        self.listener.update(PortState::Shutdown).get().await;
    }

    async fn run_impl(&mut self) -> Result<(), Shutdown> {
        loop {
            self.listener.update(PortState::Disabled).get().await;
            self.session.wait_for_enabled().await?;
            if let Err(StateChange::Shutdown) = self.run_enabled().await {
                return Err(Shutdown);
            }
        }
    }

    async fn run_enabled(&mut self) -> Result<(), StateChange> {
        loop {
            match crate::serial::open(self.path.as_str(), self.serial_settings) {
                Err(err) => {
                    tracing::warn!(
                        "{} - waiting {} ms to re-open port",
                        err,
                        self.retry_delay.as_millis()
                    );
                    self.listener
                        .update(PortState::Wait(self.retry_delay))
                        .get()
                        .await;
                    self.session.wait_for_retry(self.retry_delay).await?;
                }
                Ok(serial) => {
                    let mut io = PhysLayer::Serial(serial);
                    tracing::info!("serial port open");
                    self.listener.update(PortState::Open).get().await;
                    match self
                        .session
                        .run(&mut io, &mut self.writer, &mut self.reader)
                        .await
                    {
                        RunError::State(x) => {
                            return Err(x);
                        }
                        RunError::Link(err) => {
                            tracing::warn!("serial port error: {}", err);
                            tracing::info!(
                                "waiting {} ms to re-open",
                                self.retry_delay.as_millis()
                            );
                            self.listener
                                .update(PortState::Wait(self.retry_delay))
                                .get()
                                .await;
                            self.session.wait_for_retry(self.retry_delay).await?;
                        }
                    }
                }
            }
        }
    }
}
