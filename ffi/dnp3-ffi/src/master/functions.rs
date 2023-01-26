use std::ffi::CStr;
use std::ptr::{null, null_mut};
use std::time::Duration;

use dnp3::app::{
    BufferSize, ConnectStrategy, Listener, MaybeAsync, RetryStrategy, Timeout, TimeoutRangeError,
    Timestamp,
};
use dnp3::link::{EndpointAddress, SpecialAddressError};
use dnp3::master::*;
use dnp3::tcp::ClientState;

#[cfg(feature = "serial")]
use dnp3::serial::*;
#[cfg(feature = "tls")]
use dnp3::tcp::tls::*;

use crate::ffi;

pub struct MasterChannel {
    pub(crate) runtime: crate::runtime::RuntimeHandle,
    pub(crate) handle: dnp3::master::MasterChannel,
}

pub(crate) unsafe fn master_channel_create_tcp(
    runtime: *mut crate::runtime::Runtime,
    link_error_mode: ffi::LinkErrorMode,
    config: ffi::MasterChannelConfig,
    endpoints: *const crate::EndpointList,
    connect_strategy: ffi::ConnectStrategy,
    listener: ffi::ClientStateListener,
) -> Result<*mut MasterChannel, ffi::ParamError> {
    master_channel_create_tcp_2(
        runtime,
        link_error_mode,
        config,
        endpoints,
        connect_strategy,
        null_mut(),
        listener,
    )
}

pub(crate) unsafe fn master_channel_create_tcp_2(
    runtime: *mut crate::runtime::Runtime,
    link_error_mode: ffi::LinkErrorMode,
    config: ffi::MasterChannelConfig,
    endpoints: *const crate::EndpointList,
    connect_strategy: ffi::ConnectStrategy,
    options: *mut crate::ConnectOptions,
    listener: ffi::ClientStateListener,
) -> Result<*mut MasterChannel, ffi::ParamError> {
    let runtime = runtime.as_ref().ok_or(ffi::ParamError::NullParameter)?;
    let config = convert_config(config)?;
    let endpoints = endpoints.as_ref().ok_or(ffi::ParamError::NullParameter)?;
    let connect_options = options
        .as_ref()
        .map(|x| x.inner)
        .unwrap_or_else(Default::default);

    // enter the runtime context so that we can spawn
    let _enter = runtime.enter();

    let channel = dnp3::tcp::spawn_master_tcp_client_2(
        link_error_mode.into(),
        config,
        endpoints.clone(),
        connect_strategy.into(),
        connect_options,
        Box::new(listener),
    );

    let channel = MasterChannel {
        runtime: runtime.handle(),
        handle: channel,
    };

    Ok(Box::into_raw(Box::new(channel)))
}

pub(crate) unsafe fn master_channel_create_tls(
    runtime: *mut crate::runtime::Runtime,
    link_error_mode: ffi::LinkErrorMode,
    config: ffi::MasterChannelConfig,
    endpoints: *const crate::EndpointList,
    connect_strategy: ffi::ConnectStrategy,
    listener: ffi::ClientStateListener,
    tls_config: ffi::TlsClientConfig,
) -> Result<*mut MasterChannel, ffi::ParamError> {
    master_channel_create_tls_2(
        runtime,
        link_error_mode,
        config,
        endpoints,
        connect_strategy,
        null(),
        listener,
        tls_config,
    )
}

#[cfg(not(feature = "tls"))]
#[allow(clippy::too_many_arguments)]
pub(crate) unsafe fn master_channel_create_tls_2(
    _runtime: *mut crate::runtime::Runtime,
    _link_error_mode: ffi::LinkErrorMode,
    _config: ffi::MasterChannelConfig,
    _endpoints: *const crate::EndpointList,
    _connect_strategy: ffi::ConnectStrategy,
    _connect_options: *const crate::ConnectOptions,
    _listener: ffi::ClientStateListener,
    _tls_config: ffi::TlsClientConfig,
) -> Result<*mut MasterChannel, ffi::ParamError> {
    Err(ffi::ParamError::NoSupport)
}

