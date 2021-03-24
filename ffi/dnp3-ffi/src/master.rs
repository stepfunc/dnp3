use std::ffi::CStr;
use std::time::Duration;

use dnp3::app::Timeout;
use dnp3::app::Timestamp;
use dnp3::app::{ReconnectStrategy, RetryStrategy};
use dnp3::link::{EndpointAddress, LinkStatusResult};
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
) -> *mut Master {
    let config = if let Some(config) = config.into() {
        config
    } else {
        return std::ptr::null_mut();
    };

    let endpoints = if let Some(endpoints) = endpoints.as_ref() {
        endpoints
    } else {
        return std::ptr::null_mut();
    };
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

    if let Some(runtime) = runtime.as_ref() {
        runtime.inner.spawn(future);

        let master = Master {
            runtime: runtime.handle(),
            handle,
        };

        Box::into_raw(Box::new(master))
    } else {
        std::ptr::null_mut()
    }
}

pub(crate) unsafe fn master_create_serial_session(
    runtime: *mut crate::runtime::Runtime,
    config: ffi::MasterConfig,
    path: &CStr,
    serial_params: ffi::SerialPortSettings,
    retry_delay: Duration,
    listener: ffi::PortStateListener,
) -> *mut Master {
    let config = if let Some(config) = config.into() {
        config
    } else {
        return std::ptr::null_mut();
    };
    let listener = PortStateListenerAdapter::new(listener);

    let (future, handle) = create_master_serial(
        config,
        &path.to_string_lossy().to_string(),
        serial_params.into(),
        retry_delay,
        listener.into_listener(),
    );

    if let Some(runtime) = runtime.as_ref() {
        runtime.inner.spawn(future);

        let master = Master {
            runtime: runtime.handle(),
            handle,
        };

        Box::into_raw(Box::new(master))
    } else {
        std::ptr::null_mut()
    }
}

pub unsafe fn master_destroy(master: *mut Master) {
    if !master.is_null() {
        Box::from_raw(master);
    }
}

pub unsafe fn master_enable(master: *mut crate::Master) {
    if let Some(master) = master.as_mut() {
        if let Some(runtime) = master.runtime.get() {
            let _ = runtime.block_on(master.handle.enable());
        }
    }
}

pub unsafe fn master_disable(master: *mut crate::Master) {
    if let Some(master) = master.as_mut() {
        if let Some(runtime) = master.runtime.get() {
            let _ = runtime.block_on(master.handle.disable());
        }
    }
}

pub unsafe fn master_add_association(
    master: *mut Master,
    address: u16,
    config: ffi::AssociationConfig,
    read_handler: ffi::ReadHandler,
    time_provider: ffi::TimeProvider,
) -> ffi::AssociationId {
    let master = match master.as_mut() {
        Some(master) => master,
        None => {
            unimplemented!()
        }
    };

    let address = match EndpointAddress::from(address) {
        Ok(x) => x,
        Err(err) => {
            tracing::warn!(
                "special addresses may not be used for the master address: {}",
                err.address
            );
            // TODO - return error
            unimplemented!()
        }
    };

    let runtime = match master.runtime.get() {
        Some(x) => x,
        None => {
            tracing::warn!("runtime has been shut down");
            unimplemented!()
        }
    };

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
    };

    let handler = AssociationHandlerAdapter {
        read_handler,
        time_provider,
    };

    if runtime
        .block_on(
            master
                .handle
                .add_association(address, config, Box::new(handler)),
        )
        .is_ok()
    {
        ffi::AssociationId {
            address: address.raw_value(),
        }
    } else {
        tracing::warn!("Associate creation failure");
        unimplemented!()
    }
}

