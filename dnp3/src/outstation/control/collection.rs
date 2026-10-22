use crate::app::control::*;
use crate::app::gen::prefixed::PrefixedVariation;
use crate::app::parse::count::CountSequence;
use crate::app::parse::parser::{HeaderCollection, HeaderDetails, HeaderIterator, ObjectHeader};
use crate::app::parse::prefix::Prefix;
use crate::app::parse::traits::{FixedSizeVariation, Index};
use crate::app::{QualifierCode, Variation};
use crate::outstation::control::control_type::ControlType;
use crate::outstation::control::prefix::PrefixWriter;
use crate::outstation::database::DatabaseHandle;
use crate::outstation::traits::{ControlHandler, ControlSupport, OperateType};

use scursor::{WriteCursor, WriteError};

pub(crate) struct ControlTransaction<'a> {
    started: bool,
    handler: &'a mut dyn ControlHandler,
}

impl<'a> ControlTransaction<'a> {
    pub(crate) async fn execute<F, R>(
        handler: &'a mut dyn ControlHandler,
        database: &mut DatabaseHandle,
        mut func: F,
    ) -> R
    where
        F: FnMut(&mut Self, &mut DatabaseHandle) -> R,
    {
        let mut tx = ControlTransaction {
            started: false,
            handler,
        };

        let ret = func(&mut tx, database);

        if tx.started {
            tx.handler.end_fragment(database).get().await
        }

        ret
    }

    fn start(&mut self) {
        if !self.started {
            self.started = true;
            self.handler.begin_fragment();
        }
    }
}

impl ControlSupport<Group12Var1> for ControlTransaction<'_> {
    fn select(
        &mut self,
        control: Group12Var1,
        index: u16,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.start();
        self.handler.select(control, index, database)
    }

    fn operate(
        &mut self,
        control: Group12Var1,
        index: u16,
        op_type: OperateType,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.start();
        self.handler.operate(control, index, op_type, database)
    }
}

impl ControlSupport<Group41Var1> for ControlTransaction<'_> {
    fn select(
        &mut self,
        control: Group41Var1,
        index: u16,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.start();
        self.handler.select(control, index, database)
    }

    fn operate(
        &mut self,
        control: Group41Var1,
        index: u16,
        op_type: OperateType,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.start();
        self.handler.operate(control, index, op_type, database)
    }
}

impl ControlSupport<Group41Var2> for ControlTransaction<'_> {
    fn select(
        &mut self,
        control: Group41Var2,
        index: u16,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.start();
        self.handler.select(control, index, database)
    }

    fn operate(
        &mut self,
        control: Group41Var2,
        index: u16,
        op_type: OperateType,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.start();
        self.handler.operate(control, index, op_type, database)
    }
}

impl ControlSupport<Group41Var3> for ControlTransaction<'_> {
    fn select(
        &mut self,
        control: Group41Var3,
        index: u16,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.start();
        self.handler.select(control, index, database)
    }

    fn operate(
        &mut self,
        control: Group41Var3,
        index: u16,
        op_type: OperateType,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.start();
        self.handler.operate(control, index, op_type, database)
    }
}

