use std::ffi::CStr;
use std::time::Duration;

use dnp3::app::{ConnectStrategy, Listener, RetryStrategy, Timeout, Timestamp};
use dnp3::link::{EndpointAddress, SpecialAddressError};
use dnp3::master::*;
use dnp3::serial::*;
use dnp3::tcp::ClientState;

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
    let config = convert_config(config)?;
    let endpoints = endpoints.as_ref().ok_or(ffi::ParamError::NullParameter)?;

    let connect_strategy = ConnectStrategy::new(
        connect_strategy.min_connect_delay(),
        connect_strategy.max_connect_delay(),
        connect_strategy.reconnect_delay(),
    );

    let (future, handle) = dnp3::tcp::create_master_tcp_client(
        link_error_mode.into(),
        config,
        endpoints.clone(),
        connect_strategy,
        Box::new(listener),
    );

    let runtime = runtime.as_ref().ok_or(ffi::ParamError::NullParameter)?;
    runtime.inner.spawn(future);
    let channel = MasterChannel {
        runtime: runtime.handle(),
        handle,
    };

    Ok(Box::into_raw(Box::new(channel)))
}

pub(crate) unsafe fn master_channel_create_serial(
    runtime: *mut crate::runtime::Runtime,
    config: ffi::MasterChannelConfig,
    path: &CStr,
    serial_params: ffi::SerialPortSettings,
    retry_delay: Duration,
    listener: ffi::PortStateListener,
) -> Result<*mut MasterChannel, ffi::ParamError> {
    let config = convert_config(config)?;

    let (future, handle) = create_master_serial(
        config,
        &path.to_string_lossy().to_string(),
        serial_params.into(),
        retry_delay,
        Box::new(listener),
    );

    let runtime = runtime.as_ref().ok_or(ffi::ParamError::NullParameter)?;
    runtime.inner.spawn(future);

    let channel = MasterChannel {
        runtime: runtime.handle(),
        handle,
    };

    Ok(Box::into_raw(Box::new(channel)))
}

pub unsafe fn master_channel_destroy(channel: *mut MasterChannel) {
    if !channel.is_null() {
        Box::from_raw(channel);
    }
}

pub unsafe fn master_channel_enable(
    channel: *mut crate::MasterChannel,
) -> Result<(), ffi::ParamError> {
    let channel = channel.as_mut().ok_or(ffi::ParamError::NullParameter)?;
    channel.runtime.block_on(channel.handle.enable())??;
    Ok(())
}

pub unsafe fn master_channel_disable(
    channel: *mut crate::MasterChannel,
) -> Result<(), ffi::ParamError> {
    let channel = channel.as_mut().ok_or(ffi::ParamError::NullParameter)?;
    channel.runtime.block_on(channel.handle.disable())??;
    Ok(())
}

pub unsafe fn master_channel_add_association(
    channel: *mut MasterChannel,
    address: u16,
    config: ffi::AssociationConfig,
    read_handler: ffi::ReadHandler,
    assoc_handler: ffi::AssociationHandler,
) -> Result<ffi::AssociationId, ffi::ParamError> {
    let channel = channel.as_mut().ok_or(ffi::ParamError::NullParameter)?;
    let address = EndpointAddress::from(address)?;

    let config = AssociationConfig {
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
    ))??;
    Ok(ffi::AssociationId {
        address: address.raw_value(),
    })
}

pub(crate) unsafe fn master_channel_remove_association(
    channel: *mut crate::MasterChannel,
    id: ffi::AssociationId,
) -> Result<(), ffi::ParamError> {
    let channel = channel.as_mut().ok_or(ffi::ParamError::NullParameter)?;
    let endpoint = EndpointAddress::from(id.address)?;

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
    let address = EndpointAddress::from(id.address)?;
    let request = request.as_ref().ok_or(ffi::ParamError::NullParameter)?;

    let mut association = AssociationHandle::create(address, channel.handle.clone());
    let handle = channel
        .runtime
        .block_on(association.add_poll(request.build(), period))??;

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
    let endpoint = EndpointAddress::from(poll.association_id)?;

    let poll = PollHandle::create(
        AssociationHandle::create(endpoint, channel.handle.clone()),
        poll.id,
    );

    channel.runtime.block_on(poll.remove())??;

    Ok(())
}

pub unsafe fn master_channel_demand_poll(
    channel: *mut crate::MasterChannel,
    poll: ffi::PollId,
) -> Result<(), ffi::ParamError> {
    let channel = channel.as_mut().ok_or(ffi::ParamError::NullParameter)?;
    let endpoint = EndpointAddress::from(poll.association_id)?;

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
    let address = EndpointAddress::from(association.address)?;
    let request = request
        .as_ref()
        .ok_or(ffi::ParamError::NullParameter)?
        .build();

    let mut handle = AssociationHandle::create(address, channel.handle.clone());

    let task = async move {
        match handle.read(request).await {
            Ok(()) => callback.on_complete(ffi::Nothing::Nothing),
            Err(err) => callback.on_failure(err.into()),
        };
    };

    channel.runtime.spawn(task)?;
    Ok(())
}