pub(crate) unsafe fn master_remove_association(
    master: *mut crate::Master,
    id: ffi::AssociationId,
) -> bool {
    let master = match master.as_mut() {
        Some(master) => master,
        None => {
            unimplemented!()
        }
    };

    let endpoint = match EndpointAddress::from(id.address) {
        Ok(x) => x,
        Err(_) => {
            unimplemented!()
        }
    };

    let runtime = match master.runtime.get() {
        Some(x) => x,
        None => {
            unimplemented!()
        }
    };

    if runtime
        .block_on(master.handle.remove_association(endpoint))
        .is_err()
    {
        unimplemented!()
    }

    true
}

pub(crate) unsafe fn master_add_poll(
    master: *mut Master,
    id: ffi::AssociationId,
    request: *mut crate::Request,
    period: std::time::Duration,
) -> ffi::PollId {
    let master = match master.as_mut() {
        Some(master) => master,
        None => {
            unimplemented!()
        }
    };

    let runtime = match master.runtime.get() {
        Some(master) => master,
        None => {
            unimplemented!()
        }
    };

    let address = match EndpointAddress::from(id.address) {
        Ok(x) => x,
        Err(_) => {
            unimplemented!()
        }
    };

    let request = match request.as_ref() {
        Some(x) => x,
        None => {
            unimplemented!()
        }
    };

    let mut association = AssociationHandle::create(address, master.handle.clone());
    let handle = match runtime.block_on(association.add_poll(request.build(), period)) {
        Ok(x) => x,
        Err(_) => {
            unimplemented!()
        }
    };

    ffi::PollId {
        association_id: id.address,
        id: handle.get_id(),
    }
}

pub(crate) unsafe fn master_remove_poll(master: *mut crate::Master, poll: ffi::PollId) {
    let master = match master.as_mut() {
        Some(master) => master,
        None => {
            unimplemented!()
        }
    };

    let runtime = match master.runtime.get() {
        Some(master) => master,
        None => {
            unimplemented!()
        }
    };

    let endpoint = match EndpointAddress::from(poll.association_id) {
        Ok(x) => x,
        Err(_) => {
            unimplemented!()
        }
    };

    let poll = PollHandle::create(
        AssociationHandle::create(endpoint, master.handle.clone()),
        poll.id,
    );

    if runtime.block_on(poll.remove()).is_err() {
        unimplemented!()
    }
}

pub unsafe extern "C" fn master_demand_poll(master: *mut crate::Master, poll: ffi::PollId) {
    let master = match master.as_mut() {
        Some(master) => master,
        None => {
            unimplemented!()
        }
    };

    let runtime = match master.runtime.get() {
        Some(master) => master,
        None => {
            unimplemented!()
        }
    };

    let endpoint = match EndpointAddress::from(poll.association_id) {
        Ok(x) => x,
        Err(_) => {
            unimplemented!()
        }
    };

    let mut poll = PollHandle::create(
        AssociationHandle::create(endpoint, master.handle.clone()),
        poll.id,
    );

    if runtime.block_on(poll.demand()).is_err() {
        unimplemented!()
    }
}

pub(crate) unsafe fn master_read(
    master: *mut crate::Master,
    association: ffi::AssociationId,
    request: *mut crate::Request,
    callback: ffi::ReadTaskCallback,
) {
    let master = match master.as_mut() {
        Some(master) => master,
        None => {
            // don't need to callback here because the whole function will become fallible
            unimplemented!()
        }
    };

    let runtime = match master.runtime.get() {
        Some(master) => master,
        None => {
            unimplemented!()
        }
    };

    let address = match EndpointAddress::from(association.address) {
        Ok(x) => x,
        Err(_) => {
            unimplemented!()
        }
    };

    let request = match request.as_ref() {
        Some(x) => x.build(),
        None => {
            unimplemented!()
        }
    };

    let mut handle = AssociationHandle::create(address, master.handle.clone());

    let task = async move {
        let result = match handle.read(request).await {
            Ok(_) => ffi::ReadResult::Success,
            Err(_) => ffi::ReadResult::TaskError,
        };

        callback.on_complete(result);
    };

    runtime.spawn(task);
}

