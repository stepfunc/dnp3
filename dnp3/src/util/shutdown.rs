// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

/// Create a shutdown token/listener pair
pub(crate) fn shutdown_token() -> (ShutdownToken, ShutdownListener) {
    let (tx, rx) = tokio::sync::watch::channel(());
    (ShutdownToken { _inner: tx }, ShutdownListener { inner: rx })
}

/// An opaque token, that when dropped, causes every [`ShutdownListener`] to be notified
pub(crate) struct ShutdownToken {
    _inner: tokio::sync::watch::Sender<()>,
}

/// Component which can be used to asynchronously listen for shutdown
#[derive(Clone)]
pub(crate) struct ShutdownListener {
    inner: tokio::sync::watch::Receiver<()>,
}

impl ShutdownListener {
    /// Listen for the paired [`ShutdownToken`] to be dropped
    pub(crate) async fn listen(&mut self) {
        loop {
            match self.inner.changed().await {
                Ok(()) => {}
                // indicates that the sender was dropped
                Err(_) => return,
            }
        }
    }
}
