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
use crate::app::QualifierCode;
use crate::app::parse::range::{RangedSequence, Range};
use crate::app::parse::parser::*;
use crate::app::parse::bytes::*;
use crate::app::parse::bit::{BitSequence, DoubleBitSequence};
use crate::master::{ReadHandler, HeaderInfo};
use crate::app::ObjectParseError;

use scursor::ReadCursor;

#[derive(Debug, PartialEq)]
pub(crate) enum RangedVariation<'a> {
    /// Device Attributes - Non-Specific All Attributes Request
    Group0Var254,
    /// Device Attributes - Specific Attribute
    /// variation and optional attribute
    Group0(u8, Option<crate::app::attr::Attribute<'a>>),
    /// Binary Input - Any Variation
    Group1Var0,
    /// Binary Input - Packed Format
    Group1Var1(BitSequence<'a>),
    /// Binary Input - With Flags
    Group1Var2(RangedSequence<'a, Group1Var2>),
    /// Double-bit Binary Input - Any Variation
    Group3Var0,
    /// Double-bit Binary Input - Packed Format
    Group3Var1(DoubleBitSequence<'a>),
    /// Double-bit Binary Input - With Flags
    Group3Var2(RangedSequence<'a, Group3Var2>),
    /// Binary Output - Any Variation
    Group10Var0,
    /// Binary Output - Packed Format
    Group10Var1(BitSequence<'a>),
    /// Binary Output - Output Status With Flags
    Group10Var2(RangedSequence<'a, Group10Var2>),
    /// Counter - Any Variation
    Group20Var0,
    /// Counter - 32-bit With Flag
    Group20Var1(RangedSequence<'a, Group20Var1>),
    /// Counter - 16-bit With Flag
    Group20Var2(RangedSequence<'a, Group20Var2>),
    /// Counter - 32-bit Without Flag
    Group20Var5(RangedSequence<'a, Group20Var5>),
    /// Counter - 16-bit Without Flag
    Group20Var6(RangedSequence<'a, Group20Var6>),
    /// Frozen Counter - Any Variation
    Group21Var0,
    /// Frozen Counter - 32-bit With Flag
    Group21Var1(RangedSequence<'a, Group21Var1>),
    /// Frozen Counter - 16-bit With Flag
    Group21Var2(RangedSequence<'a, Group21Var2>),
    /// Frozen Counter - 32-bit With Flag and Time
    Group21Var5(RangedSequence<'a, Group21Var5>),
    /// Frozen Counter - 16-bit With Flag and Time
    Group21Var6(RangedSequence<'a, Group21Var6>),
    /// Frozen Counter - 32-bit Without Flag
    Group21Var9(RangedSequence<'a, Group21Var9>),
    /// Frozen Counter - 16-bit Without Flag
    Group21Var10(RangedSequence<'a, Group21Var10>),
    /// Analog Input - Any Variation
    Group30Var0,
    /// Analog Input - 32-bit With Flag
    Group30Var1(RangedSequence<'a, Group30Var1>),
    /// Analog Input - 16-bit With Flag
    Group30Var2(RangedSequence<'a, Group30Var2>),
    /// Analog Input - 32-bit Without Flag
    Group30Var3(RangedSequence<'a, Group30Var3>),
    /// Analog Input - 16-bit Without Flag
    Group30Var4(RangedSequence<'a, Group30Var4>),
    /// Analog Input - Single-precision With Flag
    Group30Var5(RangedSequence<'a, Group30Var5>),
    /// Analog Input - Double-precision With Flag
    Group30Var6(RangedSequence<'a, Group30Var6>),
    /// Frozen Analog Input - Any Variation
    Group31Var0,
    /// Frozen Analog Input - 32-bit With Flag
    Group31Var1(RangedSequence<'a, Group31Var1>),
    /// Frozen Analog Input - 16-bit With Flag
    Group31Var2(RangedSequence<'a, Group31Var2>),
    /// Frozen Analog Input - 32-bit with Flag and Time-of-Freeze
    Group31Var3(RangedSequence<'a, Group31Var3>),
    /// Frozen Analog Input - 16-bit with Flag and Time-of-Freeze
    Group31Var4(RangedSequence<'a, Group31Var4>),
    /// Frozen Analog Input - 32-bit Without Flag
    Group31Var5(RangedSequence<'a, Group31Var5>),
    /// Frozen Analog Input - 16-bit Without Flag
    Group31Var6(RangedSequence<'a, Group31Var6>),
    /// Frozen Analog Input - Single-precision With Flag
    Group31Var7(RangedSequence<'a, Group31Var7>),
    /// Frozen Analog Input - Double-precision With Flag
    Group31Var8(RangedSequence<'a, Group31Var8>),
    /// Analog Input Reporting Deadband - 16-bit
    Group34Var1(RangedSequence<'a, Group34Var1>),
    /// Analog Input Reporting Deadband - 32-bit
    Group34Var2(RangedSequence<'a, Group34Var2>),
    /// Analog Input Reporting Deadband - Single-precision
    Group34Var3(RangedSequence<'a, Group34Var3>),
    /// Analog Output Status - Any Variation
    Group40Var0,
    /// Analog Output Status - 32-bit With Flag
    Group40Var1(RangedSequence<'a, Group40Var1>),
    /// Analog Output Status - 16-bit With Flag
    Group40Var2(RangedSequence<'a, Group40Var2>),
    /// Analog Output Status - Single-precision With Flag
    Group40Var3(RangedSequence<'a, Group40Var3>),
    /// Analog Output Status - Double-precision With Flag
    Group40Var4(RangedSequence<'a, Group40Var4>),
    /// Internal Indications - Packed Format
    Group80Var1(BitSequence<'a>),
    /// Unsigned Integer - Any Variation
    Group102Var0,
    /// Unsigned Integer - 8-bit
    Group102Var1(RangedSequence<'a, Group102Var1>),
    /// Octet String - Sized by variation
    Group110Var0,
    Group110VarX(u8, RangedBytesSequence<'a>),
}

impl<'a> RangedVariation<'a> {
    pub(crate) fn parse_non_read(v: Variation, qualifier: QualifierCode, range: Range, cursor: &mut ReadCursor<'a>) -> Result<RangedVariation<'a>, ObjectParseError> {
        match v {
            Variation::Group0Var254 => Ok(RangedVariation::Group0Var254),
            Variation::Group0(var) => Ok(RangedVariation::Group0(var, Some(crate::app::attr::Attribute::parse_from_range(var, range, cursor)?))),
            Variation::Group1Var0 => Ok(RangedVariation::Group1Var0),
            Variation::Group1Var1 => Ok(RangedVariation::Group1Var1(BitSequence::parse(range, cursor)?)),
            Variation::Group1Var2 => Ok(RangedVariation::Group1Var2(RangedSequence::parse(range, cursor)?)),
            Variation::Group3Var0 => Ok(RangedVariation::Group3Var0),
            Variation::Group3Var1 => Ok(RangedVariation::Group3Var1(DoubleBitSequence::parse(range, cursor)?)),
            Variation::Group3Var2 => Ok(RangedVariation::Group3Var2(RangedSequence::parse(range, cursor)?)),
            Variation::Group10Var0 => Ok(RangedVariation::Group10Var0),
            Variation::Group10Var1 => Ok(RangedVariation::Group10Var1(BitSequence::parse(range, cursor)?)),
            Variation::Group10Var2 => Ok(RangedVariation::Group10Var2(RangedSequence::parse(range, cursor)?)),
            Variation::Group20Var0 => Ok(RangedVariation::Group20Var0),
            Variation::Group20Var1 => Ok(RangedVariation::Group20Var1(RangedSequence::parse(range, cursor)?)),
            Variation::Group20Var2 => Ok(RangedVariation::Group20Var2(RangedSequence::parse(range, cursor)?)),
            Variation::Group20Var5 => Ok(RangedVariation::Group20Var5(RangedSequence::parse(range, cursor)?)),
            Variation::Group20Var6 => Ok(RangedVariation::Group20Var6(RangedSequence::parse(range, cursor)?)),
            Variation::Group21Var0 => Ok(RangedVariation::Group21Var0),
            Variation::Group21Var1 => Ok(RangedVariation::Group21Var1(RangedSequence::parse(range, cursor)?)),
            Variation::Group21Var2 => Ok(RangedVariation::Group21Var2(RangedSequence::parse(range, cursor)?)),
            Variation::Group21Var5 => Ok(RangedVariation::Group21Var5(RangedSequence::parse(range, cursor)?)),
            Variation::Group21Var6 => Ok(RangedVariation::Group21Var6(RangedSequence::parse(range, cursor)?)),
            Variation::Group21Var9 => Ok(RangedVariation::Group21Var9(RangedSequence::parse(range, cursor)?)),
            Variation::Group21Var10 => Ok(RangedVariation::Group21Var10(RangedSequence::parse(range, cursor)?)),
            Variation::Group30Var0 => Ok(RangedVariation::Group30Var0),
            Variation::Group30Var1 => Ok(RangedVariation::Group30Var1(RangedSequence::parse(range, cursor)?)),
            Variation::Group30Var2 => Ok(RangedVariation::Group30Var2(RangedSequence::parse(range, cursor)?)),
            Variation::Group30Var3 => Ok(RangedVariation::Group30Var3(RangedSequence::parse(range, cursor)?)),
            Variation::Group30Var4 => Ok(RangedVariation::Group30Var4(RangedSequence::parse(range, cursor)?)),
            Variation::Group30Var5 => Ok(RangedVariation::Group30Var5(RangedSequence::parse(range, cursor)?)),
            Variation::Group30Var6 => Ok(RangedVariation::Group30Var6(RangedSequence::parse(range, cursor)?)),
            Variation::Group31Var0 => Ok(RangedVariation::Group31Var0),
            Variation::Group31Var1 => Ok(RangedVariation::Group31Var1(RangedSequence::parse(range, cursor)?)),
            Variation::Group31Var2 => Ok(RangedVariation::Group31Var2(RangedSequence::parse(range, cursor)?)),
            Variation::Group31Var3 => Ok(RangedVariation::Group31Var3(RangedSequence::parse(range, cursor)?)),
            Variation::Group31Var4 => Ok(RangedVariation::Group31Var4(RangedSequence::parse(range, cursor)?)),
            Variation::Group31Var5 => Ok(RangedVariation::Group31Var5(RangedSequence::parse(range, cursor)?)),
            Variation::Group31Var6 => Ok(RangedVariation::Group31Var6(RangedSequence::parse(range, cursor)?)),
            Variation::Group31Var7 => Ok(RangedVariation::Group31Var7(RangedSequence::parse(range, cursor)?)),
            Variation::Group31Var8 => Ok(RangedVariation::Group31Var8(RangedSequence::parse(range, cursor)?)),
            Variation::Group34Var1 => Ok(RangedVariation::Group34Var1(RangedSequence::parse(range, cursor)?)),
            Variation::Group34Var2 => Ok(RangedVariation::Group34Var2(RangedSequence::parse(range, cursor)?)),
            Variation::Group34Var3 => Ok(RangedVariation::Group34Var3(RangedSequence::parse(range, cursor)?)),
            Variation::Group40Var0 => Ok(RangedVariation::Group40Var0),
            Variation::Group40Var1 => Ok(RangedVariation::Group40Var1(RangedSequence::parse(range, cursor)?)),
            Variation::Group40Var2 => Ok(RangedVariation::Group40Var2(RangedSequence::parse(range, cursor)?)),
            Variation::Group40Var3 => Ok(RangedVariation::Group40Var3(RangedSequence::parse(range, cursor)?)),
            Variation::Group40Var4 => Ok(RangedVariation::Group40Var4(RangedSequence::parse(range, cursor)?)),
            Variation::Group80Var1 => Ok(RangedVariation::Group80Var1(BitSequence::parse(range, cursor)?)),
            Variation::Group102Var0 => Ok(RangedVariation::Group102Var0),
            Variation::Group102Var1 => Ok(RangedVariation::Group102Var1(RangedSequence::parse(range, cursor)?)),
            Variation::Group110(0) => Err(ObjectParseError::ZeroLengthOctetData),
            Variation::Group110(x) => {
                Ok(RangedVariation::Group110VarX(x, RangedBytesSequence::parse(x, range.get_start(), range.get_count(), cursor)?))
            },
            _ => Err(ObjectParseError::InvalidQualifierForVariation(v, qualifier)),
        }
    }
    
    pub(crate) fn parse_read(v: Variation, qualifier: QualifierCode) -> Result<RangedVariation<'a>, ObjectParseError> {
        match v {
            Variation::Group0Var254 => Ok(RangedVariation::Group0Var254),
            Variation::Group0(var) => Ok(RangedVariation::Group0(var, None)),
            Variation::Group1Var0 => Ok(RangedVariation::Group1Var0),
            Variation::Group1Var1 => Ok(RangedVariation::Group1Var1(BitSequence::empty())),
            Variation::Group1Var2 => Ok(RangedVariation::Group1Var2(RangedSequence::empty())),
            Variation::Group3Var0 => Ok(RangedVariation::Group3Var0),
            Variation::Group3Var1 => Ok(RangedVariation::Group3Var1(DoubleBitSequence::empty())),
            Variation::Group3Var2 => Ok(RangedVariation::Group3Var2(RangedSequence::empty())),
            Variation::Group10Var0 => Ok(RangedVariation::Group10Var0),
            Variation::Group10Var1 => Ok(RangedVariation::Group10Var1(BitSequence::empty())),
            Variation::Group10Var2 => Ok(RangedVariation::Group10Var2(RangedSequence::empty())),
            Variation::Group20Var0 => Ok(RangedVariation::Group20Var0),
            Variation::Group20Var1 => Ok(RangedVariation::Group20Var1(RangedSequence::empty())),
            Variation::Group20Var2 => Ok(RangedVariation::Group20Var2(RangedSequence::empty())),
            Variation::Group20Var5 => Ok(RangedVariation::Group20Var5(RangedSequence::empty())),
            Variation::Group20Var6 => Ok(RangedVariation::Group20Var6(RangedSequence::empty())),
            Variation::Group21Var0 => Ok(RangedVariation::Group21Var0),
            Variation::Group21Var1 => Ok(RangedVariation::Group21Var1(RangedSequence::empty())),
            Variation::Group21Var2 => Ok(RangedVariation::Group21Var2(RangedSequence::empty())),
            Variation::Group21Var5 => Ok(RangedVariation::Group21Var5(RangedSequence::empty())),
            Variation::Group21Var6 => Ok(RangedVariation::Group21Var6(RangedSequence::empty())),
            Variation::Group21Var9 => Ok(RangedVariation::Group21Var9(RangedSequence::empty())),
            Variation::Group21Var10 => Ok(RangedVariation::Group21Var10(RangedSequence::empty())),
            Variation::Group30Var0 => Ok(RangedVariation::Group30Var0),
            Variation::Group30Var1 => Ok(RangedVariation::Group30Var1(RangedSequence::empty())),
            Variation::Group30Var2 => Ok(RangedVariation::Group30Var2(RangedSequence::empty())),
            Variation::Group30Var3 => Ok(RangedVariation::Group30Var3(RangedSequence::empty())),
            Variation::Group30Var4 => Ok(RangedVariation::Group30Var4(RangedSequence::empty())),
            Variation::Group30Var5 => Ok(RangedVariation::Group30Var5(RangedSequence::empty())),
            Variation::Group30Var6 => Ok(RangedVariation::Group30Var6(RangedSequence::empty())),
            Variation::Group31Var0 => Ok(RangedVariation::Group31Var0),
            Variation::Group31Var1 => Ok(RangedVariation::Group31Var1(RangedSequence::empty())),
            Variation::Group31Var2 => Ok(RangedVariation::Group31Var2(RangedSequence::empty())),
            Variation::Group31Var3 => Ok(RangedVariation::Group31Var3(RangedSequence::empty())),
            Variation::Group31Var4 => Ok(RangedVariation::Group31Var4(RangedSequence::empty())),
            Variation::Group31Var5 => Ok(RangedVariation::Group31Var5(RangedSequence::empty())),
            Variation::Group31Var6 => Ok(RangedVariation::Group31Var6(RangedSequence::empty())),
            Variation::Group31Var7 => Ok(RangedVariation::Group31Var7(RangedSequence::empty())),
            Variation::Group31Var8 => Ok(RangedVariation::Group31Var8(RangedSequence::empty())),
            Variation::Group34Var1 => Ok(RangedVariation::Group34Var1(RangedSequence::empty())),
            Variation::Group34Var2 => Ok(RangedVariation::Group34Var2(RangedSequence::empty())),
            Variation::Group34Var3 => Ok(RangedVariation::Group34Var3(RangedSequence::empty())),
            Variation::Group40Var0 => Ok(RangedVariation::Group40Var0),
            Variation::Group40Var1 => Ok(RangedVariation::Group40Var1(RangedSequence::empty())),
            Variation::Group40Var2 => Ok(RangedVariation::Group40Var2(RangedSequence::empty())),
            Variation::Group40Var3 => Ok(RangedVariation::Group40Var3(RangedSequence::empty())),
            Variation::Group40Var4 => Ok(RangedVariation::Group40Var4(RangedSequence::empty())),
            Variation::Group80Var1 => Ok(RangedVariation::Group80Var1(BitSequence::empty())),
            Variation::Group102Var0 => Ok(RangedVariation::Group102Var0),
            Variation::Group102Var1 => Ok(RangedVariation::Group102Var1(RangedSequence::empty())),
            Variation::Group110(0) => Ok(RangedVariation::Group110Var0),
            _ => Err(ObjectParseError::InvalidQualifierForVariation(v, qualifier)),
        }
    }
    
    pub(crate) fn format_objects(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RangedVariation::Group0Var254 => Ok(()),
            RangedVariation::Group0(_, x) => format_optional_attribute(f, x),
            RangedVariation::Group1Var0 => Ok(()),
            RangedVariation::Group1Var1(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group1Var2(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group3Var0 => Ok(()),
            RangedVariation::Group3Var1(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group3Var2(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group10Var0 => Ok(()),
            RangedVariation::Group10Var1(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group10Var2(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group20Var0 => Ok(()),
            RangedVariation::Group20Var1(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group20Var2(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group20Var5(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group20Var6(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group21Var0 => Ok(()),
            RangedVariation::Group21Var1(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group21Var2(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group21Var5(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group21Var6(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group21Var9(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group21Var10(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group30Var0 => Ok(()),
            RangedVariation::Group30Var1(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group30Var2(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group30Var3(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group30Var4(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group30Var5(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group30Var6(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group31Var0 => Ok(()),
            RangedVariation::Group31Var1(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group31Var2(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group31Var3(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group31Var4(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group31Var5(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group31Var6(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group31Var7(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group31Var8(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group34Var1(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group34Var2(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group34Var3(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group40Var0 => Ok(()),
            RangedVariation::Group40Var1(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group40Var2(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group40Var3(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group40Var4(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group80Var1(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group102Var0 => Ok(()),
            RangedVariation::Group102Var1(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group110Var0 => Ok(()),
            RangedVariation::Group110VarX(_, seq) => format_indexed_items(f, seq.iter().map(|(x, i)| (Bytes::new(x), i))),
        }
    }
    
    pub(crate) fn extract_measurements_to(&self, var: Variation, qualifier: QualifierCode, handler: &mut dyn ReadHandler) -> bool {
        match self {
            RangedVariation::Group0Var254 => {
                false // extraction not supported
            }
            RangedVariation::Group0(_, attr) => {
                crate::master::handle_attribute(var, qualifier, attr, handler);
                true
            }
            RangedVariation::Group1Var0 => {
                false // extraction not supported
            }
            RangedVariation::Group1Var1(seq) => {
                handler.handle_binary_input(
                    HeaderInfo::new(var, qualifier, false, false),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group1Var2(seq) => {
                handler.handle_binary_input(
                    HeaderInfo::new(var, qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group3Var0 => {
                false // extraction not supported
            }
            RangedVariation::Group3Var1(seq) => {
                handler.handle_double_bit_binary_input(
                    HeaderInfo::new(var, qualifier, false, false),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group3Var2(seq) => {
                handler.handle_double_bit_binary_input(
                    HeaderInfo::new(var, qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group10Var0 => {
                false // extraction not supported
            }
            RangedVariation::Group10Var1(seq) => {
                handler.handle_binary_output_status(
                    HeaderInfo::new(var, qualifier, false, false),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group10Var2(seq) => {
                handler.handle_binary_output_status(
                    HeaderInfo::new(var, qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group20Var0 => {
                false // extraction not supported
            }
            RangedVariation::Group20Var1(seq) => {
                handler.handle_counter(
                    HeaderInfo::new(var, qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group20Var2(seq) => {
                handler.handle_counter(
                    HeaderInfo::new(var, qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group20Var5(seq) => {
                handler.handle_counter(
                    HeaderInfo::new(var, qualifier, false, false),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group20Var6(seq) => {
                handler.handle_counter(
                    HeaderInfo::new(var, qualifier, false, false),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group21Var0 => {
                false // extraction not supported
            }
            RangedVariation::Group21Var1(seq) => {
                handler.handle_frozen_counter(
                    HeaderInfo::new(var, qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group21Var2(seq) => {
                handler.handle_frozen_counter(
                    HeaderInfo::new(var, qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group21Var5(seq) => {
                handler.handle_frozen_counter(
                    HeaderInfo::new(var, qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group21Var6(seq) => {
                handler.handle_frozen_counter(
                    HeaderInfo::new(var, qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group21Var9(seq) => {
                handler.handle_frozen_counter(
                    HeaderInfo::new(var, qualifier, false, false),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group21Var10(seq) => {
                handler.handle_frozen_counter(
                    HeaderInfo::new(var, qualifier, false, false),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group30Var0 => {
                false // extraction not supported
            }
            RangedVariation::Group30Var1(seq) => {
                handler.handle_analog_input(
                    HeaderInfo::new(var, qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group30Var2(seq) => {
                handler.handle_analog_input(
                    HeaderInfo::new(var, qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group30Var3(seq) => {
                handler.handle_analog_input(
                    HeaderInfo::new(var, qualifier, false, false),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group30Var4(seq) => {
                handler.handle_analog_input(
                    HeaderInfo::new(var, qualifier, false, false),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group30Var5(seq) => {
                handler.handle_analog_input(
                    HeaderInfo::new(var, qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group30Var6(seq) => {
                handler.handle_analog_input(
                    HeaderInfo::new(var, qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group31Var0 => {
                false // extraction not supported
            }
            RangedVariation::Group31Var1(seq) => {
                handler.handle_frozen_analog_input(
                    HeaderInfo::new(var, qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group31Var2(seq) => {
                handler.handle_frozen_analog_input(
                    HeaderInfo::new(var, qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group31Var3(seq) => {
                handler.handle_frozen_analog_input(
                    HeaderInfo::new(var, qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group31Var4(seq) => {
                handler.handle_frozen_analog_input(
                    HeaderInfo::new(var, qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group31Var5(seq) => {
                handler.handle_frozen_analog_input(
                    HeaderInfo::new(var, qualifier, false, false),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group31Var6(seq) => {
                handler.handle_frozen_analog_input(
                    HeaderInfo::new(var, qualifier, false, false),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group31Var7(seq) => {
                handler.handle_frozen_analog_input(
                    HeaderInfo::new(var, qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group31Var8(seq) => {
                handler.handle_frozen_analog_input(
                    HeaderInfo::new(var, qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group34Var1(seq) => {
                handler.handle_analog_input_dead_band(
                    HeaderInfo::new(var, qualifier, false, false),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group34Var2(seq) => {
                handler.handle_analog_input_dead_band(
                    HeaderInfo::new(var, qualifier, false, false),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group34Var3(seq) => {
                handler.handle_analog_input_dead_band(
                    HeaderInfo::new(var, qualifier, false, false),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group40Var0 => {
                false // extraction not supported
            }
            RangedVariation::Group40Var1(seq) => {
                handler.handle_analog_output_status(
                    HeaderInfo::new(var, qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group40Var2(seq) => {
                handler.handle_analog_output_status(
                    HeaderInfo::new(var, qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group40Var3(seq) => {
                handler.handle_analog_output_status(
                    HeaderInfo::new(var, qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group40Var4(seq) => {
                handler.handle_analog_output_status(
                    HeaderInfo::new(var, qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group80Var1(_) => {
                false // internal indications
            }
            RangedVariation::Group102Var0 => {
                false // extraction not supported
            }
            RangedVariation::Group102Var1(seq) => {
                handler.handle_unsigned_integer(
                    HeaderInfo::new(var, qualifier, false, false),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group110Var0 => {
                false
            }
            RangedVariation::Group110VarX(_,seq) => {
                handler.handle_octet_string(
                    HeaderInfo::new(var, qualifier, false, false),
                    &mut seq.iter()
                );
                true
            }
        }
    }
}
