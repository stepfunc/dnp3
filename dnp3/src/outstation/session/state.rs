// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! State shared by the session handlers.

use super::deferred::DeferredRead;
use crate::app::parse::parser::{HeaderCollection, Request};
use crate::app::{Iin1, ObjectParseError, RequestHeader, ResponseHeader, Sequence};
use crate::link::header::BroadcastConfirmMode;
use crate::master::EventClasses;
use crate::outstation::control::select::SelectState;
use crate::outstation::traits::BroadcastAction;
use crate::transport::{FragmentAddr, FragmentInfo};

#[derive(Copy, Clone)]
pub(super) enum TimeoutStatus {
    Yes,
    No,
}

/// A control request, with what its function needs to know about the selection
#[derive(Copy, Clone)]
pub(super) enum ControlRequest {
    Select {
        /// the request's hash, which the selection records to recognize the SELECT's retry
        request_hash: u64,
    },
    Operate {
        /// the selection that this request ended
        selection: Option<SelectState>,
    },
    DirectOperate,
}

#[derive(Copy, Clone, PartialEq)]
pub(super) struct ResponseSeries {
    pub(super) ecsn: Sequence,
    pub(super) fin: bool,
}

impl ResponseSeries {
    pub(super) fn new(ecsn: Sequence, fin: bool) -> Self {
        Self { ecsn, fin }
    }
}

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

    /// allow no further retries
    pub(super) fn stop(&mut self) {
        self.retries = Some(0);
    }
}

#[derive(Copy, Clone)]
pub(super) struct LastValidRequest {
    pub(super) seq: Sequence,
    pub(super) request_hash: u64,
}

impl LastValidRequest {
    pub(super) fn new(seq: Sequence, request_hash: u64) -> Self {
        LastValidRequest { seq, request_hash }
    }
}

#[derive(Copy, Clone)]
pub(super) enum UnsolicitedState {
    /// need to perform NULL unsolicited
    NullRequired,
    Ready(Option<tokio::time::Instant>),
}

/// a broadcast whose IIN1.0 hasn't been cleared
#[derive(Copy, Clone)]
struct PendingBroadcast {
    mode: BroadcastConfirmMode,
    /// a solicited response has carried IIN1.0 for this broadcast
    solicited_reported: bool,
    /// an unsolicited response has carried IIN1.0 for this broadcast
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
    /// broadcast requires a confirm
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
    /// reported it
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

/// state that mutates while the session runs
pub(super) struct SessionState {
    pub(super) restart_iin_asserted: bool,
    pub(super) enabled_unsolicited_classes: EventClasses,
    pub(super) last_valid_request: Option<LastValidRequest>,
    pub(super) select: Option<SelectState>,
    pub(super) unsolicited: UnsolicitedState,
    pub(super) unsolicited_seq: Sequence,
    pub(super) deferred_read: DeferredRead,
    pub(super) last_recorded_time: Option<tokio::time::Instant>,
    pub(super) broadcast: BroadcastIin,
}

impl SessionState {
    pub(super) fn new(max_read_headers: u16) -> Self {
        Self {
            enabled_unsolicited_classes: EventClasses::none(),
            restart_iin_asserted: true,
            last_valid_request: None,
            select: None,
            unsolicited: UnsolicitedState::NullRequired,
            unsolicited_seq: Sequence::default(),
            deferred_read: DeferredRead::new(max_read_headers),
            last_recorded_time: None,
            broadcast: BroadcastIin::default(),
        }
    }

