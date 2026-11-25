// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! State shared by the outstation protocol state machine.

use core::time::Duration;

use super::deferred::DeferredRead;
use crate::app::parse::parser::HeaderCollection;
use crate::app::{Iin1, ObjectParseError, RequestHeader, ResponseHeader, Sequence};
use crate::link::header::BroadcastConfirmMode;
use crate::master::EventClasses;
use crate::outstation::config::Feature;
use crate::outstation::control::select::SelectState;
use crate::transport::{FragmentAddr, FragmentInfo};
use crate::util::clock::MonotonicTime;

/// A control request, with what its function needs to know about the selection.
#[derive(Copy, Clone)]
pub(super) enum ControlRequest {
    Select {
        /// The request's hash, which the selection records to recognize the SELECT's retry.
        request_hash: u64,
    },
    Operate {
        /// The selection that this request ended.
        selection: Option<SelectState>,
    },
    DirectOperate,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(super) struct ResponseSeries {
    pub(super) ecsn: Sequence,
    pub(super) fin: bool,
}

impl ResponseSeries {
    pub(super) fn new(ecsn: Sequence, fin: bool) -> Self {
        Self { ecsn, fin }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(super) struct RetryCounter {
    retries: Option<usize>,
}

impl RetryCounter {
    pub(super) fn new(limit: Option<usize>) -> Self {
        Self { retries: limit }
    }

    pub(super) fn decrement(&mut self) -> bool {
        match self.retries {
            None => true,
            Some(x) => match x.checked_sub(1) {
                Some(remaining) => {
                    self.retries = Some(remaining);
                    true
                }
                None => false,
            },
        }
    }

    /// Allow no further retries.
    pub(super) fn stop(&mut self) {
        self.retries = Some(0);
    }
}

#[derive(Copy, Clone, Debug)]
pub(super) struct LastValidRequest {
    pub(super) seq: Sequence,
    pub(super) request_hash: u64,
}

impl LastValidRequest {
    pub(super) fn new(seq: Sequence, request_hash: u64) -> Self {
        LastValidRequest { seq, request_hash }
    }
}

/// Unsolicited response readiness and configuration state.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(super) enum UnsolicitedState {
    /// Unsolicited responses are disabled by configuration.
    Disabled,
    /// Must send and get confirmation of the startup NULL unsolicited response first.
    NullRequired,
    /// Ready to send unsolicited events, optionally gated by a retry delay.
    Ready { retry_at: Option<MonotonicTime> },
}

impl UnsolicitedState {
    fn from_feature(feature: Feature) -> Self {
        match feature {
            Feature::Disabled => Self::Disabled,
            Feature::Enabled => Self::NullRequired,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub(super) struct SolConfirmWait {
    pub(super) series: ResponseSeries,
    pub(super) deadline: MonotonicTime,
}

/// What kind of unsolicited response is awaiting confirmation.
#[derive(Copy, Clone, Debug)]
pub(super) enum UnsolicitedWaitKind {
    /// Startup NULL unsolicited response: 0 retries, regenerated with a new sequence number on
    /// timeout, and confirming it does not emit event-clear callbacks.
    Null,
    /// Regular event unsolicited response: retried byte-for-byte up to `retries` times.
    Events { retries: RetryCounter },
}

impl UnsolicitedWaitKind {
    pub(super) fn stop_retries(&mut self) {
        if let Self::Events { retries } = self {
            retries.stop();
        }
    }
}

/// What the unsolicited confirm wait expects (IEEE 1815-2012 Table 6-1 rows 20-34).
#[derive(Copy, Clone, Debug)]
pub(super) struct UnsolConfirmWait {
    /// The sequence number of the unsolicited response.
    pub(super) seq: Sequence,
    /// Confirm timeout deadline for the unsolicited response.
    pub(super) deadline: MonotonicTime,
    /// The confirm of a solicited response sent during the wait, if one asked for it.
    pub(super) solicited: Option<ExpectedConfirm>,
    /// Whether this wait is for a NULL response or an event response (with its remaining retries).
    pub(super) kind: UnsolicitedWaitKind,
}

/// A solicited response's confirm, expected until the confirm timeout. The timeout is checked
/// when a confirm arrives, rather than with a timer.
#[derive(Copy, Clone, Debug)]
pub(super) struct ExpectedConfirm {
    pub(super) ecsn: Sequence,
    pub(super) deadline: MonotonicTime,
}

/// Tracing span identity for an active confirm wait.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(super) enum ConfirmWaitInfo {
    Solicited(Sequence),
    Unsolicited(Sequence),
}

/// Keep-alive (REQUEST_LINK_STATUS) timer state.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(super) enum KeepAlive {
    Disabled,
    Enabled {
        timeout: Duration,
        next: MonotonicTime,
    },
}

impl KeepAlive {
    pub(super) fn new(timeout: Option<Duration>, now: MonotonicTime) -> Self {
        match timeout {
            Some(timeout) => Self::Enabled {
                timeout,
                next: now.saturating_add(timeout),
            },
            None => Self::Disabled,
        }
    }

