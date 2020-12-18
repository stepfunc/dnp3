use crate::ffi;
use crate::*;
use dnp3::app::retry::RetryStrategy;
use dnp3::prelude::master::*;
use std::ffi::CStr;
use std::net::SocketAddr;
use std::ptr::null_mut;
use std::str::FromStr;
use std::time::Duration;

use dnp3::entry::EndpointAddress;
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
    config: ffi::MasterConfiguration,
    endpoint: &CStr,
    listener: ffi::ClientStateListener,
) -> *mut Master {
    let config = if let Some(config) = config.into() {
        config
    } else {
        return std::ptr::null_mut();
    };

    let endpoint = if let Ok(endpoint) = SocketAddr::from_str(&endpoint.to_string_lossy()) {
        endpoint
    } else {
        return std::ptr::null_mut();
    };
    let listener = ClientStateListenerAdapter::new(listener);

    let (future, handle) = create_master_tcp_client(config, endpoint, listener.into_listener());

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

pub(crate) unsafe fn runtime_add_master_serial(
    runtime: *mut tokio::runtime::Runtime,
    config: ffi::MasterConfiguration,
    path: &CStr,
    serial_params: ffi::SerialPortSettings,
    listener: ffi::ClientStateListener,
) -> *mut Master {
    let config = if let Some(config) = config.into() {
        config
    } else {
        return std::ptr::null_mut();
    };
    let listener = ClientStateListenerAdapter::new(listener);

    let (future, handle) = create_master_serial_client(
        config,
        path.to_string_lossy().to_string(),
        serial_params.into(),
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

impl ffi::MasterConfiguration {
    fn into(self) -> Option<MasterConfiguration> {
        let address = match EndpointAddress::from(self.address()) {
            Ok(x) => x,
            Err(err) => {
                log::warn!(
                    "special addresses may not be used for the master address: {}",
                    err.address
                );
                return None;
            }
        };

        let strategy = RetryStrategy::new(
            self.reconnection_strategy().min_delay(),
            self.reconnection_strategy.max_delay(),
        );

        Some(MasterConfiguration {
            address,
            level: self.level().into(),
            reconnection_strategy: strategy,
            response_timeout: Timeout::from_duration(self.response_timeout()).unwrap(),
            tx_buffer_size: self.tx_buffer_size() as usize,
            rx_buffer_size: self.rx_buffer_size() as usize,
            bubble_framing_errors: self.bubble_framing_errors(),
        })
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

impl From<ffi::SerialPortSettings> for SerialPortSettings {
    fn from(from: ffi::SerialPortSettings) -> Self {
        Self {
            baud_rate: from.baud_rate(),
            data_bits: match from.data_bits() {
                ffi::DataBits::Five => DataBits::Five,
                ffi::DataBits::Six => DataBits::Six,
                ffi::DataBits::Seven => DataBits::Seven,
                ffi::DataBits::Eight => DataBits::Eight,
            },
            flow_control: match from.flow_control() {
                ffi::FlowControl::None => FlowControl::None,
                ffi::FlowControl::Software => FlowControl::Software,
                ffi::FlowControl::Hardware => FlowControl::Hardware,
            },
            parity: match from.parity() {
                ffi::Parity::None => Parity::None,
                ffi::Parity::Odd => Parity::Odd,
                ffi::Parity::Even => Parity::Even,
            },
            stop_bits: match from.stop_bits() {
                ffi::StopBits::One => StopBits::One,
                ffi::StopBits::Two => StopBits::Two,
            },
            timeout: Duration::from_millis(1),
        }
    }
}
