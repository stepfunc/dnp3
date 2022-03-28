use std::future::Future;

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
/// It is preferable to use this method instead of `create_outstation_serial(..)` when using `[tokio::main]`.
pub fn spawn_outstation_serial(
    path: &str,
    settings: SerialSettings,
    config: OutstationConfig,
    application: Box<dyn OutstationApplication>,
    information: Box<dyn OutstationInformation>,
    control_handler: Box<dyn ControlHandler>,
) -> std::io::Result<OutstationHandle> {
    let (future, handle) = create_outstation_serial(
        path,
        settings,
        config,
        application,
        information,
        control_handler,
    )?;
    crate::tokio::spawn(future);
    Ok(handle)
}

/// Create an outstation future, which can be spawned onto a runtime, along with a controlling handle.
///
/// Once spawned or otherwise executed using the `run` method, the task runs until the handle
/// is dropped or the serial port is removed from the OS.
///
/// **Note**: This function is required instead of `spawn` when using a runtime to directly spawn
/// tasks instead of within the context of a runtime, e.g. in applications that cannot use
/// `[tokio::main]` such as C language bindings.
pub fn create_outstation_serial(
    path: &str,
    settings: SerialSettings,
    config: OutstationConfig,
    application: Box<dyn OutstationApplication>,
    information: Box<dyn OutstationInformation>,
    control_handler: Box<dyn ControlHandler>,
) -> std::io::Result<(impl Future<Output = ()> + 'static, OutstationHandle)> {
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
    Ok((future, handle))
}
