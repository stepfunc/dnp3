use std::ffi::CStr;

use dnp3::outstation::database::EventBufferConfig;
use dnp3::outstation::traits::RestartDelay;

use crate::ffi;

pub fn event_buffer_config_all_types(max: u16) -> ffi::EventBufferConfig {
    EventBufferConfig::all_types(max).into()
}

pub fn event_buffer_config_no_events() -> ffi::EventBufferConfig {
    EventBufferConfig::no_events().into()
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

pub fn timestamp_invalid() -> ffi::Timestamp {
    ffi::TimestampFields {
        value: 0,
        quality: ffi::TimeQuality::Invalid,
    }
    .into()
}

pub fn timestamp_synchronized(value: u64) -> ffi::Timestamp {
    ffi::TimestampFields {
        value,
        quality: ffi::TimeQuality::Synchronized,
    }
    .into()
}

pub fn timestamp_not_synchronized(value: u64) -> ffi::Timestamp {
    ffi::TimestampFields {
        value,
        quality: ffi::TimeQuality::NotSynchronized,
    }
    .into()
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

impl From<&AddressFilter> for dnp3::outstation::tcp::AddressFilter {
    fn from(from: &AddressFilter) -> Self {
        match from {
            AddressFilter::Any => dnp3::outstation::tcp::AddressFilter::Any,
            AddressFilter::AnyOf(set) => dnp3::outstation::tcp::AddressFilter::AnyOf(set.clone()),
        }
    }
}
