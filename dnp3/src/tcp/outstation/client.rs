use tracing::Instrument;

use crate::app::{ConnectStrategy, Listener};
use crate::link::reader::LinkModes;
use crate::link::LinkErrorMode;
use crate::outstation::task::OutstationTask;
use crate::outstation::*;
use crate::tcp::client::ClientTask;
use crate::tcp::{ClientState, ConnectOptions, EndpointList, PostConnectionHandler};
use crate::util::phys::PhysAddr;
use crate::util::session::{Enabled, Session};

/// Spawn a TCP client task onto the `Tokio` runtime. The task runs until the returned handle is dropped.
///
/// **Note**: This function may only be called from within the runtime itself, and panics otherwise.
/// Use Runtime::enter() if required.
#[allow(clippy::too_many_arguments)]
pub fn spawn_outstation_tcp_client(
    link_error_mode: LinkErrorMode,
    endpoints: EndpointList,
    connect_strategy: ConnectStrategy,
    connect_options: ConnectOptions,
    config: OutstationConfig,
    application: Box<dyn OutstationApplication>,
    information: Box<dyn OutstationInformation>,
    control_handler: Box<dyn ControlHandler>,
    listener: Box<dyn Listener<ClientState>>,
) -> OutstationHandle {
    let main_addr = endpoints.main_addr().to_string();
    let (task, handle) = OutstationTask::create(
        Enabled::No,
        LinkModes::stream(link_error_mode),
        config,
        PhysAddr::None,
        application,
        information,
        control_handler,
    );
    let session = Session::outstation(task);
    let mut client = ClientTask::new(
        session,
        endpoints,
        connect_strategy,
        connect_options,
        PostConnectionHandler::Tcp,
        listener,
    );

    let future = async move {
        client
            .run()
            .instrument(tracing::info_span!("dnp3-outstation-tcp-client", "endpoint" = ?main_addr))
            .await;
    };
    tokio::spawn(future);
    handle
}
