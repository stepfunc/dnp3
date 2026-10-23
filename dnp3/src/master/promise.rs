// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

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
