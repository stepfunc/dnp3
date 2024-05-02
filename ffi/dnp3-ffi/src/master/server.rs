use crate::ffi;
use crate::ffi::ParamError;
use dnp3::tcp::{AcceptAction, AcceptConfig};

pub struct AcceptHandler {
    action: Option<AcceptAction>,
}

pub struct IdentifiedLinkHandler {
    action: Option<AcceptConfig>,
}

pub unsafe fn accept_handler_accept(
    instance: *mut AcceptHandler,
    error_mode: ffi::LinkErrorMode,
    config: ffi::MasterChannelConfig,
) -> ParamError {
    unsafe fn inner(
        instance: *mut AcceptHandler,
        error_mode: ffi::LinkErrorMode,
        config: ffi::MasterChannelConfig,
    ) -> Result<(), ParamError> {
        let handler = instance.as_mut().ok_or(ParamError::NullParameter)?;

        handler.action = Some(AcceptAction::Accept(AcceptConfig {
            error_mode: error_mode.into(),
            config: config.try_into()?,
        }));

        Ok(())
    }

    match inner(instance, error_mode, config) {
        Ok(()) => ParamError::Ok,
        Err(err) => err,
    }
}

pub unsafe fn identified_link_handler_accept(
    instance: *mut IdentifiedLinkHandler,
    error_mode: ffi::LinkErrorMode,
    config: ffi::MasterChannelConfig,
) -> ParamError {
    unsafe fn inner(
        instance: *mut IdentifiedLinkHandler,
        error_mode: ffi::LinkErrorMode,
        config: ffi::MasterChannelConfig,
    ) -> Result<(), ParamError> {
        let handler = instance.as_mut().ok_or(ParamError::NullParameter)?;

        handler.action = Some(AcceptConfig {
            error_mode: error_mode.into(),
            config: config.try_into()?,
        });

        Ok(())
    }

    match inner(instance, error_mode, config) {
        Ok(()) => ParamError::Ok,
        Err(err) => err,
    }
}

pub unsafe fn accept_handler_get_link_identity(instance: *mut AcceptHandler) -> ParamError {
    let handler = match instance.as_mut() {
        None => return ParamError::NullParameter,
        Some(x) => x,
    };

    handler.action = Some(AcceptAction::GetLinkIdentity);

    ParamError::Ok
}
