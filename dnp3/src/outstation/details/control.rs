use crate::app::enums::CommandStatus;
use crate::app::parse::count::CountSequence;
use crate::app::parse::parser::ControlHeader;
use crate::app::parse::prefix::Prefix;
use crate::app::parse::traits::{FixedSizeVariation, Index};
use crate::app::variations::{Group12Var1, Group41Var1, Group41Var2, Group41Var3, Group41Var4};
use crate::outstation::database::Database;
use crate::outstation::details::prefix::PrefixWriter;
use crate::outstation::traits::{ControlHandler, OperateType};
use crate::util::cursor::{WriteCursor, WriteError};
use std::fmt::Debug;

pub(crate) trait ControlType: Debug {
    /// make a copy of this control type with a new status code
    fn with_status(&self, status: CommandStatus) -> Self;

    /// get the command status
    fn status(&self) -> CommandStatus;

    /// select a control on a handler
    fn select(
        self,
        handler: &mut dyn ControlHandler,
        index: u16,
        database: &mut Database,
    ) -> CommandStatus;
    /// operate a control on a handler
    fn operate(
        self,
        handler: &mut dyn ControlHandler,
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
        handler: &mut dyn ControlHandler,
        index: u16,
        database: &mut Database,
    ) -> CommandStatus {
        handler.select(self, index, database)
    }

    fn operate(
        self,
        handler: &mut dyn ControlHandler,
        index: u16,
        op_type: OperateType,
        database: &mut Database,
    ) -> CommandStatus {
        handler.operate(self, index, op_type, database)
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
        handler: &mut dyn ControlHandler,
        index: u16,
        database: &mut Database,
    ) -> CommandStatus {
        handler.select(self, index, database)
    }

    fn operate(
        self,
        handler: &mut dyn ControlHandler,
        index: u16,
        op_type: OperateType,
        database: &mut Database,
    ) -> CommandStatus {
        handler.operate(self, index, op_type, database)
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
        handler: &mut dyn ControlHandler,
        index: u16,
        database: &mut Database,
    ) -> CommandStatus {
        handler.select(self, index, database)
    }

    fn operate(
        self,
        handler: &mut dyn ControlHandler,
        index: u16,
        op_type: OperateType,
        database: &mut Database,
    ) -> CommandStatus {
        handler.operate(self, index, op_type, database)
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
        handler: &mut dyn ControlHandler,
        index: u16,
        database: &mut Database,
    ) -> CommandStatus {
        handler.select(self, index, database)
    }

    fn operate(
        self,
        handler: &mut dyn ControlHandler,
        index: u16,
        op_type: OperateType,
        database: &mut Database,
    ) -> CommandStatus {
        handler.operate(self, index, op_type, database)
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
        handler: &mut dyn ControlHandler,
        index: u16,
        database: &mut Database,
    ) -> CommandStatus {
        handler.select(self, index, database)
    }

    fn operate(
        self,
        handler: &mut dyn ControlHandler,
        index: u16,
        op_type: OperateType,
        database: &mut Database,
    ) -> CommandStatus {
        handler.operate(self, index, op_type, database)
    }
}

impl<'a> ControlHeader<'a> {
    pub(crate) fn select_with_response(
        &self,
        cursor: &mut WriteCursor,
        handler: &mut dyn ControlHandler,
        database: &mut Database,
    ) -> Result<CommandStatus, WriteError> {
        match self {
            Self::OneByteGroup12Var1(seq) => {
                select_header_with_response(cursor, seq, database, handler)
            }
            Self::OneByteGroup41Var1(seq) => {
                select_header_with_response(cursor, seq, database, handler)
            }
            Self::OneByteGroup41Var2(seq) => {
                select_header_with_response(cursor, seq, database, handler)
            }
            Self::OneByteGroup41Var3(seq) => {
                select_header_with_response(cursor, seq, database, handler)
            }
            Self::OneByteGroup41Var4(seq) => {
                select_header_with_response(cursor, seq, database, handler)
            }
            Self::TwoByteGroup12Var1(seq) => {
                select_header_with_response(cursor, seq, database, handler)
            }
            Self::TwoByteGroup41Var1(seq) => {
                select_header_with_response(cursor, seq, database, handler)
            }
            Self::TwoByteGroup41Var2(seq) => {
                select_header_with_response(cursor, seq, database, handler)
            }
            Self::TwoByteGroup41Var3(seq) => {
                select_header_with_response(cursor, seq, database, handler)
            }
            Self::TwoByteGroup41Var4(seq) => {
                select_header_with_response(cursor, seq, database, handler)
            }
        }
    }

