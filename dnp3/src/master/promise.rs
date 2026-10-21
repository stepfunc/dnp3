/// A reply that's sent once, if anyone is waiting for it
pub(crate) struct Promise<T> {
    reply: Option<tokio::sync::oneshot::Sender<T>>,
}

impl<T> Promise<T> {
    pub(crate) fn null() -> Self {
        Self { reply: None }
    }

    pub(crate) fn one_shot() -> (Self, tokio::sync::oneshot::Receiver<T>) {
        let (tx, rx) = tokio::sync::oneshot::channel();
        (Self { reply: Some(tx) }, rx)
    }

    pub(crate) fn complete(mut self, value: T) {
        if let Some(reply) = self.reply.take() {
            reply.send(value).ok();
        }
    }
}
