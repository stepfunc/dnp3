use crate::app::{Listener, MaybeAsync, RetryStrategy};
use tracing::Instrument;

use crate::link::reader::LinkModes;
use crate::outstation::task::OutstationTask;
use crate::outstation::{
    ControlHandler, OutstationApplication, OutstationConfig, OutstationHandle,
    OutstationInformation,
};
use crate::serial::task::SerialTask;
use crate::serial::{PortState, SerialSettings};
use crate::util::phys::{PhysAddr, PhysLayer};
use crate::util::session::{Enabled, Session};

/// Spawn an outstation task onto the `Tokio` runtime. The task runs until the returned handle is dropped or
/// a serial port error occurs, e.g. a serial port is removed from the OS. It attempts to open
/// the serial port immediately, and fails if it cannot.
///
/// Most users should prefer [`spawn_outstation_serial_2`]. This function remains for API
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
        Enabled::Yes,
        LinkModes::serial(),
        config,
        PhysAddr::None,
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

struct NullListener;

impl Listener<PortState> for NullListener {
    fn update(&mut self, _: PortState) -> MaybeAsync<()> {
        MaybeAsync::ready(())
    }
}

/// Spawns an outstation task onto the `Tokio` runtime. The task runs until the returned handle is dropped.
/// It is tolerant to the serial port being unavailable at startup or being removed from the OS. It
/// uses the provided `RetryStrategy` to determine when to retry the port if the port cannot be
/// opened or fails.
///
/// This function should be preferred over [`spawn_outstation_serial`] and will become the only method
/// available in a future 2.0 release.
///
/// **Note**: This function may only be called from within the runtime itself, and panics otherwise.
/// Use Runtime::enter() if required.
#[allow(clippy::too_many_arguments)]
pub fn spawn_outstation_serial_2(
    path: &str,
    settings: SerialSettings,
    config: OutstationConfig,
    retry: RetryStrategy,
    application: Box<dyn OutstationApplication>,
    information: Box<dyn OutstationInformation>,
    control_handler: Box<dyn ControlHandler>,
    listener: Box<dyn Listener<PortState>>,
) -> OutstationHandle {
    let (task, handle) = OutstationTask::create(
        Enabled::Yes,
        LinkModes::serial(),
        config,
        PhysAddr::None,
        application,
        information,
        control_handler,
    );

    let mut serial = SerialTask::new(path, settings, Session::outstation(task), retry, listener);

    let log_path = path.to_owned();
    let future = async move {
        serial
            .run()
            .instrument(tracing::info_span!("dnp3-outstation-serial", "port" = ?log_path))
            .await;
    };
    tokio::spawn(future);
    handle
}

/// This function was added post 1.0 to provide fault tolerance for outstation serial ports.
///
/// This function is implemented by calling [`spawn_outstation_serial_2`] with a post listener that does nothing.
pub fn spawn_outstation_serial_fault_tolerant(
    path: &str,
    settings: SerialSettings,
    config: OutstationConfig,
    retry: RetryStrategy,
    application: Box<dyn OutstationApplication>,
    information: Box<dyn OutstationInformation>,
    control_handler: Box<dyn ControlHandler>,
) -> OutstationHandle {
    spawn_outstation_serial_2(
        path,
        settings,
        config,
        retry,
        application,
        information,
        control_handler,
        Box::new(NullListener),
    )
}
