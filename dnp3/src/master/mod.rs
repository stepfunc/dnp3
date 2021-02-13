/// types related to associations
pub mod association;
/// error types related to creating associations and making requests
pub mod error;
/// handles and callback types for controlling a master and associations
pub mod handle;
/// types related to making requests on an Association
pub mod request;
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
