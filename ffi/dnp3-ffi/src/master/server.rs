use crate::ffi;
use crate::ffi::ParamError;
use dnp3::app::Timeout;
use dnp3::master::MasterChannel;
use dnp3::tcp::{AcceptAction, AcceptConfig, LinkIdConfig, Reject, ServerHandle};
use std::ffi::{CStr, CString};
use std::net::SocketAddr;
use std::num::NonZeroUsize;

pub struct MasterServer {
    _handle: ServerHandle,
}

pub struct AcceptHandler {
    action: Option<AcceptAction>,
}

pub struct IdentifiedLinkHandler {
    action: Option<AcceptConfig>,
}

pub(crate) unsafe fn master_server_destroy(instance: *mut MasterServer) {
    if !instance.is_null() {
        drop(Box::from_raw(instance));
    }
}

pub(crate) unsafe fn create_master_tcp_server(
    runtime: *mut crate::Runtime,
    local_addr: &CStr,
    link_id_config: ffi::LinkIdConfig,
    connection_handler: ffi::ConnectionHandler,
) -> Result<*mut MasterServer, ParamError> {
    let runtime = runtime.as_ref().ok_or(ParamError::NullParameter)?;
    let local_addr: SocketAddr = local_addr.to_str()?.parse()?;
    let link_id_config: LinkIdConfig = link_id_config.into();
    let future = dnp3::tcp::spawn_master_tcp_server(local_addr, link_id_config, connection_handler);
    let handle = runtime.handle().block_on(future)??;
    Ok(Box::into_raw(Box::new(MasterServer { _handle: handle })))
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

impl From<ffi::LinkIdConfig> for LinkIdConfig {
    fn from(value: ffi::LinkIdConfig) -> Self {
        LinkIdConfig::new()
            .max_tasks(
                NonZeroUsize::new(value.max_tasks.into()).unwrap_or(NonZeroUsize::new(1).unwrap()),
            )
            .timeout(Timeout::saturating(value.timeout()))
            .decode_level(value.decode_level().into())
    }
}

impl dnp3::tcp::ConnectionHandler for ffi::ConnectionHandler {
    async fn accept(&mut self, addr: SocketAddr) -> Result<AcceptAction, Reject> {
        let mut handler = AcceptHandler { action: None };
        let addr = CString::new(addr.to_string()).map_err(|_| Reject)?;
        Self::accept(self, &addr, &mut handler);
        match handler.action {
            None => Err(Reject),
            Some(x) => Ok(x),
        }
    }

    async fn start(&mut self, channel: MasterChannel, addr: SocketAddr) {
        let addr = match CString::new(addr.to_string()) {
            Ok(x) => x,
            Err(_) => return,
        };

        let runtime = tokio::runtime::Handle::current();

        let channel = crate::MasterChannel {
            runtime: crate::RuntimeHandle::new(runtime),
            handle: channel,
        };

        Self::start(self, &addr, Box::into_raw(Box::new(channel)));
    }

    async fn accept_link_id(
        &mut self,
        addr: SocketAddr,
        source: u16,
        destination: u16,
    ) -> Result<AcceptConfig, Reject> {
        let mut handler = IdentifiedLinkHandler { action: None };
        let addr = CString::new(addr.to_string()).map_err(|_| Reject)?;
        Self::accept_with_link_id(self, &addr, source, destination, &mut handler);
        match handler.action {
            None => Err(Reject),
            Some(x) => Ok(x),
        }
    }

    async fn start_with_link_id(
        &mut self,
        channel: MasterChannel,
        addr: SocketAddr,
        source: u16,
        destination: u16,
    ) {
        let addr = match CString::new(addr.to_string()) {
            Ok(x) => x,
            Err(_) => return,
        };

        let runtime = tokio::runtime::Handle::current();

        let channel = crate::MasterChannel {
            runtime: crate::RuntimeHandle::new(runtime),
            handle: channel,
        };

        Self::start_with_link_id(
            self,
            &addr,
            source,
            destination,
            Box::into_raw(Box::new(channel)),
        );
    }
}
