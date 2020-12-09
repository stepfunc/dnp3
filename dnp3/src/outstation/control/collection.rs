use crate::app::enums::{CommandStatus, QualifierCode};
use crate::app::gen::prefixed::PrefixedVariation;
use crate::app::parse::count::CountSequence;
use crate::app::parse::parser::{HeaderCollection, HeaderDetails, HeaderIterator, ObjectHeader};
use crate::app::parse::prefix::Prefix;
use crate::app::parse::traits::{FixedSizeVariation, Index};
use crate::app::variations::{
    Group12Var1, Group41Var1, Group41Var2, Group41Var3, Group41Var4, Variation,
};
use crate::outstation::control::control_type::ControlType;
use crate::outstation::control::prefix::PrefixWriter;
use crate::outstation::database::Database;
use crate::outstation::traits::{ControlHandler, ControlSupport, OperateType};
use crate::util::cursor::{WriteCursor, WriteError};

pub(crate) struct ControlTransaction<'a> {
    stared: bool,
    handler: &'a mut dyn ControlHandler,
}

impl<'a> ControlTransaction<'a> {
    pub(crate) fn new(handler: &'a mut dyn ControlHandler) -> Self {
        ControlTransaction {
            stared: false,
            handler,
        }
    }

    fn start(&mut self) {
        if !self.stared {
            self.stared = true;
            self.handler.begin_fragment();
        }
    }
}

impl<'a> Drop for ControlTransaction<'a> {
    fn drop(&mut self) {
        if self.stared {
            self.handler.end_fragment();
        }
    }
}

impl<'a> ControlSupport<Group12Var1> for ControlTransaction<'a> {
    fn select(
        &mut self,
        control: Group12Var1,
        index: u16,
        database: &mut Database,
    ) -> CommandStatus {
        self.start();
        self.handler.select(control, index, database)
    }

    fn operate(
        &mut self,
        control: Group12Var1,
        index: u16,
        op_type: OperateType,
        database: &mut Database,
    ) -> CommandStatus {
        self.start();
        self.handler.operate(control, index, op_type, database)
    }
}

impl<'a> ControlSupport<Group41Var1> for ControlTransaction<'a> {
    fn select(
        &mut self,
        control: Group41Var1,
        index: u16,
        database: &mut Database,
    ) -> CommandStatus {
        self.start();
        self.handler.select(control, index, database)
    }

    fn operate(
        &mut self,
        control: Group41Var1,
        index: u16,
        op_type: OperateType,
        database: &mut Database,
    ) -> CommandStatus {
        self.start();
        self.handler.operate(control, index, op_type, database)
    }
}

impl<'a> ControlSupport<Group41Var2> for ControlTransaction<'a> {
    fn select(
        &mut self,
        control: Group41Var2,
        index: u16,
        database: &mut Database,
    ) -> CommandStatus {
        self.start();
        self.handler.select(control, index, database)
    }

    fn operate(
        &mut self,
        control: Group41Var2,
        index: u16,
        op_type: OperateType,
        database: &mut Database,
    ) -> CommandStatus {
        self.start();
        self.handler.operate(control, index, op_type, database)
    }
}

impl<'a> ControlSupport<Group41Var3> for ControlTransaction<'a> {
    fn select(
        &mut self,
        control: Group41Var3,
        index: u16,
        database: &mut Database,
    ) -> CommandStatus {
        self.start();
        self.handler.select(control, index, database)
    }

    fn operate(
        &mut self,
        control: Group41Var3,
        index: u16,
        op_type: OperateType,
        database: &mut Database,
    ) -> CommandStatus {
        self.start();
        self.handler.operate(control, index, op_type, database)
    }
}

impl<'a> ControlSupport<Group41Var4> for ControlTransaction<'a> {
    fn select(
        &mut self,
        control: Group41Var4,
        index: u16,
        database: &mut Database,
    ) -> CommandStatus {
        self.start();
        self.handler.select(control, index, database)
    }

    fn operate(
        &mut self,
        control: Group41Var4,
        index: u16,
        op_type: OperateType,
        database: &mut Database,
    ) -> CommandStatus {
        self.start();
        self.handler.operate(control, index, op_type, database)
    }
}

