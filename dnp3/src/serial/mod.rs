pub use tokio_one_serial::{DataBits, FlowControl, Parity, StopBits};
// re-export these from the serial crate
pub use tokio_one_serial::Settings as SerialSettings;

pub use master::*;
pub use outstation::*;

mod master;
mod outstation;

/// State of the serial port
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PortState {
    /// Disabled and idle until enabled
    Disabled,
    /// waiting to perform an open retry
    Wait(std::time::Duration),
    /// Port is open
    Open,
    /// Task has been shut down
    Shutdown,
}
