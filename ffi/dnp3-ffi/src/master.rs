use std::time::Duration;

use crate::association::Association;

use crate::ffi;

use dnp3::app::retry::{ReconnectStrategy, RetryStrategy};
use dnp3::app::timeout::Timeout;
use dnp3::app::types::Timestamp;
use dnp3::config::EndpointAddress;
use dnp3::master::association::AssociationConfig;
use dnp3::master::handle::{AssociationHandler, Listener, MasterConfig, MasterHandle, ReadHandler};
use dnp3::master::request::{Classes, EventClasses, TimeSyncProcedure};
use dnp3::master::serial::{create_master_serial_client, DataBits, FlowControl, Parity, StopBits};
use dnp3::master::ClientState;
use dnp3::prelude::master::create_master_tcp_client;
use std::ffi::CStr;

pub struct Master {
    pub(crate) runtime: crate::runtime::RuntimeHandle,
    pub(crate) handle: MasterHandle,
}

pub(crate) unsafe fn master_create_tcp_session(
    runtime: *mut crate::runtime::Runtime,
    link_error_mode: ffi::LinkErrorMode,
    config: ffi::MasterConfig,
    endpoints: *const crate::EndpointList,
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

    let (future, handle) = create_master_tcp_client(
        link_error_mode.into(),
        config,
        endpoints.clone(),
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
        &path.to_string_lossy().to_string(),
        serial_params.into(),
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

pub unsafe fn master_add_association(
    master: *mut Master,
    address: u16,
    config: ffi::AssociationConfig,
    handlers: ffi::AssociationHandlers,
    time_provider: ffi::TimeProvider,
) -> *mut Association {
    let master = match master.as_mut() {
        Some(master) => master,
        None => return std::ptr::null_mut(),
    };

    let address = match EndpointAddress::from(address) {
        Ok(x) => x,
        Err(err) => {
            tracing::warn!(
                "special addresses may not be used for the master address: {}",
                err.address
            );
            return std::ptr::null_mut();
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
        integrity_handler: handlers.integrity_handler,
        unsolicited_handler: handlers.unsolicited_handler,
        default_poll_handler: handlers.default_poll_handler,
        time_provider,
    };

    if tokio::runtime::Handle::try_current().is_err() {
        if let Ok(handle) = master
            .runtime
            .unwrap()
            .block_on(
                master
                    .handle
                    .add_association(address, config, Box::new(handler)),
            )
        {
            let association = Association {
                runtime: master.runtime.clone(),
                handle,
            };
            Box::into_raw(Box::new(association))
        } else {
            tracing::warn!("Associate creation failure");
            std::ptr::null_mut()
        }
    } else {
        tracing::warn!("Tried calling 'master_add_association' from within a tokio thread");
        std::ptr::null_mut()
    }
}

pub unsafe fn master_set_decode_level(master: *mut Master, level: ffi::DecodeLevel) {
    if let Some(master) = master.as_mut() {
        master
            .runtime
            .unwrap()
            .spawn(master.handle.set_decode_level(level.into()));
    }
}

pub unsafe fn master_get_decode_level(master: *mut Master) -> ffi::DecodeLevel {
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

pub fn classes_all() -> ffi::Classes {
    ffi::Classes {
        class0: true,
        class1: true,
        class2: true,
        class3: true,
    }
}

pub fn classes_none() -> ffi::Classes {
    ffi::Classes {
        class0: false,
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
    integrity_handler: ffi::ReadHandler,
    unsolicited_handler: ffi::ReadHandler,
    default_poll_handler: ffi::ReadHandler,
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

    fn get_integrity_handler(&mut self) -> &mut dyn ReadHandler {
        &mut self.integrity_handler
    }

    fn get_unsolicited_handler(&mut self) -> &mut dyn ReadHandler {
        &mut self.unsolicited_handler
    }

    fn get_default_poll_handler(&mut self) -> &mut dyn ReadHandler {
        &mut self.default_poll_handler
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

pub type EndpointList = dnp3::master::tcp::EndpointList;

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

        let strategy = ReconnectStrategy::new(
            RetryStrategy::new(
                self.reconnection_strategy().min_delay(),
                self.reconnection_strategy().max_delay(),
            ),
            if self.reconnection_delay() != Duration::from_millis(0) {
                Some(self.reconnection_delay())
            } else {
                None
            },
        );

        Some(MasterConfig {
            address,
            decode_level: self.decode_level().clone().into(),
            reconnection_strategy: strategy,
            response_timeout: Timeout::from_duration(self.response_timeout()).unwrap(),
            tx_buffer_size: self.tx_buffer_size() as usize,
            rx_buffer_size: self.rx_buffer_size() as usize,
        })
    }
}

impl From<ffi::SerialPortSettings> for dnp3::master::serial::SerialSettings {
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
