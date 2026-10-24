// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//  _   _         ______    _ _ _   _             _ _ _
// | \ | |       |  ____|  | (_) | (_)           | | | |
// |  \| | ___   | |__   __| |_| |_ _ _ __   __ _| | | |
// | . ` |/ _ \  |  __| / _` | | __| | '_ \ / _` | | | |
// | |\  | (_) | | |___| (_| | | |_| | | | | (_| |_|_|_|
// |_| \_|\___/  |______\__,_|_|\__|_|_| |_|\__, (_|_|_)
//                                           __/ |
//                                          |___/
//
// This file is auto-generated. Do not edit manually
//

use crate::app::variations::*;
use crate::app::parse::count::CountSequence;
use crate::app::parse::parser::*;
use crate::app::parse::traits::Index;
use crate::app::parse::prefix::Prefix;
use crate::app::parse::bytes::*;
use crate::app::parse::options::ParseOptions;
use crate::app::ObjectParseError;

use scursor::ReadCursor;

#[derive(Debug)]
pub(crate) enum PrefixedVariation<'a, I> where I : Index {
    /// Device Attributes - Specific attribute (variations 1 to 253 and 255)
    Group0(crate::app::attr::Attribute<'a>),
    /// Binary Input Event - Without time
    Group2Var1(CountSequence<'a, Prefix<I, Group2Var1>>),
    /// Binary Input Event - With absolute time
    Group2Var2(CountSequence<'a, Prefix<I, Group2Var2>>),
    /// Binary Input Event - With relative time
    Group2Var3(CountSequence<'a, Prefix<I, Group2Var3>>),
    /// Double-bit Binary Input Event - Without time
    Group4Var1(CountSequence<'a, Prefix<I, Group4Var1>>),
    /// Double-bit Binary Input Event - With absolute time
    Group4Var2(CountSequence<'a, Prefix<I, Group4Var2>>),
    /// Double-bit Binary Input Event - With relative time
    Group4Var3(CountSequence<'a, Prefix<I, Group4Var3>>),
    /// Binary Output Event - Without time
    Group11Var1(CountSequence<'a, Prefix<I, Group11Var1>>),
    /// Binary Output Event - With time
    Group11Var2(CountSequence<'a, Prefix<I, Group11Var2>>),
    /// Binary Output Command - Control relay output block
    Group12Var1(CountSequence<'a, Prefix<I, Group12Var1>>),
    /// Binary Output Command Event - Without time
    Group13Var1(CountSequence<'a, Prefix<I, Group13Var1>>),
    /// Binary Output Command Event - With time
    Group13Var2(CountSequence<'a, Prefix<I, Group13Var2>>),
    /// Counter Event - 32-bit without time
    Group22Var1(CountSequence<'a, Prefix<I, Group22Var1>>),
    /// Counter Event - 16-bit without time
    Group22Var2(CountSequence<'a, Prefix<I, Group22Var2>>),
    /// Counter Event - 32-bit with time
    Group22Var5(CountSequence<'a, Prefix<I, Group22Var5>>),
    /// Counter Event - 16-bit with time
    Group22Var6(CountSequence<'a, Prefix<I, Group22Var6>>),
    /// Frozen Counter Event - 32-bit without time
    Group23Var1(CountSequence<'a, Prefix<I, Group23Var1>>),
    /// Frozen Counter Event - 16-bit without time
    Group23Var2(CountSequence<'a, Prefix<I, Group23Var2>>),
    /// Frozen Counter Event - 32-bit with time
    Group23Var5(CountSequence<'a, Prefix<I, Group23Var5>>),
    /// Frozen Counter Event - 16-bit with time
    Group23Var6(CountSequence<'a, Prefix<I, Group23Var6>>),
    /// Analog Input Event - 32-bit without time
    Group32Var1(CountSequence<'a, Prefix<I, Group32Var1>>),
    /// Analog Input Event - 16-bit without time
    Group32Var2(CountSequence<'a, Prefix<I, Group32Var2>>),
    /// Analog Input Event - 32-bit with time
    Group32Var3(CountSequence<'a, Prefix<I, Group32Var3>>),
    /// Analog Input Event - 16-bit with time
    Group32Var4(CountSequence<'a, Prefix<I, Group32Var4>>),
    /// Analog Input Event - Single-precision floating point without time
    Group32Var5(CountSequence<'a, Prefix<I, Group32Var5>>),
    /// Analog Input Event - Double-precision floating point without time
    Group32Var6(CountSequence<'a, Prefix<I, Group32Var6>>),
    /// Analog Input Event - Single-precision floating point with time
    Group32Var7(CountSequence<'a, Prefix<I, Group32Var7>>),
    /// Analog Input Event - Double-precision floating point with time
    Group32Var8(CountSequence<'a, Prefix<I, Group32Var8>>),
    /// Frozen Analog Input Event - 32-bit without time
    Group33Var1(CountSequence<'a, Prefix<I, Group33Var1>>),
    /// Frozen Analog Input Event - 16-bit without time
    Group33Var2(CountSequence<'a, Prefix<I, Group33Var2>>),
    /// Frozen Analog Input Event - 32-bit with time of freeze
    Group33Var3(CountSequence<'a, Prefix<I, Group33Var3>>),
    /// Frozen Analog Input Event - 16-bit with time of freeze
    Group33Var4(CountSequence<'a, Prefix<I, Group33Var4>>),
    /// Frozen Analog Input Event - Single-precision floating point without time
    Group33Var5(CountSequence<'a, Prefix<I, Group33Var5>>),
    /// Frozen Analog Input Event - Double-precision floating point without time
    Group33Var6(CountSequence<'a, Prefix<I, Group33Var6>>),
    /// Frozen Analog Input Event - Single-precision floating point with time of freeze
    Group33Var7(CountSequence<'a, Prefix<I, Group33Var7>>),
    /// Frozen Analog Input Event - Double-precision floating point with time of freeze
    Group33Var8(CountSequence<'a, Prefix<I, Group33Var8>>),
    /// Analog Input Reporting Deadband - 16-bit
    Group34Var1(CountSequence<'a, Prefix<I, Group34Var1>>),
    /// Analog Input Reporting Deadband - 32-bit
    Group34Var2(CountSequence<'a, Prefix<I, Group34Var2>>),
    /// Analog Input Reporting Deadband - Single-precision floating point
    Group34Var3(CountSequence<'a, Prefix<I, Group34Var3>>),
    /// Analog Output - 32-bit
    Group41Var1(CountSequence<'a, Prefix<I, Group41Var1>>),
    /// Analog Output - 16-bit
    Group41Var2(CountSequence<'a, Prefix<I, Group41Var2>>),
    /// Analog Output - Single-precision floating point
    Group41Var3(CountSequence<'a, Prefix<I, Group41Var3>>),
    /// Analog Output - Double-precision floating point
    Group41Var4(CountSequence<'a, Prefix<I, Group41Var4>>),
    /// Analog Output Event - 32-bit without time
    Group42Var1(CountSequence<'a, Prefix<I, Group42Var1>>),
    /// Analog Output Event - 16-bit without time
    Group42Var2(CountSequence<'a, Prefix<I, Group42Var2>>),
    /// Analog Output Event - 32-bit with time
    Group42Var3(CountSequence<'a, Prefix<I, Group42Var3>>),
    /// Analog Output Event - 16-bit with time
    Group42Var4(CountSequence<'a, Prefix<I, Group42Var4>>),
    /// Analog Output Event - Single-precision floating point without time
    Group42Var5(CountSequence<'a, Prefix<I, Group42Var5>>),
    /// Analog Output Event - Double-precision floating point without time
    Group42Var6(CountSequence<'a, Prefix<I, Group42Var6>>),
    /// Analog Output Event - Single-precision floating point with time
    Group42Var7(CountSequence<'a, Prefix<I, Group42Var7>>),
    /// Analog Output Event - Double-precision floating point with time
    Group42Var8(CountSequence<'a, Prefix<I, Group42Var8>>),
    /// Analog Output Command Event - 32-bit without time
    Group43Var1(CountSequence<'a, Prefix<I, Group43Var1>>),
    /// Analog Output Command Event - 16-bit without time
    Group43Var2(CountSequence<'a, Prefix<I, Group43Var2>>),
    /// Analog Output Command Event - 32-bit with time
    Group43Var3(CountSequence<'a, Prefix<I, Group43Var3>>),
    /// Analog Output Command Event - 16-bit with time
    Group43Var4(CountSequence<'a, Prefix<I, Group43Var4>>),
    /// Analog Output Command Event - Single-precision floating point without time
    Group43Var5(CountSequence<'a, Prefix<I, Group43Var5>>),
    /// Analog Output Command Event - Double-precision floating point without time
    Group43Var6(CountSequence<'a, Prefix<I, Group43Var6>>),
    /// Analog Output Command Event - Single-precision floating point with time
    Group43Var7(CountSequence<'a, Prefix<I, Group43Var7>>),
    /// Analog Output Command Event - Double-precision floating point with time
    Group43Var8(CountSequence<'a, Prefix<I, Group43Var8>>),
    /// Octet String Event - Sized by variation
    Group111VarX(u8, PrefixedBytesSequence<'a, I>),
}

impl<'a, I> PrefixedVariation<'a, I> where I : Index {
    pub(crate) fn parse(v: Variation, count: u16, options: ParseOptions, cursor: &mut ReadCursor<'a>) -> Result<PrefixedVariation<'a, I>, ObjectParseError> {
        match v {
            Variation::Group0(var) => Ok(PrefixedVariation::Group0(crate::app::attr::Attribute::parse_prefixed::<I>(var, count, cursor)?)),
            Variation::Group2Var1 => Ok(PrefixedVariation::Group2Var1(CountSequence::parse(count, cursor)?)),
            Variation::Group2Var2 => Ok(PrefixedVariation::Group2Var2(CountSequence::parse(count, cursor)?)),
            Variation::Group2Var3 => Ok(PrefixedVariation::Group2Var3(CountSequence::parse(count, cursor)?)),
            Variation::Group4Var1 => Ok(PrefixedVariation::Group4Var1(CountSequence::parse(count, cursor)?)),
            Variation::Group4Var2 => Ok(PrefixedVariation::Group4Var2(CountSequence::parse(count, cursor)?)),
            Variation::Group4Var3 => Ok(PrefixedVariation::Group4Var3(CountSequence::parse(count, cursor)?)),
            Variation::Group11Var1 => Ok(PrefixedVariation::Group11Var1(CountSequence::parse(count, cursor)?)),
            Variation::Group11Var2 => Ok(PrefixedVariation::Group11Var2(CountSequence::parse(count, cursor)?)),
            Variation::Group12Var1 => Ok(PrefixedVariation::Group12Var1(CountSequence::parse(count, cursor)?)),
            Variation::Group13Var1 => Ok(PrefixedVariation::Group13Var1(CountSequence::parse(count, cursor)?)),
            Variation::Group13Var2 => Ok(PrefixedVariation::Group13Var2(CountSequence::parse(count, cursor)?)),
            Variation::Group22Var1 => Ok(PrefixedVariation::Group22Var1(CountSequence::parse(count, cursor)?)),
            Variation::Group22Var2 => Ok(PrefixedVariation::Group22Var2(CountSequence::parse(count, cursor)?)),
            Variation::Group22Var5 => Ok(PrefixedVariation::Group22Var5(CountSequence::parse(count, cursor)?)),
            Variation::Group22Var6 => Ok(PrefixedVariation::Group22Var6(CountSequence::parse(count, cursor)?)),
            Variation::Group23Var1 => Ok(PrefixedVariation::Group23Var1(CountSequence::parse(count, cursor)?)),
            Variation::Group23Var2 => Ok(PrefixedVariation::Group23Var2(CountSequence::parse(count, cursor)?)),
            Variation::Group23Var5 => Ok(PrefixedVariation::Group23Var5(CountSequence::parse(count, cursor)?)),
            Variation::Group23Var6 => Ok(PrefixedVariation::Group23Var6(CountSequence::parse(count, cursor)?)),
            Variation::Group32Var1 => Ok(PrefixedVariation::Group32Var1(CountSequence::parse(count, cursor)?)),
            Variation::Group32Var2 => Ok(PrefixedVariation::Group32Var2(CountSequence::parse(count, cursor)?)),
            Variation::Group32Var3 => Ok(PrefixedVariation::Group32Var3(CountSequence::parse(count, cursor)?)),
            Variation::Group32Var4 => Ok(PrefixedVariation::Group32Var4(CountSequence::parse(count, cursor)?)),
            Variation::Group32Var5 => Ok(PrefixedVariation::Group32Var5(CountSequence::parse(count, cursor)?)),
            Variation::Group32Var6 => Ok(PrefixedVariation::Group32Var6(CountSequence::parse(count, cursor)?)),
            Variation::Group32Var7 => Ok(PrefixedVariation::Group32Var7(CountSequence::parse(count, cursor)?)),
            Variation::Group32Var8 => Ok(PrefixedVariation::Group32Var8(CountSequence::parse(count, cursor)?)),
            Variation::Group33Var1 => Ok(PrefixedVariation::Group33Var1(CountSequence::parse(count, cursor)?)),
            Variation::Group33Var2 => Ok(PrefixedVariation::Group33Var2(CountSequence::parse(count, cursor)?)),
            Variation::Group33Var3 => Ok(PrefixedVariation::Group33Var3(CountSequence::parse(count, cursor)?)),
            Variation::Group33Var4 => Ok(PrefixedVariation::Group33Var4(CountSequence::parse(count, cursor)?)),
            Variation::Group33Var5 => Ok(PrefixedVariation::Group33Var5(CountSequence::parse(count, cursor)?)),
            Variation::Group33Var6 => Ok(PrefixedVariation::Group33Var6(CountSequence::parse(count, cursor)?)),
            Variation::Group33Var7 => Ok(PrefixedVariation::Group33Var7(CountSequence::parse(count, cursor)?)),
            Variation::Group33Var8 => Ok(PrefixedVariation::Group33Var8(CountSequence::parse(count, cursor)?)),
            Variation::Group34Var1 => Ok(PrefixedVariation::Group34Var1(CountSequence::parse(count, cursor)?)),
            Variation::Group34Var2 => Ok(PrefixedVariation::Group34Var2(CountSequence::parse(count, cursor)?)),
            Variation::Group34Var3 => Ok(PrefixedVariation::Group34Var3(CountSequence::parse(count, cursor)?)),
            Variation::Group41Var1 => Ok(PrefixedVariation::Group41Var1(CountSequence::parse(count, cursor)?)),
            Variation::Group41Var2 => Ok(PrefixedVariation::Group41Var2(CountSequence::parse(count, cursor)?)),
            Variation::Group41Var3 => Ok(PrefixedVariation::Group41Var3(CountSequence::parse(count, cursor)?)),
            Variation::Group41Var4 => Ok(PrefixedVariation::Group41Var4(CountSequence::parse(count, cursor)?)),
            Variation::Group42Var1 => Ok(PrefixedVariation::Group42Var1(CountSequence::parse(count, cursor)?)),
            Variation::Group42Var2 => Ok(PrefixedVariation::Group42Var2(CountSequence::parse(count, cursor)?)),
            Variation::Group42Var3 => Ok(PrefixedVariation::Group42Var3(CountSequence::parse(count, cursor)?)),
            Variation::Group42Var4 => Ok(PrefixedVariation::Group42Var4(CountSequence::parse(count, cursor)?)),
            Variation::Group42Var5 => Ok(PrefixedVariation::Group42Var5(CountSequence::parse(count, cursor)?)),
            Variation::Group42Var6 => Ok(PrefixedVariation::Group42Var6(CountSequence::parse(count, cursor)?)),
            Variation::Group42Var7 => Ok(PrefixedVariation::Group42Var7(CountSequence::parse(count, cursor)?)),
            Variation::Group42Var8 => Ok(PrefixedVariation::Group42Var8(CountSequence::parse(count, cursor)?)),
            Variation::Group43Var1 => Ok(PrefixedVariation::Group43Var1(CountSequence::parse(count, cursor)?)),
            Variation::Group43Var2 => Ok(PrefixedVariation::Group43Var2(CountSequence::parse(count, cursor)?)),
            Variation::Group43Var3 => Ok(PrefixedVariation::Group43Var3(CountSequence::parse(count, cursor)?)),
            Variation::Group43Var4 => Ok(PrefixedVariation::Group43Var4(CountSequence::parse(count, cursor)?)),
            Variation::Group43Var5 => Ok(PrefixedVariation::Group43Var5(CountSequence::parse(count, cursor)?)),
            Variation::Group43Var6 => Ok(PrefixedVariation::Group43Var6(CountSequence::parse(count, cursor)?)),
            Variation::Group43Var7 => Ok(PrefixedVariation::Group43Var7(CountSequence::parse(count, cursor)?)),
            Variation::Group43Var8 => Ok(PrefixedVariation::Group43Var8(CountSequence::parse(count, cursor)?)),
            Variation::Group111(x) => Ok(PrefixedVariation::Group111VarX(x, PrefixedBytesSequence::parse(options, x, count, cursor)?)),
            _ => Err(ObjectParseError::InvalidQualifierForVariation(v, I::COUNT_AND_PREFIX_QUALIFIER)),
        }
    }
    
    pub(crate) fn variation(&self) -> Variation {
        match self {
            PrefixedVariation::Group0(attr) => Variation::Group0(attr.variation),
            PrefixedVariation::Group2Var1(_) => Variation::Group2Var1,
            PrefixedVariation::Group2Var2(_) => Variation::Group2Var2,
            PrefixedVariation::Group2Var3(_) => Variation::Group2Var3,
            PrefixedVariation::Group4Var1(_) => Variation::Group4Var1,
            PrefixedVariation::Group4Var2(_) => Variation::Group4Var2,
            PrefixedVariation::Group4Var3(_) => Variation::Group4Var3,
            PrefixedVariation::Group11Var1(_) => Variation::Group11Var1,
            PrefixedVariation::Group11Var2(_) => Variation::Group11Var2,
            PrefixedVariation::Group12Var1(_) => Variation::Group12Var1,
            PrefixedVariation::Group13Var1(_) => Variation::Group13Var1,
            PrefixedVariation::Group13Var2(_) => Variation::Group13Var2,
            PrefixedVariation::Group22Var1(_) => Variation::Group22Var1,
            PrefixedVariation::Group22Var2(_) => Variation::Group22Var2,
            PrefixedVariation::Group22Var5(_) => Variation::Group22Var5,
            PrefixedVariation::Group22Var6(_) => Variation::Group22Var6,
            PrefixedVariation::Group23Var1(_) => Variation::Group23Var1,
            PrefixedVariation::Group23Var2(_) => Variation::Group23Var2,
            PrefixedVariation::Group23Var5(_) => Variation::Group23Var5,
            PrefixedVariation::Group23Var6(_) => Variation::Group23Var6,
            PrefixedVariation::Group32Var1(_) => Variation::Group32Var1,
            PrefixedVariation::Group32Var2(_) => Variation::Group32Var2,
            PrefixedVariation::Group32Var3(_) => Variation::Group32Var3,
            PrefixedVariation::Group32Var4(_) => Variation::Group32Var4,
            PrefixedVariation::Group32Var5(_) => Variation::Group32Var5,
            PrefixedVariation::Group32Var6(_) => Variation::Group32Var6,
            PrefixedVariation::Group32Var7(_) => Variation::Group32Var7,
            PrefixedVariation::Group32Var8(_) => Variation::Group32Var8,
            PrefixedVariation::Group33Var1(_) => Variation::Group33Var1,
            PrefixedVariation::Group33Var2(_) => Variation::Group33Var2,
            PrefixedVariation::Group33Var3(_) => Variation::Group33Var3,
            PrefixedVariation::Group33Var4(_) => Variation::Group33Var4,
            PrefixedVariation::Group33Var5(_) => Variation::Group33Var5,
            PrefixedVariation::Group33Var6(_) => Variation::Group33Var6,
            PrefixedVariation::Group33Var7(_) => Variation::Group33Var7,
            PrefixedVariation::Group33Var8(_) => Variation::Group33Var8,
            PrefixedVariation::Group34Var1(_) => Variation::Group34Var1,
            PrefixedVariation::Group34Var2(_) => Variation::Group34Var2,
            PrefixedVariation::Group34Var3(_) => Variation::Group34Var3,
            PrefixedVariation::Group41Var1(_) => Variation::Group41Var1,
            PrefixedVariation::Group41Var2(_) => Variation::Group41Var2,
            PrefixedVariation::Group41Var3(_) => Variation::Group41Var3,
            PrefixedVariation::Group41Var4(_) => Variation::Group41Var4,
            PrefixedVariation::Group42Var1(_) => Variation::Group42Var1,
            PrefixedVariation::Group42Var2(_) => Variation::Group42Var2,
            PrefixedVariation::Group42Var3(_) => Variation::Group42Var3,
            PrefixedVariation::Group42Var4(_) => Variation::Group42Var4,
            PrefixedVariation::Group42Var5(_) => Variation::Group42Var5,
            PrefixedVariation::Group42Var6(_) => Variation::Group42Var6,
            PrefixedVariation::Group42Var7(_) => Variation::Group42Var7,
            PrefixedVariation::Group42Var8(_) => Variation::Group42Var8,
            PrefixedVariation::Group43Var1(_) => Variation::Group43Var1,
            PrefixedVariation::Group43Var2(_) => Variation::Group43Var2,
            PrefixedVariation::Group43Var3(_) => Variation::Group43Var3,
            PrefixedVariation::Group43Var4(_) => Variation::Group43Var4,
            PrefixedVariation::Group43Var5(_) => Variation::Group43Var5,
            PrefixedVariation::Group43Var6(_) => Variation::Group43Var6,
            PrefixedVariation::Group43Var7(_) => Variation::Group43Var7,
            PrefixedVariation::Group43Var8(_) => Variation::Group43Var8,
            PrefixedVariation::Group111VarX(x, _) => Variation::Group111(*x),
        }
    }
    
    /// True if objects of this variation have a time relative to a common time of occurrence
    pub(crate) fn has_relative_time(&self) -> bool {
        matches!(self, PrefixedVariation::Group2Var3(_) | PrefixedVariation::Group4Var3(_))
    }
    
    pub(crate) fn format_objects(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PrefixedVariation::Group0(attr) => attr.format(f),
            PrefixedVariation::Group2Var1(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group2Var2(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group2Var3(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group4Var1(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group4Var2(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group4Var3(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group11Var1(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group11Var2(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group12Var1(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group13Var1(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group13Var2(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group22Var1(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group22Var2(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group22Var5(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group22Var6(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group23Var1(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group23Var2(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group23Var5(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group23Var6(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group32Var1(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group32Var2(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group32Var3(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group32Var4(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group32Var5(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group32Var6(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group32Var7(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group32Var8(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group33Var1(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group33Var2(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group33Var3(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group33Var4(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group33Var5(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group33Var6(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group33Var7(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group33Var8(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group34Var1(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group34Var2(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group34Var3(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group41Var1(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group41Var2(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group41Var3(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group41Var4(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group42Var1(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group42Var2(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group42Var3(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group42Var4(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group42Var5(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group42Var6(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group42Var7(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group42Var8(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group43Var1(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group43Var2(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group43Var3(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group43Var4(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group43Var5(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group43Var6(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group43Var7(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group43Var8(seq) => format_prefixed_items(f, seq.iter()),
            PrefixedVariation::Group111VarX(_, seq) => format_indexed_items(f, seq.iter().map(|(x, i)| (Bytes::new(x), i))),
        }
    }
}
