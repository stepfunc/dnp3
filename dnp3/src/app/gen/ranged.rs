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
use crate::util::cursor::ReadCursor;
use crate::app::parse::parser::*;
use crate::app::parse::bytes::RangedBytesSequence;
use crate::app::parse::bit::{BitSequence, DoubleBitSequence};
use crate::master::{ReadHandler, HeaderInfo};
use crate::app::ObjectParseError;

#[derive(Debug, PartialEq)]
pub(crate) enum RangedVariation<'a> {
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
    /// Octet String - Sized by variation
    Group110Var0,
    Group110VarX(u8, RangedBytesSequence<'a>),
}

impl<'a> RangedVariation<'a> {
    pub(crate) fn parse_non_read(v: Variation, qualifier: QualifierCode, range: Range, cursor: &mut ReadCursor<'a>) -> Result<RangedVariation<'a>, ObjectParseError> {
        match v {
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
            Variation::Group40Var0 => Ok(RangedVariation::Group40Var0),
            Variation::Group40Var1 => Ok(RangedVariation::Group40Var1(RangedSequence::parse(range, cursor)?)),
            Variation::Group40Var2 => Ok(RangedVariation::Group40Var2(RangedSequence::parse(range, cursor)?)),
            Variation::Group40Var3 => Ok(RangedVariation::Group40Var3(RangedSequence::parse(range, cursor)?)),
            Variation::Group40Var4 => Ok(RangedVariation::Group40Var4(RangedSequence::parse(range, cursor)?)),
            Variation::Group80Var1 => Ok(RangedVariation::Group80Var1(BitSequence::parse(range, cursor)?)),
            Variation::Group110(0) => Err(ObjectParseError::ZeroLengthOctetData),
            Variation::Group110(x) => {
                Ok(RangedVariation::Group110VarX(x, RangedBytesSequence::parse(x, range.get_start(), range.get_count(), cursor)?))
            },
            _ => Err(ObjectParseError::InvalidQualifierForVariation(v, qualifier)),
        }
    }
    
    pub(crate) fn parse_read(v: Variation, qualifier: QualifierCode) -> Result<RangedVariation<'a>, ObjectParseError> {
        match v {
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
            Variation::Group40Var0 => Ok(RangedVariation::Group40Var0),
            Variation::Group40Var1 => Ok(RangedVariation::Group40Var1(RangedSequence::empty())),
            Variation::Group40Var2 => Ok(RangedVariation::Group40Var2(RangedSequence::empty())),
            Variation::Group40Var3 => Ok(RangedVariation::Group40Var3(RangedSequence::empty())),
            Variation::Group40Var4 => Ok(RangedVariation::Group40Var4(RangedSequence::empty())),
            Variation::Group80Var1 => Ok(RangedVariation::Group80Var1(BitSequence::empty())),
            Variation::Group110(0) => Ok(RangedVariation::Group110Var0),
            _ => Err(ObjectParseError::InvalidQualifierForVariation(v, qualifier)),
        }
    }
    
