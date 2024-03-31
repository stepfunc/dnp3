use std::fmt::Debug;

use crate::app::control::*;
use crate::outstation::control::collection::ControlTransaction;
use crate::outstation::database::DatabaseHandle;
use crate::outstation::traits::ControlSupport;
use crate::outstation::traits::OperateType;

pub(crate) trait ControlType: Debug {
    /// make a copy of this control type with a new status code
    fn with_status(&self, status: CommandStatus) -> Self;

    /// select a control on a handler
    fn select(
        self,
        transaction: &mut ControlTransaction,
        index: u16,
        database: &mut DatabaseHandle,
    ) -> CommandStatus;
    /// operate a control on a handler
    fn operate(
        self,
        transaction: &mut ControlTransaction,
        index: u16,
        op_type: OperateType,
        database: &mut DatabaseHandle,
    ) -> CommandStatus;
}

impl ControlType for Group12Var1 {
    fn with_status(&self, status: CommandStatus) -> Self {
        Self { status, ..*self }
    }

    fn select(
        self,
        transaction: &mut ControlTransaction,
        index: u16,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        transaction.select(self, index, database)
    }

    fn operate(
        self,
        transaction: &mut ControlTransaction,
        index: u16,
        op_type: OperateType,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        transaction.operate(self, index, op_type, database)
    }
}

impl ControlType for Group41Var1 {
    fn with_status(&self, status: CommandStatus) -> Self {
        Self { status, ..*self }
    }

    fn select(
        self,
        transaction: &mut ControlTransaction,
        index: u16,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        transaction.select(self, index, database)
    }

    fn operate(
        self,
        transaction: &mut ControlTransaction,
        index: u16,
        op_type: OperateType,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        transaction.operate(self, index, op_type, database)
    }
}

impl ControlType for Group41Var2 {
    fn with_status(&self, status: CommandStatus) -> Self {
        Self { status, ..*self }
    }

    fn select(
        self,
        transaction: &mut ControlTransaction,
        index: u16,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        transaction.select(self, index, database)
    }

    fn operate(
        self,
        transaction: &mut ControlTransaction,
        index: u16,
        op_type: OperateType,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        transaction.operate(self, index, op_type, database)
    }
}

impl ControlType for Group41Var3 {
    fn with_status(&self, status: CommandStatus) -> Self {
        Self { status, ..*self }
    }

    fn select(
        self,
        transaction: &mut ControlTransaction,
        index: u16,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        transaction.select(self, index, database)
    }

    fn operate(
        self,
        transaction: &mut ControlTransaction,
        index: u16,
        op_type: OperateType,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        transaction.operate(self, index, op_type, database)
    }
}

impl ControlType for Group41Var4 {
    fn with_status(&self, status: CommandStatus) -> Self {
        Self { status, ..*self }
    }

    fn select(
        self,
        transaction: &mut ControlTransaction,
        index: u16,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        transaction.select(self, index, database)
    }

    fn operate(
        self,
        transaction: &mut ControlTransaction,
        index: u16,
        op_type: OperateType,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        transaction.operate(self, index, op_type, database)
    }
}
