use crate::app::parse::DecodeLogLevel;
use crate::entry::EndpointAddress;
use crate::outstation::database::DatabaseConfig;
use crate::util::buffer::Buffer;

/// Validated buffer size for use in the outstation
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct BufferSize {
    size: usize,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum BufferSizeError {
    /// provided size
    TooSmall(usize),
}

impl BufferSize {
    pub const MIN: usize = 249; // 1 link frame
    pub const DEFAULT: usize = 2048; // default max ASDU size for DNP3

    pub(crate) fn create_buffer(&self) -> Buffer {
        Buffer::new(self.size)
    }

    pub(crate) fn value(&self) -> usize {
        self.size
    }

    pub fn min() -> Self {
        Self { size: Self::MIN }
    }

    pub fn new(size: usize) -> Result<Self, BufferSizeError> {
        if size < Self::MIN {
            return Err(BufferSizeError::TooSmall(size));
        }
        Ok(Self { size })
    }
}

impl Default for BufferSize {
    fn default() -> Self {
        Self {
            size: Self::DEFAULT,
        }
    }
}

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
    pub solicited_buffer_size: BufferSize,
    pub unsolicited_buffer_size: BufferSize,
    pub rx_buffer_size: BufferSize,
    pub log_level: DecodeLogLevel,
    pub confirm_timeout: std::time::Duration,
    pub select_timeout: std::time::Duration,
    pub features: Features,
    pub max_unsolicited_retries: Option<usize>,
    pub unsolicited_retry_delay: std::time::Duration,
    pub max_read_headers_per_request: u16,
    pub keep_alive_timeout: Option<std::time::Duration>,
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
    pub const DEFAULT_CONFIRM_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
    pub const DEFAULT_SELECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
    pub const DEFAULT_UNSOLICITED_RETRY_DELAY: std::time::Duration =
        std::time::Duration::from_secs(5);

    /// constructs an `OutstationConfig` with default settings, except for the
    /// master and outstation link addresses which really don't have good defaults
    pub fn new(outstation_address: EndpointAddress, master_address: EndpointAddress) -> Self {
        Self {
            outstation_address,
            master_address,
            solicited_buffer_size: BufferSize::default(),
            unsolicited_buffer_size: BufferSize::default(),
            rx_buffer_size: BufferSize::default(),
            log_level: DecodeLogLevel::Nothing,
            confirm_timeout: Self::DEFAULT_CONFIRM_TIMEOUT,
            select_timeout: Self::DEFAULT_SELECT_TIMEOUT,
            features: Features::default(),
            max_unsolicited_retries: None,
            unsolicited_retry_delay: Self::DEFAULT_UNSOLICITED_RETRY_DELAY,
            max_read_headers_per_request: DatabaseConfig::DEFAULT_MAX_READ_REQUEST_HEADERS,
            keep_alive_timeout: Some(std::time::Duration::from_secs(60)),
        }
    }
}

impl std::fmt::Display for BufferSizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::TooSmall(size) => write!(
                f,
                "provided size {} is less than the minimum allowed size of {}",
                size,
                BufferSize::MIN
            ),
        }
    }
}

impl std::error::Error for BufferSizeError {}
