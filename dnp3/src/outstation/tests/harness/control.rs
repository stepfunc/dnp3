use crate::app::control::CommandStatus;
use crate::app::variations::{Group12Var1, Group41Var1, Group41Var2, Group41Var3, Group41Var4};
use crate::app::MaybeAsync;
use crate::outstation::database::DatabaseHandle;
use crate::outstation::tests::harness::{Control, Event, EventSender};
use crate::outstation::traits::{ControlHandler, ControlSupport, OperateType};

pub(crate) struct MockControlHandler {
    events: EventSender,
}

impl MockControlHandler {
    pub(crate) fn new(events: EventSender) -> Box<dyn ControlHandler> {
        Box::new(Self { events })
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
        CommandStatus::Success
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
        CommandStatus::Success
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
        CommandStatus::Success
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
        CommandStatus::Success
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
        CommandStatus::Success
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
        CommandStatus::Success
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
        CommandStatus::Success
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
        CommandStatus::Success
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
        CommandStatus::Success
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
        CommandStatus::Success
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