#[cfg(feature = "tls")]
#[allow(clippy::too_many_arguments)]
pub(crate) unsafe fn master_channel_create_tls_2(
    runtime: *mut crate::runtime::Runtime,
    link_error_mode: ffi::LinkErrorMode,
    config: ffi::MasterChannelConfig,
    endpoints: *const crate::EndpointList,
    connect_strategy: ffi::ConnectStrategy,
    connect_options: *const crate::ConnectOptions,
    listener: ffi::ClientStateListener,
    tls_config: ffi::TlsClientConfig,
) -> Result<*mut MasterChannel, ffi::ParamError> {
    use std::path::Path;

    let runtime = runtime.as_ref().ok_or(ffi::ParamError::NullParameter)?;
    let config = convert_config(config)?;
    let endpoints = endpoints.as_ref().ok_or(ffi::ParamError::NullParameter)?;
    let connect_options = connect_options
        .as_ref()
        .map(|x| x.inner)
        .unwrap_or_else(Default::default);

    let password = tls_config.password().to_string_lossy();
    let optional_password = match password.as_ref() {
        "" => None,
        password => Some(password),
    };

    let tls_config = TlsClientConfig::new(
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

    // enter the runtime context so that we can spawn
    let _enter = runtime.enter();

    let channel = dnp3::tcp::tls::spawn_master_tls_client_2(
        link_error_mode.into(),
        config,
        endpoints.clone(),
        connect_strategy.into(),
        connect_options,
        Box::new(listener),
        tls_config,
    );

    let channel = MasterChannel {
        runtime: runtime.handle(),
        handle: channel,
    };

    Ok(Box::into_raw(Box::new(channel)))
}

#[cfg(not(feature = "serial"))]
pub(crate) unsafe fn master_channel_create_serial(
    _runtime: *mut crate::runtime::Runtime,
    _config: ffi::MasterChannelConfig,
    _path: &CStr,
    _serial_params: ffi::SerialSettings,
    _retry_delay: Duration,
    _listener: ffi::PortStateListener,
) -> Result<*mut MasterChannel, ffi::ParamError> {
    Err(ffi::ParamError::NoSupport)
}

#[cfg(feature = "serial")]
pub(crate) unsafe fn master_channel_create_serial(
    runtime: *mut crate::runtime::Runtime,
    config: ffi::MasterChannelConfig,
    path: &CStr,
    serial_params: ffi::SerialSettings,
    retry_delay: Duration,
    listener: ffi::PortStateListener,
) -> Result<*mut MasterChannel, ffi::ParamError> {
    let runtime = runtime.as_ref().ok_or(ffi::ParamError::NullParameter)?;
    let config = convert_config(config)?;

    // enter the runtime context so that we can spawn
    let _enter = runtime.enter();

    let channel = spawn_master_serial(
        config,
        &path.to_string_lossy(),
        serial_params.into(),
        retry_delay,
        Box::new(listener),
    );

    let channel = MasterChannel {
        runtime: runtime.handle(),
        handle: channel,
    };

    Ok(Box::into_raw(Box::new(channel)))
}

pub(crate) unsafe fn master_channel_destroy(channel: *mut MasterChannel) {
    if !channel.is_null() {
        drop(Box::from_raw(channel));
    }
}

pub(crate) unsafe fn master_channel_enable(
    channel: *mut crate::MasterChannel,
) -> Result<(), ffi::ParamError> {
    let channel = channel.as_mut().ok_or(ffi::ParamError::NullParameter)?;
    channel.runtime.block_on(channel.handle.enable())??;
    Ok(())
}

pub(crate) unsafe fn master_channel_disable(
    channel: *mut crate::MasterChannel,
) -> Result<(), ffi::ParamError> {
    let channel = channel.as_mut().ok_or(ffi::ParamError::NullParameter)?;
    channel.runtime.block_on(channel.handle.disable())??;
    Ok(())
}

pub(crate) unsafe fn master_channel_add_association(
    channel: *mut MasterChannel,
    address: u16,
    config: ffi::AssociationConfig,
    read_handler: ffi::ReadHandler,
    assoc_handler: ffi::AssociationHandler,
    assoc_info: ffi::AssociationInformation,
) -> Result<ffi::AssociationId, ffi::ParamError> {
    let channel = channel.as_mut().ok_or(ffi::ParamError::NullParameter)?;
    let address = EndpointAddress::try_new(address)?;

    let config = AssociationConfig {
        response_timeout: Timeout::from_duration(Duration::from_millis(config.response_timeout))?,
        disable_unsol_classes: convert_event_classes(config.disable_unsol_classes()),
        enable_unsol_classes: convert_event_classes(config.enable_unsol_classes()),
        startup_integrity_classes: convert_classes(config.startup_integrity_classes()),
        auto_time_sync: convert_auto_time_sync(&config.auto_time_sync()),
        auto_tasks_retry_strategy: RetryStrategy::new(
            config.auto_tasks_retry_strategy.min_delay(),
            config.auto_tasks_retry_strategy.max_delay(),
        ),
        keep_alive_timeout: if config.keep_alive_timeout() == Duration::from_secs(0) {
            None
        } else {
            Some(config.keep_alive_timeout())
        },
        auto_integrity_scan_on_buffer_overflow: config.auto_integrity_scan_on_buffer_overflow(),
        event_scan_on_events_available: convert_event_classes(
            config.event_scan_on_events_available(),
        ),
        max_queued_user_requests: config.max_queued_user_requests as usize,
    };

    channel.runtime.block_on(channel.handle.add_association(
        address,
        config,
        Box::new(read_handler),
        Box::new(assoc_handler),
        Box::new(assoc_info),
    ))??;
    Ok(ffi::AssociationId {
        address: address.raw_value(),
    })
}

impl From<TimeoutRangeError> for ffi::ParamError {
    fn from(_: TimeoutRangeError) -> Self {
        ffi::ParamError::InvalidTimeout
    }
}

pub(crate) unsafe fn master_channel_remove_association(
    channel: *mut crate::MasterChannel,
    id: ffi::AssociationId,
) -> Result<(), ffi::ParamError> {
    let channel = channel.as_mut().ok_or(ffi::ParamError::NullParameter)?;
    let endpoint = EndpointAddress::try_new(id.address)?;

    channel
        .runtime
        .block_on(channel.handle.remove_association(endpoint))??;

    Ok(())
}

pub(crate) unsafe fn master_channel_add_poll(
    channel: *mut MasterChannel,
    id: ffi::AssociationId,
    request: *mut crate::Request,
    period: std::time::Duration,
) -> Result<ffi::PollId, ffi::ParamError> {
    let channel = channel.as_mut().ok_or(ffi::ParamError::NullParameter)?;
    let address = EndpointAddress::try_new(id.address)?;
    let request = request.as_ref().ok_or(ffi::ParamError::NullParameter)?;

    let mut association = AssociationHandle::create(address, channel.handle.clone());
    let handle = channel
        .runtime
        .block_on(association.add_poll(request.build_read_request(), period))??;

    Ok(ffi::PollId {
        association_id: id.address,
        id: handle.get_id(),
    })
}

pub(crate) unsafe fn master_channel_remove_poll(
    channel: *mut crate::MasterChannel,
    poll: ffi::PollId,
) -> Result<(), ffi::ParamError> {
    let channel = channel.as_mut().ok_or(ffi::ParamError::NullParameter)?;
    let endpoint = EndpointAddress::try_new(poll.association_id)?;

    let poll = PollHandle::create(
        AssociationHandle::create(endpoint, channel.handle.clone()),
        poll.id,
    );

    channel.runtime.block_on(poll.remove())??;

    Ok(())
}

pub(crate) unsafe fn master_channel_demand_poll(
    channel: *mut crate::MasterChannel,
    poll: ffi::PollId,
) -> Result<(), ffi::ParamError> {
    let channel = channel.as_mut().ok_or(ffi::ParamError::NullParameter)?;
    let endpoint = EndpointAddress::try_new(poll.association_id)?;

    let mut poll = PollHandle::create(
        AssociationHandle::create(endpoint, channel.handle.clone()),
        poll.id,
    );

    channel.runtime.block_on(poll.demand())??;

    Ok(())
}

pub(crate) unsafe fn master_channel_read(
    channel: *mut crate::MasterChannel,
    association: ffi::AssociationId,
    request: *mut crate::Request,
    callback: ffi::ReadTaskCallback,
) -> Result<(), ffi::ParamError> {
    let channel = channel.as_mut().ok_or(ffi::ParamError::NullParameter)?;
    let address = EndpointAddress::try_new(association.address)?;
    let request = request
        .as_ref()
        .ok_or(ffi::ParamError::NullParameter)?
        .build_read_request();

    let promise = sfio_promise::wrap(callback);

    let mut handle = AssociationHandle::create(address, channel.handle.clone());

    let task = async move {
        let res = handle.read(request).await;
        promise.complete(res);
    };

    channel.runtime.spawn(task)?;
    Ok(())
}

pub(crate) unsafe fn master_channel_write_dead_bands(
    channel: *mut crate::MasterChannel,
    association: ffi::AssociationId,
    request: *mut crate::WriteDeadBandRequest,
    callback: ffi::EmptyResponseCallback,
) -> Result<(), ffi::ParamError> {
    let channel = channel.as_mut().ok_or(ffi::ParamError::NullParameter)?;
    let address = EndpointAddress::try_new(association.address)?;
    let request = request.as_mut().ok_or(ffi::ParamError::NullParameter)?;
    let promise = sfio_promise::wrap(callback);
    let headers = request.build();

    let mut handle = AssociationHandle::create(address, channel.handle.clone());

    let task = async move {
        let res = handle.write_dead_bands(headers).await;
        promise.complete(res);
    };

    channel.runtime.spawn(task)?;
    Ok(())
}

pub(crate) unsafe fn master_channel_send_and_expect_empty_response(
    channel: *mut crate::MasterChannel,
    association: ffi::AssociationId,
    function: ffi::FunctionCode,
    headers: *mut crate::Request,
    callback: ffi::EmptyResponseCallback,
) -> Result<(), ffi::ParamError> {
    let channel = channel.as_mut().ok_or(ffi::ParamError::NullParameter)?;
    let address = EndpointAddress::try_new(association.address)?;
    let function: dnp3::app::FunctionCode = function.into();
    let headers = headers
        .as_mut()
        .ok_or(ffi::ParamError::NullParameter)?
        .build_headers();
    let promise = sfio_promise::wrap(callback);

    let mut handle = AssociationHandle::create(address, channel.handle.clone());

    let task = async move {
        let res = handle
            .send_and_expect_empty_response(function, headers)
            .await;
        promise.complete(res);
    };

    channel.runtime.spawn(task)?;
    Ok(())
}

pub(crate) unsafe fn master_channel_read_with_handler(
    channel: *mut crate::MasterChannel,
    association: ffi::AssociationId,
    request: *mut crate::Request,
    handler: ffi::ReadHandler,
    callback: ffi::ReadTaskCallback,
) -> Result<(), ffi::ParamError> {
    let channel = channel.as_mut().ok_or(ffi::ParamError::NullParameter)?;
    let address = EndpointAddress::try_new(association.address)?;
    let request = request
        .as_ref()
        .ok_or(ffi::ParamError::NullParameter)?
        .build_read_request();

    let promise = sfio_promise::wrap(callback);

    let mut handle = AssociationHandle::create(address, channel.handle.clone());

    let task = async move {
        let res = handle.read_with_handler(request, Box::new(handler)).await;
        promise.complete(res);
    };

    channel.runtime.spawn(task)?;
    Ok(())
}

pub(crate) unsafe fn master_channel_operate(
    channel: *mut crate::MasterChannel,
    association: ffi::AssociationId,
    mode: ffi::CommandMode,
    commands: *mut crate::CommandSet,
    callback: ffi::CommandTaskCallback,
) -> Result<(), ffi::ParamError> {
    let channel = channel.as_mut().ok_or(ffi::ParamError::NullParameter)?;
    let address = EndpointAddress::try_new(association.address)?;
    let headers = commands
        .as_ref()
        .ok_or(ffi::ParamError::NullParameter)?
        .clone()
        .build();

    let promise = sfio_promise::wrap(callback);

    let mut handle = AssociationHandle::create(address, channel.handle.clone());

    let task = async move {
        let res = handle.operate(mode.into(), headers).await;
        promise.complete(res);
    };

    channel.runtime.spawn(task)?;
    Ok(())
}

impl From<CommandError> for ffi::CommandError {
    fn from(value: CommandError) -> Self {
        match value {
            CommandError::Task(err) => err.into(),
            CommandError::Response(err) => match err {
                CommandResponseError::Request(err) => err.into(),
                CommandResponseError::BadStatus(_) => Self::BadStatus,
                CommandResponseError::HeaderCountMismatch => Self::HeaderMismatch,
                CommandResponseError::HeaderTypeMismatch => Self::HeaderMismatch,
                CommandResponseError::ObjectCountMismatch => Self::HeaderMismatch,
                CommandResponseError::ObjectValueMismatch => Self::HeaderMismatch,
            },
        }
    }
}

impl From<TimeSyncError> for ffi::TimeSyncError {
    fn from(err: TimeSyncError) -> Self {
        match err {
            TimeSyncError::Task(err) => err.into(),
            TimeSyncError::ClockRollback => Self::ClockRollback,
            TimeSyncError::SystemTimeNotUnix => Self::SystemTimeNotUnix,
            TimeSyncError::BadOutstationTimeDelay(_) => Self::BadOutstationTimeDelay,
            TimeSyncError::Overflow => Self::Overflow,
            TimeSyncError::StillNeedsTime => Self::StillNeedsTime,
            TimeSyncError::SystemTimeNotAvailable => Self::SystemTimeNotAvailable,
            TimeSyncError::IinError(_) => Self::IinError,
        }
    }
}

pub(crate) unsafe fn master_channel_synchronize_time(
    channel: *mut crate::MasterChannel,
    association: ffi::AssociationId,
    mode: ffi::TimeSyncMode,
    callback: ffi::TimeSyncTaskCallback,
) -> Result<(), ffi::ParamError> {
    let channel = channel.as_mut().ok_or(ffi::ParamError::NullParameter)?;
    let address = EndpointAddress::try_new(association.address)?;

    let mut association = AssociationHandle::create(address, channel.handle.clone());
    let promise = sfio_promise::wrap(callback);

    let task = async move {
        let res = association.synchronize_time(mode.into()).await;
        promise.complete(res);
    };

    channel.runtime.spawn(task)?;
    Ok(())
}

pub(crate) unsafe fn master_channel_cold_restart(
    channel: *mut crate::MasterChannel,
    association: ffi::AssociationId,
    callback: ffi::RestartTaskCallback,
) -> Result<(), ffi::ParamError> {
    let channel = channel.as_mut().ok_or(ffi::ParamError::NullParameter)?;
    let address = EndpointAddress::try_new(association.address)?;

    let mut association = AssociationHandle::create(address, channel.handle.clone());
    let promise = sfio_promise::wrap(callback);

    let task = async move {
        let res = association.cold_restart().await;
        promise.complete(res);
    };

    channel.runtime.spawn(task)?;
    Ok(())
}

pub(crate) unsafe fn master_channel_warm_restart(
    channel: *mut crate::MasterChannel,
    association: ffi::AssociationId,
    callback: ffi::RestartTaskCallback,
) -> Result<(), ffi::ParamError> {
    let channel = channel.as_mut().ok_or(ffi::ParamError::NullParameter)?;
    let address = EndpointAddress::try_new(association.address)?;

    let mut association = AssociationHandle::create(address, channel.handle.clone());
    let promise = sfio_promise::wrap(callback);

    let task = async move {
        let res = association.warm_restart().await;
        promise.complete(res);
    };

    channel.runtime.spawn(task)?;
    Ok(())
}

pub(crate) unsafe fn master_channel_check_link_status(
    channel: *mut crate::MasterChannel,
    association: ffi::AssociationId,
    callback: ffi::LinkStatusCallback,
) -> Result<(), ffi::ParamError> {
    let channel = channel.as_mut().ok_or(ffi::ParamError::NullParameter)?;
    let address = EndpointAddress::try_new(association.address)?;

    let mut association = AssociationHandle::create(address, channel.handle.clone());
    let promise = sfio_promise::wrap(callback);

    let task = async move {
        let res = association.check_link_status().await;
        promise.complete(res);
    };

    channel.runtime.spawn(task)?;
    Ok(())
}

pub(crate) unsafe fn master_channel_set_decode_level(
    channel: *mut MasterChannel,
    level: ffi::DecodeLevel,
) -> Result<(), ffi::ParamError> {
    let channel = channel.as_mut().ok_or(ffi::ParamError::NullParameter)?;
    channel
        .runtime
        .spawn(channel.handle.set_decode_level(level.into()))?;
    Ok(())
}

pub(crate) unsafe fn master_channel_get_decode_level(
    channel: *mut MasterChannel,
) -> Result<ffi::DecodeLevel, ffi::ParamError> {
    let channel = channel.as_mut().ok_or(ffi::ParamError::NullParameter)?;

    let result = channel
        .runtime
        .block_on(channel.handle.get_decode_level())??;

    Ok(result.into())
}

fn convert_event_classes(config: &ffi::EventClasses) -> EventClasses {
    EventClasses::new(config.class1, config.class2, config.class3)
}

fn convert_classes(config: &ffi::Classes) -> Classes {
    Classes::new(
        config.class0,
        EventClasses::new(config.class1, config.class2, config.class3),
    )
}

fn convert_auto_time_sync(config: &ffi::AutoTimeSync) -> Option<TimeSyncProcedure> {
    match config {
        ffi::AutoTimeSync::None => None,
        ffi::AutoTimeSync::Lan => Some(TimeSyncProcedure::Lan),
        ffi::AutoTimeSync::NonLan => Some(TimeSyncProcedure::NonLan),
    }
}

impl From<ffi::UtcTimestamp> for Option<Timestamp> {
    fn from(from: ffi::UtcTimestamp) -> Self {
        if from.is_valid {
            Some(Timestamp::new(from.value))
        } else {
            None
        }
    }
}

impl Listener<ClientState> for ffi::ClientStateListener {
    fn update(&mut self, value: ClientState) -> MaybeAsync<()> {
        self.on_change(value.into());
        MaybeAsync::ready(())
    }
}

impl From<ClientState> for ffi::ClientState {
    fn from(value: ClientState) -> Self {
        match value {
            ClientState::Disabled => Self::Disabled,
            ClientState::Connecting => Self::Connecting,
            ClientState::Connected => Self::Connected,
            ClientState::WaitAfterFailedConnect(_) => Self::WaitAfterFailedConnect,
            ClientState::WaitAfterDisconnect(_) => Self::WaitAfterDisconnect,
            ClientState::Shutdown => Self::Shutdown,
        }
    }
}

#[cfg(feature = "serial")]
impl Listener<PortState> for ffi::PortStateListener {
    fn update(&mut self, value: PortState) -> MaybeAsync<()> {
        self.on_change(value.into());
        MaybeAsync::ready(())
    }
}

#[cfg(feature = "serial")]
impl From<PortState> for ffi::PortState {
    fn from(value: PortState) -> Self {
        match value {
            PortState::Disabled => Self::Disabled,
            PortState::Wait(_) => Self::Wait,
            PortState::Open => Self::Open,
            PortState::Shutdown => Self::Shutdown,
        }
    }
}

pub type EndpointList = dnp3::tcp::EndpointList;

pub(crate) unsafe fn endpoint_list_create(main_endpoint: &CStr) -> *mut EndpointList {
    Box::into_raw(Box::new(EndpointList::single(
        main_endpoint.to_string_lossy().to_string(),
    )))
}

pub(crate) unsafe fn endpoint_list_destroy(list: *mut EndpointList) {
    drop(Box::from_raw(list));
}

pub(crate) unsafe fn endpoint_list_add(list: *mut EndpointList, endpoint: &CStr) {
    if let Some(list) = list.as_mut() {
        list.add(endpoint.to_string_lossy().to_string());
    }
}

fn convert_config(
    config: ffi::MasterChannelConfig,
) -> Result<MasterChannelConfig, ffi::ParamError> {
    let address = EndpointAddress::try_new(config.address())?;

    Ok(MasterChannelConfig {
        master_address: address,
        decode_level: config.decode_level().clone().into(),
        tx_buffer_size: BufferSize::new(config.tx_buffer_size() as usize)?,
        rx_buffer_size: BufferSize::new(config.rx_buffer_size() as usize)?,
    })
}

impl From<ffi::RetryStrategy> for dnp3::app::RetryStrategy {
    fn from(x: ffi::RetryStrategy) -> Self {
        dnp3::app::RetryStrategy::new(x.min_delay(), x.max_delay())
    }
}

#[cfg(feature = "serial")]
impl From<ffi::SerialSettings> for dnp3::serial::SerialSettings {
    fn from(from: ffi::SerialSettings) -> Self {
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
        }
    }
}

