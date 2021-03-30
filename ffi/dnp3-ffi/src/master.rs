use std::ffi::CStr;
use std::time::Duration;

use dnp3::app::Timeout;
use dnp3::app::Timestamp;
use dnp3::app::{ReconnectStrategy, RetryStrategy};
use dnp3::link::{EndpointAddress, LinkStatusResult, SpecialAddressError};
use dnp3::master::*;
use dnp3::serial::*;
use dnp3::tcp::ClientState;

use crate::ffi;

pub struct Master {
    pub(crate) runtime: crate::runtime::RuntimeHandle,
    pub(crate) handle: MasterHandle,
}

pub(crate) unsafe fn master_create_tcp_session(
    runtime: *mut crate::runtime::Runtime,
    link_error_mode: ffi::LinkErrorMode,
    config: ffi::MasterConfig,
    endpoints: *const crate::EndpointList,
    connect_strategy: ffi::RetryStrategy,
    reconnect_delay: Duration,
    listener: ffi::ClientStateListener,
) -> Result<*mut Master, ffi::Dnp3Error> {
    let config = convert_config(config)?;
    let endpoints = endpoints.as_ref().ok_or(ffi::Dnp3Error::NullParameter)?;
    let listener = ClientStateListenerAdapter::new(listener);

    let reconnect_delay = if reconnect_delay == Duration::from_millis(0) {
        None
    } else {
        Some(reconnect_delay)
    };

    let (future, handle) = dnp3::tcp::create_master_tcp_client(
        link_error_mode.into(),
        config,
        endpoints.clone(),
        ReconnectStrategy::new(connect_strategy.into(), reconnect_delay),
        listener.into_listener(),
    );

    let runtime = runtime.as_ref().ok_or(ffi::Dnp3Error::NullParameter)?;
    runtime.inner.spawn(future);
    let master = Master {
        runtime: runtime.handle(),
        handle,
    };

    Ok(Box::into_raw(Box::new(master)))
}

pub(crate) unsafe fn master_create_serial_session(
    runtime: *mut crate::runtime::Runtime,
    config: ffi::MasterConfig,
    path: &CStr,
    serial_params: ffi::SerialPortSettings,
    retry_delay: Duration,
    listener: ffi::PortStateListener,
) -> Result<*mut Master, ffi::Dnp3Error> {
    let config = convert_config(config)?;
    let listener = PortStateListenerAdapter::new(listener);

    let (future, handle) = create_master_serial(
        config,
        &path.to_string_lossy().to_string(),
        serial_params.into(),
        retry_delay,
        listener.into_listener(),
    );

    let runtime = runtime.as_ref().ok_or(ffi::Dnp3Error::NullParameter)?;
    runtime.inner.spawn(future);

    let master = Master {
        runtime: runtime.handle(),
        handle,
    };

    Ok(Box::into_raw(Box::new(master)))
}

pub unsafe fn master_destroy(master: *mut Master) {
    if !master.is_null() {
        Box::from_raw(master);
    }
}

pub unsafe fn master_enable(master: *mut crate::Master) -> Result<(), ffi::Dnp3Error> {
    let master = master.as_mut().ok_or(ffi::Dnp3Error::NullParameter)?;
    master.runtime.block_on(master.handle.enable())??;
    Ok(())
}

pub unsafe fn master_disable(master: *mut crate::Master) -> Result<(), ffi::Dnp3Error> {
    let master = master.as_mut().ok_or(ffi::Dnp3Error::NullParameter)?;
    master.runtime.block_on(master.handle.disable())??;
    Ok(())
}

pub unsafe fn master_add_association(
    master: *mut Master,
    address: u16,
    config: ffi::AssociationConfig,
    read_handler: ffi::ReadHandler,
    time_provider: ffi::TimeProvider,
) -> Result<ffi::AssociationId, ffi::Dnp3Error> {
    let master = master.as_mut().ok_or(ffi::Dnp3Error::NullParameter)?;
    let address = EndpointAddress::from(address)?;

    let config = AssociationConfig {
        disable_unsol_classes: convert_event_classes(&config.disable_unsol_classes()),
        enable_unsol_classes: convert_event_classes(&config.enable_unsol_classes()),
        startup_integrity_classes: convert_classes(&config.startup_integrity_classes()),
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
            &config.event_scan_on_events_available(),
        ),
        max_queued_user_requests: config.max_queued_user_requests as usize,
    };

    let handler = AssociationHandlerAdapter {
        read_handler,
        time_provider,
    };

    master.runtime.block_on(master.handle.add_association(
        address,
        config,
        Box::new(handler),
    ))??;
    Ok(ffi::AssociationId {
        address: address.raw_value(),
    })
}

