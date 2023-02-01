use std::sync::atomic::Ordering;
use std::sync::Arc;

pub(crate) struct Canceler {
    inner: Arc<std::sync::atomic::AtomicBool>,
}

#[derive(Clone)]
pub(crate) struct Canceled {
    inner: Arc<std::sync::atomic::AtomicBool>,
}

impl Canceled {
    pub(crate) fn get(&self) -> bool {
        self.inner.load(Ordering::Relaxed)
    }
}

impl Canceler {
    pub(crate) fn cancel(&self) {
        self.inner.store(true, Ordering::Relaxed)
    }
}

pub(crate) fn tokens() -> (Canceler, Canceled) {
    let value = Arc::new(std::sync::atomic::AtomicBool::new(false));
    (
        Canceler {
            inner: value.clone(),
        },
        Canceled { inner: value },
    )
}