impl From<ffi::CommandMode> for CommandMode {
    fn from(x: ffi::CommandMode) -> Self {
        match x {
            ffi::CommandMode::DirectOperate => Self::DirectOperate,
            ffi::CommandMode::SelectBeforeOperate => Self::SelectBeforeOperate,
        }
    }
}

impl From<ffi::TimeSyncMode> for TimeSyncProcedure {
    fn from(x: ffi::TimeSyncMode) -> Self {
        match x {
            ffi::TimeSyncMode::Lan => Self::Lan,
            ffi::TimeSyncMode::NonLan => Self::NonLan,
        }
    }
}

impl From<SpecialAddressError> for ffi::ParamError {
    fn from(_: SpecialAddressError) -> Self {
        Self::InvalidDnp3Address
    }
}

impl From<AssociationError> for ffi::ParamError {
    fn from(error: AssociationError) -> Self {
        match error {
            AssociationError::Shutdown => Self::MasterAlreadyShutdown,
            AssociationError::DuplicateAddress(_) => Self::AssociationDuplicateAddress,
        }
    }
}

impl From<PollError> for ffi::ParamError {
    fn from(error: PollError) -> Self {
        match error {
            PollError::Shutdown => Self::MasterAlreadyShutdown,
            PollError::NoSuchAssociation(_) => Self::AssociationDoesNotExist,
        }
    }
}