    pub(crate) fn format_objects(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
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
            RangedVariation::Group40Var0 => Ok(()),
            RangedVariation::Group40Var1(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group40Var2(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group40Var3(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group40Var4(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group80Var1(seq) => format_indexed_items(f, seq.iter()),
            RangedVariation::Group110Var0 => Ok(()),
            RangedVariation::Group110VarX(_,seq) =>  format_indexed_items(f, seq.iter()),
        }
    }
    
    pub(crate) fn extract_measurements_to(&self, qualifier: QualifierCode, handler: &mut dyn ReadHandler) -> bool {
        match self {
            RangedVariation::Group1Var0 => {
                false // qualifier 0x06
            }
            RangedVariation::Group1Var1(seq) => {
                handler.handle_binary(
                    HeaderInfo::new(self.variation(), qualifier),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group1Var2(seq) => {
                handler.handle_binary(
                    HeaderInfo::new(self.variation(), qualifier),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group3Var0 => {
                false // qualifier 0x06
            }
            RangedVariation::Group3Var1(seq) => {
                handler.handle_double_bit_binary(
                    HeaderInfo::new(self.variation(), qualifier),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group3Var2(seq) => {
                handler.handle_double_bit_binary(
                    HeaderInfo::new(self.variation(), qualifier),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group10Var0 => {
                false // qualifier 0x06
            }
            RangedVariation::Group10Var1(seq) => {
                handler.handle_binary_output_status(
                    HeaderInfo::new(self.variation(), qualifier),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group10Var2(seq) => {
                handler.handle_binary_output_status(
                    HeaderInfo::new(self.variation(), qualifier),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group20Var0 => {
                false // qualifier 0x06
            }
            RangedVariation::Group20Var1(seq) => {
                handler.handle_counter(
                    HeaderInfo::new(self.variation(), qualifier),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group20Var2(seq) => {
                handler.handle_counter(
                    HeaderInfo::new(self.variation(), qualifier),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group20Var5(seq) => {
                handler.handle_counter(
                    HeaderInfo::new(self.variation(), qualifier),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group20Var6(seq) => {
                handler.handle_counter(
                    HeaderInfo::new(self.variation(), qualifier),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group21Var0 => {
                false // qualifier 0x06
            }
            RangedVariation::Group21Var1(seq) => {
                handler.handle_frozen_counter(
                    HeaderInfo::new(self.variation(), qualifier),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group21Var2(seq) => {
                handler.handle_frozen_counter(
                    HeaderInfo::new(self.variation(), qualifier),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group21Var5(seq) => {
                handler.handle_frozen_counter(
                    HeaderInfo::new(self.variation(), qualifier),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group21Var6(seq) => {
                handler.handle_frozen_counter(
                    HeaderInfo::new(self.variation(), qualifier),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group21Var9(seq) => {
                handler.handle_frozen_counter(
                    HeaderInfo::new(self.variation(), qualifier),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group21Var10(seq) => {
                handler.handle_frozen_counter(
                    HeaderInfo::new(self.variation(), qualifier),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group30Var0 => {
                false // qualifier 0x06
            }
            RangedVariation::Group30Var1(seq) => {
                handler.handle_analog(
                    HeaderInfo::new(self.variation(), qualifier),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group30Var2(seq) => {
                handler.handle_analog(
                    HeaderInfo::new(self.variation(), qualifier),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group30Var3(seq) => {
                handler.handle_analog(
                    HeaderInfo::new(self.variation(), qualifier),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group30Var4(seq) => {
                handler.handle_analog(
                    HeaderInfo::new(self.variation(), qualifier),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group30Var5(seq) => {
                handler.handle_analog(
                    HeaderInfo::new(self.variation(), qualifier),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group30Var6(seq) => {
                handler.handle_analog(
                    HeaderInfo::new(self.variation(), qualifier),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group40Var0 => {
                false // qualifier 0x06
            }
            RangedVariation::Group40Var1(seq) => {
                handler.handle_analog_output_status(
                    HeaderInfo::new(self.variation(), qualifier),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group40Var2(seq) => {
                handler.handle_analog_output_status(
                    HeaderInfo::new(self.variation(), qualifier),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group40Var3(seq) => {
                handler.handle_analog_output_status(
                    HeaderInfo::new(self.variation(), qualifier),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group40Var4(seq) => {
                handler.handle_analog_output_status(
                    HeaderInfo::new(self.variation(), qualifier),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group80Var1(_) => {
                false // internal indications
            }
            RangedVariation::Group110Var0 => {
                false
            }
            RangedVariation::Group110VarX(_,seq) => {
                handler.handle_octet_string(
                    HeaderInfo::new(self.variation(), qualifier),
                    &mut seq.iter()
                );
                true
            }
        }
    }
    
    pub(crate) fn variation(&self) -> Variation {
        match self {
            RangedVariation::Group1Var0 => Variation::Group1Var0,
            RangedVariation::Group1Var1(_) => Variation::Group1Var1,
            RangedVariation::Group1Var2(_) => Variation::Group1Var2,
            RangedVariation::Group3Var0 => Variation::Group3Var0,
            RangedVariation::Group3Var1(_) => Variation::Group3Var1,
            RangedVariation::Group3Var2(_) => Variation::Group3Var2,
            RangedVariation::Group10Var0 => Variation::Group10Var0,
            RangedVariation::Group10Var1(_) => Variation::Group10Var1,
            RangedVariation::Group10Var2(_) => Variation::Group10Var2,
            RangedVariation::Group20Var0 => Variation::Group20Var0,
            RangedVariation::Group20Var1(_) => Variation::Group20Var1,
            RangedVariation::Group20Var2(_) => Variation::Group20Var2,
            RangedVariation::Group20Var5(_) => Variation::Group20Var5,
            RangedVariation::Group20Var6(_) => Variation::Group20Var6,
            RangedVariation::Group21Var0 => Variation::Group21Var0,
            RangedVariation::Group21Var1(_) => Variation::Group21Var1,
            RangedVariation::Group21Var2(_) => Variation::Group21Var2,
            RangedVariation::Group21Var5(_) => Variation::Group21Var5,
            RangedVariation::Group21Var6(_) => Variation::Group21Var6,
            RangedVariation::Group21Var9(_) => Variation::Group21Var9,
            RangedVariation::Group21Var10(_) => Variation::Group21Var10,
            RangedVariation::Group30Var0 => Variation::Group30Var0,
            RangedVariation::Group30Var1(_) => Variation::Group30Var1,
            RangedVariation::Group30Var2(_) => Variation::Group30Var2,
            RangedVariation::Group30Var3(_) => Variation::Group30Var3,
            RangedVariation::Group30Var4(_) => Variation::Group30Var4,
            RangedVariation::Group30Var5(_) => Variation::Group30Var5,
            RangedVariation::Group30Var6(_) => Variation::Group30Var6,
            RangedVariation::Group40Var0 => Variation::Group40Var0,
            RangedVariation::Group40Var1(_) => Variation::Group40Var1,
            RangedVariation::Group40Var2(_) => Variation::Group40Var2,
            RangedVariation::Group40Var3(_) => Variation::Group40Var3,
            RangedVariation::Group40Var4(_) => Variation::Group40Var4,
            RangedVariation::Group80Var1(_) => Variation::Group80Var1,
            RangedVariation::Group110Var0 => Variation::Group110(0),
            RangedVariation::Group110VarX(x, _) => Variation::Group110(*x),
        }
    }
}
