use std::ffi::CStr;
use std::net::IpAddr;

use dnp3::outstation::database::EventBufferConfig;
use dnp3::outstation::RestartDelay;
use dnp3::tcp::{BadIpv4Wildcard, WildcardIPv4};

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
    WildcardIpv4(WildcardIPv4),
    AnyOf(std::collections::HashSet<std::net::IpAddr>),
}

pub fn address_filter_any() -> *mut AddressFilter {
    Box::into_raw(Box::new(AddressFilter::Any))
}

fn parse_address_filter(s: &str) -> Result<AddressFilter, ffi::ParamError> {
    // first try to parse it as a normal IP
    match s.parse::<IpAddr>() {
        Ok(x) => {
            let mut set = std::collections::HashSet::new();
            set.insert(x);
            Ok(AddressFilter::AnyOf(set))
        }
        Err(_) => {
            // now try to parse as a wildcard
            let wc: WildcardIPv4 = s.parse()?;
            Ok(AddressFilter::WildcardIpv4(wc))
        }
    }
}

impl From<BadIpv4Wildcard> for ffi::ParamError {
    fn from(_: BadIpv4Wildcard) -> Self {
        ffi::ParamError::InvalidSocketAddress
    }
}

pub fn address_filter_create(address: &CStr) -> Result<*mut AddressFilter, ffi::ParamError> {
    let address = parse_address_filter(address.to_string_lossy().as_ref())?;
    Ok(Box::into_raw(Box::new(address)))
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
        AddressFilter::Any => {
            // can't add addresses to an "any" specification
            return Err(ffi::ParamError::AddressFilterConflict);
        }
        AddressFilter::AnyOf(set) => {
            set.insert(address);
        }
        AddressFilter::WildcardIpv4(_) => {
            // can't add addresses to a wildcard specification
            return Err(ffi::ParamError::AddressFilterConflict);
        }
    }
    Ok(())
}

pub unsafe fn address_filter_destroy(address_filter: *mut AddressFilter) {
    if !address_filter.is_null() {
        drop(Box::from_raw(address_filter));
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
                    restart_type: ffi::RestartDelayType::MilliSeconds,
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
            AddressFilter::WildcardIpv4(wc) => dnp3::tcp::AddressFilter::WildcardIpv4(*wc),
        }
    }
}
