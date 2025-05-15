use tracing::Instrument;

use crate::app::parse::options::ParseOptions;
use crate::app::{ConnectStrategy, Listener};
use crate::link::reader::LinkModes;
use crate::link::LinkErrorMode;
use crate::outstation::task::OutstationTask;
use crate::outstation::*;
use crate::tcp::client::ClientTask;
use crate::tcp::{
    ClientConnectionHandler, ClientState, ConnectOptions, EndpointList, PostConnectionHandler,
    SimpleConnectHandler,
};
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
    let connect_handler =
        SimpleConnectHandler::create(endpoints, connect_options, connect_strategy);
    spawn_outstation_tcp_client_2(
        link_error_mode,
        config,
        connect_handler,
        application,
        information,
        control_handler,
        listener,
    )
}

/// Spawn a TCP client task onto the `Tokio` runtime. The task runs until the returned handle is dropped.
///
/// This function is similar to [`spawn_outstation_tcp_client`] but provides more fine-grained control over
/// connection management via a user implementation of the [`ClientConnectionHandler`] trait.
///
/// **Note**: This function may only be called from within the runtime itself, and panics otherwise.
/// Use Runtime::enter() if required.
#[allow(clippy::too_many_arguments)]
pub fn spawn_outstation_tcp_client_2(
    link_error_mode: LinkErrorMode,
    config: OutstationConfig,
    connect_handler: Box<dyn ClientConnectionHandler>,
    application: Box<dyn OutstationApplication>,
    information: Box<dyn OutstationInformation>,
    control_handler: Box<dyn ControlHandler>,
    listener: Box<dyn Listener<ClientState>>,
) -> OutstationHandle {
    let name = connect_handler.endpoint_span_name();
    let (task, handle) = OutstationTask::create(
        Enabled::No,
        LinkModes::stream(link_error_mode),
        ParseOptions::get_static(),
        config,
        PhysAddr::None,
        application,
        information,
        control_handler,
    );
    let session = Session::outstation(task);
    let mut client = ClientTask::new(
        session,
        connect_handler,
        PostConnectionHandler::Tcp,
        listener,
    );

    let future = async move {
        client
            .run()
            .instrument(tracing::info_span!("dnp3-outstation-tcp-client", "endpoint" = ?name))
            .await;
    };
    tokio::spawn(future);
    handle
}
