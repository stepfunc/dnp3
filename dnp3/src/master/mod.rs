mod association;
mod error;
mod handle;
mod request;

pub use association::*;
pub use error::*;
pub use handle::*;
pub use poll::PollHandle;
pub use request::*;

/// entry points for creating and spawning serial-based master tasks
pub mod serial;
/// entry points for creating and spawning TCP-based master tasks
pub mod tcp;

/// state of TCP client connection
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ClientState {
    Connecting,
    Connected,
    WaitAfterFailedConnect(std::time::Duration),
    WaitAfterDisconnect(std::time::Duration),
    Shutdown,
}

pub(crate) mod convert;
pub(crate) mod extract;
pub(crate) mod messages;
pub(crate) mod poll;
pub(crate) mod session;
pub(crate) mod tasks;

#[cfg(test)]
mod tests;
