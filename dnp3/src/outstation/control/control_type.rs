use crate::app::enums::CommandStatus;
use crate::app::variations::{Group12Var1, Group41Var1, Group41Var2, Group41Var3, Group41Var4};
use crate::outstation::control::collection::ControlTransaction;
use crate::outstation::database::Database;
use crate::outstation::traits::ControlSupport;
use crate::outstation::traits::OperateType;
use std::fmt::Debug;

pub(crate) trait ControlType: Debug {
    /// make a copy of this control type with a new status code
    fn with_status(&self, status: CommandStatus) -> Self;

    /// get the command status
    fn status(&self) -> CommandStatus;

    /// select a control on a handler
    fn select(
        self,
        transaction: &mut ControlTransaction,
        index: u16,
        database: &mut Database,
    ) -> CommandStatus;
    /// operate a control on a handler
    fn operate(
        self,
        transaction: &mut ControlTransaction,
        index: u16,
        op_type: OperateType,
        database: &mut Database,
    ) -> CommandStatus;
}

impl ControlType for Group12Var1 {
    fn with_status(&self, status: CommandStatus) -> Self {
        Self { status, ..*self }
    }

    fn status(&self) -> CommandStatus {
        self.status
    }

    fn select(
        self,
        transaction: &mut ControlTransaction,
        index: u16,
        database: &mut Database,
    ) -> CommandStatus {
        transaction.select(self, index, database)
    }

    fn operate(
        self,
        transaction: &mut ControlTransaction,
        index: u16,
        op_type: OperateType,
        database: &mut Database,
    ) -> CommandStatus {
        transaction.operate(self, index, op_type, database)
    }
}

impl ControlType for Group41Var1 {
    fn with_status(&self, status: CommandStatus) -> Self {
        Self { status, ..*self }
    }

    fn status(&self) -> CommandStatus {
        self.status
    }

    fn select(
        self,
        transaction: &mut ControlTransaction,
        index: u16,
        database: &mut Database,
    ) -> CommandStatus {
        transaction.select(self, index, database)
    }

    fn operate(
        self,
        transaction: &mut ControlTransaction,
        index: u16,
        op_type: OperateType,
        database: &mut Database,
    ) -> CommandStatus {
        transaction.operate(self, index, op_type, database)
    }
}

impl ControlType for Group41Var2 {
    fn with_status(&self, status: CommandStatus) -> Self {
        Self { status, ..*self }
    }

    fn status(&self) -> CommandStatus {
        self.status
    }

    fn select(
        self,
        transaction: &mut ControlTransaction,
        index: u16,
        database: &mut Database,
    ) -> CommandStatus {
        transaction.select(self, index, database)
    }

    fn operate(
        self,
        transaction: &mut ControlTransaction,
        index: u16,
        op_type: OperateType,
        database: &mut Database,
    ) -> CommandStatus {
        transaction.operate(self, index, op_type, database)
    }
}

impl ControlType for Group41Var3 {
    fn with_status(&self, status: CommandStatus) -> Self {
        Self { status, ..*self }
    }

    fn status(&self) -> CommandStatus {
        self.status
    }

    fn select(
        self,
        transaction: &mut ControlTransaction,
        index: u16,
        database: &mut Database,
    ) -> CommandStatus {
        transaction.select(self, index, database)
    }

    fn operate(
        self,
        transaction: &mut ControlTransaction,
        index: u16,
        op_type: OperateType,
        database: &mut Database,
    ) -> CommandStatus {
        transaction.operate(self, index, op_type, database)
    }
}

impl ControlType for Group41Var4 {
    fn with_status(&self, status: CommandStatus) -> Self {
        Self { status, ..*self }
    }

    fn status(&self) -> CommandStatus {
        self.status
    }

    fn select(
        self,
        transaction: &mut ControlTransaction,
        index: u16,
        database: &mut Database,
    ) -> CommandStatus {
        transaction.select(self, index, database)
    }

    fn operate(
        self,
        transaction: &mut ControlTransaction,
        index: u16,
        op_type: OperateType,
        database: &mut Database,
    ) -> CommandStatus {
        transaction.operate(self, index, op_type, database)
    }
}
