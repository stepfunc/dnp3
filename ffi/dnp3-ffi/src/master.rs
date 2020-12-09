use crate::association::Association;
use crate::ffi;

use dnp3::app::retry::RetryStrategy;
use dnp3::app::types::Timestamp;
use dnp3::entry::EndpointAddress;
use dnp3::master::association::Configuration;
use dnp3::master::handle::{AssociationHandler, MasterHandle, ReadHandler};
use dnp3::master::request::{Classes, EventClasses, TimeSyncProcedure};

pub struct Master {
    pub runtime: tokio::runtime::Handle,
    pub handle: MasterHandle,
}

pub unsafe fn master_destroy(master: *mut Master) {
    if !master.is_null() {
        Box::from_raw(master);
    }
}

pub unsafe fn master_add_association(
    master: *mut Master,
    address: u16,
    config: ffi::AssociationConfiguration,
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
            log::warn!(
                "special addresses may not be used for the master address: {}",
                err.address
            );
            return std::ptr::null_mut();
        }
    };

    let config = Configuration::new(
        convert_event_classes(&config.disable_unsol_classes()),
        convert_event_classes(&config.enable_unsol_classes()),
        convert_classes(&config.startup_integrity_classes()),
        convert_auto_time_sync(&config.auto_time_sync()),
        RetryStrategy::new(
            config.auto_tasks_retry_strategy.min_delay(),
            config.auto_tasks_retry_strategy.max_delay(),
        ),
        None, // TODO: modify this
    );

    let handler = AssociationHandlerAdapter {
        integrity_handler: handlers.integrity_handler,
        unsolicited_handler: handlers.unsolicited_handler,
        default_poll_handler: handlers.default_poll_handler,
        time_provider,
    };

    if tokio::runtime::Handle::try_current().is_err() {
        if let Ok(handle) = master.runtime.block_on(master.handle.add_association(
            address,
            config,
            Box::new(handler),
        )) {
            let association = Association {
                runtime: master.runtime.clone(),
                handle,
            };
            Box::into_raw(Box::new(association))
        } else {
            log::warn!("Associate creation failure");
            std::ptr::null_mut()
        }
    } else {
        log::warn!("Tried calling 'master_add_association' from within a tokio thread");
        std::ptr::null_mut()
    }
}

pub unsafe fn master_set_decode_log_level(master: *mut Master, level: ffi::DecodeLogLevel) {
    if let Some(master) = master.as_mut() {
        master
            .runtime
            .spawn(master.handle.set_decode_log_level(level.into()));
    }
}

pub unsafe fn master_get_decode_log_level(master: *mut Master) -> ffi::DecodeLogLevel {
    if tokio::runtime::Handle::try_current().is_err() {
        if let Some(master) = master.as_mut() {
            if let Ok(level) = master
                .runtime
                .block_on(master.handle.get_decode_log_level())
            {
                return level.into();
            }
        }
    } else {
        log::warn!("Tried calling 'master_get_decode_log_level' from within a tokio thread");
    }

    ffi::DecodeLogLevel::Nothing
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
        ffi::AutoTimeSync::LAN => Some(TimeSyncProcedure::LAN),
        ffi::AutoTimeSync::NonLAN => Some(TimeSyncProcedure::NonLAN),
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