impl From<WriteError> for ffi::EmptyResponseError {
    fn from(value: WriteError) -> Self {
        match value {
            WriteError::Task(x) => x.into(),
            WriteError::IinError(_) => Self::RejectedByIin2,
        }
    }
}

#[cfg(feature = "tls")]
impl From<TlsError> for ffi::ParamError {
    fn from(error: TlsError) -> Self {
        match error {
            TlsError::InvalidDnsName => Self::InvalidDnsName,
            TlsError::InvalidPeerCertificate(_) => Self::InvalidPeerCertificate,
            TlsError::InvalidLocalCertificate(_) => Self::InvalidLocalCertificate,
            TlsError::InvalidPrivateKey(_) => Self::InvalidPrivateKey,
            TlsError::Other(_) => Self::OtherTlsError,
        }
    }
}

#[cfg(feature = "tls")]
impl From<ffi::MinTlsVersion> for MinTlsVersion {
    fn from(from: ffi::MinTlsVersion) -> Self {
        match from {
            ffi::MinTlsVersion::V12 => Self::V12,
            ffi::MinTlsVersion::V13 => Self::V13,
        }
    }
}

#[cfg(feature = "tls")]
impl From<ffi::CertificateMode> for CertificateMode {
    fn from(from: ffi::CertificateMode) -> Self {
        match from {
            ffi::CertificateMode::AuthorityBased => Self::AuthorityBased,
            ffi::CertificateMode::SelfSigned => Self::SelfSigned,
        }
    }
}

