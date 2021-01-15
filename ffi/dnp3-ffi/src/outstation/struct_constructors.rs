use std::ffi::CStr;
use std::time::Duration;

use dnp3::entry::EndpointAddress;
use dnp3::outstation::config::{Feature, Features, OutstationConfig};
use dnp3::outstation::database::{ClassZeroConfig, DatabaseConfig, EventBufferConfig};
use dnp3::outstation::traits::RestartDelay;

use crate::ffi;

pub fn outstation_features_default() -> ffi::OutstationFeatures {
    Features::default().into()
}

pub fn class_zero_config_default() -> ffi::ClassZeroConfig {
    ClassZeroConfig::default().into()
}

pub fn event_buffer_config_all_types(max: u16) -> ffi::EventBufferConfig {
    EventBufferConfig::all_types(max).into()
}

pub fn event_buffer_config_no_events() -> ffi::EventBufferConfig {
    EventBufferConfig::no_events().into()
}

pub fn database_config_default() -> ffi::DatabaseConfig {
    DatabaseConfig::default().into()
}

pub fn outstation_config_default(
    outstation_address: u16,
    master_address: u16,
) -> ffi::OutstationConfig {
    // TODO: check what to do with these unwraps
    let outstation_address = EndpointAddress::from(outstation_address).unwrap();
    let master_address = EndpointAddress::from(master_address).unwrap();
    OutstationConfig::new(outstation_address, master_address).into()
}

pub fn restart_delay_not_supported() -> ffi::RestartDelay {
    None.into()
}

pub fn restart_delay_seconds(value: u16) -> ffi::RestartDelay {
    Some(RestartDelay::Seconds(value)).into()
}

pub fn restart_delay_millis(value: u16) -> ffi::RestartDelay {
    Some(RestartDelay::Milliseconds(value)).into()
}

pub enum AddressFilter {
    Any,
    AnyOf(std::collections::HashSet<std::net::IpAddr>),
}

pub fn address_filter_any() -> *mut AddressFilter {
    Box::into_raw(Box::new(AddressFilter::Any))
}

pub fn address_filter_new(address: &CStr) -> *mut AddressFilter {
    let address = match address.to_string_lossy().parse() {
        Ok(address) => address,
        Err(_) => return std::ptr::null_mut(),
    };

    let mut set = std::collections::HashSet::new();
    set.insert(address);

    Box::into_raw(Box::new(AddressFilter::AnyOf(set)))
}

pub unsafe fn address_filter_add(address_filter: *mut AddressFilter, address: &CStr) {
    let address_filter = match address_filter.as_mut() {
        Some(address_filter) => address_filter,
        None => return,
    };

    let address = match address.to_string_lossy().parse() {
        Ok(address) => address,
        Err(_) => return,
    };

    match address_filter {
        AddressFilter::Any => (),
        AddressFilter::AnyOf(set) => {
            set.insert(address);
        }
    }
}

pub unsafe fn address_filter_destroy(address_filter: *mut AddressFilter) {
    if !address_filter.is_null() {
        Box::from_raw(address_filter);
    }
}

impl From<Features> for ffi::OutstationFeatures {
    fn from(from: Features) -> Self {
        ffi::OutstationFeaturesFields {
            self_address: matches!(from.self_address, Feature::Enabled),
            broadcast: matches!(from.broadcast, Feature::Enabled),
            unsolicited: matches!(from.unsolicited, Feature::Enabled),
        }
        .into()
    }
}

impl From<ClassZeroConfig> for ffi::ClassZeroConfig {
    fn from(from: ClassZeroConfig) -> Self {
        ffi::ClassZeroConfigFields {
            binary: from.binary,
            double_bit_binary: from.double_bit_binary,
            binary_output_status: from.binary_output_status,
            counter: from.counter,
            frozen_counter: from.frozen_counter,
            analog: from.analog,
            analog_output_status: from.analog_output_status,
            octet_strings: from.octet_strings,
        }
        .into()
    }
}

impl From<EventBufferConfig> for ffi::EventBufferConfig {
    fn from(from: EventBufferConfig) -> Self {
        ffi::EventBufferConfigFields {
            max_binary: from.max_binary,
            max_double_bit_binary: from.max_double_binary,
            max_binary_output_status: from.max_binary_output_status,
            max_counter: from.max_counter,
            max_frozen_counter: from.max_frozen_counter,
            max_analog: from.max_analog,
            max_analog_output_status: from.max_analog_output_status,
            max_octet_string: from.max_octet_string,
        }
        .into()
    }
}

impl From<DatabaseConfig> for ffi::DatabaseConfig {
    fn from(from: DatabaseConfig) -> Self {
        ffi::DatabaseConfigFields {
            max_read_request_headers: from
                .max_read_request_headers
                .unwrap_or(DatabaseConfig::DEFAULT_MAX_READ_REQUEST_HEADERS),
            class_zero: from.class_zero.into(),
            events: from.events.into(),
        }
        .into()
    }
}

impl From<OutstationConfig> for ffi::OutstationConfig {
    fn from(from: OutstationConfig) -> Self {
        ffi::OutstationConfigFields {
            outstation_address: from.outstation_address.raw_value(),
            master_address: from.master_address.raw_value(),
            solicited_buffer_size: from.solicited_buffer_size.value() as u16,
            unsolicited_buffer_size: from.unsolicited_buffer_size.value() as u16,
            rx_buffer_size: from.rx_buffer_size.value() as u16,
            bubble_framing_errors: from.bubble_framing_errors,
            log_level: from.log_level.into(),
            confirm_timeout: from.confirm_timeout,
            select_timeout: from.select_timeout,
            features: from.features.into(),
            max_unsolicited_retries: from
                .max_unsolicited_retries
                .map(|e| e as u32)
                .unwrap_or(std::u32::MAX),
            unsolicited_retry_delay: from.unsolicited_retry_delay,
            max_read_headers_per_request: from.max_read_headers_per_request,
            keep_alive_timeout: from
                .keep_alive_timeout
                .unwrap_or_else(|| Duration::from_secs(0)),
        }
        .into()
    }
}

impl From<Option<RestartDelay>> for ffi::RestartDelay {
    fn from(from: Option<RestartDelay>) -> Self {
        match from {
            None => ffi::RestartDelayFields {
                restart_type: ffi::RestartDelayType::NotSupported,
                value: 0,
            }
            .into(),
            Some(delay) => match delay {
                RestartDelay::Seconds(value) => ffi::RestartDelayFields {
                    restart_type: ffi::RestartDelayType::Seconds,
                    value,
                }
                .into(),
                RestartDelay::Milliseconds(value) => ffi::RestartDelayFields {
                    restart_type: ffi::RestartDelayType::Milliseconds,
                    value,
                }
                .into(),
            },
        }
    }
}

impl From<&AddressFilter> for dnp3::entry::outstation::AddressFilter {
    fn from(from: &AddressFilter) -> Self {
        match from {
            AddressFilter::Any => dnp3::entry::outstation::AddressFilter::Any,
            AddressFilter::AnyOf(set) => dnp3::entry::outstation::AddressFilter::AnyOf(set.clone()),
        }
    }
}