impl<'a> ObjectHeader<'a> {
    pub(crate) fn to_control_header(&self) -> Result<ControlHeader<'a>, BadControlHeader> {
        match self.details {
            // one byte headers
            HeaderDetails::OneByteCountAndPrefix(_, PrefixedVariation::Group12Var1(seq)) => {
                Ok(ControlHeader::OneByteGroup12Var1(seq))
            }
            HeaderDetails::OneByteCountAndPrefix(_, PrefixedVariation::Group41Var1(seq)) => {
                Ok(ControlHeader::OneByteGroup41Var1(seq))
            }
            HeaderDetails::OneByteCountAndPrefix(_, PrefixedVariation::Group41Var2(seq)) => {
                Ok(ControlHeader::OneByteGroup41Var2(seq))
            }
            HeaderDetails::OneByteCountAndPrefix(_, PrefixedVariation::Group41Var3(seq)) => {
                Ok(ControlHeader::OneByteGroup41Var3(seq))
            }
            HeaderDetails::OneByteCountAndPrefix(_, PrefixedVariation::Group41Var4(seq)) => {
                Ok(ControlHeader::OneByteGroup41Var4(seq))
            }
            // two byte headers
            HeaderDetails::TwoByteCountAndPrefix(_, PrefixedVariation::Group12Var1(seq)) => {
                Ok(ControlHeader::TwoByteGroup12Var1(seq))
            }
            HeaderDetails::TwoByteCountAndPrefix(_, PrefixedVariation::Group41Var1(seq)) => {
                Ok(ControlHeader::TwoByteGroup41Var1(seq))
            }
            HeaderDetails::TwoByteCountAndPrefix(_, PrefixedVariation::Group41Var2(seq)) => {
                Ok(ControlHeader::TwoByteGroup41Var2(seq))
            }
            HeaderDetails::TwoByteCountAndPrefix(_, PrefixedVariation::Group41Var3(seq)) => {
                Ok(ControlHeader::TwoByteGroup41Var3(seq))
            }
            HeaderDetails::TwoByteCountAndPrefix(_, PrefixedVariation::Group41Var4(seq)) => {
                Ok(ControlHeader::TwoByteGroup41Var4(seq))
            }
            _ => Err(BadControlHeader::new(
                self.variation,
                self.details.qualifier(),
            )),
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum ControlHeader<'a> {
    OneByteGroup12Var1(CountSequence<'a, Prefix<u8, Group12Var1>>),
    OneByteGroup41Var1(CountSequence<'a, Prefix<u8, Group41Var1>>),
    OneByteGroup41Var2(CountSequence<'a, Prefix<u8, Group41Var2>>),
    OneByteGroup41Var3(CountSequence<'a, Prefix<u8, Group41Var3>>),
    OneByteGroup41Var4(CountSequence<'a, Prefix<u8, Group41Var4>>),
    TwoByteGroup12Var1(CountSequence<'a, Prefix<u16, Group12Var1>>),
    TwoByteGroup41Var1(CountSequence<'a, Prefix<u16, Group41Var1>>),
    TwoByteGroup41Var2(CountSequence<'a, Prefix<u16, Group41Var2>>),
    TwoByteGroup41Var3(CountSequence<'a, Prefix<u16, Group41Var3>>),
    TwoByteGroup41Var4(CountSequence<'a, Prefix<u16, Group41Var4>>),
}

#[derive(Debug, PartialEq)]
pub(crate) struct BadControlHeader {
    pub(crate) variation: Variation,
    pub(crate) qualifier: QualifierCode,
}

impl BadControlHeader {
    pub(crate) fn new(variation: Variation, qualifier: QualifierCode) -> Self {
        Self {
            variation,
            qualifier,
        }
    }
}

pub(crate) struct ControlCollection<'a> {
    inner: HeaderCollection<'a>,
}

impl<'a> ControlCollection<'a> {
    pub(crate) fn from(headers: HeaderCollection<'a>) -> Result<Self, BadControlHeader> {
        // do one pass to ensure that all headers are control headers
        let non_control_header: Option<BadControlHeader> = headers.iter().find_map(|x| {
            if let Err(header) = x.to_control_header() {
                Some(header)
            } else {
                None
            }
        });

        if let Some(err) = non_control_header {
            return Err(err);
        }

        Ok(ControlCollection { inner: headers })
    }

    fn iter(&self) -> ControlHeaderIterator<'a> {
        ControlHeaderIterator {
            inner: self.inner.iter(),
        }
    }

    pub(crate) fn respond_with_status(
        &self,
        cursor: &mut WriteCursor,
        status: CommandStatus,
    ) -> Result<(), WriteError> {
        for header in self.iter() {
            header.respond_with_status(cursor, status)?;
        }
        Ok(())
    }

    pub(crate) fn select_with_response(
        &self,
        cursor: &mut WriteCursor,
        transaction: &mut ControlTransaction,
        database: &mut Database,
    ) -> Result<CommandStatus, WriteError> {
        let mut error = CommandStatus::Success;
        for header in self.iter() {
            let status = header.select_with_response(cursor, transaction, database)?;
            error = error.first_error(status);
        }
        Ok(error)
    }

    pub(crate) fn operate_with_response(
        &self,
        cursor: &mut WriteCursor,
        operate_type: OperateType,
        transaction: &mut ControlTransaction,
        database: &mut Database,
    ) -> Result<(), WriteError> {
        for header in self.iter() {
            header.operate_with_response(operate_type, cursor, transaction, database)?;
        }
        Ok(())
    }

    pub(crate) fn operate_no_ack(
        &self,
        transaction: &mut ControlTransaction,
        database: &mut Database,
    ) {
        for header in self.iter() {
            header.operate_no_ack(transaction, database);
        }
    }
}

#[derive(Copy, Clone)]
pub(crate) struct ControlHeaderIterator<'a> {
    inner: HeaderIterator<'a>,
}

impl<'a> Iterator for ControlHeaderIterator<'a> {
    type Item = ControlHeader<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.next() {
            None => None,
            // this should always be some b/c of pre-validation
            Some(x) => x.to_control_header().ok(),
        }
    }
}

impl<'a> ControlHeader<'a> {
    fn respond_with_status(
        &self,
        cursor: &mut WriteCursor,
        status: CommandStatus,
    ) -> Result<(), WriteError> {
        match self {
            Self::OneByteGroup12Var1(seq) => respond_with_status(cursor, seq, status),
            Self::OneByteGroup41Var1(seq) => respond_with_status(cursor, seq, status),
            Self::OneByteGroup41Var2(seq) => respond_with_status(cursor, seq, status),
            Self::OneByteGroup41Var3(seq) => respond_with_status(cursor, seq, status),
            Self::OneByteGroup41Var4(seq) => respond_with_status(cursor, seq, status),
            Self::TwoByteGroup12Var1(seq) => respond_with_status(cursor, seq, status),
            Self::TwoByteGroup41Var1(seq) => respond_with_status(cursor, seq, status),
            Self::TwoByteGroup41Var2(seq) => respond_with_status(cursor, seq, status),
            Self::TwoByteGroup41Var3(seq) => respond_with_status(cursor, seq, status),
            Self::TwoByteGroup41Var4(seq) => respond_with_status(cursor, seq, status),
        }
    }

    fn select_with_response(
        &self,
        cursor: &mut WriteCursor,
        transaction: &mut ControlTransaction,
        database: &mut Database,
    ) -> Result<CommandStatus, WriteError> {
        match self {
            Self::OneByteGroup12Var1(seq) => {
                select_header_with_response(cursor, seq, database, transaction)
            }
            Self::OneByteGroup41Var1(seq) => {
                select_header_with_response(cursor, seq, database, transaction)
            }
            Self::OneByteGroup41Var2(seq) => {
                select_header_with_response(cursor, seq, database, transaction)
            }
            Self::OneByteGroup41Var3(seq) => {
                select_header_with_response(cursor, seq, database, transaction)
            }
            Self::OneByteGroup41Var4(seq) => {
                select_header_with_response(cursor, seq, database, transaction)
            }
            Self::TwoByteGroup12Var1(seq) => {
                select_header_with_response(cursor, seq, database, transaction)
            }
            Self::TwoByteGroup41Var1(seq) => {
                select_header_with_response(cursor, seq, database, transaction)
            }
            Self::TwoByteGroup41Var2(seq) => {
                select_header_with_response(cursor, seq, database, transaction)
            }
            Self::TwoByteGroup41Var3(seq) => {
                select_header_with_response(cursor, seq, database, transaction)
            }
            Self::TwoByteGroup41Var4(seq) => {
                select_header_with_response(cursor, seq, database, transaction)
            }
        }
    }

    fn operate_with_response(
        &self,
        operate_type: OperateType,
        cursor: &mut WriteCursor,
        transaction: &mut ControlTransaction,
        database: &mut Database,
    ) -> Result<(), WriteError> {
        match self {
            Self::OneByteGroup12Var1(seq) => {
                operate_header_with_response(cursor, seq, database, operate_type, transaction)
            }
            Self::OneByteGroup41Var1(seq) => {
                operate_header_with_response(cursor, seq, database, operate_type, transaction)
            }
            Self::OneByteGroup41Var2(seq) => {
                operate_header_with_response(cursor, seq, database, operate_type, transaction)
            }
            Self::OneByteGroup41Var3(seq) => {
                operate_header_with_response(cursor, seq, database, operate_type, transaction)
            }
            Self::OneByteGroup41Var4(seq) => {
                operate_header_with_response(cursor, seq, database, operate_type, transaction)
            }
            Self::TwoByteGroup12Var1(seq) => {
                operate_header_with_response(cursor, seq, database, operate_type, transaction)
            }
            Self::TwoByteGroup41Var1(seq) => {
                operate_header_with_response(cursor, seq, database, operate_type, transaction)
            }
            Self::TwoByteGroup41Var2(seq) => {
                operate_header_with_response(cursor, seq, database, operate_type, transaction)
            }
            Self::TwoByteGroup41Var3(seq) => {
                operate_header_with_response(cursor, seq, database, operate_type, transaction)
            }
            Self::TwoByteGroup41Var4(seq) => {
                operate_header_with_response(cursor, seq, database, operate_type, transaction)
            }
        }
    }

    fn operate_no_ack(&self, transaction: &mut ControlTransaction, database: &mut Database) {
        match self {
            Self::OneByteGroup12Var1(seq) => operate_header_no_ack(seq, database, transaction),
            Self::OneByteGroup41Var1(seq) => operate_header_no_ack(seq, database, transaction),
            Self::OneByteGroup41Var2(seq) => operate_header_no_ack(seq, database, transaction),
            Self::OneByteGroup41Var3(seq) => operate_header_no_ack(seq, database, transaction),
            Self::OneByteGroup41Var4(seq) => operate_header_no_ack(seq, database, transaction),
            Self::TwoByteGroup12Var1(seq) => operate_header_no_ack(seq, database, transaction),
            Self::TwoByteGroup41Var1(seq) => operate_header_no_ack(seq, database, transaction),
            Self::TwoByteGroup41Var2(seq) => operate_header_no_ack(seq, database, transaction),
            Self::TwoByteGroup41Var3(seq) => operate_header_no_ack(seq, database, transaction),
            Self::TwoByteGroup41Var4(seq) => operate_header_no_ack(seq, database, transaction),
        }
    }
}

fn respond_with_status<I, V>(
    cursor: &mut WriteCursor,
    seq: &CountSequence<Prefix<I, V>>,
    status: CommandStatus,
) -> Result<(), WriteError>
where
    I: Index,
    V: FixedSizeVariation + ControlType,
{
    let mut writer = PrefixWriter::new();
    for item in seq.iter() {
        writer.write(cursor, item.value.with_status(status), item.index)?;
    }
    Ok(())
}

fn select_header_with_response<I, V>(
    cursor: &mut WriteCursor,
    seq: &CountSequence<Prefix<I, V>>,
    database: &mut Database,
    transaction: &mut ControlTransaction,
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
            .select(transaction, item.index.widen_to_u16(), database);
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
    transaction: &mut ControlTransaction,
) -> Result<(), WriteError>
where
    I: Index,
    V: FixedSizeVariation + ControlType,
{
    let mut writer = PrefixWriter::new();
    for item in seq.iter() {
        let status = item.value.operate(
            transaction,
            item.index.widen_to_u16(),
            operate_type,
            database,
        );
        writer.write(cursor, item.value.with_status(status), item.index)?;
    }
    Ok(())
}

fn operate_header_no_ack<I, V>(
    seq: &CountSequence<Prefix<I, V>>,
    database: &mut Database,
    transaction: &mut ControlTransaction,
) where
    I: Index,
    V: FixedSizeVariation + ControlType,
{
    for item in seq.iter() {
        item.value.operate(
            transaction,
            item.index.widen_to_u16(),
            OperateType::DirectOperateNoAck,
            database,
        );
    }
}
