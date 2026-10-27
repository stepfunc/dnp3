// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::parse::parser::{HeaderCollection, HeaderIterator, ObjectHeader, ReadQualifier};
use crate::app::{FunctionCode, Iin2, QualifierCode, Variation};
use crate::outstation::database::details::range::static_db::IndexRange;
use crate::outstation::database::variations::*;
use crate::outstation::database::EventClass;
use crate::outstation::read_log::ReadLog;
use crate::outstation::target::{Selector, Target};

mod target;
#[cfg(test)]
mod target_tests;

/// The static points of one type that a READ selects, in the variation it asks for, if any
#[derive(Copy, Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub(crate) enum StaticTarget {
    Binary(Option<StaticBinaryInputVariation>),
    DoubleBitBinary(Option<StaticDoubleBitBinaryInputVariation>),
    BinaryOutputStatus(Option<StaticBinaryOutputStatusVariation>),
    Counter(Option<StaticCounterVariation>),
    FrozenCounter(Option<StaticFrozenCounterVariation>),
    Analog(Option<StaticAnalogInputVariation>),
    #[expect(dead_code, reason = "frozen analog reads are not supported yet")]
    FrozenAnalog(Option<StaticFrozenAnalogInputVariation>),
    AnalogOutputStatus(Option<StaticAnalogOutputStatusVariation>),
    OctetString,
    AnalogInputDeadBand(Option<AnalogInputDeadBandVariation>),
}

/// The events that a READ selects: those of a class, or those of one type in the variation it
/// asks for, if any
#[derive(Copy, Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub(crate) enum EventTarget {
    Class(EventClass),
    Binary(Option<EventBinaryInputVariation>),
    DoubleBitBinary(Option<EventDoubleBitBinaryInputVariation>),
    BinaryOutputStatus(Option<EventBinaryOutputStatusVariation>),
    Counter(Option<EventCounterVariation>),
    FrozenCounter(Option<EventFrozenCounterVariation>),
    Analog(Option<EventAnalogInputVariation>),
    #[expect(dead_code, reason = "frozen analog reads are not supported yet")]
    FrozenAnalog(Option<EventFrozenAnalogInputVariation>),
    AnalogOutputStatus(Option<EventAnalogOutputStatusVariation>),
    OctetString,
}

#[derive(Copy, Clone)]
pub(crate) enum AttrHeader {
    /// read a variation for all sets
    All(u8),
    /// read a variation for specific set(s)
    Specific(u8, IndexRange),
}

/// What a header in a READ request selects
///
/// This type does not borrow any data so doesn't have lifetime constraints like the object
/// header types in the parser
#[derive(Copy, Clone)]
pub(crate) enum ReadHeader {
    /// the static points that class 0 includes
    Class0,
    /// the static points of one type: all of them, or those in a range
    Static(StaticTarget, Option<IndexRange>),
    /// the events of a class or of one type: all of them, or at most a count
    Event(EventTarget, Option<usize>),
    Attr(AttrHeader),
}

/// What a READ of a variation selects, whatever its qualifier
#[derive(Copy, Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
enum ReadTarget {
    Class0,
    Static(StaticTarget),
    Event(EventTarget),
    Attr(u8),
}

impl ReadHeader {
    /// What a header in a READ request selects, or the error for one that READ doesn't support
    fn parse(header: ObjectHeader, selector: &mut Selector) -> Result<ReadHeader, Iin2> {
        selector.select::<ReadTarget>(header)
    }
}

/// What each header of a READ request that READ supports selects, in request order, with the
/// variation and qualifier it was read with
///
/// The headers that READ doesn't support are skipped. [`ReadHeaders::finish`] gives their error
/// and logs them once for the request.
pub(crate) struct ReadHeaders<'a> {
    headers: HeaderIterator<'a>,
    selector: Selector,
    unsupported: Iin2,
}

impl<'a> ReadHeaders<'a> {
    pub(crate) fn new(headers: &HeaderCollection<'a>) -> Self {
        Self {
            headers: headers.iter(),
            selector: Selector::new(FunctionCode::Read),
            unsupported: Iin2::default(),
        }
    }

    /// The error IIN2 bits of every header in the request that READ doesn't support, which this
    /// logs
    pub(crate) fn finish(mut self, log: impl ReadLog) -> Iin2 {
        self.by_ref().for_each(drop);
        if let Some((header, count)) = self.selector.unsupported() {
            log.unsupported_headers(header, count);
        }
        self.unsupported
    }
}

impl Iterator for ReadHeaders<'_> {
    type Item = (ReadHeader, (Variation, QualifierCode));

    fn next(&mut self) -> Option<Self::Item> {
        for header in self.headers.by_ref() {
            let read_with = (header.variation(), header.qualifier());
            match ReadHeader::parse(header, &mut self.selector) {
                Ok(read) => return Some((read, read_with)),
                Err(err) => self.unsupported |= err,
            }
        }
        None
    }
}

impl Target for ReadTarget {
    type Header<'a> = ReadHeader;

    fn with(self, header: ObjectHeader<'_>) -> Option<ReadHeader> {
        let qualifier = match header {
            ObjectHeader::Read(_, qualifier) => qualifier,
            ObjectHeader::AllObjects(_)
            | ObjectHeader::Range(_, _, _)
            | ObjectHeader::Count(_, _, _)
            | ObjectHeader::CountAndPrefix8(_, _)
            | ObjectHeader::CountAndPrefix16(_, _)
            | ObjectHeader::FreeFormat(_) => return None,
        };
        match (self, qualifier) {
            (Self::Class0, ReadQualifier::AllObjects) => Some(ReadHeader::Class0),
            (Self::Static(target), ReadQualifier::AllObjects) => {
                Some(ReadHeader::Static(target, None))
            }
            (Self::Static(target), ReadQualifier::Range(_, range)) => {
                Some(ReadHeader::Static(target, Some(range.into())))
            }
            (Self::Event(target), ReadQualifier::AllObjects) => {
                Some(ReadHeader::Event(target, None))
            }
            (Self::Event(target), ReadQualifier::Count(_, count)) => {
                Some(ReadHeader::Event(target, Some(count.into())))
            }
            (Self::Attr(var), ReadQualifier::AllObjects) => {
                Some(ReadHeader::Attr(AttrHeader::All(var)))
            }
            (Self::Attr(var), ReadQualifier::Range(_, range)) => {
                Some(ReadHeader::Attr(AttrHeader::Specific(var, range.into())))
            }
            (Self::Class0, ReadQualifier::Range(_, _) | ReadQualifier::Count(_, _))
            | (Self::Static(_), ReadQualifier::Count(_, _))
            | (Self::Event(_), ReadQualifier::Range(_, _))
            | (Self::Attr(_), ReadQualifier::Count(_, _)) => None,
        }
    }

    fn get(variation: Variation) -> Option<Self> {
        target::get(variation)
    }
}
