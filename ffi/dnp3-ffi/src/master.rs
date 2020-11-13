use crate::association::Association;
use crate::ffi;

use dnp3::master::association::Configuration;
use dnp3::master::handle::{AssociationHandler, MasterHandle, ReadHandler};
use dnp3::master::request::{EventClasses, TimeSyncProcedure};

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
) -> *mut Association {
    let master = match master.as_mut() {
        Some(master) => master,
        None => return std::ptr::null_mut(),
    };

    let config = Configuration::new(
        convert_event_classes(&config.disable_unsol_classes()),
        convert_event_classes(&config.enable_unsol_classes()),
        convert_auto_time_sync(&config.auto_time_sync()),
    );

    let handler = AssociationHandlerAdapter {
        integrity_handler: handlers.integrity_handler,
        unsolicited_handler: handlers.unsolicited_handler,
        default_poll_handler: handlers.default_poll_handler,
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

fn convert_event_classes(config: &ffi::EventClasses) -> EventClasses {
    EventClasses::new(config.class1, config.class2, config.class3)
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
}

impl AssociationHandler for AssociationHandlerAdapter {
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
