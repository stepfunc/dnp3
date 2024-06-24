use crate::app::gen::all::AllObjectsVariation;
use crate::app::gen::count::CountVariation;
use crate::app::gen::ranged::RangedVariation;
use crate::app::parse::parser::{HeaderDetails, ObjectHeader};
use crate::outstation::database::config::*;
use crate::outstation::database::details::range::static_db::IndexRange;

#[derive(Copy, Clone)]
pub(crate) enum StaticReadHeader {
    Class0,
    Binary(Option<StaticBinaryInputVariation>, Option<IndexRange>),
    DoubleBitBinary(
        Option<StaticDoubleBitBinaryInputVariation>,
        Option<IndexRange>,
    ),
    BinaryOutputStatus(
        Option<StaticBinaryOutputStatusVariation>,
        Option<IndexRange>,
    ),
    Counter(Option<StaticCounterVariation>, Option<IndexRange>),
    FrozenCounter(Option<StaticFrozenCounterVariation>, Option<IndexRange>),
    Analog(Option<StaticAnalogInputVariation>, Option<IndexRange>),
    #[allow(dead_code)] // TODO - this can be removed if we support these request
    FrozenAnalog(Option<StaticFrozenAnalogInputVariation>, Option<IndexRange>),
    AnalogOutputStatus(
        Option<StaticAnalogOutputStatusVariation>,
        Option<IndexRange>,
    ),
    OctetString(Option<IndexRange>),
    AnalogInputDeadBand(Option<AnalogInputDeadBandVariation>, Option<IndexRange>),
}

#[derive(Copy, Clone)]
pub(crate) enum EventReadHeader {
    // event classes with optional count limits
    Class1(Option<usize>),
    Class2(Option<usize>),
    Class3(Option<usize>),
    // events with optional count limits
    Binary(Option<EventBinaryInputVariation>, Option<usize>),
    DoubleBitBinary(Option<EventDoubleBitBinaryInputVariation>, Option<usize>),
    BinaryOutputStatus(Option<EventBinaryOutputStatusVariation>, Option<usize>),
    Counter(Option<EventCounterVariation>, Option<usize>),
    FrozenCounter(Option<EventFrozenCounterVariation>, Option<usize>),
    Analog(Option<EventAnalogInputVariation>, Option<usize>),
    #[allow(dead_code)] // TODO - this can be removed if we support these request
    FrozenAnalog(Option<EventFrozenAnalogInputVariation>, Option<usize>),
    AnalogOutputStatus(Option<EventAnalogOutputStatusVariation>, Option<usize>),
    OctetString(Option<usize>),
}

#[derive(Copy, Clone)]
pub(crate) enum AttrHeader {
    /// read a variation for all sets
    All(u8),
    /// read a variation for specific set(s)
    Specific(u8, IndexRange),
}

/// Enum representation of all header types that can be in a READ request
/// This type does not borrow any data so doesn't have lifetime constraints like
/// the object header types in the parser
#[derive(Copy, Clone)]
pub(crate) enum ReadHeader {
    Static(StaticReadHeader),
    Event(EventReadHeader),
    Attr(AttrHeader),
}

impl From<AttrHeader> for ReadHeader {
    fn from(value: AttrHeader) -> Self {
        ReadHeader::Attr(value)
    }
}

impl From<StaticReadHeader> for ReadHeader {
    fn from(x: StaticReadHeader) -> Self {
        ReadHeader::Static(x)
    }
}

impl From<EventReadHeader> for ReadHeader {
    fn from(x: EventReadHeader) -> Self {
        ReadHeader::Event(x)
    }
}

impl ReadHeader {
    pub(crate) fn get(header: &ObjectHeader) -> Option<ReadHeader> {
        let res = Self::get_impl(&header.details);
        if res.is_none() {
            tracing::warn!(
                "{} - {} not supported in READ requests",
                header.variation,
                header.details.qualifier()
            );
        }
        res
    }

    fn get_impl(header: &HeaderDetails) -> Option<ReadHeader> {
        match header {
            HeaderDetails::AllObjects(x) => Self::from_all_objects(x),
            HeaderDetails::OneByteCount(count, x) => Self::from_count(x, *count as usize),
            HeaderDetails::TwoByteCount(count, x) => Self::from_count(x, *count as usize),
            HeaderDetails::OneByteStartStop(start, stop, x) => {
                Self::from_range(x, IndexRange::new(*start as u16, *stop as u16))
            }
            HeaderDetails::TwoByteStartStop(start, stop, x) => {
                Self::from_range(x, IndexRange::new(*start, *stop))
            }
            HeaderDetails::OneByteCountAndPrefix(_, _) => None,
            HeaderDetails::TwoByteCountAndPrefix(_, _) => None,
            HeaderDetails::TwoByteFreeFormat(_, _) => None,
        }
    }