impl From<ffi::ConnectStrategy> for ConnectStrategy {
    fn from(value: ffi::ConnectStrategy) -> Self {
        ConnectStrategy::new(
            value.min_connect_delay(),
            value.max_connect_delay(),
            value.reconnect_delay(),
        )
    }
}

macro_rules! define_task_from_impl {
    ($name:ident) => {
        impl From<TaskError> for ffi::$name {
            fn from(err: TaskError) -> Self {
                match err {
                    TaskError::TooManyRequests => Self::TooManyRequests,
                    TaskError::Link(_) => Self::NoConnection,
                    TaskError::Transport => Self::NoConnection,
                    TaskError::MalformedResponse(_) => Self::BadResponse,
                    TaskError::UnexpectedResponseHeaders => Self::BadResponse,
                    TaskError::NonFinWithoutCon => Self::BadResponse,
                    TaskError::NeverReceivedFir => Self::BadResponse,
                    TaskError::UnexpectedFir => Self::BadResponse,
                    TaskError::MultiFragmentResponse => Self::BadResponse,
                    TaskError::ResponseTimeout => Self::ResponseTimeout,
                    TaskError::WriteError => Self::WriteError,
                    TaskError::NoSuchAssociation(_) => Self::AssociationRemoved,
                    TaskError::NoConnection => Self::NoConnection,
                    TaskError::Shutdown => Self::Shutdown,
                    TaskError::Disabled => Self::NoConnection,
                    TaskError::BadEncoding(_) => Self::BadEncoding,
                }
            }
        }
    };
}

