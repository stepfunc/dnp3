pub(crate) mod layer;
mod master;
mod outstation;
mod task;

pub use master::*;
pub use outstation::*;

/// Describes how the UDP socket reads and writes datagrams from remote endpoint(s)
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UdpSocketMode {
    /// The UDP endpoint will only communicate with the specified remote endpoint
    OneToOne,
    /// The UDP endpoint will accept packets any remote endpoint.
    ///
    /// When this mode is used with an outstation, the outstation will respond to the address from which the request was sent.
    /// It will use supplied remote endpoint only for sending unsolicited responses.
    OneToMany,
}
