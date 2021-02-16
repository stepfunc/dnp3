pub use address_filter::*;
pub use endpoint_list::*;
pub use master::*;
pub use outstation::*;

mod address_filter;
mod endpoint_list;
mod master;
mod outstation;

/// state of TCP client connection
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ClientState {
    Connecting,
    Connected,
    WaitAfterFailedConnect(std::time::Duration),
    WaitAfterDisconnect(std::time::Duration),
    Shutdown,
}
