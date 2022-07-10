use tracing::Instrument;

use crate::link::LinkErrorMode;
use crate::outstation::task::OutstationTask;
use crate::outstation::{
    ControlHandler, OutstationApplication, OutstationConfig, OutstationHandle,
    OutstationInformation,
};
use crate::serial::SerialSettings;
use crate::util::phys::PhysLayer;

/// Spawn an outstation task onto the `Tokio` runtime. The task runs until the returned handle is dropped or
/// a serial port error occurs, e.g. a serial port is removed from the OS.
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
            .instrument(tracing::info_span!("dnp3-master-serial", "port" = ?log_path))
            .await;
    };
    tokio::spawn(future);
    Ok(handle)
}