pub unsafe extern "C" fn master_operate(
    master: *mut crate::Master,
    association: ffi::AssociationId,
    mode: ffi::CommandMode,
    commands: *mut crate::Commands,
    callback: ffi::CommandTaskCallback,
) {
    let master = match master.as_mut() {
        Some(master) => master,
        None => {
            // don't need to callback here because the whole function will become fallible
            unimplemented!()
        }
    };

    let runtime = match master.runtime.get() {
        Some(x) => x,
        None => {
            unimplemented!()
        }
    };

    let address = match EndpointAddress::from(association.address) {
        Ok(x) => x,
        Err(_) => {
            unimplemented!()
        }
    };

    let headers = match commands.as_ref() {
        Some(x) => x.clone().build(),
        None => {
            unimplemented!()
        }
    };

    let mut handle = AssociationHandle::create(address, master.handle.clone());

    let task = async move {
        let result = match handle.operate(mode.into(), headers).await {
            Ok(_) => ffi::CommandResult::Success,
            Err(CommandError::Task(_)) => ffi::CommandResult::TaskError,
            Err(CommandError::Response(err)) => match err {
                CommandResponseError::Request(_) => ffi::CommandResult::TaskError,
                CommandResponseError::BadStatus(_) => ffi::CommandResult::BadStatus,
                CommandResponseError::HeaderCountMismatch => {
                    ffi::CommandResult::HeaderCountMismatch
                }
                CommandResponseError::HeaderTypeMismatch => ffi::CommandResult::HeaderTypeMismatch,
                CommandResponseError::ObjectCountMismatch => {
                    ffi::CommandResult::ObjectCountMismatch
                }
                CommandResponseError::ObjectValueMismatch => {
                    ffi::CommandResult::ObjectValueMismatch
                }
            },
        };

        callback.on_complete(result);
    };

    runtime.spawn(task);
}

