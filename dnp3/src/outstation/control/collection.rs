use crate::app::control::*;
use crate::app::gen::prefixed::PrefixedVariation;
use crate::app::parse::count::CountSequence;
use crate::app::parse::parser::{HeaderCollection, HeaderDetails, HeaderIterator, ObjectHeader};
use crate::app::parse::prefix::Prefix;
use crate::app::parse::traits::Index;
use crate::app::{QualifierCode, Variation};
use crate::outstation::control::control_type::ControlType;
use crate::outstation::control::prefix::PrefixWriter;
use crate::outstation::database::DatabaseHandle;
use crate::outstation::traits::{ControlHandler, OperateType};

use scursor::{WriteCursor, WriteError};

pub(crate) struct ControlTransaction<'a> {
    started: bool,
    handler: &'a mut dyn ControlHandler,
}

impl<'a> ControlTransaction<'a> {
    pub(crate) async fn execute<F, R>(
        handler: &'a mut dyn ControlHandler,
        database: &mut DatabaseHandle,
        func: F,
    ) -> R
    where
        F: FnOnce(&mut Self, &mut DatabaseHandle) -> R,
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

    fn select<V: ControlType>(
        &mut self,
        control: V,
        index: u16,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.start();
        control.select(self.handler, index, database)
    }

    fn operate<V: ControlType>(
        &mut self,
        control: V,
        index: u16,
        op_type: OperateType,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.start();
        control.operate(self.handler, index, op_type, database)
    }
}

/// Counts the controls in a request against the configured maximum
pub(crate) struct ControlLimit {
    max: Option<u16>,
    seen: u16,
}

impl ControlLimit {
    pub(crate) fn new(max: Option<u16>) -> Self {
        Self { max, seen: 0 }
    }

    /// Count a control, returning whether it's within the limit
    fn admit(&mut self) -> bool {
        let admitted = self.max.is_none_or(|max| self.seen < max);
        self.seen = self.seen.saturating_add(1);
        admitted
    }
}

/// How the handler executes each control
#[derive(Copy, Clone)]
pub(crate) enum ControlMode {
    Select,
    Operate(OperateType),
}

impl<'a> ObjectHeader<'a> {
    pub(crate) fn to_control_header(&self) -> Result<ControlHeader<'a>, BadControlHeader> {
        let header = match &self.details {
            HeaderDetails::CountAndPrefix8(_, objects) => {
                ControlObjects::new(objects).map(ControlHeader::OneByte)
            }
            HeaderDetails::CountAndPrefix16(_, objects) => {
                ControlObjects::new(objects).map(ControlHeader::TwoByte)
            }
            _ => None,
        };

        header.ok_or_else(|| BadControlHeader::new(self.variation(), self.details.qualifier()))
    }
}