    // reset items that should reset between communication (TCP) sessions
    pub(super) fn reset(&mut self) {
        self.last_valid_request = None;
        self.select = None;
        self.deferred_read.clear();
    }
}

pub(super) enum Confirm {
    Yes(FragmentAddr),
    Timeout,
    NewRequest,
}

/// How the solicited confirm wait ended
#[derive(Copy, Clone)]
pub(super) enum SolicitedWait {
    /// the response series was confirmed or timed out, or never waited for a confirm
    Done,
    /// a new request ended it, retained to be processed from idle at once, before any unsolicited
    /// response (IEEE 1815-2012 Table 6-1 rows 15 and 18)
    RequestPending,
}

/// How the confirm wait for a response ended
#[derive(Copy, Clone)]
pub(super) enum ConfirmOutcome {
    /// the master confirmed the response, so its events were delivered
    Confirmed,
    /// the response wasn't confirmed, so its events must be reported again
    Unconfirmed,
}

/// What the unsolicited confirm wait expects (IEEE 1815-2012 Table 6-1 rows 20-34)
pub(super) struct UnsolicitedWait {
    /// the sequence number of the unsolicited response
    pub(super) seq: Sequence,
    /// the confirm of a solicited response sent during the wait, if one asked for it
    pub(super) solicited: Option<ExpectedConfirm>,
    /// the retries left. A DISABLE_UNSOLICITED processed during the wait stops them, so the wait
    /// ends on timeout without a retry (TB2015-002a rule 15, decision 0007)
    pub(super) retries: RetryCounter,
}

/// A solicited response's confirm, expected until the confirm timeout. The timeout is checked
/// when a confirm arrives, rather than with a timer
#[derive(Copy, Clone)]
pub(super) struct ExpectedConfirm {
    pub(super) ecsn: Sequence,
    pub(super) deadline: tokio::time::Instant,
}

/// What [`OutstationSession::accept`](crate::outstation::session::OutstationSession::accept) did with a request
#[derive(Copy, Clone)]
pub(super) struct Accepted {
    /// the confirm that the answer waits for
    pub(super) confirm: Option<ResponseSeries>,
    /// how a broadcast request was processed
    pub(super) broadcast: Option<BroadcastAction>,
}

#[derive(Copy, Clone)]
pub(super) enum UnsolicitedWaitResult {
    Timeout,
    ReadNext,
    Complete(ConfirmOutcome),
}

/// A request, classified against the last valid request
#[derive(Copy, Clone)]
pub(super) enum FragmentType<'a> {
    MalformedRequest(u64, ObjectParseError),
    NewRead(u64, HeaderCollection<'a>),
    RepeatRead(u64, HeaderCollection<'a>),
    NewNonRead(u64, HeaderCollection<'a>),
    RepeatNonRead(u64),
    Broadcast(BroadcastConfirmMode),
}

impl FragmentType<'_> {
    pub(super) fn is_read(&self) -> bool {
        match self {
            FragmentType::NewRead(..) | FragmentType::RepeatRead(..) => true,
            FragmentType::MalformedRequest(..)
            | FragmentType::NewNonRead(..)
            | FragmentType::RepeatNonRead(_)
            | FragmentType::Broadcast(_) => false,
        }
    }
}

/// A fragment from the master, after [`OutstationSession::receive`](crate::outstation::session::OutstationSession::receive) filtered and classified it
pub(super) enum Intake<'a> {
    /// nothing to answer: no fragment, a link-layer message, a fragment that isn't a valid
    /// request, or a SELECT that the selection discards
    Nothing,
    /// a request with an unknown function code, answered with IIN2.0
    UnknownFunction(FragmentAddr, Sequence),
    /// the confirm of a solicited or an unsolicited response
    Confirm(FragmentAddr, RequestHeader),
    /// a request, for the state to answer or defer
    Request(Received<'a>),
}

#[derive(Copy, Clone)]
pub(super) struct Received<'a> {
    pub(super) info: FragmentInfo,
    pub(super) request: Request<'a>,
    pub(super) fragment: FragmentType<'a>,
}

/// What a state does with a READ
#[derive(Copy, Clone)]
pub(super) enum Reads {
    /// answer it now
    Answer,
    /// keep it until the session returns to idle (IEEE 1815-2012 Table 6-1 row 25)
    Defer,
}

#[derive(Copy, Clone)]
pub(super) enum ConfirmAction {
    Confirmed(FragmentAddr),
    NewRequest,
    EchoLastResponse(FragmentAddr),
    ContinueWait,
}

#[derive(Copy, Clone)]
pub(super) enum NextIdleAction {
    NoSleep,
    SleepUntilEvent,
    SleepUntil(tokio::time::Instant),
}

impl NextIdleAction {
    pub(super) fn select_earliest(self, instant: Option<tokio::time::Instant>) -> Self {
        match instant {
            None => self,
            Some(instant) => match self {
                NextIdleAction::NoSleep => self,
                NextIdleAction::SleepUntilEvent => Self::SleepUntil(instant),
                NextIdleAction::SleepUntil(other) => {
                    Self::SleepUntil(tokio::time::Instant::min(instant, other))
                }
            },
        }
    }
}
