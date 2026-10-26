// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Test support shared across the crate: the configuration every property runs with
//! ([`proptest_config`]), the wall-clock watchdog and the paused runtime, polling a future once,
//! link frames, outstation and TLS configurations, and timelines of the states a listener reports

use std::fmt::Debug;
use std::future::Future;
use std::io::Write;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::time::Instant;

use crate::app::{Listener, MaybeAsync};
use crate::link::EndpointAddress;
use crate::outstation::database::EventBufferConfig;
use crate::outstation::{OutstationApplication, OutstationConfig, OutstationInformation};

/// request link status from outstation 1024 to master 1, and the master's reply
pub(crate) const OUTSTATION_REQUEST: [u8; 10] =
    [0x05, 0x64, 0x05, 0x49, 0x01, 0x00, 0x00, 0x04, 0xD2, 0x36];
pub(crate) const MASTER_REPLY: [u8; 10] =
    [0x05, 0x64, 0x05, 0x8B, 0x00, 0x04, 0x01, 0x00, 0x2E, 0xF7];

/// request link status from master 1 to outstation 1024, and the outstation's reply
pub(crate) const MASTER_REQUEST: [u8; 10] =
    [0x05, 0x64, 0x05, 0xC9, 0x00, 0x04, 0x01, 0x00, 0x98, 0x81];
pub(crate) const OUTSTATION_REPLY: [u8; 10] =
    [0x05, 0x64, 0x05, 0x0B, 0x01, 0x00, 0x00, 0x04, 0x64, 0x40];

/// an ACK from outstation 1024 to master 1
pub(crate) const ACK_FROM_OUTSTATION: [u8; 10] =
    [0x05, 0x64, 0x05, 0x00, 0x01, 0x00, 0x00, 0x04, 0x27, 0x70];

/// the link header of unconfirmed user data from outstation 1024 to master 1 holding 5 bytes
pub(crate) const NULL_UNSOLICITED_HEADER: [u8; 10] =
    [0x05, 0x64, 0x0A, 0x44, 0x01, 0x00, 0x00, 0x04, 0x67, 0x88];

/// An outstation application that keeps every default
pub(crate) struct NullApplication;
impl OutstationApplication for NullApplication {}

/// Outstation information that ignores every notification
pub(crate) struct NullInformation;
impl OutstationInformation for NullInformation {}

/// The configuration of outstation `outstation` answering master `master`, with no event buffers
pub(crate) fn outstation_config(outstation: u16, master: u16) -> OutstationConfig {
    OutstationConfig::new(
        EndpointAddress::try_new(outstation).unwrap(),
        EndpointAddress::try_new(master).unwrap(),
        EventBufferConfig::all_types(0),
    )
}

/// The self-signed certificates in `certs/self_signed`: the client (master, or outstation client)
/// holds entity1's key and expects entity2's certificate, and the server the reverse
#[cfg(feature = "enable-tls")]
pub(crate) mod tls {
    use std::path::PathBuf;

    use sfio_rustls_config::ProtocolVersions;
    use tokio_rustls::rustls;

    use crate::tcp::tls::{MinTlsVersion, TlsClientConfig, TlsServerConfig};

    /// The path of a file in the self-signed certificate directory
    pub(crate) fn cert(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../certs/self_signed")
            .join(name)
    }

    /// The path of a file in the certificate authority directory
    pub(crate) fn ca_chain(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../certs/ca_chain")
            .join(name)
    }

    pub(crate) fn client_config(min: MinTlsVersion) -> TlsClientConfig {
        TlsClientConfig::self_signed(
            &cert("entity2_cert.pem"),
            &cert("entity1_cert.pem"),
            &cert("entity1_key.pem"),
            None,
            min,
        )
        .unwrap()
    }

    pub(crate) fn server_config(min: MinTlsVersion) -> TlsServerConfig {
        TlsServerConfig::self_signed(
            &cert("entity1_cert.pem"),
            &cert("entity2_cert.pem"),
            &cert("entity2_key.pem"),
            None,
            min,
        )
        .unwrap()
    }

