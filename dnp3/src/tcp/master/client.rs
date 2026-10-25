// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use tracing::Instrument;

use crate::app::{ConnectStrategy, Listener};
use crate::link::reader::LinkModes;
use crate::link::LinkErrorMode;
use crate::master::session::task::MasterTask;
use crate::master::{MasterChannel, MasterChannelConfig, MasterChannelType};
use crate::session::Session;
use crate::tcp::client::ClientTask;
use crate::tcp::{ClientConnectionHandler, EndpointList, SimpleConnectHandler};
use crate::tcp::{ClientState, ConnectOptions, PostConnectionHandler};
use crate::util::lifecycle::Enabled;

/// Spawn a task onto the `Tokio` runtime. The task runs until the returned handle, and any
/// `AssociationHandle` created from it is dropped.
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
    let connect_handler =
        SimpleConnectHandler::create(endpoints, connect_options, connect_strategy);
    spawn_master_tcp_client_3(link_error_mode, config, connect_handler, listener)
}

/// Just like [spawn_master_tcp_client_2], but this variant provides fine-grained
/// control over the connection management using a user-defined implementation
/// of [`ClientConnectionHandler`]
pub fn spawn_master_tcp_client_3(
    link_error_mode: LinkErrorMode,
    config: MasterChannelConfig,
    connection_handler: Box<dyn ClientConnectionHandler>,
    listener: Box<dyn Listener<ClientState>>,
) -> MasterChannel {
    spawn_master_client(
        link_error_mode,
        config,
        connection_handler,
        PostConnectionHandler::Tcp,
        listener,
        |name| tracing::info_span!("dnp3-master-tcp-client", "endpoint" = ?name),
    )
}

/// Spawn a master client task over TCP or TLS, running in the span built from the endpoint name
pub(crate) fn spawn_master_client(
    link_error_mode: LinkErrorMode,
    config: MasterChannelConfig,
    connect_handler: Box<dyn ClientConnectionHandler>,
    post_connection: PostConnectionHandler,
    listener: Box<dyn Listener<ClientState>>,
    span: fn(&str) -> tracing::Span,
) -> MasterChannel {
    let name = connect_handler.endpoint_span_name();
    let (mut task, handle) = wire_master_client(
        LinkModes::stream(link_error_mode),
        MasterChannelType::Stream,
        connect_handler,
        config,
        post_connection,
        listener,
    );
    tokio::spawn(async move { task.run().instrument(span(&name)).await });
    handle
}

pub(crate) fn wire_master_client(
    link_modes: LinkModes,
    channel_type: MasterChannelType,
    connect_handler: Box<dyn ClientConnectionHandler>,
    config: MasterChannelConfig,
    connection_handler: PostConnectionHandler,
    listener: Box<dyn Listener<ClientState>>,
) -> (ClientTask, MasterChannel) {
    let (tx, rx) = crate::util::channel::request_channel();
    let session = Session::master(MasterTask::new(Enabled::No, link_modes, config, rx));
    let client = ClientTask::new(session, connect_handler, connection_handler, listener);
    (client, MasterChannel::new(tx, channel_type))
}
