use crate::entry::master::{ClientState, MasterTask};
use crate::master::handle::{Listener, MasterConfiguration, MasterHandle};
use std::future::Future;
use std::path::PathBuf;
use tokio_serial::Serial;

pub use tokio_serial::DataBits;
pub use tokio_serial::FlowControl;
pub use tokio_serial::Parity;
/// Serial port settings
pub use tokio_serial::SerialPortSettings;
pub use tokio_serial::StopBits;
use tracing::Instrument;

/// Spawn a task onto the `Tokio` runtime. The task runs until the returned handle, and any
/// `AssociationHandle` created from it, are dropped.
///
/// **Note**: This function may only be called from within the runtime itself, and panics otherwise.
/// It is preferable to use this method instead of `create(..)` when using `[tokio::main]`.
pub fn spawn_master_serial_client<P: Into<PathBuf>>(
    config: MasterConfiguration,
    path: P,
    serial_settings: SerialPortSettings,
    listener: Listener<ClientState>,
) -> MasterHandle {
    let path = path.into();
    let (mut task, handle) = MasterTask::new(
        move || std::future::ready(Serial::from_path(path.as_path(), &serial_settings)),
        config,
        listener,
    );
    crate::tokio::spawn(async move {
        task.run()
            // TODO - make the 'device' parameter a string representation of the port
            .instrument(tracing::trace_span!("MasterSerial", "device" = "TODO"))
            .await
    });
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
pub fn create_master_serial_client<P: Into<PathBuf>>(
    config: MasterConfiguration,
    path: P,
    serial_settings: SerialPortSettings,
    listener: Listener<ClientState>,
) -> (impl Future<Output = ()> + 'static, MasterHandle) {
    let path = path.into();
    let (mut task, handle) = MasterTask::new(
        move || std::future::ready(Serial::from_path(path.as_path(), &serial_settings)),
        config,
        listener,
    );
    (async move { task.run().await }, handle)
}
