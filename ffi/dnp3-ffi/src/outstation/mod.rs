use std::ffi::CStr;
use std::net::AddrParseError;
use std::time::Duration;

pub use database::*;
use dnp3::app::{BufferSize, BufferSizeError, Listener, MaybeAsync, Timeout};
use dnp3::link::{EndpointAddress, LinkErrorMode, LinkReadMode};
use dnp3::outstation::database::{ClassZeroConfig, EventBufferConfig};
use dnp3::outstation::{ConnectionState, Feature, Features, OutstationConfig, OutstationHandle};
use dnp3::tcp::{FilterError, ServerHandle};
pub use struct_constructors::*;

use crate::{ffi, Runtime, RuntimeHandle};

#[cfg(feature = "tls")]
use dnp3::tcp::tls::TlsServerConfig;
use dnp3::udp::{OutstationUdpConfig, UdpSocketMode};

mod adapters;
mod database;
mod struct_constructors;

enum OutstationServerState {
    Configuring(dnp3::tcp::Server),
    #[allow(dead_code)]
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
        drop(Box::from_raw(server));
    }
}

#[cfg(not(feature = "tls"))]
pub unsafe fn outstation_server_create_tls_server(
    _runtime: *mut Runtime,
    _link_error_mode: ffi::LinkErrorMode,
    _address: &CStr,
    _tls_config: ffi::TlsServerConfig,
) -> Result<*mut OutstationServer, ffi::ParamError> {
    Err(ffi::ParamError::NoSupport)
}

#[cfg(feature = "tls")]
pub unsafe fn outstation_server_create_tls_server(
    runtime: *mut Runtime,
    link_error_mode: ffi::LinkErrorMode,
    address: &CStr,
    tls_config: ffi::TlsServerConfig,
) -> Result<*mut OutstationServer, ffi::ParamError> {
    use std::path::Path;

    let runtime = runtime.as_ref().ok_or(ffi::ParamError::NullParameter)?;
    let address = address.to_str()?.parse()?;

    let password = match tls_config.password().to_str()? {
        "" => None,
        password => Some(password),
    };

    let peer_cert_path = Path::new(tls_config.peer_cert_path().to_str()?);
    let local_cert_path = Path::new(tls_config.local_cert_path().to_str()?);
    let private_key_path = Path::new(tls_config.private_key_path().to_str()?);
    let min_tls_version: dnp3::tcp::tls::MinTlsVersion = tls_config.min_tls_version().into();

    let tls_config = match tls_config.certificate_mode() {
        ffi::CertificateMode::SelfSigned => TlsServerConfig::self_signed(
            peer_cert_path,
            local_cert_path,
            private_key_path,
            password,
            min_tls_version,
        ),
        ffi::CertificateMode::AuthorityBased => {
            let expected_subject_name = tls_config.dns_name().to_str()?;

            let expected_subject_name =
                if tls_config.allow_client_name_wildcard && expected_subject_name == "*" {
                    None
                } else {
                    Some(expected_subject_name.to_string())
                };

            TlsServerConfig::full_pki(
                expected_subject_name,
                peer_cert_path,
                local_cert_path,
                private_key_path,
                password,
                min_tls_version,
            )
        }
    }
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
    // TODO - this pattern doesn't look correct
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
        drop(Box::from_raw(outstation));
    }
}

#[allow(clippy::too_many_arguments)] // TODO
pub(crate) unsafe fn outstation_create_tcp_client(
    runtime: *mut crate::Runtime,
    link_error_mode: ffi::LinkErrorMode,
    endpoints: *mut crate::EndpointList,
    connect_strategy: ffi::ConnectStrategy,
    connect_options: *mut crate::ConnectOptions,
    config: ffi::OutstationConfig,
    application: ffi::OutstationApplication,
    information: ffi::OutstationInformation,
    control_handler: ffi::ControlHandler,
    listener: ffi::ClientStateListener,
) -> Result<*mut crate::Outstation, ffi::ParamError> {
    let runtime = runtime.as_ref().ok_or(ffi::ParamError::NullParameter)?;
    let endpoints = endpoints.as_ref().ok_or(ffi::ParamError::NullParameter)?;
    let connect_options = connect_options
        .as_ref()
        .map(|x| x.inner)
        .unwrap_or_else(Default::default);

    let config = convert_outstation_config(config)?;

    let _enter = runtime.enter();
    let handle = dnp3::tcp::spawn_outstation_tcp_client(
        link_error_mode.into(),
        endpoints.clone(),
        connect_strategy.into(),
        connect_options,
        config,
        Box::new(application),
        Box::new(information),
        Box::new(control_handler),
        Box::new(listener),
    );

    let handle = Box::new(crate::Outstation {
        handle,
        runtime: runtime.handle(),
    });

    Ok(Box::into_raw(handle))
}

