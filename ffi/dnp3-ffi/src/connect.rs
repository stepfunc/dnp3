// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::ffi::ParamError;
use std::net::SocketAddr;
use std::str::FromStr;

#[derive(Default)]
pub struct ConnectOptions {
    pub(crate) inner: dnp3::tcp::ConnectOptions,
}

pub(crate) fn connect_options_create() -> *mut ConnectOptions {
    Box::into_raw(Box::default())
}

pub(crate) unsafe fn connect_options_destroy(instance: *mut ConnectOptions) {
    if !instance.is_null() {
        drop(Box::from_raw(instance));
    }
}

pub(crate) unsafe fn connect_options_set_timeout(
    instance: *mut ConnectOptions,
    timeout: std::time::Duration,
) {
    let options = match instance.as_mut() {
        Some(x) => x,
        None => return,
    };

    options.inner.set_connect_timeout(timeout);
}

pub(crate) unsafe fn connect_options_set_local_endpoint(
    instance: *mut crate::ConnectOptions,
    endpoint: &std::ffi::CStr,
) -> Result<(), ParamError> {
    let options = match instance.as_mut() {
        Some(x) => x,
        None => return Err(ParamError::NullParameter),
    };

    let addr: SocketAddr = SocketAddr::from_str(&endpoint.to_string_lossy())
        .map_err(|_| ParamError::InvalidSocketAddress)?;

    options.inner.set_local_endpoint(addr);

    Ok(())
}
