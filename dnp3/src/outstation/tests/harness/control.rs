use crate::app::enums::{CommandStatus, FunctionCode};
use crate::app::variations::{Group12Var1, Group41Var1, Group41Var2, Group41Var3, Group41Var4};
use crate::outstation::database::Database;
use crate::outstation::tests::harness::{Control, Event, EventHandle};
use crate::outstation::traits::{
    BroadcastAction, ControlHandler, ControlSupport, OperateType, OutstationInformation,
};

pub(crate) struct MockControlHandler {
    events: EventHandle,
}

pub(crate) struct MockOutstationInformation {
    events: EventHandle,
}

impl MockControlHandler {
    pub(crate) fn new(events: EventHandle) -> Box<dyn ControlHandler> {
        Box::new(Self { events })
    }
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
}

impl ControlSupport<Group12Var1> for MockControlHandler {
    fn select(&mut self, control: Group12Var1, index: u16, _: &mut Database) -> CommandStatus {
        self.events
            .push(Event::Select(Control::G12V1(control, index)));
        CommandStatus::Success
    }

    fn operate(
        &mut self,
        control: Group12Var1,
        index: u16,
        op_type: OperateType,
        _: &mut Database,
    ) -> CommandStatus {
        self.events
            .push(Event::Operate(Control::G12V1(control, index), op_type));
        CommandStatus::Success
    }
}

impl ControlSupport<Group41Var1> for MockControlHandler {
    fn select(&mut self, control: Group41Var1, index: u16, _: &mut Database) -> CommandStatus {
        self.events
            .push(Event::Select(Control::G41V1(control, index)));
        CommandStatus::Success
    }

    fn operate(
        &mut self,
        control: Group41Var1,
        index: u16,
        op_type: OperateType,
        _: &mut Database,
    ) -> CommandStatus {
        self.events
            .push(Event::Operate(Control::G41V1(control, index), op_type));
        CommandStatus::Success
    }
}

impl ControlSupport<Group41Var2> for MockControlHandler {
    fn select(&mut self, control: Group41Var2, index: u16, _: &mut Database) -> CommandStatus {
        self.events
            .push(Event::Select(Control::G41V2(control, index)));
        CommandStatus::Success
    }

    fn operate(
        &mut self,
        control: Group41Var2,
        index: u16,
        op_type: OperateType,
        _: &mut Database,
    ) -> CommandStatus {
        self.events
            .push(Event::Operate(Control::G41V2(control, index), op_type));
        CommandStatus::Success
    }
}

impl ControlSupport<Group41Var3> for MockControlHandler {
    fn select(&mut self, control: Group41Var3, index: u16, _: &mut Database) -> CommandStatus {
        self.events
            .push(Event::Select(Control::G41V3(control, index)));
        CommandStatus::Success
    }

    fn operate(
        &mut self,
        control: Group41Var3,
        index: u16,
        op_type: OperateType,
        _: &mut Database,
    ) -> CommandStatus {
        self.events
            .push(Event::Operate(Control::G41V3(control, index), op_type));
        CommandStatus::Success
    }
}

impl ControlSupport<Group41Var4> for MockControlHandler {
    fn select(&mut self, control: Group41Var4, index: u16, _: &mut Database) -> CommandStatus {
        self.events
            .push(Event::Select(Control::G41V4(control, index)));
        CommandStatus::Success
    }

    fn operate(
        &mut self,
        control: Group41Var4,
        index: u16,
        op_type: OperateType,
        _: &mut Database,
    ) -> CommandStatus {
        self.events
            .push(Event::Operate(Control::G41V4(control, index), op_type));
        CommandStatus::Success
    }
}

impl ControlHandler for MockControlHandler {
    fn begin_fragment(&mut self) {
        self.events.push(Event::BeginControls);
    }

    fn end_fragment(&mut self) {
        self.events.push(Event::EndControls);
    }
}