#[cfg(not(feature = "tls"))]
#[allow(clippy::too_many_arguments)] // TODO
pub(crate) unsafe fn outstation_create_tls_client(
    _runtime: *mut crate::Runtime,
    _link_error_mode: ffi::LinkErrorMode,
    _endpoints: *mut crate::EndpointList,
    _connect_strategy: ffi::ConnectStrategy,
    _connect_options: *mut crate::ConnectOptions,
    _config: ffi::OutstationConfig,
    _application: ffi::OutstationApplication,
    _information: ffi::OutstationInformation,
    _control_handler: ffi::ControlHandler,
    _listener: ffi::ClientStateListener,
    _tls_config: ffi::TlsClientConfig,
) -> Result<*mut crate::Outstation, ffi::ParamError> {
    Err(ffi::ParamError::NoSupport)
}

#[cfg(feature = "tls")]
#[allow(clippy::too_many_arguments)] // TODO
pub(crate) unsafe fn outstation_create_tls_client(
    runtime: *mut crate::Runtime,
    link_error_mode: ffi::LinkErrorMode,
    endpoints: *mut crate::EndpointList,
    connect_strategy: ffi::ConnectStrategy,
    connect_options: *mut crate::ConnectOptions,
    config: ffi::OutstationConfig,
    application: ffi::OutstationApplication,
    information: ffi::OutstationInformation,
    control_handler: ffi::ControlHandler,
    listener: ffi::ClientStateListener,
    tls_config: ffi::TlsClientConfig,
) -> Result<*mut crate::Outstation, ffi::ParamError> {
    let runtime = runtime.as_ref().ok_or(ffi::ParamError::NullParameter)?;
    let endpoints = endpoints.as_ref().ok_or(ffi::ParamError::NullParameter)?;
    let connect_options = connect_options
        .as_ref()
        .map(|x| x.inner)
        .unwrap_or_else(Default::default);

    let config = convert_outstation_config(config)?;
    let tls_config = tls_config.try_into()?;

    let _enter = runtime.enter();
    let handle = dnp3::tcp::tls::spawn_outstation_tls_client(
        link_error_mode.into(),
        endpoints.clone(),
        connect_strategy.into(),
        connect_options,
        config,
        Box::new(application),
        Box::new(information),
        Box::new(control_handler),
        Box::new(listener),
        tls_config,
    );

    let handle = Box::new(crate::Outstation {
        handle,
        runtime: runtime.handle(),
    });

    Ok(Box::into_raw(handle))
}

#[cfg(not(feature = "serial"))]
#[allow(clippy::too_many_arguments)] // TODO
pub unsafe fn outstation_create_serial_session(
    _runtime: *mut crate::Runtime,
    _serial_path: &CStr,
    _settings: ffi::SerialSettings,
    _config: ffi::OutstationConfig,
    _application: ffi::OutstationApplication,
    _information: ffi::OutstationInformation,
    _control_handler: ffi::ControlHandler,
) -> Result<*mut crate::Outstation, ffi::ParamError> {
    Err(ffi::ParamError::NoSupport)
}

#[cfg(feature = "serial")]
pub unsafe fn outstation_create_serial_session(
    runtime: *mut crate::Runtime,
    serial_path: &CStr,
    settings: ffi::SerialSettings,
    config: ffi::OutstationConfig,
    application: ffi::OutstationApplication,
    information: ffi::OutstationInformation,
    control_handler: ffi::ControlHandler,
) -> Result<*mut crate::Outstation, ffi::ParamError> {
    let runtime = runtime.as_ref().ok_or(ffi::ParamError::NullParameter)?;
    let serial_path = serial_path.to_string_lossy();
    let config = convert_outstation_config(config)?;

    let _enter = runtime.enter();
    let handle = dnp3::serial::spawn_outstation_serial(
        &serial_path,
        settings.into(),
        config,
        Box::new(application),
        Box::new(information),
        Box::new(control_handler),
    )?;

    let handle = Box::new(crate::Outstation {
        handle,
        runtime: runtime.handle(),
    });

    Ok(Box::into_raw(handle))
}

/// This variant is just implemented in terms of another so we don't need feature checking
#[allow(clippy::too_many_arguments)] // TODO
pub(crate) unsafe fn outstation_create_serial_session_fault_tolerant(
    runtime: *mut crate::Runtime,
    serial_path: &CStr,
    settings: ffi::SerialSettings,
    open_retry_delay: std::time::Duration,
    config: ffi::OutstationConfig,
    application: ffi::OutstationApplication,
    information: ffi::OutstationInformation,
    control_handler: ffi::ControlHandler,
) -> Result<*mut crate::Outstation, ffi::ParamError> {
    let port_listener = ffi::PortStateListener {
        on_change: None,
        on_destroy: None,
        ctx: std::ptr::null_mut(),
    };

    outstation_create_serial_session_2(
        runtime,
        serial_path,
        settings,
        open_retry_delay,
        config,
        application,
        information,
        control_handler,
        port_listener,
    )
}