    pub(super) fn next_deadline(&self) -> Option<MonotonicTime> {
        match *self {
            Self::Disabled => None,
            Self::Enabled { next, .. } => Some(next),
        }
    }

    pub(super) fn is_due(&self, now: MonotonicTime) -> bool {
        match *self {
            Self::Disabled => false,
            Self::Enabled { next, .. } => now >= next,
        }
    }

    pub(super) fn on_activity(&mut self, now: MonotonicTime) {
        if let Self::Enabled { timeout, next } = self {
            *next = now.saturating_add(*timeout);
        }
    }
}

/// A broadcast whose IIN1.0 hasn't been cleared.
#[derive(Copy, Clone)]
struct PendingBroadcast {
    mode: BroadcastConfirmMode,
    /// A solicited response has carried IIN1.0 for this broadcast.
    solicited_reported: bool,
    /// An unsolicited response has carried IIN1.0 for this broadcast.
    unsolicited_reported: bool,
}

#[derive(Copy, Clone)]
pub(super) enum ResponseKind {
    Solicited,
    Unsolicited,
}

/// IIN1.0 for the last broadcast received. It ends with the first response that reports it, or,
/// when the broadcast requires a confirm, once the master confirms a response that reported it.
#[derive(Default)]
pub(super) struct BroadcastIin(Option<PendingBroadcast>);

impl BroadcastIin {
    pub(super) fn received(&mut self, mode: BroadcastConfirmMode) {
        self.0 = Some(PendingBroadcast {
            mode,
            solicited_reported: false,
            unsolicited_reported: false,
        });
    }

    /// Report the broadcast in a response: IIN1.0, and in a solicited response, CON if the
    /// broadcast requires a confirm.
    pub(super) fn report(&mut self, header: &mut ResponseHeader, kind: ResponseKind) {
        let Some(pending) = &mut self.0 else {
            return;
        };

        header.iin |= Iin1::BROADCAST;

        if pending.mode != BroadcastConfirmMode::Mandatory {
            self.0 = None;
            return;
        }

        match kind {
            ResponseKind::Solicited => {
                header.control.con = true;
                pending.solicited_reported = true;
            }
            ResponseKind::Unsolicited => pending.unsolicited_reported = true,
        }
    }

    /// The master confirmed a response of this kind, which ends the IIN1.0 if such a response
    /// reported it.
    pub(super) fn confirmed(&mut self, kind: ResponseKind) {
        let reported = self.0.is_some_and(|pending| match kind {
            ResponseKind::Solicited => pending.solicited_reported,
            ResponseKind::Unsolicited => pending.unsolicited_reported,
        });
        if reported {
            self.0 = None;
        }
    }
}

/// State that mutates while the outstation runs.
pub(super) struct SessionState {
    pub(super) restart_iin_asserted: bool,
    pub(super) enabled_unsolicited_classes: EventClasses,
    pub(super) last_valid_request: Option<LastValidRequest>,
    pub(super) select: Option<SelectState>,
    pub(super) unsolicited: UnsolicitedState,
    pub(super) unsolicited_seq: Sequence,
    pub(super) deferred_read: DeferredRead,
    pub(super) last_recorded_time: Option<MonotonicTime>,
    pub(super) broadcast: BroadcastIin,
}

impl SessionState {
    pub(super) fn new(max_read_headers: u16, unsolicited: Feature) -> Self {
        Self {
            enabled_unsolicited_classes: EventClasses::none(),
            restart_iin_asserted: true,
            last_valid_request: None,
            select: None,
            unsolicited: UnsolicitedState::from_feature(unsolicited),
            unsolicited_seq: Sequence::default(),
            deferred_read: DeferredRead::new(max_read_headers),
            last_recorded_time: None,
            broadcast: BroadcastIin::default(),
        }
    }

