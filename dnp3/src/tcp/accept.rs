use std::future::Future;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::time::Duration;

use tokio::net::{TcpListener, TcpStream};
use tokio::time::Instant;

use crate::util::phys::Socket;

/// How long a server waits before accepting again after `accept()` fails for a reason other
/// than the incoming connection itself
pub(crate) const ACCEPT_ERROR_DELAY: Duration = Duration::from_secs(1);

/// Source of accepted TCP connections
pub(crate) trait Listen: Send {
    type Stream: Socket;

    fn accept(
        &mut self,
    ) -> impl Future<Output = std::io::Result<(Self::Stream, SocketAddr)>> + Send;
}

impl Listen for TcpListener {
    type Stream = TcpStream;

    fn accept(&mut self) -> impl Future<Output = std::io::Result<(TcpStream, SocketAddr)>> + Send {
        TcpListener::accept(self)
    }
}

/// When to call `accept()` again after it fails
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Retry {
    /// The error belongs to one incoming connection, which the failed call consumed
    Immediately,
    /// The error is about the listener or the system, e.g. running out of descriptors
    AfterDelay,
}

impl Retry {
    pub(crate) fn after(err: &std::io::Error) -> Self {
        match err.kind() {
            ErrorKind::ConnectionAborted
            | ErrorKind::ConnectionReset
            | ErrorKind::ConnectionRefused
            | ErrorKind::Interrupted => Self::Immediately,
            _ if is_windows_interrupt(err) || is_pending_network_error(err) => Self::Immediately,
            _ => Self::AfterDelay,
        }
    }
}

/// Rust leaves WSAEINTR uncategorized, so `ErrorKind::Interrupted` doesn't cover it
fn is_windows_interrupt(err: &std::io::Error) -> bool {
    const WSAEINTR: i32 = 10004;
    cfg!(windows) && err.raw_os_error() == Some(WSAEINTR)
}

/// Linux returns network errors already pending on the new socket from `accept()`, and
/// accept(2) says to retry them like EAGAIN
#[cfg(any(target_os = "linux", target_os = "android"))]
fn is_pending_network_error(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        ErrorKind::TimedOut
            | ErrorKind::NetworkDown
            | ErrorKind::NetworkUnreachable
            | ErrorKind::HostUnreachable
    ) || err
        .raw_os_error()
        .is_some_and(errno::is_pending_network_error)
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn is_pending_network_error(_: &std::io::Error) -> bool {
    false
}

