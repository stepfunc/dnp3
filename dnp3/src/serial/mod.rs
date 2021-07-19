pub use tokio_serial::{DataBits, FlowControl, Parity, StopBits};

/// serial port settings
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SerialSettings {
    /// baud rate of the port
    pub baud_rate: u32,
    /// number of data bits
    pub data_bits: DataBits,
    /// types of flow control
    pub flow_control: FlowControl,
    /// number of stop bits
    pub stop_bits: StopBits,
    /// parity setting
    pub parity: Parity,
}

impl SerialSettings {

}

impl Default for SerialSettings {
    fn default() -> Self {
        Self {
            baud_rate: 9600,
            data_bits: DataBits::Eight,
            flow_control: FlowControl::None,
            stop_bits: StopBits::One,
            parity: Parity::None,
        }
    }
}


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
