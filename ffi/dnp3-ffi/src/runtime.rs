use crate::ffi;
use crate::*;
use dnp3::prelude::master::*;
use std::ffi::CStr;
use std::net::SocketAddr;
use std::ptr::null_mut;
use std::str::FromStr;
use std::time::Duration;

use dnp3::entry::NormalAddress;
pub use tokio::runtime::Runtime;

fn build_runtime<F>(f: F) -> std::result::Result<tokio::runtime::Runtime, std::io::Error>
where
    F: Fn(&mut tokio::runtime::Builder) -> &mut tokio::runtime::Builder,
{
    f(tokio::runtime::Builder::new()
        .enable_all()
        .threaded_scheduler())
    .build()
}

pub(crate) unsafe fn runtime_new(
    config: Option<&ffi::RuntimeConfig>,
) -> *mut tokio::runtime::Runtime {
    let result = match config {
        None => build_runtime(|r| r),
        Some(x) => build_runtime(|r| r.core_threads(x.num_core_threads as usize)),
    };

    match result {
        Ok(r) => Box::into_raw(Box::new(r)),
        Err(_) => {
            //log::error!("Unable to build runtime: {}", err);
            null_mut()
        }
    }
}

pub(crate) unsafe fn runtime_destroy(runtime: *mut tokio::runtime::Runtime) {
    if !runtime.is_null() {
        Box::from_raw(runtime);
    };
}

pub(crate) unsafe fn runtime_add_master_tcp(
    runtime: *mut tokio::runtime::Runtime,
    address: u16,
    level: ffi::DecodeLogLevel,
    strategy: ffi::ReconnectStrategy,
    response_timeout: Duration,
    endpoint: &CStr,
    listener: ffi::ClientStateListener,
) -> *mut Master {
    let strategy = ReconnectStrategy::new(strategy.min_delay(), strategy.max_delay());
    let response_timeout = response_timeout;
    let endpoint = if let Ok(endpoint) = SocketAddr::from_str(&endpoint.to_string_lossy()) {
        endpoint
    } else {
        return std::ptr::null_mut();
    };
    let listener = ClientStateListenerAdapter::new(listener);

    let address = match NormalAddress::from(address) {
        Ok(x) => x,
        Err(err) => {
            log::warn!(
                "special addresses may not be used for the master address: {}",
                err.address
            );
            return std::ptr::null_mut();
        }
    };

    let (future, handle) = create_master_tcp_client(
        address,
        level.into(),
        strategy,
        Timeout::from_duration(response_timeout).unwrap(),
        endpoint,
        listener.into_listener(),
    );

    if let Some(runtime) = runtime.as_ref() {
        runtime.spawn(future);

        let master = Master {
            runtime: runtime.handle().clone(),
            handle,
        };

        Box::into_raw(Box::new(master))
    } else {
        std::ptr::null_mut()
    }
}

struct ClientStateListenerAdapter {
    native_cb: ffi::ClientStateListener,
}

impl ClientStateListenerAdapter {
    fn new(native_cb: ffi::ClientStateListener) -> Self {
        Self { native_cb }
    }

    fn into_listener(self) -> Listener<ClientState> {
        Listener::BoxedFn(Box::new(move |value| {
            let value = match value {
                ClientState::Connecting => ffi::ClientState::Connecting,
                ClientState::Connected => ffi::ClientState::Connected,
                ClientState::WaitAfterFailedConnect(_) => ffi::ClientState::WaitAfterFailedConnect,
                ClientState::WaitAfterDisconnect(_) => ffi::ClientState::WaitAfterDisconnect,
                ClientState::Shutdown => ffi::ClientState::Shutdown,
            };
            self.native_cb.on_change(value);
        }))
    }
}
