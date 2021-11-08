use std::future::Future;

use dnp3::app::Shutdown;
use tokio::runtime::Handle;

use crate::ffi;

pub struct Runtime {
    pub(crate) inner: std::sync::Arc<tokio::runtime::Runtime>,
}

impl Runtime {
    fn new(inner: tokio::runtime::Runtime) -> Self {
        Self {
            inner: std::sync::Arc::new(inner),
        }
    }

    pub(crate) fn handle(&self) -> RuntimeHandle {
        RuntimeHandle {
            inner: std::sync::Arc::downgrade(&self.inner),
        }
    }
}

#[derive(Clone)]
pub(crate) struct RuntimeHandle {
    inner: std::sync::Weak<tokio::runtime::Runtime>,
}

impl RuntimeHandle {
    pub(crate) fn block_on<F: Future>(&self, future: F) -> Result<F::Output, ffi::ParamError> {
        let inner = self
            .inner
            .upgrade()
            .ok_or(ffi::ParamError::RuntimeDestroyed)?;
        if Handle::try_current().is_ok() {
            return Err(ffi::ParamError::RuntimeCannotBlockWithinAsync);
        }
        Ok(inner.block_on(future))
    }

    pub(crate) fn spawn<F>(&self, future: F) -> Result<(), ffi::ParamError>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let inner = self
            .inner
            .upgrade()
            .ok_or(ffi::ParamError::RuntimeDestroyed)?;
        inner.spawn(future);
        Ok(())
    }
}

fn build_runtime<F>(f: F) -> std::result::Result<tokio::runtime::Runtime, std::io::Error>
where
    F: Fn(&mut tokio::runtime::Builder) -> &mut tokio::runtime::Builder,
{
    let mut builder = tokio::runtime::Builder::new_multi_thread();
    f(&mut builder).enable_all().build()
}

pub(crate) unsafe fn runtime_create(
    config: ffi::RuntimeConfig,
) -> Result<*mut crate::runtime::Runtime, ffi::ParamError> {
    let num_threads = if config.num_core_threads == 0 {
        num_cpus::get()
    } else {
        config.num_core_threads as usize
    };

    tracing::info!("creating runtime with {} threads", num_threads);
    let runtime = build_runtime(|r| r.worker_threads(num_threads as usize))
        .map_err(|_| ffi::ParamError::RuntimeCreationFailure)?;
    Ok(Box::into_raw(Box::new(Runtime::new(runtime))))
}

pub(crate) unsafe fn runtime_destroy(runtime: *mut crate::runtime::Runtime) {
    if !runtime.is_null() {
        Box::from_raw(runtime);
    };
}

impl From<Shutdown> for ffi::ParamError {
    fn from(_: Shutdown) -> Self {
        ffi::ParamError::MasterAlreadyShutdown
    }
}
