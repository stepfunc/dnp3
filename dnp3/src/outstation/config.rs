use crate::app::{BufferSize, Timeout};
use crate::decode::DecodeLevel;
use crate::link::EndpointAddress;
use crate::outstation::database::{ClassZeroConfig, EventBufferConfig};

/// describes whether an optional feature is enabled or disabled
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum Feature {
    /// feature is enabled
    Enabled,
    /// feature is disabled
    Disabled,
}

/// Optional features that can be enabled or disabled
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(not(feature = "ffi"), non_exhaustive)]
pub struct Features {
    /// if enabled, the outstation responds to the self address (default == Disabled)
    pub self_address: Feature,
    /// if enabled, the outstation processes valid broadcast messages (default == Enabled)
    pub broadcast: Feature,
    /// if enabled, the outstation will send process enable/disable unsolicited and produce unsolicited responses (default == Enabled)
    pub unsolicited: Feature,
    /// if enabled, the outstation will process every request as if it came from the configured master address
    ///
    /// This feature is a hack that can make configuration of some systems easier/more flexible, but
    /// should not be used when unsolicited reporting is also required.
    pub respond_to_any_master: Feature,
}

impl Default for Features {
    fn default() -> Self {
        Self {
            self_address: Feature::Disabled,
            broadcast: Feature::Enabled,
            unsolicited: Feature::Enabled,
            respond_to_any_master: Feature::Disabled,
        }
    }
}

/// Outstation configuration parameters
#[derive(Copy, Clone, Debug)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct OutstationConfig {
    /// address of the outstation
    pub outstation_address: EndpointAddress,
    /// address of the master with which the outstation will communicate
    pub master_address: EndpointAddress,
    /// event buffers configuration
    pub event_buffer_config: EventBufferConfig,
    /// buffer size for transmitted solicited responses
    #[cfg_attr(feature = "serialization", serde(default))]
    pub solicited_buffer_size: BufferSize,
    /// buffer size for transmitted unsolicited responses
    #[cfg_attr(feature = "serialization", serde(default))]
    pub unsolicited_buffer_size: BufferSize,
    /// buffer size for received requests, i.e. the transport reassembly buffer
    #[cfg_attr(feature = "serialization", serde(default))]
    pub rx_buffer_size: BufferSize,
    /// initial decoding level
    #[cfg_attr(feature = "serialization", serde(default))]
    pub decode_level: DecodeLevel,
    /// confirm timeout for solicited and unsolicited responses
    #[cfg_attr(feature = "serialization", serde(default))]
    pub confirm_timeout: Timeout,
    /// timeout after which a matching OPERATE will fail with SELECT_TIMEOUT
    #[cfg_attr(feature = "serialization", serde(default))]
    pub select_timeout: Timeout,
    /// optional features that can be enabled
    #[cfg_attr(feature = "serialization", serde(default))]
    pub features: Features,
    /// number of non-regenerated unsolicited retries to perform
    #[cfg_attr(feature = "serialization", serde(default))]
    pub max_unsolicited_retries: Option<usize>,
    /// amount of time to wait after a failed unsolicited response series before starting another series
    #[cfg_attr(
        feature = "serialization",
        serde(default = "OutstationConfig::default_unsolicited_retry_delay")
    )]
    pub unsolicited_retry_delay: std::time::Duration,
    /// time without any link activity before the outstation will send REQUEST_LINK_STATES
    ///
    /// A value of `None` will disable this feature
    #[cfg_attr(
        feature = "serialization",
        serde(default = "OutstationConfig::default_keep_alive_timeout")
    )]
    pub keep_alive_timeout: Option<std::time::Duration>,
    /// Maximum number of headers that will be processed
    /// in a READ request. Internally, this controls the size of a
    /// pre-allocated buffer used to process requests. A minimum
    /// value of [`OutstationConfig::DEFAULT_MAX_READ_REQUEST_HEADERS`] is always enforced.
    /// Requesting more than this number will result in the PARAMETER_ERROR
    /// IIN bit being set in the response.
    #[cfg_attr(feature = "serialization", serde(default))]
    pub max_read_request_headers: Option<u16>,
    /// Maximum number of controls in a single request
    #[cfg_attr(feature = "serialization", serde(default))]
    pub max_controls_per_request: Option<u16>,
    /// controls responses to class 0 READ requests
    #[cfg_attr(feature = "serialization", serde(default))]
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
    const fn default_keep_alive_timeout() -> Option<core::time::Duration> {
        Some(core::time::Duration::from_secs(60))
    }

    const fn default_unsolicited_retry_delay() -> core::time::Duration {
        std::time::Duration::from_secs(5)
    }

    /// Default number of object headers supported in a READ request
    pub const DEFAULT_MAX_READ_REQUEST_HEADERS: u16 = 64;
    /// Default confirmation timeout
    pub const DEFAULT_CONFIRM_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
    /// Default select timeout
    pub const DEFAULT_SELECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
    /// Default unsolicited retry delay between series
    pub const DEFAULT_UNSOLICITED_RETRY_DELAY: std::time::Duration =
        Self::default_unsolicited_retry_delay();

    /// constructs an `OutstationConfig` with default settings, except for the
    /// master and outstation link addresses which really don't have good defaults
    pub fn new(
        outstation_address: EndpointAddress,
        master_address: EndpointAddress,
        event_buffer_config: EventBufferConfig,
    ) -> Self {
        Self {
            outstation_address,
            master_address,
            event_buffer_config,
            solicited_buffer_size: BufferSize::default(),
            unsolicited_buffer_size: BufferSize::default(),
            rx_buffer_size: BufferSize::default(),
            decode_level: DecodeLevel::nothing(),
            confirm_timeout: Timeout::default(),
            select_timeout: Timeout::default(),
            features: Features::default(),
            max_unsolicited_retries: None,
            unsolicited_retry_delay: Self::DEFAULT_UNSOLICITED_RETRY_DELAY,
            keep_alive_timeout: Self::default_keep_alive_timeout(),
            max_read_request_headers: None,
            max_controls_per_request: None,
            class_zero: ClassZeroConfig::default(),
        }
    }
}

#[cfg(test)]
#[cfg(feature = "serialization")]
mod test {

    #[test]
    fn deserialization_allows_defaults() {
        assert!(serde_json::from_str::<super::OutstationConfig>("{}").is_err());

        let data = r#"{
            "outstation_address": 64,
            "master_address": 99,
            "event_buffer_config": {
                "max_binary": 7
            }
        }"#;

        let config = serde_json::from_str::<super::OutstationConfig>(data).unwrap();

        assert_eq!(config.outstation_address.raw_value(), 64);
        assert_eq!(config.master_address.raw_value(), 99);
        assert_eq!(config.event_buffer_config.max_binary, 7);
        assert_eq!(config.event_buffer_config.max_analog, 0);
        assert_eq!(config.rx_buffer_size.value(), 2048);
    }
}
