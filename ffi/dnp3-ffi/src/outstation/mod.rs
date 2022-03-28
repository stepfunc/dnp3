use std::ffi::CStr;
use std::net::AddrParseError;
use std::path::Path;
use std::time::Duration;

pub use database::*;
use dnp3::app::{BufferSize, BufferSizeError, Listener, MaybeAsync, Timeout};
use dnp3::link::{EndpointAddress, LinkErrorMode};
use dnp3::outstation::database::{ClassZeroConfig, EventBufferConfig};
use dnp3::outstation::{ConnectionState, Feature, Features, OutstationConfig, OutstationHandle};
use dnp3::tcp::tls::TlsServerConfig;
use dnp3::tcp::{FilterError, ServerHandle};
pub use struct_constructors::*;

use crate::{ffi, Runtime, RuntimeHandle};

use dnp3::serial::create_outstation_serial;

mod adapters;
mod database;
mod struct_constructors;

enum OutstationServerState {
    Configuring(dnp3::tcp::Server),
    Running(ServerHandle),
}

pub struct OutstationServer {
    runtime: RuntimeHandle,
    state: OutstationServerState,
}

pub struct Outstation {
    handle: OutstationHandle,
    runtime: RuntimeHandle,
}

pub unsafe fn outstation_server_create_tcp_server(
    runtime: *mut Runtime,
    link_error_mode: ffi::LinkErrorMode,
    address: &CStr,
) -> Result<*mut OutstationServer, ffi::ParamError> {
    let runtime = runtime.as_ref().ok_or(ffi::ParamError::NullParameter)?;
    let address = address.to_string_lossy().parse()?;

    let server = dnp3::tcp::Server::new_tcp_server(link_error_mode.into(), address);

    Ok(Box::into_raw(Box::new(OutstationServer {
        runtime: runtime.handle(),
        state: OutstationServerState::Configuring(server),
    })))
}

pub unsafe fn outstation_server_destroy(server: *mut OutstationServer) {
    if !server.is_null() {
        Box::from_raw(server);
    }
}

pub unsafe fn outstation_server_create_tls_server(
    runtime: *mut Runtime,
    link_error_mode: ffi::LinkErrorMode,
    address: &CStr,
    tls_config: ffi::TlsServerConfig,
) -> Result<*mut OutstationServer, ffi::ParamError> {
    let runtime = runtime.as_ref().ok_or(ffi::ParamError::NullParameter)?;
    let address = address.to_string_lossy().parse()?;

    let password = tls_config.password().to_string_lossy();
    let optional_password = match password.as_ref() {
        "" => None,
        password => Some(password),
    };

    let tls_config = TlsServerConfig::new(
        &tls_config.dns_name().to_string_lossy(),
        Path::new(tls_config.peer_cert_path().to_string_lossy().as_ref()),
        Path::new(tls_config.local_cert_path().to_string_lossy().as_ref()),
        Path::new(tls_config.private_key_path().to_string_lossy().as_ref()),
        optional_password,
        tls_config.min_tls_version().into(),
        tls_config.certificate_mode().into(),
    )
    .map_err(|err| {
        tracing::error!("TLS error: {}", err);
        err
    })?;

    let server = dnp3::tcp::Server::new_tls_server(link_error_mode.into(), address, tls_config);

    Ok(Box::into_raw(Box::new(OutstationServer {
        runtime: runtime.handle(),
        state: OutstationServerState::Configuring(server),
    })))
}

#[allow(clippy::too_many_arguments)]
pub unsafe fn outstation_server_add_outstation(
    server: *mut OutstationServer,
    config: ffi::OutstationConfig,
    application: ffi::OutstationApplication,
    information: ffi::OutstationInformation,
    control_handler: ffi::ControlHandler,
    listener: ffi::ConnectionStateListener,
    filter: *mut AddressFilter,
) -> Result<*mut Outstation, ffi::ParamError> {
    let server = server.as_mut().ok_or(ffi::ParamError::NullParameter)?;

    let server_handle = match &mut server.state {
        OutstationServerState::Configuring(server) => server,
        OutstationServerState::Running(_) => return Err(ffi::ParamError::ServerAlreadyStarted),
    };

    let config = convert_outstation_config(config)?;
    let filter = filter
        .as_ref()
        .ok_or(ffi::ParamError::NullParameter)?
        .into();

    let (outstation, task) = server_handle.add_outstation_no_spawn(
        config,
        Box::new(application),
        Box::new(information),
        Box::new(control_handler),
        Box::new(listener),
        filter,
    )?;

    server.runtime.spawn(task)?;

    Ok(Box::into_raw(Box::new(Outstation {
        handle: outstation,
        runtime: server.runtime.clone(),
    })))
}

pub unsafe fn outstation_server_bind(server: *mut OutstationServer) -> Result<(), ffi::ParamError> {
    if server.is_null() {
        return Err(ffi::ParamError::NullParameter);
    }
    let mut server = Box::from_raw(server);

    let server_handle = match server.state {
        OutstationServerState::Configuring(server) => server,
        OutstationServerState::Running(_) => return Err(ffi::ParamError::ServerAlreadyStarted),
    };

    let (handle, task) = server.runtime.block_on(server_handle.bind_no_spawn())??;

    server.runtime.spawn(task)?;
    server.state = OutstationServerState::Running(handle);
    Box::leak(server);
    Ok(())
}

