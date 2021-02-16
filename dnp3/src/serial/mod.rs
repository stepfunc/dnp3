// re-export these from the serial crate
pub use tokio_one_serial::Settings as SerialSettings;
pub use tokio_one_serial::{DataBits, FlowControl, Parity, StopBits};

mod master;
pub use master::*;
