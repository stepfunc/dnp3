use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::app::control::CommandStatus;
use crate::app::variations::{Group12Var1, Group41Var1, Group41Var2, Group41Var3, Group41Var4};
use crate::app::MaybeAsync;
use crate::outstation::database::DatabaseHandle;
use crate::outstation::tests::harness::{Control, Event, EventSender};
use crate::outstation::traits::{ControlHandler, ControlSupport, OperateType};

/// The status the mock handler returns for each index, `Success` for any index not in the map
pub(crate) type ControlStatuses = Arc<Mutex<HashMap<u16, CommandStatus>>>;

pub(crate) struct MockControlHandler {
    events: EventSender,
    statuses: ControlStatuses,
}

impl MockControlHandler {
    pub(crate) fn create(
        events: EventSender,
        statuses: ControlStatuses,
    ) -> Box<dyn ControlHandler> {
        Box::new(Self { events, statuses })
    }

    fn status(&self, index: u16) -> CommandStatus {
        self.statuses
            .lock()
            .unwrap()
            .get(&index)
            .copied()
            .unwrap_or(CommandStatus::Success)
    }
}

impl ControlSupport<Group12Var1> for MockControlHandler {
    fn select(
        &mut self,
        control: Group12Var1,
        index: u16,
        _: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.events
            .send(Event::Select(Control::G12V1(control, index)));
        self.status(index)
    }

    fn operate(
        &mut self,
        control: Group12Var1,
        index: u16,
        op_type: OperateType,
        _: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.events
            .send(Event::Operate(Control::G12V1(control, index), op_type));
        self.status(index)
    }
}

impl ControlSupport<Group41Var1> for MockControlHandler {
    fn select(
        &mut self,
        control: Group41Var1,
        index: u16,
        _: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.events
            .send(Event::Select(Control::G41V1(control, index)));
        self.status(index)
    }

    fn operate(
        &mut self,
        control: Group41Var1,
        index: u16,
        op_type: OperateType,
        _: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.events
            .send(Event::Operate(Control::G41V1(control, index), op_type));
        self.status(index)
    }
}

impl ControlSupport<Group41Var2> for MockControlHandler {
    fn select(
        &mut self,
        control: Group41Var2,
        index: u16,
        _: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.events
            .send(Event::Select(Control::G41V2(control, index)));
        self.status(index)
    }

    fn operate(
        &mut self,
        control: Group41Var2,
        index: u16,
        op_type: OperateType,
        _: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.events
            .send(Event::Operate(Control::G41V2(control, index), op_type));
        self.status(index)
    }
}

impl ControlSupport<Group41Var3> for MockControlHandler {
    fn select(
        &mut self,
        control: Group41Var3,
        index: u16,
        _: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.events
            .send(Event::Select(Control::G41V3(control, index)));
        self.status(index)
    }

    fn operate(
        &mut self,
        control: Group41Var3,
        index: u16,
        op_type: OperateType,
        _: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.events
            .send(Event::Operate(Control::G41V3(control, index), op_type));
        self.status(index)
    }
}

impl ControlSupport<Group41Var4> for MockControlHandler {
    fn select(
        &mut self,
        control: Group41Var4,
        index: u16,
        _: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.events
            .send(Event::Select(Control::G41V4(control, index)));
        self.status(index)
    }

    fn operate(
        &mut self,
        control: Group41Var4,
        index: u16,
        op_type: OperateType,
        _: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.events
            .send(Event::Operate(Control::G41V4(control, index), op_type));
        self.status(index)
    }
}

impl ControlHandler for MockControlHandler {
    fn begin_fragment(&mut self) {
        self.events.send(Event::BeginControls);
    }

    fn end_fragment(&mut self, _: &mut DatabaseHandle) -> MaybeAsync<()> {
        self.events.send(Event::EndControls);
        MaybeAsync::ready(())
    }
}