    pub(crate) fn operate_with_response(
        &self,
        operate_type: OperateType,
        cursor: &mut WriteCursor,
        handler: &mut dyn ControlHandler,
        database: &mut Database,
    ) -> Result<(), WriteError> {
        match self {
            Self::OneByteGroup12Var1(seq) => {
                operate_header_with_response(cursor, seq, database, operate_type, handler)
            }
            Self::OneByteGroup41Var1(seq) => {
                operate_header_with_response(cursor, seq, database, operate_type, handler)
            }
            Self::OneByteGroup41Var2(seq) => {
                operate_header_with_response(cursor, seq, database, operate_type, handler)
            }
            Self::OneByteGroup41Var3(seq) => {
                operate_header_with_response(cursor, seq, database, operate_type, handler)
            }
            Self::OneByteGroup41Var4(seq) => {
                operate_header_with_response(cursor, seq, database, operate_type, handler)
            }
            Self::TwoByteGroup12Var1(seq) => {
                operate_header_with_response(cursor, seq, database, operate_type, handler)
            }
            Self::TwoByteGroup41Var1(seq) => {
                operate_header_with_response(cursor, seq, database, operate_type, handler)
            }
            Self::TwoByteGroup41Var2(seq) => {
                operate_header_with_response(cursor, seq, database, operate_type, handler)
            }
            Self::TwoByteGroup41Var3(seq) => {
                operate_header_with_response(cursor, seq, database, operate_type, handler)
            }
            Self::TwoByteGroup41Var4(seq) => {
                operate_header_with_response(cursor, seq, database, operate_type, handler)
            }
        }
    }

    pub(crate) fn operate_no_ack(&self, handler: &mut dyn ControlHandler, database: &mut Database) {
        match self {
            Self::OneByteGroup12Var1(seq) => operate_header_no_ack(seq, database, handler),
            Self::OneByteGroup41Var1(seq) => operate_header_no_ack(seq, database, handler),
            Self::OneByteGroup41Var2(seq) => operate_header_no_ack(seq, database, handler),
            Self::OneByteGroup41Var3(seq) => operate_header_no_ack(seq, database, handler),
            Self::OneByteGroup41Var4(seq) => operate_header_no_ack(seq, database, handler),
            Self::TwoByteGroup12Var1(seq) => operate_header_no_ack(seq, database, handler),
            Self::TwoByteGroup41Var1(seq) => operate_header_no_ack(seq, database, handler),
            Self::TwoByteGroup41Var2(seq) => operate_header_no_ack(seq, database, handler),
            Self::TwoByteGroup41Var3(seq) => operate_header_no_ack(seq, database, handler),
            Self::TwoByteGroup41Var4(seq) => operate_header_no_ack(seq, database, handler),
        }
    }
}

fn select_header_with_response<I, V>(
    cursor: &mut WriteCursor,
    seq: &CountSequence<Prefix<I, V>>,
    database: &mut Database,
    handler: &mut dyn ControlHandler,
) -> Result<CommandStatus, WriteError>
where
    I: Index,
    V: FixedSizeVariation + ControlType,
{
    let mut writer = PrefixWriter::new();
    let mut ret = CommandStatus::Success;
    for item in seq.iter() {
        let status = item
            .value
            .select(handler, item.index.widen_to_u16(), database);
        writer.write(cursor, item.value.with_status(status), item.index)?;
        ret = ret.first_error(status);
    }
    Ok(ret)
}

fn operate_header_with_response<I, V>(
    cursor: &mut WriteCursor,
    seq: &CountSequence<Prefix<I, V>>,
    database: &mut Database,
    operate_type: OperateType,
    handler: &mut dyn ControlHandler,
) -> Result<(), WriteError>
where
    I: Index,
    V: FixedSizeVariation + ControlType,
{
    let mut writer = PrefixWriter::new();
    for item in seq.iter() {
        let status = item
            .value
            .operate(handler, item.index.widen_to_u16(), operate_type, database);
        writer.write(cursor, item.value.with_status(status), item.index)?;
    }
    Ok(())
}

fn operate_header_no_ack<I, V>(
    seq: &CountSequence<Prefix<I, V>>,
    database: &mut Database,
    handler: &mut dyn ControlHandler,
) where
    I: Index,
    V: FixedSizeVariation + ControlType,
{
    for item in seq.iter() {
        item.value.operate(
            handler,
            item.index.widen_to_u16(),
            OperateType::DirectOperateNoAck,
            database,
        );
    }
}
