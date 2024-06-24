pub(crate) type CallbackType<T> = Box<dyn FnOnce(T) + Send + Sync + 'static>;

/// A generic callback type that must be invoked once and only once.
/// The user can select to implement it using FnOnce or a
/// one-shot reply channel
enum Inner<T> {
    /// one-shot reply channel is consumed when the promise is completed
    OneShot(tokio::sync::oneshot::Sender<T>),
    /// Boxed FnOnce
    #[allow(dead_code)]
    CallBack(CallbackType<T>, T),
}

pub(crate) struct Promise<T> {
    inner: Option<Inner<T>>,
}

impl<T> Promise<T> {
    pub(crate) fn null() -> Self {
        Self { inner: None }
    }

    fn new(inner: Inner<T>) -> Self {
        Self { inner: Some(inner) }
    }

    pub(crate) fn one_shot() -> (Self, tokio::sync::oneshot::Receiver<T>) {
        let (tx, rx) = tokio::sync::oneshot::channel();
        (Self::new(Inner::OneShot(tx)), rx)
    }

    pub(crate) fn complete(mut self, value: T) {
        if let Some(x) = self.inner.take() {
            match x {
                Inner::OneShot(s) => {
                    s.send(value).ok();
                }
                Inner::CallBack(cb, _) => cb(value),
            }
        }
    }
}

impl<T> Drop for Promise<T> {
    fn drop(&mut self) {
        if let Some(x) = self.inner.take() {
            match x {
                Inner::OneShot(_) => {}
                Inner::CallBack(cb, default) => {
                    cb(default);
                }
            }
        }
    }
}
