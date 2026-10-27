// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::attr::Attribute;
use crate::app::gen::count::CountVariation;
use crate::app::gen::prefixed::PrefixedVariation;
use crate::app::gen::ranged::RangedVariation;
use crate::app::parse::bit::BitSequence;
use crate::app::parse::count::CountSequence;
use crate::app::parse::parser::ObjectHeader;
use crate::app::parse::prefix::Prefix;
use crate::app::parse::range::RangedSequence;
use crate::app::parse::traits::FixedSizeVariation;
use crate::app::variations::{Group34Var1, Group34Var2, Group34Var3, Group50Var1, Group50Var3};
use crate::app::{FunctionCode, Iin2, QualifierCode, Variation};
use crate::outstation::database::EventClass;
use crate::outstation::FreezeIndices;
use crate::util::skipped::{Count, Skipped};

/// What a function does with the objects of a variation, whatever their qualifier
///
/// The targets of a function are the one list of the headers it supports: [`Selector::select`]
/// takes the header it handles from the target, and gives the error for a header it doesn't from
/// whether the variation has a target.
pub(crate) trait Target: Copy {
    /// What a header with a qualifier that the function supports selects
    type Header<'a>;

    /// The target of a variation, or `None` if the function doesn't support it with any qualifier
    fn get(variation: Variation) -> Option<Self>;

    /// What a header of this target selects, or `None` if the function doesn't support its
    /// qualifier
    fn with(self, header: ObjectHeader<'_>) -> Option<Self::Header<'_>>;
}

/// Selects the headers of one request, and logs the ones that its function doesn't support once
/// for the request
pub(crate) struct Selector {
    function: FunctionCode,
    unsupported: Skipped<(Variation, QualifierCode)>,
}

impl Selector {
    pub(crate) const fn new(function: FunctionCode) -> Self {
        Self {
            function,
            unsupported: Skipped::new(),
        }
    }

    /// What a header in the request selects, or the error for one that the function doesn't
    /// support: a variation that the function supports with another qualifier has an unsupported
    /// qualifier, and any other is an unsupported object
    pub(crate) fn select<'a, T: Target>(
        &mut self,
        header: ObjectHeader<'a>,
    ) -> Result<T::Header<'a>, Iin2> {
        let variation = header.variation();
        let qualifier = header.qualifier();
        let target = T::get(variation);
        target.and_then(|x| x.with(header)).ok_or_else(|| {
            self.unsupported.add((variation, qualifier));
            if target.is_some() {
                Iin2::PARAMETER_ERROR
            } else {
                Iin2::OBJECT_UNKNOWN
            }
        })
    }

    /// The first header that the function doesn't support and how many, if there were any
    pub(crate) fn unsupported(&self) -> Option<((Variation, QualifierCode), Count)> {
        self.unsupported.first()
    }

    /// Log the headers that the function doesn't support, if there were any
    pub(crate) fn log(&self) {
        if let Some((header, count)) = self.unsupported() {
            log_unsupported(self.function, header, count);
        }
    }
}

/// Log the first header that a function doesn't support, and how many
pub(crate) fn log_unsupported(
    function: FunctionCode,
    (variation, qualifier): (Variation, QualifierCode),
    count: Count,
) {
    tracing::warn!("{function:?} doesn't support {variation} with qualifier {qualifier}{count}");
}

/// What a WRITE of a variation writes
#[derive(Copy, Clone)]
pub(crate) enum WriteTarget {
    Attr,
    Iin,
    AbsoluteTime,
    LastRecordedTime,
    DeadBand,
}

