mod adapters;
mod database;
mod struct_constructors;

use std::ffi::CStr;
use std::time::Duration;

use dnp3::entry::outstation::tcp::ServerHandle;
use dnp3::entry::EndpointAddress;
use dnp3::outstation::config::{BufferSize, Feature, Features, OutstationConfig};
use dnp3::outstation::database::{ClassZeroConfig, DatabaseConfig, EventBufferConfig};
use dnp3::outstation::task::OutstationHandle;

pub use database::*;
pub use struct_constructors::*;

use crate::{ffi, Runtime, RuntimeHandle};

pub struct TcpServer {
    runtime: RuntimeHandle,
    server: Option<dnp3::entry::outstation::tcp::TCPServer>,
    handle: Option<ServerHandle>,
}

pub struct Outstation {
    handle: OutstationHandle,
    runtime: RuntimeHandle,
}

pub unsafe fn tcpserver_new(runtime: *mut Runtime, address: &CStr) -> *mut TcpServer {
    let runtime = match runtime.as_ref() {
        Some(runtime) => runtime,
        None => return std::ptr::null_mut(),
    };

    let address = match address.to_string_lossy().parse() {
        Ok(address) => address,
        Err(_) => return std::ptr::null_mut(),
    };

    let server = dnp3::entry::outstation::tcp::TCPServer::new(address);

    Box::into_raw(Box::new(TcpServer {
        runtime: runtime.handle(),
        server: Some(server),
        handle: None,
    }))
}

pub unsafe fn tcpserver_destroy(server: *mut TcpServer) {
    if !server.is_null() {
        Box::from_raw(server);
    }
}

pub unsafe fn tcpserver_add_outstation(
    server: *mut TcpServer,
    config: ffi::OutstationConfig,
    database: ffi::DatabaseConfig,
    application: ffi::OutstationApplication,
    information: ffi::OutstationInformation,
    control_handler: ffi::ControlHandler,
    filter: *mut AddressFilter,
) -> *mut Outstation {
    let server = match server.as_mut() {
        Some(server) => server,
        None => return std::ptr::null_mut(),
    };

    let server_handle = match &mut server.server {
        Some(server) => server,
        None => return std::ptr::null_mut(),
    };

    let config = match convert_outstation_config(config) {
        Some(config) => config,
        None => return std::ptr::null_mut(),
    };

    let filter = match filter.as_ref() {
        Some(filter) => filter.into(),
        None => return std::ptr::null_mut(),
    };

    let outstation = match server_handle.add_outstation(
        config,
        database.into(),
        Box::new(application),
        Box::new(information),
        Box::new(control_handler),
        filter,
    ) {
        Ok((outstation, task)) => {
            server.runtime.unwrap().spawn(task);
            outstation
        }
        Err(_) => return std::ptr::null_mut(),
    };

    Box::into_raw(Box::new(Outstation {
        handle: outstation,
        runtime: server.runtime.clone(),
    }))
}

pub unsafe fn tcpserver_bind(server: *mut TcpServer) {
    let server = match server.as_mut() {
        Some(server) => server,
        None => return,
    };

    let server_handle = match server.server.take() {
        Some(server) => server,
        None => return,
    };

    let runtime = match server.runtime.get() {
        Some(runtime) => runtime,
        None => return,
    };

    let (handle, task) = match runtime.block_on(server_handle.bind()) {
        Ok((handle, task)) => (handle, task),
        Err(_) => return,
    };

    runtime.spawn(task);
    server.handle = Some(handle);
}

pub unsafe fn outstation_destroy(outstation: *mut Outstation) {
    if !outstation.is_null() {
        Box::from_raw(outstation);
    }
}

pub unsafe fn outstation_transaction(
    outstation: *mut Outstation,
    callback: ffi::OutstationTransaction,
) {
    if let Some(outstation) = outstation.as_mut() {
        outstation.handle.database.transaction(|database| {
            callback.execute(database as *mut _);
        });
    }
}

