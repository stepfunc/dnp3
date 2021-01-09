use crate::entry::master::{ClientState, MasterTask};
use crate::master::handle::{Listener, MasterConfiguration, MasterHandle};
use std::future::Future;

// re-export these from the serial crate
pub use tokio_one_serial::Settings as SerialSettings;
pub use tokio_one_serial::{DataBits, FlowControl, Parity, StopBits};

use tokio_one_serial::Settings;
use tracing::Instrument;

/// Spawn a task onto the `Tokio` runtime. The task runs until the returned handle, and any
/// `AssociationHandle` created from it, are dropped.
///
/// **Note**: This function may only be called from within the runtime itself, and panics otherwise.
/// It is preferable to use this method instead of `create(..)` when using `[tokio::main]`.
pub fn spawn_master_serial_client(
    config: MasterConfiguration,
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
    config: MasterConfiguration,
    path: &str,
    settings: Settings,
    listener: Listener<ClientState>,
) -> (impl Future<Output = ()> + 'static, MasterHandle) {
    let string_path = path.to_owned();
    let log_path = path.to_owned();
    let (mut task, handle) = MasterTask::new(
        move || std::future::ready(tokio_one_serial::open(string_path.as_str(), settings)),
        config,
        listener,
    );
    let future = async move {
        task.run()
            .instrument(tracing::info_span!("MasterSerial", "port" = ?log_path))
            .await
    };
    (future, handle)
}
