use crate::link::reader::LinkModes;
use crate::link::{LinkErrorMode, LinkReadMode};
use crate::master::task::MasterTask;
use crate::master::{MasterChannel, MasterChannelConfig, MasterChannelType};
use crate::udp::layer::UdpFactory;
use crate::udp::task::UdpTask;
use crate::util::session::{Enabled, Session};

use crate::app::Timeout;
use std::net::SocketAddr;
use tracing::Instrument;

/// Spawn a UDP master task onto the `Tokio` runtime. The task runs until the returned handle, and any
/// `AssociationHandle` created from it, are dropped.
///
/// **Note**: This function may only be called from within the runtime itself, and panics otherwise.
/// Use Runtime::enter() if required.
pub fn spawn_master_udp(
    local_endpoint: SocketAddr,
    read_mode: LinkReadMode,
    retry_delay: Timeout,
    config: MasterChannelConfig,
) -> MasterChannel {
    let (tx, rx) = crate::util::channel::request_channel();
    let link_modes = LinkModes {
        error_mode: LinkErrorMode::Discard,
        read_mode,
    };

    let session = Session::master(MasterTask::new(Enabled::No, link_modes, config, rx));
    let task = UdpTask {
        session,
        factory: UdpFactory::bound(local_endpoint),
        retry_delay,
    };
    let future = async move {
        let _ = task
            .run()
            .instrument(tracing::info_span!("dnp3-master-udp", "endpoint" = ?local_endpoint))
            .await;
    };

    tokio::spawn(future);
    MasterChannel::new(tx, MasterChannelType::Udp)
}