/// The objects of a header that WRITE supports
pub(crate) enum WriteHeader<'a> {
    Attr(Attribute<'a>),
    Iin(BitSequence<'a>),
    AbsoluteTime(CountSequence<'a, Group50Var1>),
    LastRecordedTime(CountSequence<'a, Group50Var3>),
    DeadBands16(DeadBands<'a, Group34Var1>),
    DeadBands32(DeadBands<'a, Group34Var2>),
    DeadBandsFloat(DeadBands<'a, Group34Var3>),
}

/// The analog input dead-bands of one variation in a range or a count and prefix header
pub(crate) enum DeadBands<'a, V: FixedSizeVariation> {
    Range(RangedSequence<'a, V>),
    OneBytePrefix(CountSequence<'a, Prefix<u8, V>>),
    TwoBytePrefix(CountSequence<'a, Prefix<u16, V>>),
}

impl Target for WriteTarget {
    type Header<'a> = WriteHeader<'a>;

    fn get(variation: Variation) -> Option<Self> {
        let target = match variation {
            Variation::Group0(var) if var != 255 => Self::Attr,
            Variation::Group34Var1 | Variation::Group34Var2 | Variation::Group34Var3 => {
                Self::DeadBand
            }
            Variation::Group50Var1 => Self::AbsoluteTime,
            Variation::Group50Var3 => Self::LastRecordedTime,
            Variation::Group80Var1 => Self::Iin,
            _ => return None,
        };
        Some(target)
    }

    fn with(self, header: ObjectHeader<'_>) -> Option<WriteHeader<'_>> {
        let header = match self {
            Self::Attr => match header {
                ObjectHeader::Range(_, _, RangedVariation::Group0(attr)) => WriteHeader::Attr(attr),
                _ => return None,
            },
            Self::Iin => match header {
                ObjectHeader::Range(_, _, RangedVariation::Group80Var1(bits)) => {
                    WriteHeader::Iin(bits)
                }
                _ => return None,
            },
            Self::AbsoluteTime => match header {
                ObjectHeader::Count(_, _, CountVariation::Group50Var1(seq)) => {
                    WriteHeader::AbsoluteTime(seq)
                }
                _ => return None,
            },
            Self::LastRecordedTime => match header {
                ObjectHeader::Count(_, _, CountVariation::Group50Var3(seq)) => {
                    WriteHeader::LastRecordedTime(seq)
                }
                _ => return None,
            },
            Self::DeadBand => match header {
                ObjectHeader::Range(_, _, RangedVariation::Group34Var1(seq)) => {
                    WriteHeader::DeadBands16(DeadBands::Range(seq))
                }
                ObjectHeader::Range(_, _, RangedVariation::Group34Var2(seq)) => {
                    WriteHeader::DeadBands32(DeadBands::Range(seq))
                }
                ObjectHeader::Range(_, _, RangedVariation::Group34Var3(seq)) => {
                    WriteHeader::DeadBandsFloat(DeadBands::Range(seq))
                }
                ObjectHeader::CountAndPrefix8(_, PrefixedVariation::Group34Var1(seq)) => {
                    WriteHeader::DeadBands16(DeadBands::OneBytePrefix(seq))
                }
                ObjectHeader::CountAndPrefix8(_, PrefixedVariation::Group34Var2(seq)) => {
                    WriteHeader::DeadBands32(DeadBands::OneBytePrefix(seq))
                }
                ObjectHeader::CountAndPrefix8(_, PrefixedVariation::Group34Var3(seq)) => {
                    WriteHeader::DeadBandsFloat(DeadBands::OneBytePrefix(seq))
                }
                ObjectHeader::CountAndPrefix16(_, PrefixedVariation::Group34Var1(seq)) => {
                    WriteHeader::DeadBands16(DeadBands::TwoBytePrefix(seq))
                }
                ObjectHeader::CountAndPrefix16(_, PrefixedVariation::Group34Var2(seq)) => {
                    WriteHeader::DeadBands32(DeadBands::TwoBytePrefix(seq))
                }
                ObjectHeader::CountAndPrefix16(_, PrefixedVariation::Group34Var3(seq)) => {
                    WriteHeader::DeadBandsFloat(DeadBands::TwoBytePrefix(seq))
                }
                _ => return None,
            },
        };
        Some(header)
    }
}

/// The counters that IMMEDIATE_FREEZE, FREEZE_AND_CLEAR and FREEZE_AT_TIME freeze, named by g20v0
#[derive(Copy, Clone)]
pub(crate) struct FreezeTarget;

impl Target for FreezeTarget {
    type Header<'a> = FreezeIndices;

    fn get(variation: Variation) -> Option<Self> {
        match variation {
            Variation::Group20Var0 => Some(Self),
            _ => None,
        }
    }

    fn with(self, header: ObjectHeader<'_>) -> Option<FreezeIndices> {
        match header {
            ObjectHeader::AllObjects(_) => Some(FreezeIndices::All),
            ObjectHeader::Range(_, range, RangedVariation::Group20Var0) => {
                Some(FreezeIndices::Range(range.get_start(), range.get_stop()))
            }
            _ => None,
        }
    }
}

/// The class of events that an ENABLE or DISABLE UNSOLICITED header names
impl Target for EventClass {
    type Header<'a> = Self;

    fn get(variation: Variation) -> Option<Self> {
        match variation {
            Variation::Group60Var2 => Some(Self::Class1),
            Variation::Group60Var3 => Some(Self::Class2),
            Variation::Group60Var4 => Some(Self::Class3),
            _ => None,
        }
    }

    fn with(self, header: ObjectHeader<'_>) -> Option<Self> {
        match header {
            ObjectHeader::AllObjects(_) => Some(self),
            _ => None,
        }
    }
}
