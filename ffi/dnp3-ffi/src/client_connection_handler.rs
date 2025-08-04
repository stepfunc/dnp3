use crate::ffi::ParamError;
use std::ffi::CStr;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;

pub struct NextEndpointAction {
    pub(crate) action: Option<NextEndpointActionType>,
}

pub(crate) enum NextEndpointActionType {
    ConnectTo {
        endpoint: String,
        timeout: Option<Duration>,
        local_endpoint: Option<SocketAddr>,
    },
    SleepFor(Duration),
}

impl NextEndpointAction {
    pub(crate) fn new() -> Self {
        Self { action: None }
    }
}

pub(crate) unsafe fn next_endpoint_action_connect_to(
    instance: *mut NextEndpointAction,
    endpoint: &CStr,
    timeout_ms: Duration,
    local_endpoint: &CStr,
) -> ParamError {
    let action = match instance.as_mut() {
        Some(x) => x,
        None => return ParamError::NullParameter,
    };

    let endpoint_str = match endpoint.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return ParamError::InvalidSocketAddress,
    };

    let timeout = if timeout_ms.is_zero() {
        None
    } else {
        Some(timeout_ms)
    };

    let local_addr = if local_endpoint.to_bytes().is_empty() {
        None
    } else {
        match local_endpoint.to_str() {
            Ok(s) => match SocketAddr::from_str(s) {
                Ok(addr) => Some(addr),
                Err(_) => return ParamError::InvalidSocketAddress,
            },
            Err(_) => return ParamError::InvalidSocketAddress,
        }
    };

    action.action = Some(NextEndpointActionType::ConnectTo {
        endpoint: endpoint_str,
        timeout,
        local_endpoint: local_addr,
    });

    ParamError::Ok
}

pub(crate) unsafe fn next_endpoint_action_sleep_for(
    instance: *mut NextEndpointAction,
    duration: Duration,
) -> ParamError {
    let action = match instance.as_mut() {
        Some(x) => x,
        None => return ParamError::NullParameter,
    };

    action.action = Some(NextEndpointActionType::SleepFor(duration));
    ParamError::Ok
}
