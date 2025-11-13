use crate::ffi::ParamError;
use dnp3::link::EndpointAddress;
use std::ffi::{CStr, CString};
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;

pub struct ConnectionInfo {
    endpoint: Option<dnp3::tcp::Endpoint>,
    timeout: Option<Duration>,
    local_endpoint: Option<SocketAddr>,
    master_address: Option<EndpointAddress>,
}

impl ConnectionInfo {
    pub(crate) fn new() -> Self {
        Self {
            endpoint: None,
            timeout: None,
            local_endpoint: None,
            master_address: None,
        }
    }

    pub(crate) fn set_endpoint(&mut self, endpoint: String) -> ParamError {
        if endpoint.is_empty() {
            tracing::warn!("set_endpoint called with empty endpoint string");
            return ParamError::InvalidSocketAddress;
        }
        self.endpoint = Some(dnp3::tcp::Endpoint::from(endpoint));
        ParamError::Ok
    }

    pub(crate) fn set_timeout(&mut self, timeout: Duration) {
        if !timeout.is_zero() {
            self.timeout = Some(timeout);
        }
    }

    pub(crate) fn set_local_endpoint(&mut self, local: SocketAddr) {
        self.local_endpoint = Some(local);
    }

    pub(crate) fn set_master_address(&mut self, address: u16) -> ParamError {
        match EndpointAddress::try_new(address) {
            Ok(addr) => {
                self.master_address = Some(addr);
                ParamError::Ok
            }
            Err(_) => {
                tracing::warn!(
                    "set_master_address called with invalid address: {}",
                    address
                );
                ParamError::InvalidDnp3Address
            }
        }
    }

    pub(crate) fn build(&self) -> Result<dnp3::tcp::ConnectionInfo, ParamError> {
        let endpoint = self.endpoint.clone().ok_or_else(|| {
            tracing::warn!("ConnectionInfo used without calling set_endpoint()");
            ParamError::InvalidSocketAddress
        })?;

        let mut info = dnp3::tcp::ConnectionInfo::new(endpoint);

        if let Some(timeout) = self.timeout {
            info.set_timeout(timeout);
        }
        if let Some(local) = self.local_endpoint {
            info.set_local_endpoint(local);
        }
        if let Some(addr) = self.master_address {
            info.set_master_address(addr);
        }

        Ok(info)
    }
}

pub(crate) unsafe fn connection_info_create() -> *mut ConnectionInfo {
    Box::into_raw(Box::new(ConnectionInfo::new()))
}

pub(crate) unsafe fn connection_info_set_endpoint(
    instance: *mut ConnectionInfo,
    endpoint: &CStr,
) -> ParamError {
    let info = match instance.as_mut() {
        Some(x) => x,
        None => {
            tracing::warn!("connection_info_set_endpoint called with null ConnectionInfo instance");
            return ParamError::NullParameter;
        }
    };

    let endpoint_str = match endpoint.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => {
            tracing::warn!(
                "connection_info_set_endpoint called with invalid UTF-8 endpoint string"
            );
            return ParamError::InvalidSocketAddress;
        }
    };

    info.set_endpoint(endpoint_str)
}

pub(crate) unsafe fn connection_info_set_timeout(instance: *mut ConnectionInfo, timeout: Duration) {
    if let Some(info) = instance.as_mut() {
        info.set_timeout(timeout);
    } else {
        tracing::warn!("connection_info_set_timeout called with null ConnectionInfo instance");
    }
}

pub(crate) unsafe fn connection_info_set_local_endpoint(
    instance: *mut ConnectionInfo,
    local_endpoint: &CStr,
) -> ParamError {
    let info = match instance.as_mut() {
        Some(x) => x,
        None => {
            tracing::warn!(
                "connection_info_set_local_endpoint called with null ConnectionInfo instance"
            );
            return ParamError::NullParameter;
        }
    };

    if local_endpoint.to_bytes().is_empty() {
        return ParamError::Ok; // Empty string means don't set it
    }

    let local_addr = match local_endpoint.to_str() {
        Ok(s) => match SocketAddr::from_str(s) {
            Ok(addr) => addr,
            Err(_) => {
                tracing::warn!(
                    "connection_info_set_local_endpoint called with invalid local endpoint address: {}",
                    s
                );
                return ParamError::InvalidSocketAddress;
            }
        },
        Err(_) => {
            tracing::warn!(
                "connection_info_set_local_endpoint called with invalid UTF-8 local endpoint string"
            );
            return ParamError::InvalidSocketAddress;
        }
    };

    info.set_local_endpoint(local_addr);
    ParamError::Ok
}

pub(crate) unsafe fn connection_info_set_master_address(
    instance: *mut ConnectionInfo,
    address: u16,
) -> ParamError {
    match instance.as_mut() {
        Some(info) => info.set_master_address(address),
        None => {
            tracing::warn!(
                "connection_info_set_master_address called with null ConnectionInfo instance"
            );
            ParamError::NullParameter
        }
    }
}

pub struct NextEndpointAction {
    pub(crate) action: Option<NextEndpointActionType>,
}

pub(crate) enum NextEndpointActionType {
    ConnectTo(dnp3::tcp::ConnectionInfo),
    SleepFor(Duration),
}

impl NextEndpointAction {
    pub(crate) fn new() -> Self {
        Self { action: None }
    }
}

pub(crate) unsafe fn next_endpoint_action_connect_to(
    instance: *mut NextEndpointAction,
    info: *const ConnectionInfo,
) -> ParamError {
    let action = match instance.as_mut() {
        Some(x) => x,
        None => {
            tracing::warn!(
                "next_endpoint_action_connect_to called with null NextEndpointAction instance"
            );
            return ParamError::NullParameter;
        }
    };

    let connection_info = match info.as_ref() {
        Some(x) => x,
        None => {
            tracing::warn!(
                "next_endpoint_action_connect_to called with null ConnectionInfo instance"
            );
            return ParamError::NullParameter;
        }
    };

    match connection_info.build() {
        Ok(info) => {
            action.action = Some(NextEndpointActionType::ConnectTo(info));
            ParamError::Ok
        }
        Err(err) => err,
    }
}

pub(crate) unsafe fn next_endpoint_action_sleep_for(
    instance: *mut NextEndpointAction,
    duration: Duration,
) -> ParamError {
    let action = match instance.as_mut() {
        Some(x) => x,
        None => {
            tracing::warn!(
                "next_endpoint_action_sleep_for called with null NextEndpointAction instance"
            );
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
            Some(NextEndpointActionType::ConnectTo(info)) => Ok(info),
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