    fn from_all_objects(header: &AllObjectsVariation) -> Option<ReadHeader> {
        match header {
            // group 0
            AllObjectsVariation::Group0Var254 => Some(AttrHeader::All(254).into()),
            AllObjectsVariation::Group0(x) => Some(AttrHeader::All(*x).into()),
            // group 1
            AllObjectsVariation::Group1Var0 => Some(StaticReadHeader::Binary(None, None).into()),
            AllObjectsVariation::Group1Var1 => Some(
                StaticReadHeader::Binary(Some(StaticBinaryInputVariation::Group1Var1), None).into(),
            ),
            AllObjectsVariation::Group1Var2 => Some(
                StaticReadHeader::Binary(Some(StaticBinaryInputVariation::Group1Var2), None).into(),
            ),
            // group 2
            AllObjectsVariation::Group2Var0 => Some(EventReadHeader::Binary(None, None).into()),
            AllObjectsVariation::Group2Var1 => Some(
                EventReadHeader::Binary(Some(EventBinaryInputVariation::Group2Var1), None).into(),
            ),
            AllObjectsVariation::Group2Var2 => Some(
                EventReadHeader::Binary(Some(EventBinaryInputVariation::Group2Var2), None).into(),
            ),
            AllObjectsVariation::Group2Var3 => Some(
                EventReadHeader::Binary(Some(EventBinaryInputVariation::Group2Var3), None).into(),
            ),
            // group 3
            AllObjectsVariation::Group3Var0 => {
                Some(StaticReadHeader::DoubleBitBinary(None, None).into())
            }
            AllObjectsVariation::Group3Var1 => Some(
                StaticReadHeader::DoubleBitBinary(
                    Some(StaticDoubleBitBinaryInputVariation::Group3Var1),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group3Var2 => Some(
                StaticReadHeader::DoubleBitBinary(
                    Some(StaticDoubleBitBinaryInputVariation::Group3Var2),
                    None,
                )
                .into(),
            ),
            // group 4
            AllObjectsVariation::Group4Var0 => {
                Some(EventReadHeader::DoubleBitBinary(None, None).into())
            }
            AllObjectsVariation::Group4Var1 => Some(
                EventReadHeader::DoubleBitBinary(
                    Some(EventDoubleBitBinaryInputVariation::Group4Var1),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group4Var2 => Some(
                EventReadHeader::DoubleBitBinary(
                    Some(EventDoubleBitBinaryInputVariation::Group4Var2),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group4Var3 => Some(
                EventReadHeader::DoubleBitBinary(
                    Some(EventDoubleBitBinaryInputVariation::Group4Var3),
                    None,
                )
                .into(),
            ),
            // group 10
            AllObjectsVariation::Group10Var0 => {
                Some(StaticReadHeader::BinaryOutputStatus(None, None).into())
            }
            AllObjectsVariation::Group10Var1 => Some(
                StaticReadHeader::BinaryOutputStatus(
                    Some(StaticBinaryOutputStatusVariation::Group10Var1),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group10Var2 => Some(
                StaticReadHeader::BinaryOutputStatus(
                    Some(StaticBinaryOutputStatusVariation::Group10Var2),
                    None,
                )
                .into(),
            ),
            // group 11
            AllObjectsVariation::Group11Var0 => {
                Some(EventReadHeader::BinaryOutputStatus(None, None).into())
            }
            AllObjectsVariation::Group11Var1 => Some(
                EventReadHeader::BinaryOutputStatus(
                    Some(EventBinaryOutputStatusVariation::Group11Var1),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group11Var2 => Some(
                EventReadHeader::BinaryOutputStatus(
                    Some(EventBinaryOutputStatusVariation::Group11Var2),
                    None,
                )
                .into(),
            ),
            // group 13
            AllObjectsVariation::Group13Var1 => None,
            AllObjectsVariation::Group13Var2 => None,
            // group 20
            AllObjectsVariation::Group20Var0 => Some(StaticReadHeader::Counter(None, None).into()),
            AllObjectsVariation::Group20Var1 => Some(
                StaticReadHeader::Counter(Some(StaticCounterVariation::Group20Var1), None).into(),
            ),
            AllObjectsVariation::Group20Var2 => Some(
                StaticReadHeader::Counter(Some(StaticCounterVariation::Group20Var2), None).into(),
            ),
            AllObjectsVariation::Group20Var5 => Some(
                StaticReadHeader::Counter(Some(StaticCounterVariation::Group20Var5), None).into(),
            ),
            AllObjectsVariation::Group20Var6 => Some(
                StaticReadHeader::Counter(Some(StaticCounterVariation::Group20Var6), None).into(),
            ),
            // group 21
            AllObjectsVariation::Group21Var0 => {
                Some(StaticReadHeader::FrozenCounter(None, None).into())
            }
            AllObjectsVariation::Group21Var1 => Some(
                StaticReadHeader::FrozenCounter(
                    Some(StaticFrozenCounterVariation::Group21Var1),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group21Var2 => Some(
                StaticReadHeader::FrozenCounter(
                    Some(StaticFrozenCounterVariation::Group21Var2),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group21Var5 => Some(
                StaticReadHeader::FrozenCounter(
                    Some(StaticFrozenCounterVariation::Group21Var5),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group21Var6 => Some(
                StaticReadHeader::FrozenCounter(
                    Some(StaticFrozenCounterVariation::Group21Var6),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group21Var9 => Some(
                StaticReadHeader::FrozenCounter(
                    Some(StaticFrozenCounterVariation::Group21Var9),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group21Var10 => Some(
                StaticReadHeader::FrozenCounter(
                    Some(StaticFrozenCounterVariation::Group21Var10),
                    None,
                )
                .into(),
            ),
            // group 22
            AllObjectsVariation::Group22Var0 => Some(EventReadHeader::Counter(None, None).into()),
            AllObjectsVariation::Group22Var1 => Some(
                EventReadHeader::Counter(Some(EventCounterVariation::Group22Var1), None).into(),
            ),
            AllObjectsVariation::Group22Var2 => Some(
                EventReadHeader::Counter(Some(EventCounterVariation::Group22Var2), None).into(),
            ),
            AllObjectsVariation::Group22Var5 => Some(
                EventReadHeader::Counter(Some(EventCounterVariation::Group22Var5), None).into(),
            ),
            AllObjectsVariation::Group22Var6 => Some(
                EventReadHeader::Counter(Some(EventCounterVariation::Group22Var6), None).into(),
            ),
            // group 23
            AllObjectsVariation::Group23Var0 => {
                Some(EventReadHeader::FrozenCounter(None, None).into())
            }
            AllObjectsVariation::Group23Var1 => Some(
                EventReadHeader::FrozenCounter(
                    Some(EventFrozenCounterVariation::Group23Var1),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group23Var2 => Some(
                EventReadHeader::FrozenCounter(
                    Some(EventFrozenCounterVariation::Group23Var2),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group23Var5 => Some(
                EventReadHeader::FrozenCounter(
                    Some(EventFrozenCounterVariation::Group23Var5),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group23Var6 => Some(
                EventReadHeader::FrozenCounter(
                    Some(EventFrozenCounterVariation::Group23Var6),
                    None,
                )
                .into(),
            ),
            // group 30
            AllObjectsVariation::Group30Var0 => Some(StaticReadHeader::Analog(None, None).into()),
            AllObjectsVariation::Group30Var1 => Some(
                StaticReadHeader::Analog(Some(StaticAnalogInputVariation::Group30Var1), None)
                    .into(),
            ),
            AllObjectsVariation::Group30Var2 => Some(
                StaticReadHeader::Analog(Some(StaticAnalogInputVariation::Group30Var2), None)
                    .into(),
            ),
            AllObjectsVariation::Group30Var3 => Some(
                StaticReadHeader::Analog(Some(StaticAnalogInputVariation::Group30Var3), None)
                    .into(),
            ),
            AllObjectsVariation::Group30Var4 => Some(
                StaticReadHeader::Analog(Some(StaticAnalogInputVariation::Group30Var4), None)
                    .into(),
            ),
            AllObjectsVariation::Group30Var5 => Some(
                StaticReadHeader::Analog(Some(StaticAnalogInputVariation::Group30Var5), None)
                    .into(),
            ),
            AllObjectsVariation::Group30Var6 => Some(
                StaticReadHeader::Analog(Some(StaticAnalogInputVariation::Group30Var6), None)
                    .into(),
            ),
            // group 31
            AllObjectsVariation::Group31Var0 => {
                Some(StaticReadHeader::FrozenAnalog(None, None).into())
            }
            AllObjectsVariation::Group31Var1 => Some(
                StaticReadHeader::FrozenAnalog(
                    Some(StaticFrozenAnalogInputVariation::Group31Var1),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group31Var2 => Some(
                StaticReadHeader::FrozenAnalog(
                    Some(StaticFrozenAnalogInputVariation::Group31Var2),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group31Var3 => Some(
                StaticReadHeader::FrozenAnalog(
                    Some(StaticFrozenAnalogInputVariation::Group31Var3),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group31Var4 => Some(
                StaticReadHeader::FrozenAnalog(
                    Some(StaticFrozenAnalogInputVariation::Group31Var4),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group31Var5 => Some(
                StaticReadHeader::FrozenAnalog(
                    Some(StaticFrozenAnalogInputVariation::Group31Var5),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group31Var6 => Some(
                StaticReadHeader::FrozenAnalog(
                    Some(StaticFrozenAnalogInputVariation::Group31Var6),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group31Var7 => Some(
                StaticReadHeader::FrozenAnalog(
                    Some(StaticFrozenAnalogInputVariation::Group31Var7),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group31Var8 => Some(
                StaticReadHeader::FrozenAnalog(
                    Some(StaticFrozenAnalogInputVariation::Group31Var8),
                    None,
                )
                .into(),
            ),
            // group 32
            AllObjectsVariation::Group32Var0 => Some(EventReadHeader::Analog(None, None).into()),
            AllObjectsVariation::Group32Var1 => Some(
                EventReadHeader::Analog(Some(EventAnalogInputVariation::Group32Var1), None).into(),
            ),
            AllObjectsVariation::Group32Var2 => Some(
                EventReadHeader::Analog(Some(EventAnalogInputVariation::Group32Var2), None).into(),
            ),
            AllObjectsVariation::Group32Var3 => Some(
                EventReadHeader::Analog(Some(EventAnalogInputVariation::Group32Var3), None).into(),
            ),
            AllObjectsVariation::Group32Var4 => Some(
                EventReadHeader::Analog(Some(EventAnalogInputVariation::Group32Var4), None).into(),
            ),
            AllObjectsVariation::Group32Var5 => Some(
                EventReadHeader::Analog(Some(EventAnalogInputVariation::Group32Var5), None).into(),
            ),
            AllObjectsVariation::Group32Var6 => Some(
                EventReadHeader::Analog(Some(EventAnalogInputVariation::Group32Var6), None).into(),
            ),
            AllObjectsVariation::Group32Var7 => Some(
                EventReadHeader::Analog(Some(EventAnalogInputVariation::Group32Var7), None).into(),
            ),
            AllObjectsVariation::Group32Var8 => Some(
                EventReadHeader::Analog(Some(EventAnalogInputVariation::Group32Var8), None).into(),
            ),
            // group 33
            AllObjectsVariation::Group33Var0 => {
                Some(EventReadHeader::FrozenAnalog(None, None).into())
            }
            AllObjectsVariation::Group33Var1 => Some(
                EventReadHeader::FrozenAnalog(
                    Some(EventFrozenAnalogInputVariation::Group33Var1),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group33Var2 => Some(
                EventReadHeader::FrozenAnalog(
                    Some(EventFrozenAnalogInputVariation::Group33Var2),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group33Var3 => Some(
                EventReadHeader::FrozenAnalog(
                    Some(EventFrozenAnalogInputVariation::Group33Var3),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group33Var4 => Some(
                EventReadHeader::FrozenAnalog(
                    Some(EventFrozenAnalogInputVariation::Group33Var4),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group33Var5 => Some(
                EventReadHeader::FrozenAnalog(
                    Some(EventFrozenAnalogInputVariation::Group33Var5),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group33Var6 => Some(
                EventReadHeader::FrozenAnalog(
                    Some(EventFrozenAnalogInputVariation::Group33Var6),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group33Var7 => Some(
                EventReadHeader::FrozenAnalog(
                    Some(EventFrozenAnalogInputVariation::Group33Var7),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group33Var8 => Some(
                EventReadHeader::FrozenAnalog(
                    Some(EventFrozenAnalogInputVariation::Group33Var8),
                    None,
                )
                .into(),
            ),
            // Group 34
            AllObjectsVariation::Group34Var0 => {
                Some(StaticReadHeader::AnalogInputDeadBand(None, None).into())
            }
            AllObjectsVariation::Group34Var1 => Some(
                StaticReadHeader::AnalogInputDeadBand(
                    Some(AnalogInputDeadBandVariation::Group34Var1),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group34Var2 => Some(
                StaticReadHeader::AnalogInputDeadBand(
                    Some(AnalogInputDeadBandVariation::Group34Var2),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group34Var3 => Some(
                StaticReadHeader::AnalogInputDeadBand(
                    Some(AnalogInputDeadBandVariation::Group34Var3),
                    None,
                )
                .into(),
            ),
            // group 40
            AllObjectsVariation::Group40Var0 => {
                Some(StaticReadHeader::AnalogOutputStatus(None, None).into())
            }
            AllObjectsVariation::Group40Var1 => Some(
                StaticReadHeader::AnalogOutputStatus(
                    Some(StaticAnalogOutputStatusVariation::Group40Var1),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group40Var2 => Some(
                StaticReadHeader::AnalogOutputStatus(
                    Some(StaticAnalogOutputStatusVariation::Group40Var2),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group40Var3 => Some(
                StaticReadHeader::AnalogOutputStatus(
                    Some(StaticAnalogOutputStatusVariation::Group40Var3),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group40Var4 => Some(
                StaticReadHeader::AnalogOutputStatus(
                    Some(StaticAnalogOutputStatusVariation::Group40Var4),
                    None,
                )
                .into(),
            ),
            // group 42
            AllObjectsVariation::Group42Var0 => {
                Some(EventReadHeader::AnalogOutputStatus(None, None).into())
            }
            AllObjectsVariation::Group42Var1 => Some(
                EventReadHeader::AnalogOutputStatus(
                    Some(EventAnalogOutputStatusVariation::Group42Var1),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group42Var2 => Some(
                EventReadHeader::AnalogOutputStatus(
                    Some(EventAnalogOutputStatusVariation::Group42Var2),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group42Var3 => Some(
                EventReadHeader::AnalogOutputStatus(
                    Some(EventAnalogOutputStatusVariation::Group42Var3),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group42Var4 => Some(
                EventReadHeader::AnalogOutputStatus(
                    Some(EventAnalogOutputStatusVariation::Group42Var4),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group42Var5 => Some(
                EventReadHeader::AnalogOutputStatus(
                    Some(EventAnalogOutputStatusVariation::Group42Var5),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group42Var6 => Some(
                EventReadHeader::AnalogOutputStatus(
                    Some(EventAnalogOutputStatusVariation::Group42Var6),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group42Var7 => Some(
                EventReadHeader::AnalogOutputStatus(
                    Some(EventAnalogOutputStatusVariation::Group42Var7),
                    None,
                )
                .into(),
            ),
            AllObjectsVariation::Group42Var8 => Some(
                EventReadHeader::AnalogOutputStatus(
                    Some(EventAnalogOutputStatusVariation::Group42Var8),
                    None,
                )
                .into(),
            ),
            // group 43,
            AllObjectsVariation::Group43Var1 => None,
            AllObjectsVariation::Group43Var2 => None,
            AllObjectsVariation::Group43Var3 => None,
            AllObjectsVariation::Group43Var4 => None,
            AllObjectsVariation::Group43Var5 => None,
            AllObjectsVariation::Group43Var6 => None,
            AllObjectsVariation::Group43Var7 => None,
            AllObjectsVariation::Group43Var8 => None,
            // group 60
            AllObjectsVariation::Group60Var1 => Some(StaticReadHeader::Class0.into()),
            AllObjectsVariation::Group60Var2 => Some(EventReadHeader::Class1(None).into()),
            AllObjectsVariation::Group60Var3 => Some(EventReadHeader::Class2(None).into()),
            AllObjectsVariation::Group60Var4 => Some(EventReadHeader::Class3(None).into()),
            // group 80
            AllObjectsVariation::Group80Var1 => None,
            // group 102
            AllObjectsVariation::Group102Var0 => None,
            AllObjectsVariation::Group102Var1 => None,
            // group 110
            AllObjectsVariation::Group110Var0 => Some(StaticReadHeader::OctetString(None).into()),
            // group 111
            AllObjectsVariation::Group111Var0 => Some(EventReadHeader::OctetString(None).into()),
        }
    }

    fn from_count(header: &CountVariation, count: usize) -> Option<ReadHeader> {
        match header {
            CountVariation::Group2Var0 => Some(EventReadHeader::Binary(None, Some(count)).into()),
            CountVariation::Group2Var1 => Some(
                EventReadHeader::Binary(Some(EventBinaryInputVariation::Group2Var1), Some(count))
                    .into(),
            ),
            CountVariation::Group2Var2 => Some(
                EventReadHeader::Binary(Some(EventBinaryInputVariation::Group2Var2), Some(count))
                    .into(),
            ),
            CountVariation::Group2Var3 => Some(
                EventReadHeader::Binary(Some(EventBinaryInputVariation::Group2Var3), Some(count))
                    .into(),
            ),
            CountVariation::Group4Var0 => {
                Some(EventReadHeader::DoubleBitBinary(None, Some(count)).into())
            }
            CountVariation::Group4Var1 => Some(
                EventReadHeader::DoubleBitBinary(
                    Some(EventDoubleBitBinaryInputVariation::Group4Var1),
                    Some(count),
                )
                .into(),
            ),
            CountVariation::Group4Var2 => Some(
                EventReadHeader::DoubleBitBinary(
                    Some(EventDoubleBitBinaryInputVariation::Group4Var2),
                    Some(count),
                )
                .into(),
            ),
            CountVariation::Group4Var3 => Some(
                EventReadHeader::DoubleBitBinary(
                    Some(EventDoubleBitBinaryInputVariation::Group4Var3),
                    Some(count),
                )
                .into(),
            ),
            CountVariation::Group11Var0 => {
                Some(EventReadHeader::BinaryOutputStatus(None, Some(count)).into())
            }
            CountVariation::Group11Var1 => Some(
                EventReadHeader::BinaryOutputStatus(
                    Some(EventBinaryOutputStatusVariation::Group11Var1),
                    Some(count),
                )
                .into(),
            ),
            CountVariation::Group11Var2 => Some(
                EventReadHeader::BinaryOutputStatus(
                    Some(EventBinaryOutputStatusVariation::Group11Var2),
                    Some(count),
                )
                .into(),
            ),
            CountVariation::Group13Var1 => None,
            CountVariation::Group13Var2 => None,
            CountVariation::Group22Var0 => Some(EventReadHeader::Counter(None, Some(count)).into()),
            CountVariation::Group22Var1 => Some(
                EventReadHeader::Counter(Some(EventCounterVariation::Group22Var1), Some(count))
                    .into(),
            ),
            CountVariation::Group22Var2 => Some(
                EventReadHeader::Counter(Some(EventCounterVariation::Group22Var2), Some(count))
                    .into(),
            ),
            CountVariation::Group22Var5 => Some(
                EventReadHeader::Counter(Some(EventCounterVariation::Group22Var5), Some(count))
                    .into(),
            ),
            CountVariation::Group22Var6 => Some(
                EventReadHeader::Counter(Some(EventCounterVariation::Group22Var6), Some(count))
                    .into(),
            ),
            CountVariation::Group23Var0 => {
                Some(EventReadHeader::FrozenCounter(None, Some(count)).into())
            }
            CountVariation::Group23Var1 => Some(
                EventReadHeader::FrozenCounter(
                    Some(EventFrozenCounterVariation::Group23Var1),
                    Some(count),
                )
                .into(),
            ),
            CountVariation::Group23Var2 => Some(
                EventReadHeader::FrozenCounter(
                    Some(EventFrozenCounterVariation::Group23Var2),
                    Some(count),
                )
                .into(),
            ),
            CountVariation::Group23Var5 => Some(
                EventReadHeader::FrozenCounter(
                    Some(EventFrozenCounterVariation::Group23Var5),
                    Some(count),
                )
                .into(),
            ),
            CountVariation::Group23Var6 => Some(
                EventReadHeader::FrozenCounter(
                    Some(EventFrozenCounterVariation::Group23Var6),
                    Some(count),
                )
                .into(),
            ),
            CountVariation::Group32Var0 => Some(EventReadHeader::Analog(None, Some(count)).into()),
            CountVariation::Group32Var1 => Some(
                EventReadHeader::Analog(Some(EventAnalogInputVariation::Group32Var1), Some(count))
                    .into(),
            ),
            CountVariation::Group32Var2 => Some(
                EventReadHeader::Analog(Some(EventAnalogInputVariation::Group32Var2), Some(count))
                    .into(),
            ),
            CountVariation::Group32Var3 => Some(
                EventReadHeader::Analog(Some(EventAnalogInputVariation::Group32Var3), Some(count))
                    .into(),
            ),
            CountVariation::Group32Var4 => Some(
                EventReadHeader::Analog(Some(EventAnalogInputVariation::Group32Var4), Some(count))
                    .into(),
            ),
            CountVariation::Group32Var5 => Some(
                EventReadHeader::Analog(Some(EventAnalogInputVariation::Group32Var5), Some(count))
                    .into(),
            ),
            CountVariation::Group32Var6 => Some(
                EventReadHeader::Analog(Some(EventAnalogInputVariation::Group32Var6), Some(count))
                    .into(),
            ),
            CountVariation::Group32Var7 => Some(
                EventReadHeader::Analog(Some(EventAnalogInputVariation::Group32Var7), Some(count))
                    .into(),
            ),
            CountVariation::Group32Var8 => Some(
                EventReadHeader::Analog(Some(EventAnalogInputVariation::Group32Var8), Some(count))
                    .into(),
            ),
            CountVariation::Group33Var0 => {
                Some(EventReadHeader::FrozenAnalog(None, Some(count)).into())
            }
            CountVariation::Group33Var1 => Some(
                EventReadHeader::FrozenAnalog(
                    Some(EventFrozenAnalogInputVariation::Group33Var1),
                    Some(count),
                )
                .into(),
            ),
            CountVariation::Group33Var2 => Some(
                EventReadHeader::FrozenAnalog(
                    Some(EventFrozenAnalogInputVariation::Group33Var2),
                    Some(count),
                )
                .into(),
            ),
            CountVariation::Group33Var3 => Some(
                EventReadHeader::FrozenAnalog(
                    Some(EventFrozenAnalogInputVariation::Group33Var3),
                    Some(count),
                )
                .into(),
            ),
            CountVariation::Group33Var4 => Some(
                EventReadHeader::FrozenAnalog(
                    Some(EventFrozenAnalogInputVariation::Group33Var4),
                    Some(count),
                )
                .into(),
            ),
            CountVariation::Group33Var5 => Some(
                EventReadHeader::FrozenAnalog(
                    Some(EventFrozenAnalogInputVariation::Group33Var5),
                    Some(count),
                )
                .into(),
            ),
            CountVariation::Group33Var6 => Some(
                EventReadHeader::FrozenAnalog(
                    Some(EventFrozenAnalogInputVariation::Group33Var6),
                    Some(count),
                )
                .into(),
            ),
            CountVariation::Group33Var7 => Some(
                EventReadHeader::FrozenAnalog(
                    Some(EventFrozenAnalogInputVariation::Group33Var7),
                    Some(count),
                )
                .into(),
            ),
            CountVariation::Group33Var8 => Some(
                EventReadHeader::FrozenAnalog(
                    Some(EventFrozenAnalogInputVariation::Group33Var8),
                    Some(count),
                )
                .into(),
            ),
            CountVariation::Group42Var0 => {
                Some(EventReadHeader::AnalogOutputStatus(None, Some(count)).into())
            }
            CountVariation::Group42Var1 => Some(
                EventReadHeader::AnalogOutputStatus(
                    Some(EventAnalogOutputStatusVariation::Group42Var1),
                    Some(count),
                )
                .into(),
            ),
            CountVariation::Group42Var2 => Some(
                EventReadHeader::AnalogOutputStatus(
                    Some(EventAnalogOutputStatusVariation::Group42Var2),
                    Some(count),
                )
                .into(),
            ),
            CountVariation::Group42Var3 => Some(
                EventReadHeader::AnalogOutputStatus(
                    Some(EventAnalogOutputStatusVariation::Group42Var3),
                    Some(count),
                )
                .into(),
            ),
            CountVariation::Group42Var4 => Some(
                EventReadHeader::AnalogOutputStatus(
                    Some(EventAnalogOutputStatusVariation::Group42Var4),
                    Some(count),
                )
                .into(),
            ),
            CountVariation::Group42Var5 => Some(
                EventReadHeader::AnalogOutputStatus(
                    Some(EventAnalogOutputStatusVariation::Group42Var5),
                    Some(count),
                )
                .into(),
            ),
            CountVariation::Group42Var6 => Some(
                EventReadHeader::AnalogOutputStatus(
                    Some(EventAnalogOutputStatusVariation::Group42Var6),
                    Some(count),
                )
                .into(),
            ),
            CountVariation::Group42Var7 => Some(
                EventReadHeader::AnalogOutputStatus(
                    Some(EventAnalogOutputStatusVariation::Group42Var7),
                    Some(count),
                )
                .into(),
            ),
            CountVariation::Group42Var8 => Some(
                EventReadHeader::AnalogOutputStatus(
                    Some(EventAnalogOutputStatusVariation::Group42Var8),
                    Some(count),
                )
                .into(),
            ),
            CountVariation::Group43Var1 => None,
            CountVariation::Group43Var2 => None,
            CountVariation::Group43Var3 => None,
            CountVariation::Group43Var4 => None,
            CountVariation::Group43Var5 => None,
            CountVariation::Group43Var6 => None,
            CountVariation::Group43Var7 => None,
            CountVariation::Group43Var8 => None,
            CountVariation::Group50Var1(_) => None,
            CountVariation::Group50Var2(_) => None,
            CountVariation::Group50Var3(_) => None,
            CountVariation::Group50Var4(_) => None,
            CountVariation::Group51Var1(_) => None,
            CountVariation::Group51Var2(_) => None,
            CountVariation::Group52Var1(_) => None,
            CountVariation::Group52Var2(_) => None,
            CountVariation::Group60Var2 => Some(EventReadHeader::Class1(Some(count)).into()),
            CountVariation::Group60Var3 => Some(EventReadHeader::Class2(Some(count)).into()),
            CountVariation::Group60Var4 => Some(EventReadHeader::Class3(Some(count)).into()),
            CountVariation::Group111Var0 => Some(EventReadHeader::OctetString(Some(count)).into()),
            CountVariation::Group111VarX(_) => None,
        }
    }

    fn from_range(header: &RangedVariation, range: IndexRange) -> Option<ReadHeader> {
        match header {
            // group 0
            RangedVariation::Group0Var254 => Some(AttrHeader::Specific(254, range).into()),
            RangedVariation::Group0(var, None) => Some(AttrHeader::Specific(*var, range).into()),
            // This should never happen, but if does the correct behavior is to reject it
            RangedVariation::Group0(_, Some(_)) => None,
            // group 1
            RangedVariation::Group1Var0 => Some(StaticReadHeader::Binary(None, Some(range)).into()),
            RangedVariation::Group1Var1(_) => Some(
                StaticReadHeader::Binary(Some(StaticBinaryInputVariation::Group1Var1), Some(range))
                    .into(),
            ),
            RangedVariation::Group1Var2(_) => Some(
                StaticReadHeader::Binary(Some(StaticBinaryInputVariation::Group1Var2), Some(range))
                    .into(),
            ),
            // group 3
            RangedVariation::Group3Var0 => {
                Some(StaticReadHeader::DoubleBitBinary(None, Some(range)).into())
            }
            RangedVariation::Group3Var1(_) => Some(
                StaticReadHeader::DoubleBitBinary(
                    Some(StaticDoubleBitBinaryInputVariation::Group3Var1),
                    Some(range),
                )
                .into(),
            ),
            RangedVariation::Group3Var2(_) => Some(
                StaticReadHeader::DoubleBitBinary(
                    Some(StaticDoubleBitBinaryInputVariation::Group3Var2),
                    Some(range),
                )
                .into(),
            ),
            // group 10
            RangedVariation::Group10Var0 => {
                Some(StaticReadHeader::BinaryOutputStatus(None, Some(range)).into())
            }
            RangedVariation::Group10Var1(_) => Some(
                StaticReadHeader::BinaryOutputStatus(
                    Some(StaticBinaryOutputStatusVariation::Group10Var1),
                    Some(range),
                )
                .into(),
            ),
            RangedVariation::Group10Var2(_) => Some(
                StaticReadHeader::BinaryOutputStatus(
                    Some(StaticBinaryOutputStatusVariation::Group10Var2),
                    Some(range),
                )
                .into(),
            ),
            // group 20
            RangedVariation::Group20Var0 => {
                Some(StaticReadHeader::Counter(None, Some(range)).into())
            }
            RangedVariation::Group20Var1(_) => Some(
                StaticReadHeader::Counter(Some(StaticCounterVariation::Group20Var1), Some(range))
                    .into(),
            ),
            RangedVariation::Group20Var2(_) => Some(
                StaticReadHeader::Counter(Some(StaticCounterVariation::Group20Var2), Some(range))
                    .into(),
            ),
            RangedVariation::Group20Var5(_) => Some(
                StaticReadHeader::Counter(Some(StaticCounterVariation::Group20Var5), Some(range))
                    .into(),
            ),
            RangedVariation::Group20Var6(_) => Some(
                StaticReadHeader::Counter(Some(StaticCounterVariation::Group20Var6), Some(range))
                    .into(),
            ),
            // group 21
            RangedVariation::Group21Var0 => {
                Some(StaticReadHeader::FrozenCounter(None, Some(range)).into())
            }
            RangedVariation::Group21Var1(_) => Some(
                StaticReadHeader::FrozenCounter(
                    Some(StaticFrozenCounterVariation::Group21Var1),
                    Some(range),
                )
                .into(),
            ),
            RangedVariation::Group21Var2(_) => Some(
                StaticReadHeader::FrozenCounter(
                    Some(StaticFrozenCounterVariation::Group21Var2),
                    Some(range),
                )
                .into(),
            ),
            RangedVariation::Group21Var5(_) => Some(
                StaticReadHeader::FrozenCounter(
                    Some(StaticFrozenCounterVariation::Group21Var5),
                    Some(range),
                )
                .into(),
            ),
            RangedVariation::Group21Var6(_) => Some(
                StaticReadHeader::FrozenCounter(
                    Some(StaticFrozenCounterVariation::Group21Var6),
                    Some(range),
                )
                .into(),
            ),
            RangedVariation::Group21Var9(_) => Some(
                StaticReadHeader::FrozenCounter(
                    Some(StaticFrozenCounterVariation::Group21Var9),
                    Some(range),
                )
                .into(),
            ),
            RangedVariation::Group21Var10(_) => Some(
                StaticReadHeader::FrozenCounter(
                    Some(StaticFrozenCounterVariation::Group21Var10),
                    Some(range),
                )
                .into(),
            ),
            // group 30
            RangedVariation::Group30Var0 => {
                Some(StaticReadHeader::Analog(None, Some(range)).into())
            }
            RangedVariation::Group30Var1(_) => Some(
                StaticReadHeader::Analog(
                    Some(StaticAnalogInputVariation::Group30Var1),
                    Some(range),
                )
                .into(),
            ),
            RangedVariation::Group30Var2(_) => Some(
                StaticReadHeader::Analog(
                    Some(StaticAnalogInputVariation::Group30Var2),
                    Some(range),
                )
                .into(),
            ),
            RangedVariation::Group30Var3(_) => Some(
                StaticReadHeader::Analog(
                    Some(StaticAnalogInputVariation::Group30Var3),
                    Some(range),
                )
                .into(),
            ),
            RangedVariation::Group30Var4(_) => Some(
                StaticReadHeader::Analog(
                    Some(StaticAnalogInputVariation::Group30Var4),
                    Some(range),
                )
                .into(),
            ),
            RangedVariation::Group30Var5(_) => Some(
                StaticReadHeader::Analog(
                    Some(StaticAnalogInputVariation::Group30Var5),
                    Some(range),
                )
                .into(),
            ),
            RangedVariation::Group30Var6(_) => Some(
                StaticReadHeader::Analog(
                    Some(StaticAnalogInputVariation::Group30Var6),
                    Some(range),
                )
                .into(),
            ),
            // group 31
            RangedVariation::Group31Var0 => {
                Some(StaticReadHeader::FrozenAnalog(None, Some(range)).into())
            }
            RangedVariation::Group31Var1(_) => Some(
                StaticReadHeader::FrozenAnalog(
                    Some(StaticFrozenAnalogInputVariation::Group31Var1),
                    Some(range),
                )
                .into(),
            ),
            RangedVariation::Group31Var2(_) => Some(
                StaticReadHeader::FrozenAnalog(
                    Some(StaticFrozenAnalogInputVariation::Group31Var2),
                    Some(range),
                )
                .into(),
            ),
            RangedVariation::Group31Var3(_) => Some(
                StaticReadHeader::FrozenAnalog(
                    Some(StaticFrozenAnalogInputVariation::Group31Var3),
                    Some(range),
                )
                .into(),
            ),
            RangedVariation::Group31Var4(_) => Some(
                StaticReadHeader::FrozenAnalog(
                    Some(StaticFrozenAnalogInputVariation::Group31Var4),
                    Some(range),
                )
                .into(),
            ),
            RangedVariation::Group31Var5(_) => Some(
                StaticReadHeader::FrozenAnalog(
                    Some(StaticFrozenAnalogInputVariation::Group31Var5),
                    Some(range),
                )
                .into(),
            ),
            RangedVariation::Group31Var6(_) => Some(
                StaticReadHeader::FrozenAnalog(
                    Some(StaticFrozenAnalogInputVariation::Group31Var6),
                    Some(range),
                )
                .into(),
            ),
            RangedVariation::Group31Var7(_) => Some(
                StaticReadHeader::FrozenAnalog(
                    Some(StaticFrozenAnalogInputVariation::Group31Var7),
                    Some(range),
                )
                .into(),
            ),
            RangedVariation::Group31Var8(_) => Some(
                StaticReadHeader::FrozenAnalog(
                    Some(StaticFrozenAnalogInputVariation::Group31Var8),
                    Some(range),
                )
                .into(),
            ),
            // group 34
            RangedVariation::Group34Var1(_) => Some(
                StaticReadHeader::AnalogInputDeadBand(
                    Some(AnalogInputDeadBandVariation::Group34Var1),
                    Some(range),
                )
                .into(),
            ),
            RangedVariation::Group34Var2(_) => Some(
                StaticReadHeader::AnalogInputDeadBand(
                    Some(AnalogInputDeadBandVariation::Group34Var2),
                    Some(range),
                )
                .into(),
            ),
            RangedVariation::Group34Var3(_) => Some(
                StaticReadHeader::AnalogInputDeadBand(
                    Some(AnalogInputDeadBandVariation::Group34Var3),
                    Some(range),
                )
                .into(),
            ),
            // group 40
            RangedVariation::Group40Var0 => {
                Some(StaticReadHeader::AnalogOutputStatus(None, Some(range)).into())
            }
            RangedVariation::Group40Var1(_) => Some(
                StaticReadHeader::AnalogOutputStatus(
                    Some(StaticAnalogOutputStatusVariation::Group40Var1),
                    Some(range),
                )
                .into(),
            ),
            RangedVariation::Group40Var2(_) => Some(
                StaticReadHeader::AnalogOutputStatus(
                    Some(StaticAnalogOutputStatusVariation::Group40Var2),
                    Some(range),
                )
                .into(),
            ),
            RangedVariation::Group40Var3(_) => Some(
                StaticReadHeader::AnalogOutputStatus(
                    Some(StaticAnalogOutputStatusVariation::Group40Var3),
                    Some(range),
                )
                .into(),
            ),
            RangedVariation::Group40Var4(_) => Some(
                StaticReadHeader::AnalogOutputStatus(
                    Some(StaticAnalogOutputStatusVariation::Group40Var4),
                    Some(range),
                )
                .into(),
            ),
            // group 80
            RangedVariation::Group80Var1(_) => None,
            // group 102
            RangedVariation::Group102Var0 => None,
            RangedVariation::Group102Var1(_) => None,
            // group 110
            RangedVariation::Group110Var0 => {
                Some(StaticReadHeader::OctetString(Some(range)).into())
            }
            // group 111
            RangedVariation::Group110VarX(_, _) => None,
        }
    }
}