    /// The client of [`client_config`], built with rustls directly instead of this crate's
    /// configuration
    pub(crate) fn rustls_client(versions: ProtocolVersions) -> rustls::ClientConfig {
        sfio_rustls_config::client::self_signed(
            versions,
            &cert("entity2_cert.pem"),
            &cert("entity1_cert.pem"),
            &cert("entity1_key.pem"),
            None,
        )
        .unwrap()
    }

    /// The server of [`server_config`], built with rustls directly instead of this crate's
    /// configuration
    pub(crate) fn rustls_server(versions: ProtocolVersions) -> rustls::ServerConfig {
        sfio_rustls_config::server::self_signed(
            versions,
            &cert("entity1_cert.pem"),
            &cert("entity2_cert.pem"),
            &cert("entity2_key.pem"),
            None,
        )
        .unwrap()
    }
}

/// The configuration of a property that runs `cases` cases
///
/// Every `proptest!` block in the crate uses this, so the number a property passes is its budget
/// (unless `PROPTEST_CASES` is set, which proptest applies to every property).
///
/// Ordinary runs keep proptest's failure persistence: a failing property saves its seed under
/// `dnp3/proptest-regressions/` (never committed), and later runs replay that seed before any new
/// case until the file is deleted. A failure a property finds becomes an example test.
///
/// Setting proptest's `PROPTEST_DISABLE_FAILURE_PERSISTENCE` (to any value) turns persistence off:
/// saved seeds are neither replayed nor written. Runs that measure a property, such as counting how
/// often it reaches a case or whether it catches a mutation, set it and leave `PROPTEST_CASES`
/// unset, because a saved seed, perhaps found under another mutation, would otherwise be replayed
/// before any new case and make the property look stronger than it is.
///
/// ```text
/// PROPTEST_DISABLE_FAILURE_PERSISTENCE=1 cargo test -p dnp3 --lib -- <property>
/// ```
pub(crate) fn proptest_config(cases: u32) -> proptest::test_runner::Config {
    proptest::test_runner::Config::with_cases(cases)
}

/// Poll a future once with a waker that does nothing
pub(crate) fn poll_once<F: Future>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

/// How long a test may run in wall-clock time before [`watchdog`] fails it
const WATCHDOG: Duration = Duration::from_secs(30);

/// How long a test may take to fail after [`WATCHDOG`] expires before the process is aborted
const ABORT_GRACE: Duration = Duration::from_secs(5);

/// Run a test body, failing it if it takes longer than [`WATCHDOG`] of wall-clock time
///
/// A paused clock can't bound a test: it only advances to pending timers, so a test waiting for
/// something that never happens would hang. A thread outside the runtime measures real time, and
/// it isn't a timer, so it doesn't change when a paused clock advances.
///
/// The test fails by panicking when the runtime next polls it. A test that never yields, such as
/// one busy-looping on a current-thread runtime, is never polled again, so the thread aborts the
/// process if the test still hasn't finished [`ABORT_GRACE`] after the watchdog expired.
pub(crate) async fn watchdog<T>(test: impl Future<Output = T>) -> T {
    let name = std::thread::current()
        .name()
        .unwrap_or("<unnamed>")
        .to_owned();
    let (expired_tx, expired_rx) = tokio::sync::oneshot::channel::<()>();
    let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
    let settled = Arc::new(AtomicBool::new(false));
    let _finished = Finished {
        settled: settled.clone(),
        _wake: done_tx,
    };
    std::thread::spawn(move || {
        use std::sync::mpsc::RecvTimeoutError;
        if let Err(RecvTimeoutError::Timeout) = done_rx.recv_timeout(WATCHDOG) {
            // the test may have finished meanwhile and dropped the receiver
            let _: Result<(), ()> = expired_tx.send(());
            if let Err(RecvTimeoutError::Timeout) = done_rx.recv_timeout(ABORT_GRACE) {
                // the test may finish after the timeout, so only abort if it hasn't settled first
                if settled.swap(true, Ordering::SeqCst) {
                    return;
                }
                // the test harness captures eprintln!, and the capture is lost when aborting
                drop(writeln!(
                    std::io::stderr(),
                    "test {name} didn't finish within {WATCHDOG:?} of wall-clock time, \
                     and didn't yield to fail within {ABORT_GRACE:?} more; aborting"
                ));
                std::process::abort();
            }
        }
    });
    tokio::select! {
        output = test => output,
        _ = expired_rx => panic!("the test didn't finish within {WATCHDOG:?} of wall-clock time"),
    }
}

