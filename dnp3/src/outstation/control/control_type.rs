// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::control::CommandStatus;
use crate::app::CommandObject;
use crate::outstation::database::DatabaseHandle;
use crate::outstation::traits::{ControlHandler, ControlSupport, OperateType};

/// A command object that the control handler accepts
pub(crate) trait ControlType: CommandObject {
    /// select a control on a handler
    fn select(
        self,
        handler: &mut dyn ControlHandler,
        index: u16,
        database: &mut DatabaseHandle,
    ) -> CommandStatus;

    /// operate a control on a handler
    fn operate(
        self,
        handler: &mut dyn ControlHandler,
        index: u16,
        op_type: OperateType,
        database: &mut DatabaseHandle,
    ) -> CommandStatus;
}

impl<V> ControlType for V
where
    V: CommandObject,
    dyn ControlHandler: ControlSupport<V>,
{
    fn select(
        self,
        handler: &mut dyn ControlHandler,
        index: u16,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        ControlSupport::<V>::select(handler, self, index, database)
    }

    fn operate(
        self,
        handler: &mut dyn ControlHandler,
        index: u16,
        op_type: OperateType,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        ControlSupport::<V>::operate(handler, self, index, op_type, database)
    }
}