pub unsafe fn outstation_destroy(outstation: *mut Outstation) {
    if !outstation.is_null() {
        Box::from_raw(outstation);
    }
}

#[allow(clippy::too_many_arguments)] // TODO
pub unsafe fn outstation_create_serial_session(
    runtime: *mut crate::Runtime,
    serial_path: &CStr,
    settings: ffi::SerialSettings,
    config: ffi::OutstationConfig,
    application: ffi::OutstationApplication,
    information: ffi::OutstationInformation,
    control_handler: ffi::ControlHandler,
) -> Result<*mut crate::Outstation, ffi::ParamError> {
    let runtime = runtime
        .as_ref()
        .ok_or(ffi::ParamError::NullParameter)?
        .handle();
    let serial_path = serial_path.to_string_lossy();

    let config = convert_outstation_config(config)?;

    let (task, handle) = create_outstation_serial(
        &serial_path,
        settings.into(),
        config,
        Box::new(application),
        Box::new(information),
        Box::new(control_handler),
    )?;

    runtime.spawn(task)?;

    let handle = Box::new(crate::Outstation { handle, runtime });

    Ok(Box::into_raw(handle))
}

pub unsafe fn outstation_transaction(
    outstation: *mut Outstation,
    callback: ffi::DatabaseTransaction,
) {
    if let Some(outstation) = outstation.as_mut() {
        outstation.handle.transaction(|database| {
            callback.execute(database as *mut _);
        });
    }
}

pub unsafe fn outstation_set_decode_level(
    outstation: *mut Outstation,
    level: ffi::DecodeLevel,
) -> Result<(), ffi::ParamError> {
    let outstation = outstation.as_mut().ok_or(ffi::ParamError::NullParameter)?;
    outstation
        .runtime
        .block_on(outstation.handle.set_decode_level(level.into()))??;
    Ok(())
}

fn convert_outstation_config(
    config: ffi::OutstationConfig,
) -> Result<OutstationConfig, ffi::ParamError> {
    let outstation_address = EndpointAddress::try_new(config.outstation_address())?;
    let master_address = EndpointAddress::try_new(config.master_address())?;
    let event_buffer_config = config.event_buffer_config().into();
    let solicited_buffer_size = BufferSize::new(config.solicited_buffer_size() as usize)?;
    let unsolicited_buffer_size = BufferSize::new(config.unsolicited_buffer_size() as usize)?;
    let rx_buffer_size = BufferSize::new(config.rx_buffer_size() as usize)?;

    let keep_alive_timeout = if config.keep_alive_timeout() == Duration::default() {
        None
    } else {
        Some(config.keep_alive_timeout())
    };

    Ok(OutstationConfig {
        outstation_address,
        master_address,
        event_buffer_config,
        solicited_buffer_size,
        unsolicited_buffer_size,
        rx_buffer_size,
        decode_level: config.decode_level().clone().into(),
        confirm_timeout: Timeout::from_duration(config.confirm_timeout())?,
        select_timeout: Timeout::from_duration(config.select_timeout())?,
        features: config.features().into(),
        max_unsolicited_retries: Some(config.max_unsolicited_retries() as usize),
        unsolicited_retry_delay: config.unsolicited_retry_delay(),
        keep_alive_timeout,
        class_zero: config.class_zero.into(),
        max_read_request_headers: Some(config.max_read_request_headers),
        max_controls_per_request: Some(config.max_controls_per_request),
    })
}

impl Listener<ConnectionState> for ffi::ConnectionStateListener {
    fn update(&mut self, value: ConnectionState) -> MaybeAsync<()> {
        self.on_change(value.into());
        MaybeAsync::ready(())
    }
}

impl From<ConnectionState> for ffi::ConnectionState {
    fn from(x: ConnectionState) -> Self {
        match x {
            ConnectionState::Connected => ffi::ConnectionState::Connected,
            ConnectionState::Disconnected => ffi::ConnectionState::Disconnected,
        }
    }
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

impl From<ffi::ClassZeroConfig> for ClassZeroConfig {
    fn from(from: ffi::ClassZeroConfig) -> Self {
        ClassZeroConfig {
            binary: from.binary(),
            double_bit_binary: from.double_bit_binary(),
            binary_output_status: from.binary_output_status(),
            counter: from.counter(),
            frozen_counter: from.frozen_counter(),
            analog: from.analog(),
            analog_output_status: from.analog_output_status(),
            octet_string: from.octet_string(),
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

impl From<ffi::LinkErrorMode> for LinkErrorMode {
    fn from(from: ffi::LinkErrorMode) -> Self {
        match from {
            ffi::LinkErrorMode::Close => LinkErrorMode::Close,
            ffi::LinkErrorMode::Discard => LinkErrorMode::Discard,
        }
    }
}

impl From<AddrParseError> for ffi::ParamError {
    fn from(_: AddrParseError) -> Self {
        ffi::ParamError::InvalidSocketAddress
    }
}

impl From<BufferSizeError> for ffi::ParamError {
    fn from(_: BufferSizeError) -> Self {
        ffi::ParamError::InvalidBufferSize
    }
}

impl From<FilterError> for ffi::ParamError {
    fn from(_: FilterError) -> Self {
        ffi::ParamError::AddressFilterConflict
    }
}

impl From<std::io::Error> for ffi::ParamError {
    fn from(error: std::io::Error) -> Self {
        tracing::error!("IO error: {}", error);
        ffi::ParamError::ServerBindError
    }
}
