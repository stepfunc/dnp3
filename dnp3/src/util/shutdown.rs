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