/// A header of controls with a one-byte or two-byte count and index prefix
#[derive(Debug, PartialEq)]
pub(crate) enum ControlHeader<'a> {
    OneByte(ControlObjects<'a, u8>),
    TwoByte(ControlObjects<'a, u16>),
}

/// The controls in a header, by object type
#[derive(Debug, PartialEq)]
pub(crate) enum ControlObjects<'a, I: Index> {
    Group12Var1(CountSequence<'a, Prefix<I, Group12Var1>>),
    Group41Var1(CountSequence<'a, Prefix<I, Group41Var1>>),
    Group41Var2(CountSequence<'a, Prefix<I, Group41Var2>>),
    Group41Var3(CountSequence<'a, Prefix<I, Group41Var3>>),
    Group41Var4(CountSequence<'a, Prefix<I, Group41Var4>>),
}

impl<'a, I: Index> ControlObjects<'a, I> {
    fn new(objects: &PrefixedVariation<'a, I>) -> Option<Self> {
        match objects {
            PrefixedVariation::Group12Var1(seq) => Some(Self::Group12Var1(*seq)),
            PrefixedVariation::Group41Var1(seq) => Some(Self::Group41Var1(*seq)),
            PrefixedVariation::Group41Var2(seq) => Some(Self::Group41Var2(*seq)),
            PrefixedVariation::Group41Var3(seq) => Some(Self::Group41Var3(*seq)),
            PrefixedVariation::Group41Var4(seq) => Some(Self::Group41Var4(*seq)),
            _ => None,
        }
    }

    fn visit<T: ControlVisitor>(&self, visitor: &mut T) -> T::Output {
        match self {
            Self::Group12Var1(seq) => visitor.visit(seq),
            Self::Group41Var1(seq) => visitor.visit(seq),
            Self::Group41Var2(seq) => visitor.visit(seq),
            Self::Group41Var3(seq) => visitor.visit(seq),
            Self::Group41Var4(seq) => visitor.visit(seq),
        }
    }
}

impl ControlHeader<'_> {
    fn visit<T: ControlVisitor>(&self, visitor: &mut T) -> T::Output {
        match self {
            Self::OneByte(objects) => objects.visit(visitor),
            Self::TwoByte(objects) => objects.visit(visitor),
        }
    }
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

    /// Echo every control with `status`, without executing it
    pub(crate) fn respond_with_status(
        &self,
        cursor: &mut WriteCursor,
        status: CommandStatus,
    ) -> Result<(), WriteError> {
        let mut echo = Echo { cursor, status };
        for header in self.iter() {
            header.visit(&mut echo)?;
        }
        Ok(())
    }

    /// Execute every control within the limit and echo each with its status, returning the first
    /// error
    pub(crate) fn execute_with_response(
        &self,
        cursor: &mut WriteCursor,
        mode: ControlMode,
        transaction: &mut ControlTransaction,
        database: &mut DatabaseHandle,
        limit: ControlLimit,
    ) -> Result<CommandStatus, WriteError> {
        let mut respond = Respond {
            cursor,
            executor: Executor {
                mode,
                transaction,
                database,
                limit,
            },
        };
        let mut status = CommandStatus::Success;
        for header in self.iter() {
            status = status.first_error(header.visit(&mut respond)?);
        }
        Ok(status)
    }

    pub(crate) fn operate_no_ack(
        &self,
        transaction: &mut ControlTransaction,
        database: &mut DatabaseHandle,
        limit: ControlLimit,
    ) {
        let mut executor = Executor {
            mode: ControlMode::Operate(OperateType::DirectOperateNoAck),
            transaction,
            database,
            limit,
        };
        for header in self.iter() {
            header.visit(&mut executor);
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

/// Processes the controls of one header, whose object type and index width are type parameters
trait ControlVisitor {
    type Output;

    fn visit<I: Index, V: ControlType>(
        &mut self,
        seq: &CountSequence<Prefix<I, V>>,
    ) -> Self::Output;
}

/// Echoes each control with a fixed status
struct Echo<'c, 'w> {
    cursor: &'c mut WriteCursor<'w>,
    status: CommandStatus,
}

impl ControlVisitor for Echo<'_, '_> {
    type Output = Result<(), WriteError>;

    fn visit<I: Index, V: ControlType>(
        &mut self,
        seq: &CountSequence<Prefix<I, V>>,
    ) -> Self::Output {
        let mut writer = PrefixWriter::new();
        for item in seq.iter() {
            writer.write(self.cursor, item.value.with_status(self.status), item.index)?;
        }
        Ok(())
    }
}

/// Executes each control on the handler while the limit admits it
struct Executor<'t, 'a, 'd> {
    mode: ControlMode,
    transaction: &'t mut ControlTransaction<'a>,
    database: &'d mut DatabaseHandle,
    limit: ControlLimit,
}

impl Executor<'_, '_, '_> {
    fn execute<V: ControlType>(&mut self, control: V, index: u16) -> CommandStatus {
        if !self.limit.admit() {
            return CommandStatus::TooManyOps;
        }

        match self.mode {
            ControlMode::Select => self.transaction.select(control, index, self.database),
            ControlMode::Operate(op_type) => {
                self.transaction
                    .operate(control, index, op_type, self.database)
            }
        }
    }
}

impl ControlVisitor for Executor<'_, '_, '_> {
    type Output = ();

    fn visit<I: Index, V: ControlType>(
        &mut self,
        seq: &CountSequence<Prefix<I, V>>,
    ) -> Self::Output {
        for item in seq.iter() {
            self.execute(item.value, item.index.widen_to_u16());
        }
    }
}

/// Executes each control and echoes it with the status of its execution
struct Respond<'c, 'w, 't, 'a, 'd> {
    cursor: &'c mut WriteCursor<'w>,
    executor: Executor<'t, 'a, 'd>,
}

impl ControlVisitor for Respond<'_, '_, '_, '_, '_> {
    type Output = Result<CommandStatus, WriteError>;

    fn visit<I: Index, V: ControlType>(
        &mut self,
        seq: &CountSequence<Prefix<I, V>>,
    ) -> Self::Output {
        let mut writer = PrefixWriter::new();
        let mut ret = CommandStatus::Success;
        for item in seq.iter() {
            let status = self.executor.execute(item.value, item.index.widen_to_u16());
            writer.write(self.cursor, item.value.with_status(status), item.index)?;
            ret = ret.first_error(status);
        }
        Ok(ret)
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
        assert_matches!(headers.next(), Some(ControlHeader::OneByte(ControlObjects::Group12Var1(seq))) => assert_eq!(index(seq.single()), Some(3)));
        assert_matches!(headers.next(), Some(ControlHeader::OneByte(ControlObjects::Group41Var1(seq))) => assert_eq!(seq.single(), Some(Prefix { index: 3, value: g41v1 })));
        assert_matches!(headers.next(), Some(ControlHeader::OneByte(ControlObjects::Group41Var2(seq))) => assert_eq!(seq.single(), Some(Prefix { index: 3, value: g41v2 })));
        assert_matches!(headers.next(), Some(ControlHeader::OneByte(ControlObjects::Group41Var3(seq))) => assert_eq!(seq.single(), Some(Prefix { index: 3, value: g41v3 })));
        assert_matches!(headers.next(), Some(ControlHeader::OneByte(ControlObjects::Group41Var4(seq))) => assert_eq!(seq.single(), Some(Prefix { index: 3, value: g41v4 })));

        let index = |x: Option<Prefix<u16, Group12Var1>>| x.map(|x| x.index);
        assert_matches!(headers.next(), Some(ControlHeader::TwoByte(ControlObjects::Group12Var1(seq))) => assert_eq!(index(seq.single()), Some(259)));
        assert_matches!(headers.next(), Some(ControlHeader::TwoByte(ControlObjects::Group41Var1(seq))) => assert_eq!(seq.single(), Some(Prefix { index: 259, value: g41v1 })));
        assert_matches!(headers.next(), Some(ControlHeader::TwoByte(ControlObjects::Group41Var2(seq))) => assert_eq!(seq.single(), Some(Prefix { index: 259, value: g41v2 })));
        assert_matches!(headers.next(), Some(ControlHeader::TwoByte(ControlObjects::Group41Var3(seq))) => assert_eq!(seq.single(), Some(Prefix { index: 259, value: g41v3 })));
        assert_matches!(headers.next(), Some(ControlHeader::TwoByte(ControlObjects::Group41Var4(seq))) => assert_eq!(seq.single(), Some(Prefix { index: 259, value: g41v4 })));

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