pub unsafe fn master_channel_operate(
    channel: *mut crate::MasterChannel,
    association: ffi::AssociationId,
    mode: ffi::CommandMode,
    commands: *mut crate::CommandSet,
    callback: ffi::CommandTaskCallback,
) -> Result<(), ffi::ParamError> {
    let channel = channel.as_mut().ok_or(ffi::ParamError::NullParameter)?;
    let address = EndpointAddress::from(association.address)?;
    let headers = commands
        .as_ref()
        .ok_or(ffi::ParamError::NullParameter)?
        .clone()
        .build();

    let mut handle = AssociationHandle::create(address, channel.handle.clone());

    let task = async move {
        match handle.operate(mode.into(), headers).await {
            Ok(_) => {
                callback.on_complete(ffi::Nothing::Nothing);
            }
            Err(err) => {
                let err: ffi::CommandError = match err {
                    CommandError::Task(err) => err.into(),
                    CommandError::Response(err) => match err {
                        CommandResponseError::Request(err) => err.into(),
                        CommandResponseError::BadStatus(_) => ffi::CommandError::BadStatus,
                        CommandResponseError::HeaderCountMismatch => {
                            ffi::CommandError::HeaderMismatch
                        }
                        CommandResponseError::HeaderTypeMismatch => {
                            ffi::CommandError::HeaderMismatch
                        }
                        CommandResponseError::ObjectCountMismatch => {
                            ffi::CommandError::HeaderMismatch
                        }
                        CommandResponseError::ObjectValueMismatch => {
                            ffi::CommandError::HeaderMismatch
                        }
                    },
                };
                callback.on_failure(err);
            }
        };
    };

    channel.runtime.spawn(task)?;
    Ok(())
}