pub(crate) unsafe fn master_remove_association(
    master: *mut crate::Master,
    id: ffi::AssociationId,
) -> Result<(), ffi::Dnp3Error> {
    let master = master.as_mut().ok_or(ffi::Dnp3Error::NullParameter)?;
    let endpoint = EndpointAddress::from(id.address)?;

    master
        .runtime
        .block_on(master.handle.remove_association(endpoint))??;

    Ok(())
}

pub(crate) unsafe fn master_add_poll(
    master: *mut Master,
    id: ffi::AssociationId,
    request: *mut crate::Request,
    period: std::time::Duration,
) -> Result<ffi::PollId, ffi::Dnp3Error> {
    let master = master.as_mut().ok_or(ffi::Dnp3Error::NullParameter)?;
    let address = EndpointAddress::from(id.address)?;
    let request = request.as_ref().ok_or(ffi::Dnp3Error::NullParameter)?;

    let mut association = AssociationHandle::create(address, master.handle.clone());
    let handle = master
        .runtime
        .block_on(association.add_poll(request.build(), period))??;

    Ok(ffi::PollId {
        association_id: id.address,
        id: handle.get_id(),
    })
}

pub(crate) unsafe fn master_remove_poll(
    master: *mut crate::Master,
    poll: ffi::PollId,
) -> Result<(), ffi::Dnp3Error> {
    let master = master.as_mut().ok_or(ffi::Dnp3Error::NullParameter)?;
    let endpoint = EndpointAddress::from(poll.association_id)?;

    let poll = PollHandle::create(
        AssociationHandle::create(endpoint, master.handle.clone()),
        poll.id,
    );

    master.runtime.block_on(poll.remove())??;

    Ok(())
}

pub unsafe fn master_demand_poll(
    master: *mut crate::Master,
    poll: ffi::PollId,
) -> Result<(), ffi::Dnp3Error> {
    let master = master.as_mut().ok_or(ffi::Dnp3Error::NullParameter)?;
    let endpoint = EndpointAddress::from(poll.association_id)?;

    let mut poll = PollHandle::create(
        AssociationHandle::create(endpoint, master.handle.clone()),
        poll.id,
    );

    master.runtime.block_on(poll.demand())??;

    Ok(())
}

pub(crate) unsafe fn master_read(
    master: *mut crate::Master,
    association: ffi::AssociationId,
    request: *mut crate::Request,
    callback: ffi::ReadTaskCallback,
) -> Result<(), ffi::Dnp3Error> {
    let master = master.as_mut().ok_or(ffi::Dnp3Error::NullParameter)?;
    let address = EndpointAddress::from(association.address)?;
    let request = request
        .as_ref()
        .ok_or(ffi::Dnp3Error::NullParameter)?
        .build();

    let mut handle = AssociationHandle::create(address, master.handle.clone());

    let task = async move {
        let result = match handle.read(request).await {
            Ok(_) => ffi::ReadResult::Success,
            Err(err) => err.into(),
        };

        callback.on_complete(result);
    };

    master.runtime.spawn(task)?;
    Ok(())
}

pub unsafe fn master_operate(
    master: *mut crate::Master,
    association: ffi::AssociationId,
    mode: ffi::CommandMode,
    commands: *mut crate::Commands,
    callback: ffi::CommandTaskCallback,
) -> Result<(), ffi::Dnp3Error> {
    let master = master.as_mut().ok_or(ffi::Dnp3Error::NullParameter)?;
    let address = EndpointAddress::from(association.address)?;
    let headers = commands
        .as_ref()
        .ok_or(ffi::Dnp3Error::NullParameter)?
        .clone()
        .build();

    let mut handle = AssociationHandle::create(address, master.handle.clone());

    let task = async move {
        let result = match handle.operate(mode.into(), headers).await {
            Ok(_) => ffi::CommandResult::Success,
            Err(CommandError::Task(err)) => err.into(),
            Err(CommandError::Response(err)) => match err {
                CommandResponseError::Request(err) => err.into(),
                CommandResponseError::BadStatus(_) => ffi::CommandResult::BadStatus,
                CommandResponseError::HeaderCountMismatch => ffi::CommandResult::HeaderMismatch,
                CommandResponseError::HeaderTypeMismatch => ffi::CommandResult::HeaderMismatch,
                CommandResponseError::ObjectCountMismatch => ffi::CommandResult::HeaderMismatch,
                CommandResponseError::ObjectValueMismatch => ffi::CommandResult::HeaderMismatch,
            },
        };

        callback.on_complete(result);
    };

    master.runtime.spawn(task)?;
    Ok(())
}

