use tracing::Instrument;

use crate::app::{ConnectStrategy, Listener};
use crate::link::reader::LinkModes;
use crate::link::LinkErrorMode;
use crate::master::task::MasterTask;
use crate::master::{MasterChannel, MasterChannelConfig, MasterChannelType};
use crate::tcp::client::ClientTask;
use crate::tcp::EndpointList;
use crate::tcp::{ClientState, ConnectOptions, PostConnectionHandler};
use crate::util::session::{Enabled, Session};

/// Spawn a task onto the `Tokio` runtime. The task runs until the returned handle, and any
/// `AssociationHandle` created from it, are dropped.
///
/// **Note**: This function may only be called from within the runtime itself, and panics otherwise.
/// Use Runtime::enter() if required.
pub fn spawn_master_tcp_client(
    link_error_mode: LinkErrorMode,
    config: MasterChannelConfig,
    endpoints: EndpointList,
    connect_strategy: ConnectStrategy,
    listener: Box<dyn Listener<ClientState>>,
) -> MasterChannel {
    spawn_master_tcp_client_2(
        link_error_mode,
        config,
        endpoints,
        connect_strategy,
        ConnectOptions::default(),
        listener,
    )
}

/// Just like [spawn_master_tcp_client], but this variant was added later to also accept and
/// apply [ConnectOptions].
pub fn spawn_master_tcp_client_2(
    link_error_mode: LinkErrorMode,
    config: MasterChannelConfig,
    endpoints: EndpointList,
    connect_strategy: ConnectStrategy,
    connect_options: ConnectOptions,
    listener: Box<dyn Listener<ClientState>>,
) -> MasterChannel {
    let main_addr = endpoints.main_addr().to_string();
    let (mut task, handle) = wire_master_client(
        LinkModes::stream(link_error_mode),
        MasterChannelType::Stream,
        endpoints,
        config,
        connect_strategy,
        connect_options,
        PostConnectionHandler::Tcp,
        listener,
    );
    let future = async move {
        task.run()
            .instrument(tracing::info_span!("dnp3-master-tcp-client", "endpoint" = ?main_addr))
            .await;
    };
    tokio::spawn(future);
    handle
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn wire_master_client(
    link_modes: LinkModes,
    channel_type: MasterChannelType,
    endpoints: EndpointList,
    config: MasterChannelConfig,
    connect_strategy: ConnectStrategy,
    connect_options: ConnectOptions,
    connection_handler: PostConnectionHandler,
    listener: Box<dyn Listener<ClientState>>,
) -> (ClientTask, MasterChannel) {
    let (tx, rx) = crate::util::channel::request_channel();
    let session = Session::master(MasterTask::new(Enabled::No, link_modes, config, rx));
    let client = ClientTask::new(
        session,
        endpoints,
        connect_strategy,
        connect_options,
        connection_handler,
        listener,
    );
    (client, MasterChannel::new(tx, channel_type))
}