impl From<TimeSyncError> for ffi::TimeSyncError {
    fn from(err: TimeSyncError) -> Self {
        match err {
            TimeSyncError::Task(err) => err.into(),
            TimeSyncError::ClockRollback => ffi::TimeSyncError::ClockRollback,
            TimeSyncError::SystemTimeNotUnix => ffi::TimeSyncError::SystemTimeNotUnix,
            TimeSyncError::BadOutstationTimeDelay(_) => ffi::TimeSyncError::BadOutstationTimeDelay,
            TimeSyncError::Overflow => ffi::TimeSyncError::Overflow,
            TimeSyncError::StillNeedsTime => ffi::TimeSyncError::StillNeedsTime,
            TimeSyncError::SystemTimeNotAvailable => ffi::TimeSyncError::SystemTimeNotAvailable,
            TimeSyncError::IinError(_) => ffi::TimeSyncError::IinError,
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
    let address = EndpointAddress::from(association.address)?;

    let mut association = AssociationHandle::create(address, channel.handle.clone());

    let task = async move {
        match association.synchronize_time(mode.into()).await {
            Ok(()) => callback.on_complete(ffi::Nothing::Nothing),
            Err(err) => callback.on_failure(err.into()),
        };
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
    let address = EndpointAddress::from(association.address)?;

    let mut association = AssociationHandle::create(address, channel.handle.clone());

    let task = async move {
        match association.cold_restart().await {
            Ok(x) => callback.on_complete(x),
            Err(err) => callback.on_failure(err.into()),
        };
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
    let address = EndpointAddress::from(association.address)?;

    let mut association = AssociationHandle::create(address, channel.handle.clone());

    let task = async move {
        match association.warm_restart().await {
            Ok(x) => callback.on_complete(x),
            Err(err) => callback.on_failure(err.into()),
        };
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
    let address = EndpointAddress::from(association.address)?;

    let mut association = AssociationHandle::create(address, channel.handle.clone());

    let task = async move {
        match association.check_link_status().await {
            Ok(_) => callback.on_complete(ffi::Nothing::Nothing),
            Err(err) => callback.on_failure(err.into()),
        };
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

pub fn timestamp_utc_valid(value: u64) -> ffi::UtcTimestamp {
    ffi::UtcTimestamp {
        value,
        is_valid: true,
    }
}

pub fn timestamp_utc_invalid() -> ffi::UtcTimestamp {
    ffi::UtcTimestamp {
        value: 0,
        is_valid: false,
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
    fn update(&mut self, value: ClientState) {
        let value = match value {
            ClientState::Disabled => ffi::ClientState::Disabled,
            ClientState::Connecting => ffi::ClientState::Connecting,
            ClientState::Connected => ffi::ClientState::Connected,
            ClientState::WaitAfterFailedConnect(_) => ffi::ClientState::WaitAfterFailedConnect,
            ClientState::WaitAfterDisconnect(_) => ffi::ClientState::WaitAfterDisconnect,
            ClientState::Shutdown => ffi::ClientState::Shutdown,
        };
        self.on_change(value)
    }
}

impl Listener<PortState> for ffi::PortStateListener {
    fn update(&mut self, value: PortState) {
        let value = match value {
            PortState::Disabled => ffi::PortState::Disabled,
            PortState::Wait(_) => ffi::PortState::Wait,
            PortState::Open => ffi::PortState::Open,
            PortState::Shutdown => ffi::PortState::Shutdown,
        };
        self.on_change(value);
    }
}

pub type EndpointList = dnp3::tcp::EndpointList;

pub(crate) unsafe fn endpoint_list_create(main_endpoint: &CStr) -> *mut EndpointList {
    Box::into_raw(Box::new(EndpointList::single(
        main_endpoint.to_string_lossy().to_string(),
    )))
}

pub(crate) unsafe fn endpoint_list_destroy(list: *mut EndpointList) {
    Box::from_raw(list);
}

pub(crate) unsafe fn endpoint_list_add(list: *mut EndpointList, endpoint: &CStr) {
    if let Some(list) = list.as_mut() {
        list.add(endpoint.to_string_lossy().to_string());
    }
}

fn convert_config(
    config: ffi::MasterChannelConfig,
) -> Result<MasterChannelConfig, ffi::ParamError> {
    let address = EndpointAddress::from(config.address())?;

    Ok(MasterChannelConfig {
        master_address: address,
        decode_level: config.decode_level().clone().into(),
        response_timeout: Timeout::from_duration(config.response_timeout()).unwrap(),
        tx_buffer_size: config.tx_buffer_size() as usize,
        rx_buffer_size: config.rx_buffer_size() as usize,
    })
}

impl From<ffi::RetryStrategy> for dnp3::app::RetryStrategy {
    fn from(x: ffi::RetryStrategy) -> Self {
        dnp3::app::RetryStrategy::new(x.min_delay(), x.max_delay())
    }
}

impl From<ffi::SerialPortSettings> for dnp3::serial::SerialSettings {
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
        }
    }
}

impl From<ffi::CommandMode> for CommandMode {
    fn from(x: ffi::CommandMode) -> Self {
        match x {
            ffi::CommandMode::DirectOperate => CommandMode::DirectOperate,
            ffi::CommandMode::SelectBeforeOperate => CommandMode::SelectBeforeOperate,
        }
    }
}

impl From<ffi::TimeSyncMode> for TimeSyncProcedure {
    fn from(x: ffi::TimeSyncMode) -> Self {
        match x {
            ffi::TimeSyncMode::Lan => TimeSyncProcedure::Lan,
            ffi::TimeSyncMode::NonLan => TimeSyncProcedure::NonLan,
        }
    }
}

impl From<SpecialAddressError> for ffi::ParamError {
    fn from(_: SpecialAddressError) -> Self {
        ffi::ParamError::InvalidDnp3Address
    }
}

impl From<AssociationError> for ffi::ParamError {
    fn from(error: AssociationError) -> Self {
        match error {
            AssociationError::Shutdown => ffi::ParamError::MasterAlreadyShutdown,
            AssociationError::DuplicateAddress(_) => ffi::ParamError::AssociationDuplicateAddress,
        }
    }
}

impl From<PollError> for ffi::ParamError {
    fn from(error: PollError) -> Self {
        match error {
            PollError::Shutdown => ffi::ParamError::MasterAlreadyShutdown,
            PollError::NoSuchAssociation(_) => ffi::ParamError::AssociationDoesNotExist,
        }
    }
}

macro_rules! define_task_from_impl {
    ($name:ident) => {
        impl From<TaskError> for ffi::$name {
            fn from(err: TaskError) -> Self {
                match err {
                    TaskError::TooManyRequests => ffi::$name::TooManyRequests,
                    TaskError::Link(_) => ffi::$name::NoConnection,
                    TaskError::Transport => ffi::$name::NoConnection,
                    TaskError::MalformedResponse(_) => ffi::$name::BadResponse,
                    TaskError::UnexpectedResponseHeaders => ffi::$name::BadResponse,
                    TaskError::NonFinWithoutCon => ffi::$name::BadResponse,
                    TaskError::NeverReceivedFir => ffi::$name::BadResponse,
                    TaskError::UnexpectedFir => ffi::$name::BadResponse,
                    TaskError::MultiFragmentResponse => ffi::$name::BadResponse,
                    TaskError::ResponseTimeout => ffi::$name::ResponseTimeout,
                    TaskError::WriteError => ffi::$name::WriteError,
                    TaskError::NoSuchAssociation(_) => ffi::$name::AssociationRemoved,
                    TaskError::NoConnection => ffi::$name::NoConnection,
                    TaskError::Shutdown => ffi::$name::Shutdown,
                    TaskError::Disabled => ffi::$name::NoConnection,
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
