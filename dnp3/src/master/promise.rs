/// A generic callback type that must be invoked once and only once.
/// The user can select to implement it using FnOnce or a
/// one-shot reply channel
pub(crate) enum Promise<T> {
    /// nothing happens when the promise is completed
    None,
    /// one-shot reply channel is consumed when the promise is completed
    OneShot(tokio::sync::oneshot::Sender<T>),
}

impl<T> Promise<T> {
    pub(crate) fn one_shot() -> (Self, tokio::sync::oneshot::Receiver<T>) {
        let (tx, rx) = tokio::sync::oneshot::channel();
        (Self::OneShot(tx), rx)
    }

    pub(crate) fn complete(self, value: T) {
        match self {
            Promise::None => {}
            Promise::OneShot(s) => {
                s.send(value).ok();
            }
        }
    }
}