define_task_from_impl!(CommandError);
define_task_from_impl!(TimeSyncError);
define_task_from_impl!(RestartError);
define_task_from_impl!(ReadError);
define_task_from_impl!(LinkStatusError);
define_task_from_impl!(TaskError);
define_task_from_impl!(EmptyResponseError);

impl From<ffi::FunctionCode> for dnp3::app::FunctionCode {
    fn from(value: ffi::FunctionCode) -> Self {
        match value {
            ffi::FunctionCode::Confirm => Self::Confirm,
            ffi::FunctionCode::Read => Self::Read,
            ffi::FunctionCode::Write => Self::Write,
            ffi::FunctionCode::Select => Self::Select,
            ffi::FunctionCode::Operate => Self::Operate,
            ffi::FunctionCode::DirectOperate => Self::DirectOperate,
            ffi::FunctionCode::DirectOperateNoResponse => Self::DirectOperateNoResponse,
            ffi::FunctionCode::ImmediateFreeze => Self::ImmediateFreeze,
            ffi::FunctionCode::ImmediateFreezeNoResponse => Self::ImmediateFreezeNoResponse,
            ffi::FunctionCode::FreezeClear => Self::FreezeClear,
            ffi::FunctionCode::FreezeClearNoResponse => Self::FreezeClearNoResponse,
            ffi::FunctionCode::FreezeAtTime => Self::FreezeAtTime,
            ffi::FunctionCode::FreezeAtTimeNoResponse => Self::FreezeAtTimeNoResponse,
            ffi::FunctionCode::ColdRestart => Self::ColdRestart,
            ffi::FunctionCode::WarmRestart => Self::WarmRestart,
            ffi::FunctionCode::InitializeData => Self::InitializeData,
            ffi::FunctionCode::InitializeApplication => Self::InitializeApplication,
            ffi::FunctionCode::StartApplication => Self::StartApplication,
            ffi::FunctionCode::StopApplication => Self::StopApplication,
            ffi::FunctionCode::SaveConfiguration => Self::SaveConfiguration,
            ffi::FunctionCode::EnableUnsolicited => Self::EnableUnsolicited,
            ffi::FunctionCode::DisableUnsolicited => Self::DisableUnsolicited,
            ffi::FunctionCode::AssignClass => Self::AssignClass,
            ffi::FunctionCode::DelayMeasure => Self::DelayMeasure,
            ffi::FunctionCode::RecordCurrentTime => Self::RecordCurrentTime,
            ffi::FunctionCode::OpenFile => Self::OpenFile,
            ffi::FunctionCode::CloseFile => Self::CloseFile,
            ffi::FunctionCode::DeleteFile => Self::DeleteFile,
            ffi::FunctionCode::GetFileInfo => Self::GetFileInfo,
            ffi::FunctionCode::AuthenticateFile => Self::AuthenticateFile,
            ffi::FunctionCode::AbortFile => Self::AbortFile,
            ffi::FunctionCode::Response => Self::Response,
            ffi::FunctionCode::UnsolicitedResponse => Self::UnsolicitedResponse,
        }
    }
}
