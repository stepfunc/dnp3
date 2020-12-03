use crate::app::enums::FunctionCode;
use crate::app::sequence::Sequence;
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
    fn broadcast_received(&mut self, function: FunctionCode, action: BroadcastAction) {
        self.events.push(Event::BroadcastReceived(function, action))
    }

    fn solicited_confirm_timeout(&mut self, ecsn: Sequence) {
        self.events.push(Event::SolConfirmTimeout(ecsn))
    }
}
