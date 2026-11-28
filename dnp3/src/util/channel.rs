// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::Shutdown;
use crate::util::shutdown::{shutdown_signal, ShutdownListener, ShutdownSignal};

pub(crate) struct Receiver<T> {
    inner: tokio::sync::mpsc::Receiver<T>,
}

pub(crate) struct Sender<T> {
    inner: tokio::sync::mpsc::Sender<T>,
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> std::fmt::Debug for Sender<T> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.debug_struct("Sender")
            .field("inner", &self.inner)
            .finish()
    }
}

pub(crate) fn request_channel<T>() -> (Sender<T>, Receiver<T>) {
    let (tx, rx) = tokio::sync::mpsc::channel(16); // default size for all request channels
    (Sender::new(tx), Receiver::new(rx))
}

impl<T> Receiver<T> {
    fn new(inner: tokio::sync::mpsc::Receiver<T>) -> Self {
        Receiver { inner }
    }

    pub(crate) async fn receive(&mut self) -> Result<T, Shutdown> {
        match self.inner.recv().await {
            Some(x) => Ok(x),
            None => Err(Shutdown),
        }
    }
}

impl<T> Sender<T> {
    fn new(inner: tokio::sync::mpsc::Sender<T>) -> Self {
        Sender { inner }
    }

    pub(crate) async fn send(&self, value: T) -> Result<(), Shutdown> {
        self.inner.send(value).await.map_err(|_| Shutdown)
    }

    /// True once the receiver is closed or dropped
    pub(crate) fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }

    /// Wait until the receiver is closed or dropped
    pub(crate) async fn closed(&self) {
        self.inner.closed().await
    }
}

/// Create a queue from API handles to a task, with a shutdown signal
///
/// Shutdown is requested by [`RequestSender::shutdown`] on any clone, or by dropping every clone.
pub(crate) fn request_queue<T>() -> (RequestSender<T>, RequestReceiver<T>) {
    let (tx, rx) = tokio::sync::mpsc::channel(16); // default size for all request channels
    let (signal, listener) = shutdown_signal();
    (
        RequestSender { inner: tx, signal },
        RequestReceiver {
            inner: rx,
            listener,
        },
    )
}

/// Sending half of [`request_queue`]
pub(crate) struct RequestSender<T> {
    inner: tokio::sync::mpsc::Sender<T>,
    signal: ShutdownSignal,
}

/// Receiving half of [`request_queue`], owned by the task
pub(crate) struct RequestReceiver<T> {
    inner: tokio::sync::mpsc::Receiver<T>,
    listener: ShutdownListener,
}

impl<T> Clone for RequestSender<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            signal: self.signal.clone(),
        }
    }
}

impl<T> std::fmt::Debug for RequestSender<T> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.debug_struct("RequestSender")
            .field("inner", &self.inner)
            .field("shutdown", &self.signal.is_shutdown())
            .finish()
    }
}

impl<T> RequestSender<T> {
    /// Fails once shutdown was requested. A value accepted here is either received or dropped by
    /// [`RequestReceiver::close_and_drain`].
    pub(crate) async fn send(&self, value: T) -> Result<(), Shutdown> {
        // without this, a call made right after `shutdown` returns would be accepted while the
        // task hasn't yet closed its queue
        if self.signal.is_shutdown() {
            return Err(Shutdown);
        }
        self.inner.send(value).await.map_err(|_| Shutdown)
    }

    /// Request shutdown. Calling it again, or after the task has ended, does nothing.
    pub(crate) fn shutdown(&self) {
        self.signal.shutdown();
    }

    /// True once shutdown was requested through any sender
    pub(crate) fn is_shutdown(&self) -> bool {
        self.signal.is_shutdown()
    }

    /// An observer of this queue's shutdown signal
    pub(crate) fn listener(&self) -> ShutdownListener {
        self.signal.listener()
    }
}

