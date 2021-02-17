use crate::app::FunctionCode;
use crate::app::RequestHeader;
use crate::app::Sequence;
use crate::outstation::tests::harness::{Event, EventHandle};
use crate::outstation::traits::{BroadcastAction, OutstationInformation};

pub(crate) struct MockOutstationInformation {
    events: EventHandle,
}

impl MockOutstationInformation {
    pub(crate) fn new(events: EventHandle) -> Box<dyn OutstationInformation> {
        Box::new(Self { events })
    }
}

impl OutstationInformation for MockOutstationInformation {
    fn process_request_from_idle(&mut self, _header: RequestHeader) {
        // we ignore this one in tests b/c it's just too noisy
    }

    fn broadcast_received(&mut self, function: FunctionCode, action: BroadcastAction) {
        self.events.push(Event::BroadcastReceived(function, action))
    }

    fn enter_solicited_confirm_wait(&mut self, ecsn: Sequence) {
        self.events
            .push(Event::EnterSolicitedConfirmWait(ecsn.value()))
    }

    fn solicited_confirm_timeout(&mut self, ecsn: Sequence) {
        self.events
            .push(Event::SolicitedConfirmTimeout(ecsn.value()))
    }

    fn solicited_confirm_received(&mut self, ecsn: Sequence) {
        self.events
            .push(Event::SolicitedConfirmReceived(ecsn.value()))
    }

    fn solicited_confirm_wait_new_request(&mut self) {
        self.events.push(Event::SolicitedConfirmWaitNewRequest)
    }

    fn unexpected_confirm(&mut self, unsolicited: bool, seq: Sequence) {
        self.events
            .push(Event::UnexpectedConfirm(unsolicited, seq.value()))
    }

    fn wrong_solicited_confirm_seq(&mut self, ecsn: Sequence, seq: Sequence) {
        self.events
            .push(Event::WrongSolicitedConfirmSeq(ecsn.value(), seq.value()))
    }

    fn clear_restart_iin(&mut self) {
        self.events.push(Event::ClearRestartIIN)
    }

    fn enter_unsolicited_confirm_wait(&mut self, ecsn: Sequence) {
        self.events
            .push(Event::EnterUnsolicitedConfirmWait(ecsn.value()))
    }

    fn unsolicited_confirm_timeout(&mut self, ecsn: Sequence, retry: bool) {
        self.events
            .push(Event::UnsolicitedConfirmTimeout(ecsn.value(), retry))
    }

    fn unsolicited_confirmed(&mut self, ecsn: Sequence) {
        self.events
            .push(Event::UnsolicitedConfirmReceived(ecsn.value()))
    }
}
