use crate::entry::master::{ClientState, MasterTask};
use crate::master::handle::{Listener, MasterConfiguration, MasterHandle};
use crate::tokio::net::TcpStream;
use std::future::Future;
use std::net::SocketAddr;

/// Spawn a task onto the `Tokio` runtime. The task runs until the returned handle, and any
/// `AssociationHandle` created from it, are dropped.
///
/// **Note**: This function may only be called from within the runtime itself, and panics otherwise.
/// It is preferable to use this method instead of `create(..)` when using `[tokio::main]`.
pub fn spawn_master_tcp_client(
    config: MasterConfiguration,
    endpoint: SocketAddr,
    listener: Listener<ClientState>,
) -> MasterHandle {
    let (mut task, handle) = MasterTask::new(
        move || async move { TcpStream::connect(endpoint).await },
        config,
        listener,
    );
    crate::tokio::spawn(async move { task.run().await });
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
pub fn create_master_tcp_client(
    config: MasterConfiguration,
    endpoint: SocketAddr,
    listener: Listener<ClientState>,
) -> (impl Future<Output = ()> + 'static, MasterHandle) {
    let (mut task, handle) = MasterTask::new(
        move || async move { TcpStream::connect(endpoint).await },
        config,
        listener,
    );
    (async move { task.run().await }, handle)
}
