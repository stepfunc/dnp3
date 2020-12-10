use crate::app::parse::DecodeLogLevel;
use crate::entry::EndpointAddress;

/// describes whether an optional feature is enabled or disabled
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Feature {
    /// feature is enabled
    Enabled,
    /// feature is disabled
    Disabled,
}

/// Optional features that can be enabled or disabled
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Features {
    /// if enabled, the outstation responds to the self address (default == Disabled)
    pub self_address: Feature,
    /// if enabled, the outstation processes valid broadcast messages (default == Enabled)
    pub broadcast: Feature,
    /// if enabled, the outstation will send process enable/disable unsolicited and produce unsolicited responses (default == Enabled)
    pub unsolicited: Feature,
}

impl Default for Features {
    fn default() -> Self {
        Self {
            self_address: Feature::Disabled,
            broadcast: Feature::Enabled,
            unsolicited: Feature::Enabled,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct OutstationConfig {
    pub outstation_address: EndpointAddress,
    pub master_address: EndpointAddress,
    pub tx_buffer_size: usize,
    pub rx_buffer_size: usize,
    pub log_level: DecodeLogLevel,
    pub confirm_timeout: std::time::Duration,
    pub select_timeout: std::time::Duration,
    pub features: Features,
}

impl Feature {
    pub(crate) fn is_enabled(&self) -> bool {
        *self == Feature::Enabled
    }

    pub(crate) fn is_disabled(&self) -> bool {
        *self == Feature::Disabled
    }
}

impl OutstationConfig {
    pub const MIN_TX_BUFFER_SIZE: usize = 249; // 1 link frame
    pub const DEFAULT_TX_BUFFER_SIZE: usize = 2048;

    pub const MIN_RX_BUFFER_SIZE: usize = Self::MIN_TX_BUFFER_SIZE;
    pub const DEFAULT_RX_BUFFER_SIZE: usize = Self::DEFAULT_TX_BUFFER_SIZE;

    pub const DEFAULT_CONFIRM_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
    pub const DEFAULT_SELECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

    /// constructs an `OutstationConfig` with default settings, except for the
    /// master and outstation link addresses which really don't have good defaults
    pub fn new(outstation_address: EndpointAddress, master_address: EndpointAddress) -> Self {
        Self {
            outstation_address,
            master_address,
            tx_buffer_size: Self::DEFAULT_TX_BUFFER_SIZE,
            rx_buffer_size: Self::DEFAULT_RX_BUFFER_SIZE,
            log_level: DecodeLogLevel::Nothing,
            confirm_timeout: Self::DEFAULT_CONFIRM_TIMEOUT,
            select_timeout: Self::DEFAULT_SELECT_TIMEOUT,
            features: Features::default(),
        }
    }
}
