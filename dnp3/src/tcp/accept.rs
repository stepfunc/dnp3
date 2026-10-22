use std::future::Future;
use std::net::SocketAddr;
use std::time::Duration;

use tokio::net::{TcpListener, TcpStream};
use tokio::time::Instant;

/// How long a server waits before accepting again after `accept()` fails
pub(crate) const ACCEPT_ERROR_DELAY: Duration = Duration::from_secs(1);

/// Source of accepted TCP connections
pub(crate) trait Listen: Send {
    fn accept(&mut self) -> impl Future<Output = std::io::Result<(TcpStream, SocketAddr)>> + Send;
}

impl Listen for TcpListener {
    fn accept(&mut self) -> impl Future<Output = std::io::Result<(TcpStream, SocketAddr)>> + Send {
        TcpListener::accept(self)
    }
}

/// Accepts connections, treating every accept error as transient
pub(crate) struct Acceptor<L> {
    listener: L,
    retry_at: Option<Instant>,
}

impl<L: Listen> Acceptor<L> {
    pub(crate) fn new(listener: L) -> Self {
        Self {
            listener,
            retry_at: None,
        }
    }

    /// Accept the next connection, waiting [`ACCEPT_ERROR_DELAY`] after each error.
    ///
    /// This future is cancel-safe: a pending delay survives cancellation and is still
    /// observed by the next call.
    pub(crate) async fn accept(&mut self) -> (TcpStream, SocketAddr) {
        loop {
            if let Some(retry_at) = self.retry_at {
                tokio::time::sleep_until(retry_at).await;
                self.retry_at = None;
            }
            match self.listener.accept().await {
                Ok(x) => return x,
                Err(err) => {
                    tracing::error!(
                        "accept error, retrying in {} ms: {err}",
                        ACCEPT_ERROR_DELAY.as_millis()
                    );
                    self.retry_at = Instant::now().checked_add(ACCEPT_ERROR_DELAY);
                }
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod test_util {
    use super::*;
    use tokio::sync::mpsc;

    /// Fails the accepts whose entry in `faults` is true, and accepts from a real listener
    /// otherwise and after the last entry.
    ///
    /// Reports the time of every accept attempt.
    pub(crate) struct FaultyListener {
        inner: TcpListener,
        faults: &'static [bool],
        attempts: mpsc::UnboundedSender<Instant>,
    }

    impl FaultyListener {
        pub(crate) async fn new(
            faults: &'static [bool],
        ) -> (Self, mpsc::UnboundedReceiver<Instant>) {
            let inner = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let (attempts, rx) = mpsc::unbounded_channel();
            (
                Self {
                    inner,
                    faults,
                    attempts,
                },
                rx,
            )
        }

        pub(crate) fn local_addr(&self) -> SocketAddr {
            self.inner.local_addr().unwrap()
        }
    }

    impl Listen for FaultyListener {
        async fn accept(&mut self) -> std::io::Result<(TcpStream, SocketAddr)> {
            self.attempts.send(Instant::now()).unwrap();
            if let Some((fail, rest)) = self.faults.split_first() {
                self.faults = rest;
                if *fail {
                    return Err(std::io::Error::other("injected accept error"));
                }
            }
            self.inner.accept().await
        }
    }

    /// Receive the next `count` attempt times, which must each follow the previous by the delay
    pub(crate) async fn expect_attempts_spaced_by_delay(
        attempts: &mut mpsc::UnboundedReceiver<Instant>,
        count: usize,
    ) {
        let mut previous = attempts.recv().await.unwrap();
        for _ in 1..count {
            let next = attempts.recv().await.unwrap();
            assert_eq!(next.duration_since(previous), ACCEPT_ERROR_DELAY);
            previous = next;
        }
    }
}
