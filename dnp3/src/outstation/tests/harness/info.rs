// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::app::FunctionCode;
use crate::app::RequestHeader;
use crate::app::Sequence;
use crate::outstation::tests::harness::{Event, EventSender};
use crate::outstation::traits::{BroadcastAction, OutstationInformation};

pub(crate) struct MockOutstationInformation {
    events: EventSender,
    /// whether to record `process_request_from_idle`, which most tests leave out
    requests_from_idle: Arc<AtomicBool>,
}

impl MockOutstationInformation {
    pub(crate) fn create(events: EventSender) -> (Arc<AtomicBool>, Box<dyn OutstationInformation>) {
        let requests_from_idle = Arc::new(AtomicBool::new(false));
        (
            requests_from_idle.clone(),
            Box::new(Self {
                events,
                requests_from_idle,
            }),
        )
    }
}

impl OutstationInformation for MockOutstationInformation {
    fn process_request_from_idle(&mut self, header: RequestHeader) {
        if self.requests_from_idle.load(Ordering::Relaxed) {
            self.events.send(Event::RequestFromIdle(header))
        }
    }

    fn broadcast_received(&mut self, function: FunctionCode, action: BroadcastAction) {
        self.events.send(Event::BroadcastReceived(function, action))
    }

    fn enter_solicited_confirm_wait(&mut self, ecsn: Sequence) {
        self.events
            .send(Event::EnterSolicitedConfirmWait(ecsn.value()))
    }

    fn solicited_confirm_timeout(&mut self, ecsn: Sequence) {
        self.events
            .send(Event::SolicitedConfirmTimeout(ecsn.value()))
    }

    fn solicited_confirm_received(&mut self, ecsn: Sequence) {
        self.events
            .send(Event::SolicitedConfirmReceived(ecsn.value()))
    }

    fn solicited_confirm_wait_new_request(&mut self) {
        self.events.send(Event::SolicitedConfirmWaitNewRequest)
    }

    fn unexpected_confirm(&mut self, unsolicited: bool, seq: Sequence) {
        self.events
            .send(Event::UnexpectedConfirm(unsolicited, seq.value()))
    }

    fn wrong_solicited_confirm_seq(&mut self, ecsn: Sequence, seq: Sequence) {
        self.events
            .send(Event::WrongSolicitedConfirmSeq(ecsn.value(), seq.value()))
    }

    fn clear_restart_iin(&mut self) {
        self.events.send(Event::ClearRestartIIN)
    }

    fn enter_unsolicited_confirm_wait(&mut self, ecsn: Sequence) {
        self.events
            .send(Event::EnterUnsolicitedConfirmWait(ecsn.value()))
    }

    fn unsolicited_confirm_timeout(&mut self, ecsn: Sequence, retry: bool) {
        self.events
            .send(Event::UnsolicitedConfirmTimeout(ecsn.value(), retry))
    }

    fn unsolicited_confirmed(&mut self, ecsn: Sequence) {
        self.events
            .send(Event::UnsolicitedConfirmReceived(ecsn.value()))
    }
}