    /// Reset items that should reset between communication (TCP) sessions.
    pub(super) fn reset(&mut self) {
        self.last_valid_request = None;
        self.select = None;
        self.deferred_read.clear();
    }
}

/// A request, classified against the last valid request.
#[derive(Copy, Clone)]
pub(super) enum FragmentType<'a> {
    MalformedRequest(u64, ObjectParseError),
    NewRead(u64, HeaderCollection<'a>),
    RepeatRead(u64, HeaderCollection<'a>),
    NewNonRead {
        hash: u64,
        objects: HeaderCollection<'a>,
        selection: Option<SelectState>,
    },
    RepeatNonRead,
    Broadcast {
        mode: BroadcastConfirmMode,
        objects: Result<HeaderCollection<'a>, ObjectParseError>,
    },
}

/// A fragment from the master, after filtering and classification.
pub(super) enum Intake<'a> {
    /// A request with an unknown function code, answered with IIN2.0.
    UnknownFunction(FragmentAddr, Sequence),
    /// The confirm of a solicited or an unsolicited response.
    Confirm(FragmentAddr, RequestHeader),
    /// A request, for the current phase to answer or defer.
    Request(Received<'a>),
}

#[derive(Copy, Clone)]
pub(super) struct Received<'a> {
    pub(super) info: FragmentInfo,
    pub(super) header: RequestHeader,
    pub(super) fragment: FragmentType<'a>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keep_alive_disabled_never_becomes_due() {
        let t0 = MonotonicTime::from_elapsed(Duration::ZERO);
        let max = MonotonicTime::from_elapsed(Duration::MAX);

        let mut ka = KeepAlive::new(None, t0);
        assert_eq!(ka, KeepAlive::Disabled);
        assert_eq!(ka.next_deadline(), None);
        assert!(!ka.is_due(t0));
        assert!(!ka.is_due(max));

        ka.on_activity(MonotonicTime::from_elapsed(Duration::from_secs(10)));
        assert_eq!(ka, KeepAlive::Disabled);
        assert_eq!(ka.next_deadline(), None);
        assert!(!ka.is_due(max));
    }

    #[test]
    fn keep_alive_enabled_transitions() {
        let t0 = MonotonicTime::from_elapsed(Duration::from_secs(5));
        let timeout = Duration::from_secs(60);

        let mut ka = KeepAlive::new(Some(timeout), t0);
        let expected_first = MonotonicTime::from_elapsed(Duration::from_secs(65));
        assert_eq!(ka.next_deadline(), Some(expected_first));

        // not due before the deadline
        assert!(!ka.is_due(t0));
        assert!(!ka.is_due(MonotonicTime::from_elapsed(Duration::from_millis(64_999))));

        // due at and after the deadline
        assert!(ka.is_due(expected_first));
        assert!(ka.is_due(MonotonicTime::from_elapsed(Duration::from_secs(70))));

        // activity pushes the deadline forward by timeout
        let activity_at = MonotonicTime::from_elapsed(Duration::from_secs(20));
        ka.on_activity(activity_at);
        let expected_second = MonotonicTime::from_elapsed(Duration::from_secs(80));
        assert_eq!(ka.next_deadline(), Some(expected_second));
        assert!(!ka.is_due(expected_first));
        assert!(ka.is_due(expected_second));
    }

    #[test]
    fn keep_alive_handles_zero_and_max_timeouts() {
        let t = MonotonicTime::from_elapsed(Duration::from_secs(10));

        let mut zero = KeepAlive::new(Some(Duration::ZERO), t);
        assert_eq!(zero.next_deadline(), Some(t));
        assert!(zero.is_due(t));
        zero.on_activity(t);
        assert!(zero.is_due(t));

        let mut max = KeepAlive::new(Some(Duration::MAX), t);
        let saturated = MonotonicTime::from_elapsed(Duration::MAX);
        assert_eq!(max.next_deadline(), Some(saturated));
        assert!(!max.is_due(t));
        assert!(max.is_due(saturated));
        max.on_activity(MonotonicTime::from_elapsed(Duration::from_secs(20)));
        assert_eq!(max.next_deadline(), Some(saturated));
    }
}