pub(crate) unsafe fn master_sync_time(
    master: *mut crate::Master,
    association: ffi::AssociationId,
    mode: ffi::TimeSyncMode,
    callback: ffi::TimeSyncTaskCallback,
) {
    let master = match master.as_mut() {
        Some(master) => master,
        None => {
            // don't need to callback here because the whole function will become fallible
            unimplemented!()
        }
    };

    let runtime = match master.runtime.get() {
        Some(x) => x,
        None => {
            unimplemented!()
        }
    };

    let address = match EndpointAddress::from(association.address) {
        Ok(x) => x,
        Err(_) => {
            unimplemented!()
        }
    };

    let mut association = AssociationHandle::create(address, master.handle.clone());

    let task = async move {
        let result = match association.synchronize_time(mode.into()).await {
            Ok(_) => ffi::TimeSyncResult::Success,
            Err(TimeSyncError::Task(_)) => ffi::TimeSyncResult::TaskError,
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

    runtime.spawn(task);
}

pub(crate) unsafe fn master_cold_restart(
    master: *mut crate::Master,
    association: ffi::AssociationId,
    callback: ffi::RestartTaskCallback,
) {
    let master = match master.as_mut() {
        Some(master) => master,
        None => {
            // don't need to callback here because the whole function will become fallible
            unimplemented!()
        }
    };

    let runtime = match master.runtime.get() {
        Some(x) => x,
        None => {
            unimplemented!()
        }
    };

    let address = match EndpointAddress::from(association.address) {
        Ok(x) => x,
        Err(_) => {
            unimplemented!()
        }
    };

    let mut association = AssociationHandle::create(address, master.handle.clone());

    let task = async move {
        let result = match association.cold_restart().await {
            Ok(value) => ffi::RestartResult::new_success(value),
            Err(_) => ffi::RestartResult::error(),
        };

        callback.on_complete(result);
    };

    runtime.spawn(task);
}

pub(crate) unsafe fn master_warm_restart(
    master: *mut crate::Master,
    association: ffi::AssociationId,
    callback: ffi::RestartTaskCallback,
) {
    let master = match master.as_mut() {
        Some(master) => master,
        None => {
            // don't need to callback here because the whole function will become fallible
            unimplemented!()
        }
    };

    let runtime = match master.runtime.get() {
        Some(x) => x,
        None => {
            unimplemented!()
        }
    };

    let address = match EndpointAddress::from(association.address) {
        Ok(x) => x,
        Err(_) => {
            unimplemented!()
        }
    };

    let mut association = AssociationHandle::create(address, master.handle.clone());

    let task = async move {
        let result = match association.warm_restart().await {
            Ok(value) => ffi::RestartResult::new_success(value),
            Err(_) => ffi::RestartResult::error(),
        };

        callback.on_complete(result);
    };

    runtime.spawn(task);
}

pub(crate) unsafe fn master_check_link_status(
    master: *mut crate::Master,
    association: ffi::AssociationId,
    callback: ffi::LinkStatusCallback,
) {
    let master = match master.as_mut() {
        Some(master) => master,
        None => {
            // don't need to callback here because the whole function will become fallible
            unimplemented!()
        }
    };

    let runtime = match master.runtime.get() {
        Some(x) => x,
        None => {
            unimplemented!()
        }
    };

    let address = match EndpointAddress::from(association.address) {
        Ok(x) => x,
        Err(_) => {
            unimplemented!()
        }
    };

    let mut association = AssociationHandle::create(address, master.handle.clone());

    let task = async move {
        let result = match association.check_link_status().await {
            Ok(LinkStatusResult::Success) => ffi::LinkStatusResult::Success,
            Ok(LinkStatusResult::UnexpectedResponse) => ffi::LinkStatusResult::UnexpectedResponse,
            Err(_) => ffi::LinkStatusResult::TaskError,
        };

        callback.on_complete(result);
    };

    runtime.spawn(task);
}

pub(crate) unsafe fn master_set_decode_level(master: *mut Master, level: ffi::DecodeLevel) {
    if let Some(master) = master.as_mut() {
        master
            .runtime
            .unwrap()
            .spawn(master.handle.set_decode_level(level.into()));
    }
}

pub(crate) unsafe fn master_get_decode_level(master: *mut Master) -> ffi::DecodeLevel {
    if tokio::runtime::Handle::try_current().is_err() {
        if let Some(master) = master.as_mut() {
            if let Ok(level) = master
                .runtime
                .unwrap()
                .block_on(master.handle.get_decode_level())
            {
                return level.into();
            }
        }
    } else {
        tracing::warn!("Tried calling 'master_get_decode_level' from within a tokio thread");
    }

    ffi::DecodeLevelFields {
        application: ffi::AppDecodeLevel::Nothing,
        transport: ffi::TransportDecodeLevel::Nothing,
        link: ffi::LinkDecodeLevel::Nothing,
        physical: ffi::PhysDecodeLevel::Nothing,
    }
    .into()
}

impl ffi::RestartResult {
    fn new_success(delay: Duration) -> Self {
        ffi::RestartResultFields {
            delay,
            success: ffi::RestartSuccess::Success,
        }
        .into()
    }

    fn error() -> Self {
        ffi::RestartResultFields {
            delay: Duration::from_millis(0),
            success: ffi::RestartSuccess::TaskError,
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

impl ffi::MasterConfig {
    fn into(self) -> Option<MasterConfig> {
        let address = match EndpointAddress::from(self.address()) {
            Ok(x) => x,
            Err(err) => {
                tracing::warn!(
                    "special addresses may not be used for the master address: {}",
                    err.address
                );
                return None;
            }
        };

        Some(MasterConfig {
            address,
            decode_level: self.decode_level().clone().into(),
            response_timeout: Timeout::from_duration(self.response_timeout()).unwrap(),
            tx_buffer_size: self.tx_buffer_size() as usize,
            rx_buffer_size: self.rx_buffer_size() as usize,
        })
    }
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
