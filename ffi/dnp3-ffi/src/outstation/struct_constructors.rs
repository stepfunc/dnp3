use std::ffi::CStr;

use dnp3::outstation::database::EventBufferConfig;
use dnp3::outstation::RestartDelay;

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

pub fn address_filter_new(address: &CStr) -> Result<*mut AddressFilter, ffi::ParamError> {
    let address = address.to_string_lossy().parse()?;

    let mut set = std::collections::HashSet::new();
    set.insert(address);

    Ok(Box::into_raw(Box::new(AddressFilter::AnyOf(set))))
}

pub unsafe fn address_filter_add(
    address_filter: *mut AddressFilter,
    address: &CStr,
) -> Result<(), ffi::ParamError> {
    let address_filter = address_filter
        .as_mut()
        .ok_or(ffi::ParamError::NullParameter)?;
    let address = address.to_string_lossy().parse()?;

    match address_filter {
        AddressFilter::Any => (),
        AddressFilter::AnyOf(set) => {
            set.insert(address);
        }
    }
    Ok(())
}

pub unsafe fn address_filter_destroy(address_filter: *mut AddressFilter) {
    if !address_filter.is_null() {
        Box::from_raw(address_filter);
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

impl From<&AddressFilter> for dnp3::tcp::AddressFilter {
    fn from(from: &AddressFilter) -> Self {
        match from {
            AddressFilter::Any => dnp3::tcp::AddressFilter::Any,
            AddressFilter::AnyOf(set) => dnp3::tcp::AddressFilter::AnyOf(set.clone()),
        }
    }
}
