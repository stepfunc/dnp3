use std::time::Duration;

use tracing::Instrument;

use crate::app::Listener;
use crate::link::LinkErrorMode;
use crate::master::task::MasterTask;
use crate::master::*;
use crate::serial::{PortState, SerialSettings};
use crate::util::session::Session;

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
    let (tx, rx) = crate::util::channel::request_channel();
    let task = MasterTask::new(false, LinkErrorMode::Discard, config, rx);
    let mut serial = super::task::SerialTask::new(
        path,
        serial_settings,
        Session::master(task),
        retry_delay,
        listener,
    );
    let future = async move {
        serial
            .run()
            .instrument(tracing::info_span!("dnp3-master-serial", "port" = ?log_path))
            .await;
    };
    tokio::spawn(future);
    MasterChannel::new(tx)
}
