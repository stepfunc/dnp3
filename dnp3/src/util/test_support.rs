//! Frames and listeners shared by the tests of the TCP, TLS, serial and UDP channels

use std::fmt::Debug;
use std::future::Future;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::time::Instant;

use crate::app::{Listener, MaybeAsync};

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

/// the link header of unconfirmed user data from outstation 1024 to master 1 holding 5 bytes
pub(crate) const NULL_UNSOLICITED_HEADER: [u8; 10] =
    [0x05, 0x64, 0x0A, 0x44, 0x01, 0x00, 0x00, 0x04, 0x67, 0x88];

/// How long a test may run in wall-clock time before [`watchdog`] fails it
const WATCHDOG: Duration = Duration::from_secs(30);

/// Run a test body, failing it if it takes longer than [`WATCHDOG`] of wall-clock time
///
/// A paused clock can't bound a test: it only advances to pending timers, so a test waiting for
/// something that never happens would hang. A thread outside the runtime measures real time, and
/// it isn't a timer, so it doesn't change when a paused clock advances.
pub(crate) async fn watchdog(test: impl Future<Output = ()>) {
    let (expired_tx, expired_rx) = tokio::sync::oneshot::channel::<()>();
    let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
    std::thread::spawn(move || {
        if let Err(std::sync::mpsc::RecvTimeoutError::Timeout) = done_rx.recv_timeout(WATCHDOG) {
            // the test may have finished meanwhile and dropped the receiver
            let _: Result<(), ()> = expired_tx.send(());
        }
    });
    tokio::select! {
        _ = test => {}
        _ = expired_rx => panic!("the test didn't finish within {WATCHDOG:?} of wall-clock time"),
    }
    drop(done_tx);
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
