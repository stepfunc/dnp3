use std::ptr::null_mut;

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
    pub(crate) fn unwrap(&self) -> std::sync::Arc<tokio::runtime::Runtime> {
        self.inner
            .upgrade()
            .expect("Runtime has been destroyed, RuntimeHandle is invalid")
    }

    pub(crate) fn get(&self) -> Option<std::sync::Arc<tokio::runtime::Runtime>> {
        self.inner.upgrade()
    }
}

fn build_runtime<F>(f: F) -> std::result::Result<tokio::runtime::Runtime, std::io::Error>
where
    F: Fn(&mut tokio::runtime::Builder) -> &mut tokio::runtime::Builder,
{
    let mut builder = tokio::runtime::Builder::new_multi_thread();
    f(&mut builder).enable_all().build()
}

pub(crate) unsafe fn runtime_new(config: ffi::RuntimeConfig) -> *mut crate::runtime::Runtime {

    let num_threads = if config.num_core_threads <= 0 {
        num_cpus::get()
    } else {
        config.num_core_threads as usize
    };

    tracing::info!("creating runtime with {} threads", num_threads);
    let result = build_runtime(|r| r.worker_threads(num_threads as usize));

    match result {
        Ok(r) => Box::into_raw(Box::new(Runtime::new(r))),
        Err(err) => {
            tracing::error!("Unable to build runtime: {}", err);
            null_mut()
        }
    }
}

pub(crate) unsafe fn runtime_destroy(runtime: *mut crate::runtime::Runtime) {
    if !runtime.is_null() {
        Box::from_raw(runtime);
    };
}