/// Run a test on a current-thread runtime with the clock paused, under a [`watchdog`]
pub(crate) fn run_paused<T>(test: impl Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .unwrap()
        .block_on(watchdog(test))
}

/// Settles a [`watchdog`] when the test finishes, panics or is dropped, so the watchdog thread
/// can't abort the process afterwards, then wakes the thread by dropping the sender
struct Finished {
    settled: Arc<AtomicBool>,
    _wake: std::sync::mpsc::Sender<()>,
}

impl Drop for Finished {
    fn drop(&mut self) {
        self.settled.store(true, Ordering::SeqCst);
    }
}

struct TimedListener<T>(mpsc::UnboundedSender<(Instant, T)>);

impl<T: Send + Sync> Listener<T> for TimedListener<T> {
    fn update(&mut self, value: T) -> MaybeAsync<()> {
        // the test may have finished and dropped the receiver
        drop(self.0.send((Instant::now(), value)));
        MaybeAsync::ready(())
    }
}

/// The states reported to a listener, each with the time since the timeline was created
///
/// Waits have no timeout: with the clock paused, a pending timer makes the clock jump ahead
/// whenever every task is waiting, including on I/O.
pub(crate) struct Timeline<T> {
    start: Instant,
    states: mpsc::UnboundedReceiver<(Instant, T)>,
}

impl<T: Debug + PartialEq + Send + Sync + 'static> Timeline<T> {
    pub(crate) fn new() -> (Box<dyn Listener<T>>, Self) {
        let (tx, rx) = mpsc::unbounded_channel();
        let timeline = Self {
            start: Instant::now(),
            states: rx,
        };
        (Box::new(TimedListener(tx)), timeline)
    }

    /// The next state, and when it was reported
    pub(crate) async fn next(&mut self) -> (Duration, T) {
        let (time, state) = self.states.recv().await.expect("the listener was dropped");
        (time.duration_since(self.start), state)
    }

    /// Expect the next state to be `state`, reported at `at`
    pub(crate) async fn expect(&mut self, at: Duration, state: T) {
        assert_eq!(self.next().await, (at, state));
    }

    /// Expect the next state to be `state` reported at any time, and return the time
    pub(crate) async fn expect_state(&mut self, state: T) -> Duration {
        let (at, next) = self.next().await;
        assert_eq!(next, state);
        at
    }

    /// Check that no state was reported since the last one
    pub(crate) fn assert_quiet(&mut self) {
        let next = self.states.try_recv().ok();
        assert_eq!(next.map(|(_, state)| state), None);
    }

    /// The time since the timeline was created
    pub(crate) fn now(&self) -> Duration {
        self.start.elapsed()
    }
}

/// Put `new` before the item at `index`, like `Vec::splice(index..index, new)`
pub(crate) fn insert_at<T>(items: &mut Vec<T>, index: usize, new: impl IntoIterator<Item = T>) {
    assert!(index <= items.len(), "index {index} is past the end");
    let mut old = std::mem::take(items).into_iter();
    items.extend(old.by_ref().take(index));
    items.extend(new);
    items.extend(old);
}

/// Remove and return the item at `index`, like `Vec::remove`
pub(crate) fn remove_at<T>(items: &mut Vec<T>, index: usize) -> T {
    let mut old = std::mem::take(items).into_iter();
    items.extend(old.by_ref().take(index));
    let removed = old.next().expect("index is in range");
    items.extend(old);
    removed
}