pub unsafe fn outstation_set_decode_log_level(
    outstation: *mut Outstation,
    level: ffi::DecodeLogLevel,
) {
    if let Some(outstation) = outstation.as_mut() {
        if let Some(runtime) = outstation.runtime.get() {
            runtime.spawn(outstation.handle.set_decode_log_level(level.into()));
        }
    }
}

fn convert_outstation_config(config: ffi::OutstationConfig) -> Option<OutstationConfig> {
    let outstation_address = match EndpointAddress::from(config.outstation_address()) {
        Ok(address) => address,
        Err(_) => return None,
    };

    let master_address = match EndpointAddress::from(config.master_address()) {
        Ok(address) => address,
        Err(_) => return None,
    };

    let solicited_buffer_size = match BufferSize::new(config.solicited_buffer_size() as usize) {
        Ok(buffer_size) => buffer_size,
        Err(_) => return None,
    };

    let unsolicited_buffer_size = match BufferSize::new(config.unsolicited_buffer_size() as usize) {
        Ok(buffer_size) => buffer_size,
        Err(_) => return None,
    };

    let rx_buffer_size = match BufferSize::new(config.rx_buffer_size() as usize) {
        Ok(buffer_size) => buffer_size,
        Err(_) => return None,
    };

    let keep_alive_timeout = if config.keep_alive_timeout() == Duration::default() {
        None
    } else {
        Some(config.keep_alive_timeout())
    };

    Some(OutstationConfig {
        outstation_address,
        master_address,
        solicited_buffer_size,
        unsolicited_buffer_size,
        rx_buffer_size,
        bubble_framing_errors: config.bubble_framing_errors(),
        log_level: config.log_level().into(),
        confirm_timeout: config.confirm_timeout(),
        select_timeout: config.select_timeout(),
        features: config.features().into(),
        max_unsolicited_retries: Some(config.max_unsolicited_retries() as usize),
        unsolicited_retry_delay: config.unsolicited_retry_delay(),
        max_read_headers_per_request: config.max_read_headers_per_request(),
        keep_alive_timeout,
    })
}

impl From<&ffi::OutstationFeatures> for Features {
    fn from(from: &ffi::OutstationFeatures) -> Self {
        fn to_feature(value: bool) -> Feature {
            match value {
                true => Feature::Enabled,
                false => Feature::Disabled,
            }
        }

        Features {
            self_address: to_feature(from.self_address()),
            broadcast: to_feature(from.broadcast()),
            unsolicited: to_feature(from.unsolicited()),
        }
    }
}

impl From<ffi::DatabaseConfig> for DatabaseConfig {
    fn from(from: ffi::DatabaseConfig) -> Self {
        DatabaseConfig {
            max_read_request_headers: Some(from.max_read_request_headers()),
            class_zero: from.class_zero().into(),
            events: from.events().into(),
        }
    }
}

impl From<&ffi::ClassZeroConfig> for ClassZeroConfig {
    fn from(from: &ffi::ClassZeroConfig) -> Self {
        ClassZeroConfig {
            binary: from.binary(),
            double_bit_binary: from.double_bit_binary(),
            binary_output_status: from.binary_output_status(),
            counter: from.counter(),
            frozen_counter: from.frozen_counter(),
            analog: from.analog(),
            analog_output_status: from.analog_output_status(),
            octet_strings: from.octet_strings(),
        }
    }
}

impl From<&ffi::EventBufferConfig> for EventBufferConfig {
    fn from(from: &ffi::EventBufferConfig) -> Self {
        EventBufferConfig {
            max_binary: from.max_binary(),
            max_double_binary: from.max_double_bit_binary(),
            max_binary_output_status: from.max_binary_output_status(),
            max_counter: from.max_counter(),
            max_frozen_counter: from.max_frozen_counter(),
            max_analog: from.max_analog(),
            max_analog_output_status: from.max_analog_output_status(),
            max_octet_string: from.max_octet_string(),
        }
    }
}
