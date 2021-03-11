use crate::decode::DecodeLevel;
use crate::link::EndpointAddress;
use crate::outstation::database::ClassZeroConfig;
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

    pub fn value(&self) -> usize {
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
    pub decode_level: DecodeLevel,
    pub confirm_timeout: std::time::Duration,
    pub select_timeout: std::time::Duration,
    pub features: Features,
    pub max_unsolicited_retries: Option<usize>,
    pub unsolicited_retry_delay: std::time::Duration,
    pub keep_alive_timeout: Option<std::time::Duration>,
    /// Maximum number of headers that will be processed
    /// in a READ request. Internally, this controls the size of a
    /// pre-allocated buffer used to process requests. A minimum
    /// value of `DEFAULT_READ_REQUEST_HEADERS` is always enforced.
    /// Requesting more than this number will result in the PARAMETER_ERROR
    /// IIN bit being set in the response.
    pub max_read_request_headers: Option<u16>,
    /// Maximum number of controls in a single request
    pub max_controls_per_request: Option<u16>,
    /// controls responses to class 0 READ requests
    pub class_zero: ClassZeroConfig,
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
    pub const DEFAULT_MAX_READ_REQUEST_HEADERS: u16 = 64;
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
            decode_level: DecodeLevel::nothing(),
            confirm_timeout: Self::DEFAULT_CONFIRM_TIMEOUT,
            select_timeout: Self::DEFAULT_SELECT_TIMEOUT,
            features: Features::default(),
            max_unsolicited_retries: None,
            unsolicited_retry_delay: Self::DEFAULT_UNSOLICITED_RETRY_DELAY,
            keep_alive_timeout: Some(std::time::Duration::from_secs(60)),
            max_read_request_headers: None,
            max_controls_per_request: None,
            class_zero: ClassZeroConfig::default(),
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
