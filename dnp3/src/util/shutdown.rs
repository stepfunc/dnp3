// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! A sticky shutdown signal: requested explicitly, or by dropping every requesting half.

/// Create a linked requesting half and observing half
pub(crate) fn shutdown_signal() -> (ShutdownSignal, ShutdownListener) {
    let (tx, rx) = tokio::sync::watch::channel(false);
    (ShutdownSignal { inner: tx }, ShutdownListener { inner: rx })
}

/// The requesting half. Shutdown is requested by [`ShutdownSignal::shutdown`], or when every
/// clone has been dropped.
#[derive(Clone)]
pub(crate) struct ShutdownSignal {
    inner: tokio::sync::watch::Sender<bool>,
}

impl ShutdownSignal {
    /// Request shutdown. Calling it again does nothing.
    pub(crate) fn shutdown(&self) {
        // plain `send` loses the value when no listener exists, and `send_replace` would wake
        // listeners again on every call
        self.inner.send_if_modified(|requested| {
            let changed = !*requested;
            *requested = true;
            changed
        });
    }

    /// True if [`ShutdownSignal::shutdown`] was called on any clone
    pub(crate) fn is_shutdown(&self) -> bool {
        *self.inner.borrow()
    }

    /// Create another observing half
    pub(crate) fn listener(&self) -> ShutdownListener {
        ShutdownListener {
            inner: self.inner.subscribe(),
        }
    }
}

/// The observing half. Clones observe the same signal and don't keep it alive.
#[derive(Clone)]
pub(crate) struct ShutdownListener {
    inner: tokio::sync::watch::Receiver<bool>,
}

impl ShutdownListener {
    /// Wait until shutdown is requested, or every requesting half is dropped
    pub(crate) async fn wait(&mut self) {
        // `Err` means every requesting half is gone, which also means stop. `changed()` would
        // wait past a value it has already seen.
        drop(self.inner.wait_for(|requested| *requested).await);
    }

    /// True if shutdown was requested, or every requesting half is dropped
    pub(crate) fn is_shutdown(&self) -> bool {
        // the value alone misses a closure whose last value was `false`
        *self.inner.borrow() || self.inner.is_closed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_support::watchdog;

    // catches a wait built on `changed()`, which never returns for a value it has already seen
    #[tokio::test]
    async fn wait_returns_again_for_a_shutdown_it_has_already_seen() {
        watchdog(async {
            let (signal, mut listener) = shutdown_signal();
            signal.shutdown();
            listener.wait().await;
            listener.wait().await;
        })
        .await;
    }

    // catches `send`, which drops the value when no listener exists
    #[test]
    fn shutdown_requested_with_no_listener_is_kept() {
        let (signal, listener) = shutdown_signal();
        drop(listener);
        signal.shutdown();
        assert!(signal.is_shutdown());
        assert!(signal.listener().is_shutdown());
    }

    // catches a check of the value alone, and treating one dropped clone as shutdown
    #[tokio::test]
    async fn dropping_every_requesting_half_is_shutdown() {
        watchdog(async {
            let (signal, mut listener) = shutdown_signal();
            let clone = signal.clone();
            drop(signal);
            assert!(!listener.is_shutdown());
            drop(clone);
            assert!(listener.is_shutdown());
            listener.wait().await;
        })
        .await;
    }
}
