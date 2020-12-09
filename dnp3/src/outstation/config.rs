use crate::app::parse::DecodeLogLevel;
use crate::entry::EndpointAddress;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SelfAddressSupport {
    Enabled,
    Disabled,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BroadcastAddressSupport {
    Enabled,
    Disabled,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct OutstationConfig {
    pub tx_buffer_size: usize,
    pub rx_buffer_size: usize,
    pub outstation_address: EndpointAddress,
    pub master_address: EndpointAddress,
    pub self_address_support: SelfAddressSupport,
    pub log_level: DecodeLogLevel,
    pub confirm_timeout: std::time::Duration,
    pub select_timeout: std::time::Duration,
    pub broadcast_support: BroadcastAddressSupport,
}

impl SelfAddressSupport {
    /*
    pub(crate) fn is_enabled(&self) -> bool {
        *self == SelfAddressSupport::Enabled
    }

    pub(crate) fn is_disabled(&self) -> bool {
        !self.is_enabled()
    }
     */
}

impl BroadcastAddressSupport {
    pub(crate) fn is_enabled(&self) -> bool {
        *self == BroadcastAddressSupport::Enabled
    }
    /*
    pub(crate) fn is_disabled(&self) -> bool {
        !self.is_enabled()
    }

     */
}

impl OutstationConfig {
    pub(crate) const MIN_TX_BUFFER_SIZE: usize = 249; // 1 link frame
    pub(crate) const DEFAULT_TX_BUFFER_SIZE: usize = 2048;

    pub(crate) const MIN_RX_BUFFER_SIZE: usize = Self::MIN_TX_BUFFER_SIZE;
    pub(crate) const DEFAULT_RX_BUFFER_SIZE: usize = Self::DEFAULT_TX_BUFFER_SIZE;

    /// constructs an `OutstationConfig` with default settings, except for the
    /// master and outstation link addresses which really don't have good defaults
    pub fn new(outstation_address: EndpointAddress, master_address: EndpointAddress) -> Self {
        Self {
            tx_buffer_size: Self::DEFAULT_TX_BUFFER_SIZE,
            rx_buffer_size: Self::DEFAULT_RX_BUFFER_SIZE,
            outstation_address,
            master_address,
            self_address_support: SelfAddressSupport::Disabled,
            log_level: DecodeLogLevel::Nothing,
            confirm_timeout: std::time::Duration::from_secs(5),
            select_timeout: std::time::Duration::from_secs(5),
            broadcast_support: BroadcastAddressSupport::Enabled,
        }
    }
}