impl<T> RequestReceiver<T> {
    /// Receive the next value, or fail once shutdown is requested, even with values still queued
    pub(crate) async fn receive(&mut self) -> Result<T, Shutdown> {
        tokio::select! {
            biased;
            _ = self.listener.wait() => Err(Shutdown),
            x = self.inner.recv() => x.ok_or(Shutdown),
        }
    }

    /// True once shutdown is requested, or every sender is dropped
    pub(crate) fn is_shutdown(&self) -> bool {
        self.listener.is_shutdown()
    }

    /// Wait until shutdown is requested, or every sender is dropped
    pub(crate) async fn wait_for_shutdown(&mut self) {
        self.listener.wait().await
    }

    /// Another observer of this queue's shutdown signal, for waits that don't poll the queue
    pub(crate) fn listener(&self) -> ShutdownListener {
        self.listener.clone()
    }

    /// Close the queue and drop everything in it, including values published by senders that
    /// already held a slot when it closed. Consumes the receiver: nothing can be received
    /// afterwards.
    pub(crate) async fn close_and_drain(mut self) {
        self.inner.close();
        // `recv` rather than `try_recv`: while a slot is outstanding this must yield so its holder
        // can publish, and `try_recv` can block the thread
        while self.inner.recv().await.is_some() {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_support::watchdog;

    // catches a send without the shutdown check: the task hasn't closed its queue yet
    #[tokio::test]
    async fn send_after_shutdown_fails_without_queuing() {
        watchdog(async {
            let (tx, mut rx) = request_queue::<u8>();
            tx.shutdown();
            assert_eq!(tx.send(1).await, Err(Shutdown));
            assert_eq!(
                rx.inner.try_recv(),
                Err(tokio::sync::mpsc::error::TryRecvError::Empty)
            );
        })
        .await;
    }

    // catches a receive that takes queued values before observing shutdown
    #[tokio::test]
    async fn receive_fails_after_shutdown_with_values_queued() {
        watchdog(async {
            let (tx, mut rx) = request_queue::<u8>();
            tx.send(1).await.unwrap();
            tx.shutdown();
            assert_eq!(rx.receive().await, Err(Shutdown));
        })
        .await;
    }

    // catches treating one dropped clone as shutdown
    #[tokio::test]
    async fn dropping_one_sender_leaves_the_others_working() {
        watchdog(async {
            let (tx, mut rx) = request_queue::<u8>();
            let clone = tx.clone();
            drop(tx);
            clone.send(1).await.unwrap();
            assert_eq!(rx.receive().await, Ok(1));
            drop(clone);
            assert_eq!(rx.receive().await, Err(Shutdown));
        })
        .await;
    }

    // catches a drain that doesn't close the queue: the blocked sender would get a slot
    #[tokio::test]
    async fn drain_releases_a_sender_blocked_on_a_full_queue() {
        watchdog(async {
            let (tx, rx) = request_queue::<u8>();
            while tx.inner.try_send(0).is_ok() {}
            let blocked = tokio::spawn({
                let tx = tx.clone();
                async move { tx.send(1).await }
            });
            tokio::task::yield_now().await;
            rx.close_and_drain().await;
            assert_eq!(blocked.await.unwrap(), Err(Shutdown));
        })
        .await;
    }

    // catches a `try_recv` drain, which stops while a slot is outstanding and strands the value
    // published with it
    #[tokio::test]
    async fn drain_drops_a_value_published_with_a_slot_held_across_the_close() {
        watchdog(async {
            let (tx, rx) = request_queue::<tokio::sync::oneshot::Sender<()>>();
            let permit = tx.inner.reserve().await.unwrap();
            let (reply, mut caller) = tokio::sync::oneshot::channel();
            let publish = async move {
                tokio::task::yield_now().await;
                permit.send(reply);
            };
            tokio::join!(rx.close_and_drain(), publish);
            assert_eq!(
                caller.try_recv(),
                Err(tokio::sync::oneshot::error::TryRecvError::Closed)
            );
        })
        .await;
    }
}
