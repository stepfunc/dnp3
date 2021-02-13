/// prelude specifically for using the master API
pub mod master {
    pub use crate::app::enums::*;
    pub use crate::app::retry::*;
    pub use crate::app::timeout::Timeout;
    pub use crate::app::variations::*;
    pub use crate::config::DecodeLevel;
    pub use crate::config::EndpointAddress;
    pub use crate::master::association::*;
    pub use crate::master::handle::*;
    pub use crate::master::request::*;
    pub use crate::master::tcp::*;
    pub use crate::master::*;
}
