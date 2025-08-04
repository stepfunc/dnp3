use crate::ffi::ParamError;
use std::ffi::{CStr, CString};
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
        None => {
            tracing::warn!("next_endpoint_action_connect_to called with null NextEndpointAction instance");
            return ParamError::NullParameter;
        }
    };

    let endpoint_str = match endpoint.to_str() {
        Ok(s) => {
            if s.is_empty() {
                tracing::warn!("next_endpoint_action_connect_to called with empty endpoint string");
                return ParamError::InvalidSocketAddress;
            }
            s.to_string()
        }
        Err(_) => {
            tracing::warn!("next_endpoint_action_connect_to called with invalid UTF-8 endpoint string");
            return ParamError::InvalidSocketAddress;
        }
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
                Err(_) => {
                    tracing::warn!("next_endpoint_action_connect_to called with invalid local endpoint address: {}", s);
                    return ParamError::InvalidSocketAddress;
                }
            },
            Err(_) => {
                tracing::warn!("next_endpoint_action_connect_to called with invalid UTF-8 local endpoint string");
                return ParamError::InvalidSocketAddress;
            }
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
        None => {
            tracing::warn!("next_endpoint_action_sleep_for called with null NextEndpointAction instance");
            return ParamError::NullParameter;
        }
    };

    if duration.is_zero() {
        tracing::warn!("next_endpoint_action_sleep_for called with zero duration");
    }

    action.action = Some(NextEndpointActionType::SleepFor(duration));
    ParamError::Ok
}

pub struct ClientConnectionHandlerAdapter {
    callback: crate::ffi::ClientConnectionHandler,
    span_name: String,
}

impl ClientConnectionHandlerAdapter {
    pub fn new(callback: crate::ffi::ClientConnectionHandler, span_name: String) -> Self {
        Self {
            callback,
            span_name,
        }
    }
}

impl dnp3::tcp::ClientConnectionHandler for ClientConnectionHandlerAdapter {
    fn endpoint_span_name(&self) -> String {
        self.span_name.clone()
    }

    fn disconnected(&mut self, addr: std::net::SocketAddr, hostname: Option<&str>) -> Duration {
        use std::ffi::CString;

        let addr_str = CString::new(addr.to_string()).unwrap_or_default();
        let hostname_str = CString::new(hostname.unwrap_or("")).unwrap_or_default();

        self.callback
            .disconnected(&addr_str, &hostname_str)
            .unwrap_or_else(|| Duration::from_secs(5)) // Default to 5 seconds if callback not set
    }

    fn next(&mut self) -> Result<dnp3::tcp::ConnectionInfo, Duration> {
        let mut action = NextEndpointAction::new();
        self.callback.next(&mut action);

        match action.action {
            Some(NextEndpointActionType::ConnectTo {
                endpoint,
                timeout,
                local_endpoint,
            }) => {
                let endpoint = dnp3::tcp::Endpoint::from(endpoint);
                let mut info = dnp3::tcp::ConnectionInfo::new(endpoint);
                if let Some(timeout) = timeout {
                    info.set_timeout(timeout);
                }
                if let Some(local) = local_endpoint {
                    info.set_local_endpoint(local);
                }
                Ok(info)
            }
            Some(NextEndpointActionType::SleepFor(duration)) => Err(duration),
            None => Err(Duration::from_secs(5)), // fallback if no action was set
        }
    }

    fn connecting(&mut self, addr: SocketAddr, hostname: Option<&str>) {
        let addr_str = CString::new(addr.to_string()).unwrap_or_default();
        let hostname_str = CString::new(hostname.unwrap_or("")).unwrap_or_default();

        self.callback.connecting(&addr_str, &hostname_str);
    }

    fn connect_failed(&mut self, addr: SocketAddr, hostname: Option<&str>) {
        let addr_str = CString::new(addr.to_string()).unwrap_or_default();
        let hostname_str = CString::new(hostname.unwrap_or("")).unwrap_or_default();

        self.callback.connect_failed(&addr_str, &hostname_str);
    }

    fn connected(&mut self, addr: SocketAddr, hostname: Option<&str>) {
        let addr_str = CString::new(addr.to_string()).unwrap_or_default();
        let hostname_str = CString::new(hostname.unwrap_or("")).unwrap_or_default();

        self.callback.connected(&addr_str, &hostname_str);
    }

    fn resolution_failed(&mut self, host_name: &str) {
        let hostname_str = CString::new(host_name).unwrap_or_default();
        self.callback.resolution_failed(&hostname_str);
    }
}
