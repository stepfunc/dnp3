mod endpoint_list;
mod master;

/// state of TCP client connection
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ClientState {
    Connecting,
    Connected,
    WaitAfterFailedConnect(std::time::Duration),
    WaitAfterDisconnect(std::time::Duration),
    Shutdown,
}

pub use endpoint_list::*;
pub use master::*;