/// The pending network errors with no stable `ErrorKind`, for the architectures that use the
/// generic Linux errno values (dnp3 doesn't depend on `libc`). Elsewhere they get the delay.
#[cfg(any(target_os = "linux", target_os = "android"))]
mod errno {
    #[cfg(any(
        target_arch = "x86",
        target_arch = "x86_64",
        target_arch = "arm",
        target_arch = "aarch64",
        target_arch = "riscv32",
        target_arch = "riscv64",
        target_arch = "powerpc",
        target_arch = "powerpc64",
        target_arch = "s390x",
        target_arch = "loongarch64"
    ))]
    pub(super) fn is_pending_network_error(code: i32) -> bool {
        const ENONET: i32 = 64;
        const EPROTO: i32 = 71;
        const ENOPROTOOPT: i32 = 92;
        const EOPNOTSUPP: i32 = 95;
        const EHOSTDOWN: i32 = 112;
        matches!(code, ENONET | EPROTO | ENOPROTOOPT | EOPNOTSUPP | EHOSTDOWN)
    }

    #[cfg(not(any(
        target_arch = "x86",
        target_arch = "x86_64",
        target_arch = "arm",
        target_arch = "aarch64",
        target_arch = "riscv32",
        target_arch = "riscv64",
        target_arch = "powerpc",
        target_arch = "powerpc64",
        target_arch = "s390x",
        target_arch = "loongarch64"
    )))]
    pub(super) fn is_pending_network_error(_: i32) -> bool {
        false
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

    /// Accept the next connection. After an error, retry at once if it belonged to one incoming
    /// connection, and otherwise after [`ACCEPT_ERROR_DELAY`].
    ///
    /// This future is cancel-safe: a pending delay survives cancellation and the next call
    /// waits until its original deadline.
    pub(crate) async fn accept(&mut self) -> (L::Stream, SocketAddr) {
        loop {
            if let Some(retry_at) = self.retry_at {
                tokio::time::sleep_until(retry_at).await;
                self.retry_at = None;
            }
            match self.listener.accept().await {
                Ok(x) => return x,
                Err(err) => match Retry::after(&err) {
                    Retry::Immediately => {
                        tracing::info!("failed to accept an incoming connection: {err}");
                        // a stream of errors must not starve the caller's select
                        tokio::task::yield_now().await;
                    }
                    Retry::AfterDelay => {
                        tracing::error!(
                            "accept error, retrying in {} ms: {err}",
                            ACCEPT_ERROR_DELAY.as_millis()
                        );
                        self.retry_at = Instant::now().checked_add(ACCEPT_ERROR_DELAY);
                    }
                },
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod test_util {
    use super::*;
    use std::net::IpAddr;
    use tokio::io::DuplexStream;
    use tokio::sync::mpsc;

    /// What one accept call of a [`FaultyListener`] does
    #[derive(Copy, Clone, Debug)]
    pub(crate) enum Fault {
        /// accept the next connection
        None,
        /// fail with an error of this kind
        Kind(ErrorKind),
        /// fail with this OS error code
        #[cfg_attr(
            not(any(unix, windows)),
            expect(dead_code, reason = "only the unix and windows tests use OS codes")
        )]
        Os(i32),
    }

    impl Fault {
        pub(crate) fn error(self) -> Option<std::io::Error> {
            match self {
                Self::None => None,
                Self::Kind(kind) => Some(std::io::Error::from(kind)),
                Self::Os(code) => Some(std::io::Error::from_raw_os_error(code)),
            }
        }
    }

    /// The capacity of each direction of the pipes between [`Peers`] and a [`FaultyListener`],
    /// which holds a whole TLS flight, so that a peer needn't read while it writes
    pub(crate) const PIPE: usize = 16 * 1024;

    /// The address of a peer on the loopback interface
    pub(crate) const LOOPBACK: IpAddr = IpAddr::V4(std::net::Ipv4Addr::LOCALHOST);

    /// Applies one entry of `faults` per accept, then accepts the in-memory connections that its
    /// [`Peers`] queue, reporting every attempt and every accepted connection
    pub(crate) struct FaultyListener {
        faults: &'static [Fault],
        connections: mpsc::UnboundedReceiver<(DuplexStream, SocketAddr)>,
        attempts: mpsc::UnboundedSender<Instant>,
        accepted: mpsc::UnboundedSender<(Instant, SocketAddr)>,
    }

    /// Connects to a [`FaultyListener`], and observes it
    pub(crate) struct Peers {
        connections: mpsc::UnboundedSender<(DuplexStream, SocketAddr)>,
        last_port: u16,
        /// when the listener attempted each accept, whether it failed or not
        pub(crate) attempts: mpsc::UnboundedReceiver<Instant>,
        /// when the listener accepted each connection, and from which peer
        accepted: mpsc::UnboundedReceiver<(Instant, SocketAddr)>,
    }

    impl FaultyListener {
        pub(crate) fn new(faults: &'static [Fault]) -> (Self, Peers) {
            let (connections_tx, connections) = mpsc::unbounded_channel();
            let (attempts_tx, attempts) = mpsc::unbounded_channel();
            let (accepted_tx, accepted) = mpsc::unbounded_channel();
            (
                Self {
                    faults,
                    connections,
                    attempts: attempts_tx,
                    accepted: accepted_tx,
                },
                Peers {
                    connections: connections_tx,
                    last_port: 0,
                    attempts,
                    accepted,
                },
            )
        }
    }

    impl Peers {
        /// Queue a connection from a new port on `ip`; returns this end of it and the peer
        /// address the listener reports
        pub(crate) fn connect(&mut self, ip: IpAddr) -> (DuplexStream, SocketAddr) {
            self.last_port = self.last_port.wrapping_add(1);
            let addr = SocketAddr::new(ip, self.last_port);
            let (local, remote) = tokio::io::duplex(PIPE);
            self.connections.send((remote, addr)).unwrap();
            (local, addr)
        }

        /// Wait for the listener to accept the next connection, which must be from `peer`, and
        /// return when it did
        pub(crate) async fn accepts(&mut self, peer: SocketAddr) -> Instant {
            let (at, addr) = self.accepted.recv().await.unwrap();
            assert_eq!(addr, peer);
            at
        }
    }

    impl Listen for FaultyListener {
        type Stream = DuplexStream;

        async fn accept(&mut self) -> std::io::Result<(DuplexStream, SocketAddr)> {
            self.attempts.send(Instant::now()).unwrap();
            if let Some((fault, rest)) = self.faults.split_first() {
                self.faults = rest;
                if let Some(err) = fault.error() {
                    return Err(err);
                }
            }
            // receiving is cancel-safe, so an accept that is dropped loses no queued connection
            match self.connections.recv().await {
                Some((stream, addr)) => {
                    // the test may not be watching
                    self.accepted.send((Instant::now(), addr)).ok();
                    Ok((stream, addr))
                }
                // like a listener that nobody connects to any more
                None => std::future::pending().await,
            }
        }
    }

    /// Receive one attempt, and then one more per gap, each following the previous by that gap
    pub(crate) async fn expect_attempt_gaps(
        attempts: &mut mpsc::UnboundedReceiver<Instant>,
        gaps: &[Duration],
    ) {
        let mut previous = attempts.recv().await.unwrap();
        for gap in gaps {
            let next = attempts.recv().await.unwrap();
            assert_eq!(next.duration_since(previous), *gap);
            previous = next;
        }
    }

    /// An unknown error, retried after the delay
    pub(crate) const UNKNOWN: Fault = Fault::Kind(ErrorKind::Other);
    /// A connection error, retried at once
    pub(crate) const RESET: Fault = Fault::Kind(ErrorKind::ConnectionReset);
}

#[cfg(test)]
mod tests {
    use super::test_util::*;
    use super::*;
    use crate::util::test_support::watchdog;

    const D: Duration = ACCEPT_ERROR_DELAY;
    const NOW: Duration = Duration::ZERO;

    fn retry(fault: Fault) -> Retry {
        Retry::after(&fault.error().unwrap())
    }

    #[test]
    fn connection_errors_are_retried_immediately() {
        for kind in [
            ErrorKind::ConnectionAborted,
            ErrorKind::ConnectionReset,
            ErrorKind::ConnectionRefused,
            ErrorKind::Interrupted,
        ] {
            assert_eq!(retry(Fault::Kind(kind)), Retry::Immediately, "{kind:?}");
        }
    }

    #[test]
    fn unknown_and_resource_errors_are_retried_after_the_delay() {
        for kind in [
            ErrorKind::Other,
            ErrorKind::OutOfMemory,
            ErrorKind::WouldBlock,
            ErrorKind::InvalidInput,
        ] {
            assert_eq!(retry(Fault::Kind(kind)), Retry::AfterDelay, "{kind:?}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn unix_resource_errors_are_retried_after_the_delay() {
        const ENOMEM: i32 = 12;
        const ENFILE: i32 = 23;
        const EMFILE: i32 = 24;
        #[cfg(any(target_os = "linux", target_os = "android"))]
        const ENOBUFS: i32 = 105;
        #[cfg(not(any(target_os = "linux", target_os = "android")))]
        const ENOBUFS: i32 = 55;
        for code in [ENOMEM, ENFILE, EMFILE, ENOBUFS] {
            assert_eq!(retry(Fault::Os(code)), Retry::AfterDelay, "{code}");
        }
    }

    #[cfg(all(
        any(target_os = "linux", target_os = "android"),
        any(target_arch = "x86_64", target_arch = "aarch64")
    ))]
    #[test]
    fn linux_pending_network_errors_are_retried_immediately() {
        // ENONET, EPROTO, ENOPROTOOPT, EOPNOTSUPP, ENETDOWN, ENETUNREACH, ETIMEDOUT, EHOSTDOWN,
        // EHOSTUNREACH
        for code in [64, 71, 92, 95, 100, 101, 110, 112, 113] {
            assert_eq!(retry(Fault::Os(code)), Retry::Immediately, "{code}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_errors_are_classified() {
        const WSAEINTR: i32 = 10004;
        const WSAECONNABORTED: i32 = 10053;
        const WSAECONNRESET: i32 = 10054;
        const WSAECONNREFUSED: i32 = 10061;
        const WSAEMFILE: i32 = 10024;
        const WSAENETDOWN: i32 = 10050;
        const WSAENOBUFS: i32 = 10055;
        for code in [WSAEINTR, WSAECONNABORTED, WSAECONNRESET, WSAECONNREFUSED] {
            assert_eq!(retry(Fault::Os(code)), Retry::Immediately, "{code}");
        }
        for code in [WSAEMFILE, WSAENETDOWN, WSAENOBUFS] {
            assert_eq!(retry(Fault::Os(code)), Retry::AfterDelay, "{code}");
        }
    }

    /// Accept once from `faults` with a client waiting, checking the gaps between attempts
    async fn accepts_with_gaps(faults: &'static [Fault], gaps: &[Duration]) {
        let (listener, mut peers) = FaultyListener::new(faults);
        let _client = peers.connect(LOOPBACK);
        let mut acceptor = Acceptor::new(listener);
        acceptor.accept().await;
        expect_attempt_gaps(&mut peers.attempts, gaps).await;
    }

    #[tokio::test(start_paused = true)]
    async fn accepts_at_once_after_a_connection_error() {
        watchdog(accepts_with_gaps(&[RESET], &[NOW])).await;
    }

    #[tokio::test(start_paused = true)]
    async fn accepts_after_the_delay_following_a_resource_error() {
        watchdog(accepts_with_gaps(
            &[Fault::Kind(ErrorKind::OutOfMemory)],
            &[D],
        ))
        .await;
    }

    #[tokio::test(start_paused = true)]
    async fn accepts_after_the_delay_following_an_unknown_error() {
        watchdog(accepts_with_gaps(&[UNKNOWN], &[D])).await;
    }

    #[tokio::test(start_paused = true)]
    async fn mixed_errors_delay_only_after_non_connection_errors() {
        const FAULTS: &[Fault] = &[
            RESET,
            Fault::Kind(ErrorKind::OutOfMemory),
            Fault::Kind(ErrorKind::ConnectionAborted),
            UNKNOWN,
            Fault::Kind(ErrorKind::ConnectionRefused),
            Fault::Kind(ErrorKind::Interrupted),
        ];
        watchdog(accepts_with_gaps(FAULTS, &[NOW, D, NOW, D, NOW, NOW])).await;
    }

    #[tokio::test(start_paused = true)]
    async fn cancelled_delay_keeps_its_original_deadline() {
        watchdog(async {
            let (listener, mut peers) = FaultyListener::new(&[UNKNOWN]);
            let _client = peers.connect(LOOPBACK);
            let mut acceptor = Acceptor::new(listener);

            let cancel_after = Duration::from_millis(400);
            tokio::select! {
                _ = acceptor.accept() => panic!("accepted during the delay"),
                _ = tokio::time::sleep(cancel_after) => {}
            }
            let failed_at = peers.attempts.recv().await.unwrap();
            assert_eq!(failed_at.elapsed(), cancel_after);

            acceptor.accept().await;
            let retried_at = peers.attempts.recv().await.unwrap();
            assert_eq!(retried_at.duration_since(failed_at), D);
        })
        .await;
    }
}