#[cfg(not(feature = "serial"))]
#[allow(clippy::too_many_arguments)]
pub(crate) unsafe fn outstation_create_serial_session_2(
    _runtime: *mut crate::Runtime,
    _serial_path: &CStr,
    _settings: ffi::SerialSettings,
    _open_retry_delay: std::time::Duration,
    _config: ffi::OutstationConfig,
    _application: ffi::OutstationApplication,
    _information: ffi::OutstationInformation,
    _control_handler: ffi::ControlHandler,
    _port_listener: ffi::PortStateListener,
) -> Result<*mut crate::Outstation, ffi::ParamError> {
    Err(ffi::ParamError::NoSupport)
}

#[cfg(feature = "serial")]
#[allow(clippy::too_many_arguments)]
pub(crate) unsafe fn outstation_create_serial_session_2(
    runtime: *mut crate::Runtime,
    serial_path: &CStr,
    settings: ffi::SerialSettings,
    open_retry_delay: std::time::Duration,
    config: ffi::OutstationConfig,
    application: ffi::OutstationApplication,
    information: ffi::OutstationInformation,
    control_handler: ffi::ControlHandler,
    port_listener: ffi::PortStateListener,
) -> Result<*mut crate::Outstation, ffi::ParamError> {
    let runtime = runtime.as_ref().ok_or(ffi::ParamError::NullParameter)?;
    let serial_path = serial_path.to_string_lossy();
    let config = convert_outstation_config(config)?;

    let _enter = runtime.enter();
    let handle = dnp3::serial::spawn_outstation_serial_2(
        &serial_path,
        settings.into(),
        config,
        dnp3::app::RetryStrategy::new(open_retry_delay, open_retry_delay),
        Box::new(application),
        Box::new(information),
        Box::new(control_handler),
        Box::new(port_listener),
    );

    let handle = Box::new(crate::Outstation {
        handle,
        runtime: runtime.handle(),
    });

    Ok(Box::into_raw(handle))
}

pub(crate) unsafe fn outstation_create_udp(
    runtime: *mut crate::Runtime,
    udp_config: ffi::OutstationUdpConfig,
    config: ffi::OutstationConfig,
    application: ffi::OutstationApplication,
    information: ffi::OutstationInformation,
    control_handler: ffi::ControlHandler,
) -> Result<*mut crate::Outstation, ffi::ParamError> {
    let runtime = runtime.as_ref().ok_or(ffi::ParamError::NullParameter)?;
    let config = convert_outstation_config(config)?;
    let udp_config = convert_udp_config(udp_config)?;

    let _enter = runtime.enter();
    let handle = dnp3::udp::spawn_outstation_udp(
        udp_config,
        config,
        Box::new(application),
        Box::new(information),
        Box::new(control_handler),
    );

    let handle = Box::new(crate::Outstation {
        handle,
        runtime: runtime.handle(),
    });

    Ok(Box::into_raw(handle))
}

pub(crate) unsafe fn outstation_transaction(
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

pub unsafe fn outstation_enable(outstation: *mut crate::Outstation) -> Result<(), ffi::ParamError> {
    let outstation = outstation.as_mut().ok_or(ffi::ParamError::NullParameter)?;
    outstation.runtime.block_on(outstation.handle.enable())??;
    Ok(())
}

pub unsafe fn outstation_disable(
    outstation: *mut crate::Outstation,
) -> Result<(), ffi::ParamError> {
    let outstation = outstation.as_mut().ok_or(ffi::ParamError::NullParameter)?;
    outstation.runtime.block_on(outstation.handle.disable())??;
    Ok(())
}

unsafe fn convert_udp_config(
    config: ffi::OutstationUdpConfig,
) -> Result<OutstationUdpConfig, ffi::ParamError> {
    Ok(OutstationUdpConfig {
        local_endpoint: CStr::from_ptr(config.local_endpoint).to_str()?.parse()?,
        remote_endpoint: CStr::from_ptr(config.remote_endpoint).to_str()?.parse()?,
        socket_mode: config.socket_mode().into(),
        link_read_mode: config.link_read_mode().into(),
        retry_delay: Timeout::from_millis(config.retry_delay)?,
    })
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

impl From<ffi::UdpSocketMode> for UdpSocketMode {
    fn from(value: ffi::UdpSocketMode) -> Self {
        match value {
            ffi::UdpSocketMode::OneToOne => UdpSocketMode::OneToOne,
            ffi::UdpSocketMode::OneToMany => UdpSocketMode::OneToMany,
        }
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
            respond_to_any_master: to_feature(from.respond_to_any_master()),
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

impl From<ffi::LinkReadMode> for LinkReadMode {
    fn from(from: ffi::LinkReadMode) -> Self {
        match from {
            ffi::LinkReadMode::Datagram => LinkReadMode::Datagram,
            ffi::LinkReadMode::Stream => LinkReadMode::Stream,
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
