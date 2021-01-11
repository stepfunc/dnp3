use std::time::Duration;

/// entry points for creating and spawning serial-based master tasks
pub mod serial;
/// entry points for creating and spawning TCP-based master tasks
pub mod tcp;

/// state of TCP client connection
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ClientState {
    Connecting,
    Connected,
    WaitAfterFailedConnect(Duration),
    WaitAfterDisconnect(Duration),
    Shutdown,
}