impl ControlSupport<Group41Var4> for ControlTransaction<'_> {
    fn select(
        &mut self,
        control: Group41Var4,
        index: u16,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.start();
        self.handler.select(control, index, database)
    }

    fn operate(
        &mut self,
        control: Group41Var4,
        index: u16,
        op_type: OperateType,
        database: &mut DatabaseHandle,
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
    pub(crate) fn hash(&self) -> u64 {
        self.inner.hash()
    }

    pub(crate) fn from(headers: HeaderCollection<'a>) -> Result<Self, BadControlHeader> {
        // do one pass to ensure that all headers are control headers
        let non_control_header: Option<BadControlHeader> =
            headers.iter().find_map(|x| x.to_control_header().err());

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
        database: &mut DatabaseHandle,
        max_controls_per_request: Option<u16>,
    ) -> Result<CommandStatus, WriteError> {
        let mut num_controls = 0;
        let mut status = CommandStatus::Success;
        for header in self.iter() {
            let result = header.select_with_response(
                cursor,
                transaction,
                database,
                max_controls_per_request,
                &mut num_controls,
            )?;
            status = status.first_error(result);
        }
        Ok(status)
    }

    pub(crate) fn operate_with_response(
        &self,
        cursor: &mut WriteCursor,
        operate_type: OperateType,
        transaction: &mut ControlTransaction,
        database: &mut DatabaseHandle,
        max_controls_per_request: Option<u16>,
    ) -> Result<CommandStatus, WriteError> {
        let mut num_controls = 0;
        let mut status = CommandStatus::Success;
        for header in self.iter() {
            let result = header.operate_with_response(
                operate_type,
                cursor,
                transaction,
                database,
                max_controls_per_request,
                &mut num_controls,
            )?;
            status = status.first_error(result);
        }
        Ok(status)
    }

    pub(crate) fn operate_no_ack(
        &self,
        transaction: &mut ControlTransaction,
        database: &mut DatabaseHandle,
        max_controls_per_request: Option<u16>,
    ) {
        let mut num_controls = 0;
        for header in self.iter() {
            header.operate_no_ack(
                transaction,
                database,
                max_controls_per_request,
                &mut num_controls,
            );
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

impl ControlHeader<'_> {
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
        database: &mut DatabaseHandle,
        max_controls_per_request: Option<u16>,
        num_controls: &mut u16,
    ) -> Result<CommandStatus, WriteError> {
        match self {
            Self::OneByteGroup12Var1(seq) => select_header_with_response(
                cursor,
                seq,
                database,
                transaction,
                max_controls_per_request,
                num_controls,
            ),
            Self::OneByteGroup41Var1(seq) => select_header_with_response(
                cursor,
                seq,
                database,
                transaction,
                max_controls_per_request,
                num_controls,
            ),
            Self::OneByteGroup41Var2(seq) => select_header_with_response(
                cursor,
                seq,
                database,
                transaction,
                max_controls_per_request,
                num_controls,
            ),
            Self::OneByteGroup41Var3(seq) => select_header_with_response(
                cursor,
                seq,
                database,
                transaction,
                max_controls_per_request,
                num_controls,
            ),
            Self::OneByteGroup41Var4(seq) => select_header_with_response(
                cursor,
                seq,
                database,
                transaction,
                max_controls_per_request,
                num_controls,
            ),
            Self::TwoByteGroup12Var1(seq) => select_header_with_response(
                cursor,
                seq,
                database,
                transaction,
                max_controls_per_request,
                num_controls,
            ),
            Self::TwoByteGroup41Var1(seq) => select_header_with_response(
                cursor,
                seq,
                database,
                transaction,
                max_controls_per_request,
                num_controls,
            ),
            Self::TwoByteGroup41Var2(seq) => select_header_with_response(
                cursor,
                seq,
                database,
                transaction,
                max_controls_per_request,
                num_controls,
            ),
            Self::TwoByteGroup41Var3(seq) => select_header_with_response(
                cursor,
                seq,
                database,
                transaction,
                max_controls_per_request,
                num_controls,
            ),
            Self::TwoByteGroup41Var4(seq) => select_header_with_response(
                cursor,
                seq,
                database,
                transaction,
                max_controls_per_request,
                num_controls,
            ),
        }
    }

    fn operate_with_response(
        &self,
        operate_type: OperateType,
        cursor: &mut WriteCursor,
        transaction: &mut ControlTransaction,
        database: &mut DatabaseHandle,
        max_controls_per_request: Option<u16>,
        num_controls: &mut u16,
    ) -> Result<CommandStatus, WriteError> {
        match self {
            Self::OneByteGroup12Var1(seq) => operate_header_with_response(
                cursor,
                seq,
                database,
                operate_type,
                transaction,
                max_controls_per_request,
                num_controls,
            ),
            Self::OneByteGroup41Var1(seq) => operate_header_with_response(
                cursor,
                seq,
                database,
                operate_type,
                transaction,
                max_controls_per_request,
                num_controls,
            ),
            Self::OneByteGroup41Var2(seq) => operate_header_with_response(
                cursor,
                seq,
                database,
                operate_type,
                transaction,
                max_controls_per_request,
                num_controls,
            ),
            Self::OneByteGroup41Var3(seq) => operate_header_with_response(
                cursor,
                seq,
                database,
                operate_type,
                transaction,
                max_controls_per_request,
                num_controls,
            ),
            Self::OneByteGroup41Var4(seq) => operate_header_with_response(
                cursor,
                seq,
                database,
                operate_type,
                transaction,
                max_controls_per_request,
                num_controls,
            ),
            Self::TwoByteGroup12Var1(seq) => operate_header_with_response(
                cursor,
                seq,
                database,
                operate_type,
                transaction,
                max_controls_per_request,
                num_controls,
            ),
            Self::TwoByteGroup41Var1(seq) => operate_header_with_response(
                cursor,
                seq,
                database,
                operate_type,
                transaction,
                max_controls_per_request,
                num_controls,
            ),
            Self::TwoByteGroup41Var2(seq) => operate_header_with_response(
                cursor,
                seq,
                database,
                operate_type,
                transaction,
                max_controls_per_request,
                num_controls,
            ),
            Self::TwoByteGroup41Var3(seq) => operate_header_with_response(
                cursor,
                seq,
                database,
                operate_type,
                transaction,
                max_controls_per_request,
                num_controls,
            ),
            Self::TwoByteGroup41Var4(seq) => operate_header_with_response(
                cursor,
                seq,
                database,
                operate_type,
                transaction,
                max_controls_per_request,
                num_controls,
            ),
        }
    }

    fn operate_no_ack(
        &self,
        transaction: &mut ControlTransaction,
        database: &mut DatabaseHandle,
        max_controls_per_request: Option<u16>,
        num_controls: &mut u16,
    ) {
        match self {
            Self::OneByteGroup12Var1(seq) => operate_header_no_ack(
                seq,
                database,
                transaction,
                max_controls_per_request,
                num_controls,
            ),
            Self::OneByteGroup41Var1(seq) => operate_header_no_ack(
                seq,
                database,
                transaction,
                max_controls_per_request,
                num_controls,
            ),
            Self::OneByteGroup41Var2(seq) => operate_header_no_ack(
                seq,
                database,
                transaction,
                max_controls_per_request,
                num_controls,
            ),
            Self::OneByteGroup41Var3(seq) => operate_header_no_ack(
                seq,
                database,
                transaction,
                max_controls_per_request,
                num_controls,
            ),
            Self::OneByteGroup41Var4(seq) => operate_header_no_ack(
                seq,
                database,
                transaction,
                max_controls_per_request,
                num_controls,
            ),
            Self::TwoByteGroup12Var1(seq) => operate_header_no_ack(
                seq,
                database,
                transaction,
                max_controls_per_request,
                num_controls,
            ),
            Self::TwoByteGroup41Var1(seq) => operate_header_no_ack(
                seq,
                database,
                transaction,
                max_controls_per_request,
                num_controls,
            ),
            Self::TwoByteGroup41Var2(seq) => operate_header_no_ack(
                seq,
                database,
                transaction,
                max_controls_per_request,
                num_controls,
            ),
            Self::TwoByteGroup41Var3(seq) => operate_header_no_ack(
                seq,
                database,
                transaction,
                max_controls_per_request,
                num_controls,
            ),
            Self::TwoByteGroup41Var4(seq) => operate_header_no_ack(
                seq,
                database,
                transaction,
                max_controls_per_request,
                num_controls,
            ),
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
    database: &mut DatabaseHandle,
    transaction: &mut ControlTransaction,
    max_controls_per_request: Option<u16>,
    num_controls: &mut u16,
) -> Result<CommandStatus, WriteError>
where
    I: Index,
    V: FixedSizeVariation + ControlType,
{
    let mut writer = PrefixWriter::new();
    let mut ret = CommandStatus::Success;
    for item in seq.iter() {
        let status = if max_controls_per_request.is_none_or(|max| *num_controls < max) {
            item.value
                .select(transaction, item.index.widen_to_u16(), database)
        } else {
            CommandStatus::TooManyOps
        };
        writer.write(cursor, item.value.with_status(status), item.index)?;
        ret = ret.first_error(status);
        *num_controls = num_controls.saturating_add(1);
    }
    Ok(ret)
}

fn operate_header_with_response<I, V>(
    cursor: &mut WriteCursor,
    seq: &CountSequence<Prefix<I, V>>,
    database: &mut DatabaseHandle,
    operate_type: OperateType,
    transaction: &mut ControlTransaction,
    max_controls_per_request: Option<u16>,
    num_controls: &mut u16,
) -> Result<CommandStatus, WriteError>
where
    I: Index,
    V: FixedSizeVariation + ControlType,
{
    let mut writer = PrefixWriter::new();
    let mut ret = CommandStatus::Success;
    for item in seq.iter() {
        let status = if max_controls_per_request.is_none_or(|max| *num_controls < max) {
            item.value.operate(
                transaction,
                item.index.widen_to_u16(),
                operate_type,
                database,
            )
        } else {
            CommandStatus::TooManyOps
        };
        writer.write(cursor, item.value.with_status(status), item.index)?;
        ret = ret.first_error(status);
        *num_controls = num_controls.saturating_add(1);
    }
    Ok(ret)
}

fn operate_header_no_ack<I, V>(
    seq: &CountSequence<Prefix<I, V>>,
    database: &mut DatabaseHandle,
    transaction: &mut ControlTransaction,
    max_controls_per_request: Option<u16>,
    num_controls: &mut u16,
) where
    I: Index,
    V: FixedSizeVariation + ControlType,
{
    for item in seq.iter() {
        if max_controls_per_request.is_none_or(|max| *num_controls < max) {
            item.value.operate(
                transaction,
                item.index.widen_to_u16(),
                OperateType::DirectOperateNoAck,
                database,
            );
        }
        *num_controls = num_controls.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::parse::options::ParseOptions;
    use crate::app::FunctionCode;

    const G12V1: [u8; 11] = [0x01, 0x01, 0x64, 0, 0, 0, 0xC8, 0, 0, 0, 0x00];
    const G41V1: [u8; 5] = [0x01, 0x02, 0x03, 0x04, 0x00];
    const G41V2: [u8; 3] = [0x01, 0x02, 0x00];
    const G41V3: [u8; 5] = [0x00, 0x00, 0x20, 0x40, 0x00];
    const G41V4: [u8; 9] = [0, 0, 0, 0, 0, 0, 0x04, 0x40, 0x00];

    /// the control header for each control object, with a 1-byte count and prefix for index 3
    /// and then a 2-byte count and prefix for index 259
    fn control_headers() -> Vec<u8> {
        let objects: [(u8, u8, &[u8]); 5] = [
            (12, 1, &G12V1),
            (41, 1, &G41V1),
            (41, 2, &G41V2),
            (41, 3, &G41V3),
            (41, 4, &G41V4),
        ];
        let mut input = Vec::new();
        for (group, var, data) in objects {
            input.extend([group, var, 0x17, 0x01, 0x03]);
            input.extend_from_slice(data);
        }
        for (group, var, data) in objects {
            input.extend([group, var, 0x28, 0x01, 0x00, 0x03, 0x01]);
            input.extend_from_slice(data);
        }
        input
    }

    #[test]
    fn maps_every_control_object_at_both_widths() {
        let input = control_headers();
        let headers =
            HeaderCollection::parse(ParseOptions::default(), FunctionCode::Operate, &input)
                .unwrap();
        let mut headers = headers.iter().map(|x| x.to_control_header().unwrap());

        let g41v1 = Group41Var1 {
            value: 0x04030201,
            status: CommandStatus::Success,
        };
        let g41v2 = Group41Var2 {
            value: 0x0201,
            status: CommandStatus::Success,
        };
        let g41v3 = Group41Var3 {
            value: 2.5,
            status: CommandStatus::Success,
        };
        let g41v4 = Group41Var4 {
            value: 2.5,
            status: CommandStatus::Success,
        };

        let index = |x: Option<Prefix<u8, Group12Var1>>| x.map(|x| x.index);
        assert_matches!(headers.next(), Some(ControlHeader::OneByteGroup12Var1(seq)) => assert_eq!(index(seq.single()), Some(3)));
        assert_matches!(headers.next(), Some(ControlHeader::OneByteGroup41Var1(seq)) => assert_eq!(seq.single(), Some(Prefix { index: 3, value: g41v1 })));
        assert_matches!(headers.next(), Some(ControlHeader::OneByteGroup41Var2(seq)) => assert_eq!(seq.single(), Some(Prefix { index: 3, value: g41v2 })));
        assert_matches!(headers.next(), Some(ControlHeader::OneByteGroup41Var3(seq)) => assert_eq!(seq.single(), Some(Prefix { index: 3, value: g41v3 })));
        assert_matches!(headers.next(), Some(ControlHeader::OneByteGroup41Var4(seq)) => assert_eq!(seq.single(), Some(Prefix { index: 3, value: g41v4 })));

        let index = |x: Option<Prefix<u16, Group12Var1>>| x.map(|x| x.index);
        assert_matches!(headers.next(), Some(ControlHeader::TwoByteGroup12Var1(seq)) => assert_eq!(index(seq.single()), Some(259)));
        assert_matches!(headers.next(), Some(ControlHeader::TwoByteGroup41Var1(seq)) => assert_eq!(seq.single(), Some(Prefix { index: 259, value: g41v1 })));
        assert_matches!(headers.next(), Some(ControlHeader::TwoByteGroup41Var2(seq)) => assert_eq!(seq.single(), Some(Prefix { index: 259, value: g41v2 })));
        assert_matches!(headers.next(), Some(ControlHeader::TwoByteGroup41Var3(seq)) => assert_eq!(seq.single(), Some(Prefix { index: 259, value: g41v3 })));
        assert_matches!(headers.next(), Some(ControlHeader::TwoByteGroup41Var4(seq)) => assert_eq!(seq.single(), Some(Prefix { index: 259, value: g41v4 })));

        assert!(headers.next().is_none());
    }

    #[test]
    fn rejects_other_headers_with_their_qualifier() {
        let input = [
            // g1v2, 1-byte and 2-byte start/stop
            0x01, 0x02, 0x00, 0x03, 0x03, 0x01, //
            0x01, 0x02, 0x01, 0x03, 0x01, 0x03, 0x01, 0x01, //
            // g50v1, 1-byte and 2-byte count
            0x32, 0x01, 0x07, 0x01, 0, 0, 0, 0, 0, 0, //
            0x32, 0x01, 0x08, 0x01, 0x00, 0, 0, 0, 0, 0, 0, //
            // g2v1, 1-byte and 2-byte count and prefix
            0x02, 0x01, 0x17, 0x01, 0x03, 0x01, //
            0x02, 0x01, 0x28, 0x01, 0x00, 0x03, 0x01, 0x01,
        ];
        let headers =
            HeaderCollection::parse(ParseOptions::default(), FunctionCode::Operate, &input)
                .unwrap();
        let errors: Vec<BadControlHeader> = headers
            .iter()
            .map(|x| x.to_control_header().unwrap_err())
            .collect();

        assert_eq!(
            errors,
            [
                BadControlHeader::new(Variation::Group1Var2, QualifierCode::Range8),
                BadControlHeader::new(Variation::Group1Var2, QualifierCode::Range16),
                BadControlHeader::new(Variation::Group50Var1, QualifierCode::Count8),
                BadControlHeader::new(Variation::Group50Var1, QualifierCode::Count16),
                BadControlHeader::new(Variation::Group2Var1, QualifierCode::CountAndPrefix8),
                BadControlHeader::new(Variation::Group2Var1, QualifierCode::CountAndPrefix16),
            ]
        );
    }
}
