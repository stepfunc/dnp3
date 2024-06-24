pub use connect::*;
pub use endpoint_list::*;
pub use master::*;
pub use no_delay::*;
pub use outstation::*;
pub use server_handle::*;

/// Entry points and types for TLS
#[cfg(feature = "tls")]
pub mod tls;

mod connect;
mod endpoint_list;
mod master;
mod no_delay;
mod outstation;
mod server_handle;

pub(crate) mod client;

/// state of TCP client connection
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ClientState {
    /// client is disabled
    Disabled,
    /// client attempting to establish a connection
    Connecting,
    /// client is connected
    Connected,
    /// client is waiting to retry after a failed attempt to connect
    WaitAfterFailedConnect(std::time::Duration),
    /// client is waiting to retry after a disconnection
    WaitAfterDisconnect(std::time::Duration),
    /// client has been shut down
    Shutdown,
}