pub(crate) unsafe fn master_sync_time(
    master: *mut crate::Master,
    association: ffi::AssociationId,
    mode: ffi::TimeSyncMode,
    callback: ffi::TimeSyncTaskCallback,
) -> Result<(), ffi::Dnp3Error> {
    let master = master.as_mut().ok_or(ffi::Dnp3Error::NullParameter)?;
    let address = EndpointAddress::from(association.address)?;

    let mut association = AssociationHandle::create(address, master.handle.clone());

    let task = async move {
        let result = match association.synchronize_time(mode.into()).await {
            Ok(_) => ffi::TimeSyncResult::Success,
            Err(TimeSyncError::Task(err)) => err.into(),
            Err(TimeSyncError::ClockRollback) => ffi::TimeSyncResult::ClockRollback,
            Err(TimeSyncError::SystemTimeNotUnix) => ffi::TimeSyncResult::SystemTimeNotUnix,
            Err(TimeSyncError::BadOutstationTimeDelay(_)) => {
                ffi::TimeSyncResult::BadOutstationTimeDelay
            }
            Err(TimeSyncError::Overflow) => ffi::TimeSyncResult::Overflow,
            Err(TimeSyncError::StillNeedsTime) => ffi::TimeSyncResult::StillNeedsTime,
            Err(TimeSyncError::SystemTimeNotAvailable) => {
                ffi::TimeSyncResult::SystemTimeNotAvailable
            }
            Err(TimeSyncError::IinError(_)) => ffi::TimeSyncResult::IinError,
        };

        callback.on_complete(result);
    };

    master.runtime.spawn(task)?;
    Ok(())
}

pub(crate) unsafe fn master_cold_restart(
    master: *mut crate::Master,
    association: ffi::AssociationId,
    callback: ffi::RestartTaskCallback,
) -> Result<(), ffi::Dnp3Error> {
    let master = master.as_mut().ok_or(ffi::Dnp3Error::NullParameter)?;
    let address = EndpointAddress::from(association.address)?;

    let mut association = AssociationHandle::create(address, master.handle.clone());

    let task = async move {
        let result = match association.cold_restart().await {
            Ok(value) => ffi::RestartResult::new_success(value),
            Err(err) => ffi::RestartResult::new_error(err.into()),
        };

        callback.on_complete(result);
    };

    master.runtime.spawn(task)?;
    Ok(())
}

pub(crate) unsafe fn master_warm_restart(
    master: *mut crate::Master,
    association: ffi::AssociationId,
    callback: ffi::RestartTaskCallback,
) -> Result<(), ffi::Dnp3Error> {
    let master = master.as_mut().ok_or(ffi::Dnp3Error::NullParameter)?;
    let address = EndpointAddress::from(association.address)?;

    let mut association = AssociationHandle::create(address, master.handle.clone());

    let task = async move {
        let result = match association.warm_restart().await {
            Ok(value) => ffi::RestartResult::new_success(value),
            Err(err) => ffi::RestartResult::new_error(err.into()),
        };

        callback.on_complete(result);
    };

    master.runtime.spawn(task)?;
    Ok(())
}

pub(crate) unsafe fn master_check_link_status(
    master: *mut crate::Master,
    association: ffi::AssociationId,
    callback: ffi::LinkStatusCallback,
) -> Result<(), ffi::Dnp3Error> {
    let master = master.as_mut().ok_or(ffi::Dnp3Error::NullParameter)?;
    let address = EndpointAddress::from(association.address)?;

    let mut association = AssociationHandle::create(address, master.handle.clone());

    let task = async move {
        let result = match association.check_link_status().await {
            Ok(LinkStatusResult::Success) => ffi::LinkStatusResult::Success,
            Ok(LinkStatusResult::UnexpectedResponse) => ffi::LinkStatusResult::UnexpectedResponse,
            Err(_) => ffi::LinkStatusResult::TaskError,
        };

        callback.on_complete(result);
    };

    master.runtime.spawn(task)?;
    Ok(())
}

pub(crate) unsafe fn master_set_decode_level(
    master: *mut Master,
    level: ffi::DecodeLevel,
) -> Result<(), ffi::Dnp3Error> {
    let master = master.as_mut().ok_or(ffi::Dnp3Error::NullParameter)?;
    master
        .runtime
        .spawn(master.handle.set_decode_level(level.into()))?;
    Ok(())
}

