use crate::app::{ExponentialBackOff, RetryStrategy, Shutdown};
use tracing::Instrument;

use crate::link::LinkErrorMode;
use crate::outstation::session::RunError;
use crate::outstation::task::OutstationTask;
use crate::outstation::{
    ControlHandler, OutstationApplication, OutstationConfig, OutstationHandle,
    OutstationInformation,
};
use crate::serial::SerialSettings;
use crate::util::phys::PhysLayer;

/// Spawn an outstation task onto the `Tokio` runtime. The task runs until the returned handle is dropped or
/// a serial port error occurs, e.g. a serial port is removed from the OS. It attempts to open
/// the serial port immediately, and fails if it cannot.
///
/// Most users should prefer `spawn_outstation_serial_fault_tolerant`. This function remains for API
/// compatibility reasons, but will likely be removed in future MAJOR release of the library.
///
/// **Note**: This function may only be called from within the runtime itself, and panics otherwise.
/// Use Runtime::enter() if required.
pub fn spawn_outstation_serial(
    path: &str,
    settings: SerialSettings,
    config: OutstationConfig,
    application: Box<dyn OutstationApplication>,
    information: Box<dyn OutstationInformation>,
    control_handler: Box<dyn ControlHandler>,
) -> std::io::Result<OutstationHandle> {
    let serial = crate::serial::open(path, settings)?;
    let (mut task, handle) = OutstationTask::create(
        LinkErrorMode::Discard,
        config,
        application,
        information,
        control_handler,
    );

    let log_path = path.to_owned();
    let future = async move {
        let mut io = PhysLayer::Serial(serial);
        let _ = task
            .run(&mut io)
            .instrument(tracing::info_span!("dnp3-outstation-serial", "port" = ?log_path))
            .await;
    };
    tokio::spawn(future);
    Ok(handle)
}

/// Spawns an outstation task onto the `Tokio` runtime. The task runs until the returned handle is dropped.
/// It is tolerant to the serial port being unavailable at startup or being removed from the OS. It
/// uses the provided `RetryStrategy` to determine when to retry the port if the port cannot be
/// opened or fails.
///
/// This function should be preferred over `spawn_outstation_serial`.
///
/// **Note**: This function may only be called from within the runtime itself, and panics otherwise.
/// Use Runtime::enter() if required.
pub fn spawn_outstation_serial_fault_tolerant(
    path: &str,
    settings: SerialSettings,
    config: OutstationConfig,
    retry: RetryStrategy,
    application: Box<dyn OutstationApplication>,
    information: Box<dyn OutstationInformation>,
    control_handler: Box<dyn ControlHandler>,
) -> OutstationHandle {
    let (task, handle) = OutstationTask::create(
        LinkErrorMode::Discard,
        config,
        application,
        information,
        control_handler,
    );

    let mut serial_task = SerialOutstation {
        port: path.to_string(),
        backoff: ExponentialBackOff::new(retry),
        settings,
        outstation: task,
    };

    let log_path = path.to_owned();
    let future = async move {
        let _ = serial_task
            .run()
            .instrument(tracing::info_span!("dnp3-outstation-serial", "port" = ?log_path))
            .await;
    };
    tokio::spawn(future);
    handle
}

struct SerialOutstation {
    port: String,
    backoff: ExponentialBackOff,
    settings: SerialSettings,
    outstation: OutstationTask,
}

impl SerialOutstation {
    async fn sleep_for(&mut self, delay: std::time::Duration) -> Result<(), Shutdown> {
        match tokio::time::timeout(delay, self.outstation.process_messages()).await {
            Ok(x) => x,
            // timeout
            Err(_) => Ok(()),
        }
    }

    async fn run(&mut self) -> Shutdown {
        loop {
            match crate::serial::open(&self.port, self.settings) {
                Ok(serial) => {
                    self.backoff.on_success();
                    tracing::info!("opened port");
                    // run an open port until shutdown or failure
                    let mut phys = PhysLayer::Serial(serial);
                    if let RunError::Shutdown = self.outstation.run(&mut phys).await {
                        return Shutdown;
                    }
                    let delay = self.backoff.on_failure();
                    // we wait here to prevent any kind of rapid retry scenario if the port opens and immediately fails
                    tracing::warn!("waiting {:?} to reopen port", delay);
                    if let Err(Shutdown) = self.sleep_for(delay).await {
                        return Shutdown;
                    }
                }
                Err(err) => {
                    let delay = self.backoff.on_failure();
                    tracing::warn!(
                        "unable to open serial port, retrying in {:?} - error: {}",
                        delay,
                        err
                    );
                    if let Err(Shutdown) = self.sleep_for(delay).await {
                        return Shutdown;
                    }
                }
            }
        }
    }
}