pub(crate) unsafe fn master_get_decode_level(
    master: *mut Master,
) -> Result<ffi::DecodeLevel, ffi::Dnp3Error> {
    let master = master.as_mut().ok_or(ffi::Dnp3Error::NullParameter)?;

    let result = master
        .runtime
        .block_on(master.handle.get_decode_level())??;

    Ok(result.into())
}

impl ffi::RestartResult {
    fn new_success(delay: Duration) -> Self {
        ffi::RestartResultFields {
            delay,
            error: ffi::RestartError::Ok,
        }
        .into()
    }

    fn new_error(error: ffi::RestartError) -> Self {
        ffi::RestartResultFields {
            delay: Duration::from_millis(0),
            error,
        }
        .into()
    }
}

pub(crate) fn classes_all() -> ffi::Classes {
    ffi::Classes {
        class0: true,
        class1: true,
        class2: true,
        class3: true,
    }
}

pub(crate) fn classes_none() -> ffi::Classes {
    ffi::Classes {
        class0: false,
        class1: false,
        class2: false,
        class3: false,
    }
}

pub(crate) fn event_classes_all() -> ffi::EventClasses {
    ffi::EventClasses {
        class1: true,
        class2: true,
        class3: true,
    }
}

pub(crate) fn event_classes_none() -> ffi::EventClasses {
    ffi::EventClasses {
        class1: false,
        class2: false,
        class3: false,
    }
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

struct AssociationHandlerAdapter {
    read_handler: ffi::ReadHandler,
    time_provider: ffi::TimeProvider,
}

impl AssociationHandler for AssociationHandlerAdapter {
    fn get_system_time(&self) -> Option<Timestamp> {
        if let Some(time) = self.time_provider.get_time() {
            time.into()
        } else {
            None
        }
    }

    fn get_read_handler(&mut self) -> &mut dyn ReadHandler {
        &mut self.read_handler
    }
}

pub fn timeprovidertimestamp_valid(value: u64) -> ffi::TimeProviderTimestamp {
    ffi::TimeProviderTimestamp {
        value,
        is_valid: true,
    }
}

pub fn timeprovidertimestamp_invalid() -> ffi::TimeProviderTimestamp {
    ffi::TimeProviderTimestamp {
        value: 0,
        is_valid: false,
    }
}

impl From<ffi::TimeProviderTimestamp> for Option<Timestamp> {
    fn from(from: ffi::TimeProviderTimestamp) -> Self {
        if from.is_valid {
            Some(Timestamp::new(from.value))
        } else {
            None
        }
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
                ClientState::Disabled => ffi::ClientState::Disabled,
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

struct PortStateListenerAdapter {
    native_cb: ffi::PortStateListener,
}

impl PortStateListenerAdapter {
    fn new(native_cb: ffi::PortStateListener) -> Self {
        Self { native_cb }
    }

    fn into_listener(self) -> Listener<PortState> {
        Listener::BoxedFn(Box::new(move |value| {
            let value = match value {
                PortState::Disabled => ffi::PortState::Disabled,
                PortState::Wait(_) => ffi::PortState::Wait,
                PortState::Open => ffi::PortState::Open,
                PortState::Shutdown => ffi::PortState::Shutdown,
            };
            self.native_cb.on_change(value);
        }))
    }
}

pub type EndpointList = dnp3::tcp::EndpointList;

pub(crate) unsafe fn endpoint_list_new(main_endpoint: &CStr) -> *mut EndpointList {
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

fn convert_config(config: ffi::MasterConfig) -> Result<MasterConfig, ffi::Dnp3Error> {
    let address = EndpointAddress::from(config.address())?;

    Ok(MasterConfig {
        address,
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

impl From<SpecialAddressError> for ffi::Dnp3Error {
    fn from(_: SpecialAddressError) -> Self {
        ffi::Dnp3Error::InvalidDnp3Address
    }
}

impl From<AssociationError> for ffi::Dnp3Error {
    fn from(error: AssociationError) -> Self {
        match error {
            AssociationError::Shutdown => ffi::Dnp3Error::MasterAlreadyShutdown,
            AssociationError::DuplicateAddress(_) => ffi::Dnp3Error::AssociationDuplicateAddress,
        }
    }
}

impl From<PollError> for ffi::Dnp3Error {
    fn from(error: PollError) -> Self {
        match error {
            PollError::Shutdown => ffi::Dnp3Error::MasterAlreadyShutdown,
            PollError::NoSuchAssociation(_) => ffi::Dnp3Error::AssociationDoesNotExist,
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

define_task_from_impl!(CommandResult);
define_task_from_impl!(TimeSyncResult);
define_task_from_impl!(RestartError);
define_task_from_impl!(ReadResult);
