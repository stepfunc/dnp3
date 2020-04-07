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

use crate::app::parse::range::{RangedSequence, Range};
use crate::app::gen::variations::fixed::*;
use crate::app::gen::variations::gv::Variation;
use crate::util::cursor::ReadCursor;
use crate::app::parse::parser::ObjectParseError;
use crate::app::parse::bytes::RangedBytesSequence;
use crate::app::parse::bit::{BitSequence, DoubleBitSequence};
use crate::util::logging::log_indexed_items;
use crate::master::handlers::MeasurementHandler;

#[derive(Debug, PartialEq)]
pub enum RangedVariation<'a> {
    Group1Var0,
    Group1Var1(BitSequence<'a>),
    Group1Var2(RangedSequence<'a, Group1Var2>),
    Group3Var0,
    Group3Var1(DoubleBitSequence<'a>),
    Group3Var2(RangedSequence<'a, Group3Var2>),
    Group10Var0,
    Group10Var1(BitSequence<'a>),
    Group10Var2(RangedSequence<'a, Group10Var2>),
    Group20Var0,
    Group20Var1(RangedSequence<'a, Group20Var1>),
    Group20Var2(RangedSequence<'a, Group20Var2>),
    Group20Var5(RangedSequence<'a, Group20Var5>),
    Group20Var6(RangedSequence<'a, Group20Var6>),
    Group21Var0,
    Group21Var1(RangedSequence<'a, Group21Var1>),
    Group21Var2(RangedSequence<'a, Group21Var2>),
    Group21Var5(RangedSequence<'a, Group21Var5>),
    Group21Var6(RangedSequence<'a, Group21Var6>),
    Group21Var9(RangedSequence<'a, Group21Var9>),
    Group21Var10(RangedSequence<'a, Group21Var10>),
    Group30Var0,
    Group30Var1(RangedSequence<'a, Group30Var1>),
    Group30Var2(RangedSequence<'a, Group30Var2>),
    Group30Var3(RangedSequence<'a, Group30Var3>),
    Group30Var4(RangedSequence<'a, Group30Var4>),
    Group30Var5(RangedSequence<'a, Group30Var5>),
    Group30Var6(RangedSequence<'a, Group30Var6>),
    Group40Var0,
    Group40Var1(RangedSequence<'a, Group40Var1>),
    Group40Var2(RangedSequence<'a, Group40Var2>),
    Group40Var3(RangedSequence<'a, Group40Var3>),
    Group40Var4(RangedSequence<'a, Group40Var4>),
    Group80Var1(BitSequence<'a>),
    Group110Var0,
    Group110VarX(u8, RangedBytesSequence<'a>),
}

impl<'a> RangedVariation<'a> {
    #[rustfmt::skip]
    pub fn parse_non_read(v: Variation, range: Range, cursor: &mut ReadCursor<'a>) -> Result<RangedVariation<'a>, ObjectParseError> {
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
            _ => Err(ObjectParseError::InvalidQualifierForVariation(v)),
        }
    }
    
    pub fn parse_read(v: Variation) -> Result<RangedVariation<'a>, ObjectParseError> {
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
            _ => Err(ObjectParseError::InvalidQualifierForVariation(v)),
        }
    }
    
    pub fn log(&self, level : log::Level) {
        match self {
            RangedVariation::Group1Var0 => {}
            RangedVariation::Group1Var1(seq) => log_indexed_items(level, seq.iter()),
            RangedVariation::Group1Var2(seq) => log_indexed_items(level, seq.iter()),
            RangedVariation::Group3Var0 => {}
            RangedVariation::Group3Var1(seq) => log_indexed_items(level, seq.iter()),
            RangedVariation::Group3Var2(seq) => log_indexed_items(level, seq.iter()),
            RangedVariation::Group10Var0 => {}
            RangedVariation::Group10Var1(seq) => log_indexed_items(level, seq.iter()),
            RangedVariation::Group10Var2(seq) => log_indexed_items(level, seq.iter()),
            RangedVariation::Group20Var0 => {}
            RangedVariation::Group20Var1(seq) => log_indexed_items(level, seq.iter()),
            RangedVariation::Group20Var2(seq) => log_indexed_items(level, seq.iter()),
            RangedVariation::Group20Var5(seq) => log_indexed_items(level, seq.iter()),
            RangedVariation::Group20Var6(seq) => log_indexed_items(level, seq.iter()),
            RangedVariation::Group21Var0 => {}
            RangedVariation::Group21Var1(seq) => log_indexed_items(level, seq.iter()),
            RangedVariation::Group21Var2(seq) => log_indexed_items(level, seq.iter()),
            RangedVariation::Group21Var5(seq) => log_indexed_items(level, seq.iter()),
            RangedVariation::Group21Var6(seq) => log_indexed_items(level, seq.iter()),
            RangedVariation::Group21Var9(seq) => log_indexed_items(level, seq.iter()),
            RangedVariation::Group21Var10(seq) => log_indexed_items(level, seq.iter()),
            RangedVariation::Group30Var0 => {}
            RangedVariation::Group30Var1(seq) => log_indexed_items(level, seq.iter()),
            RangedVariation::Group30Var2(seq) => log_indexed_items(level, seq.iter()),
            RangedVariation::Group30Var3(seq) => log_indexed_items(level, seq.iter()),
            RangedVariation::Group30Var4(seq) => log_indexed_items(level, seq.iter()),
            RangedVariation::Group30Var5(seq) => log_indexed_items(level, seq.iter()),
            RangedVariation::Group30Var6(seq) => log_indexed_items(level, seq.iter()),
            RangedVariation::Group40Var0 => {}
            RangedVariation::Group40Var1(seq) => log_indexed_items(level, seq.iter()),
            RangedVariation::Group40Var2(seq) => log_indexed_items(level, seq.iter()),
            RangedVariation::Group40Var3(seq) => log_indexed_items(level, seq.iter()),
            RangedVariation::Group40Var4(seq) => log_indexed_items(level, seq.iter()),
            RangedVariation::Group80Var1(seq) => log_indexed_items(level, seq.iter()),
            RangedVariation::Group110Var0 => {}
            RangedVariation::Group110VarX(_,seq) =>  log_indexed_items(level, seq.iter()),
        }
    }
    
    pub fn extract_measurements_to<T>(&self, handler: &mut T) -> bool where T: MeasurementHandler {
        match self {
            RangedVariation::Group1Var0 => {
                false // qualifier 0x06
            }
            RangedVariation::Group1Var1(seq) => {
                handler.handle_binary(seq.iter().map(|(v,i)| (v.into(), i)));
                true
            }
            RangedVariation::Group1Var2(seq) => {
                handler.handle_binary(seq.iter().map(|(v,i)| (v.into(), i)));
                true
            }
            RangedVariation::Group3Var0 => {
                false // qualifier 0x06
            }
            RangedVariation::Group3Var1(seq) => {
                handler.handle_double_bit_binary(seq.iter().map(|(v,i)| (v.into(), i)));
                true
            }
            RangedVariation::Group3Var2(seq) => {
                handler.handle_double_bit_binary(seq.iter().map(|(v,i)| (v.into(), i)));
                true
            }
            RangedVariation::Group10Var0 => {
                false // qualifier 0x06
            }
            RangedVariation::Group10Var1(seq) => {
                handler.handle_binary_output_status(seq.iter().map(|(v,i)| (v.into(), i)));
                true
            }
            RangedVariation::Group10Var2(seq) => {
                handler.handle_binary_output_status(seq.iter().map(|(v,i)| (v.into(), i)));
                true
            }
            RangedVariation::Group20Var0 => {
                false // qualifier 0x06
            }
            RangedVariation::Group20Var1(seq) => {
                handler.handle_counter(seq.iter().map(|(v,i)| (v.into(), i)));
                true
            }
            RangedVariation::Group20Var2(seq) => {
                handler.handle_counter(seq.iter().map(|(v,i)| (v.into(), i)));
                true
            }
            RangedVariation::Group20Var5(seq) => {
                handler.handle_counter(seq.iter().map(|(v,i)| (v.into(), i)));
                true
            }
            RangedVariation::Group20Var6(seq) => {
                handler.handle_counter(seq.iter().map(|(v,i)| (v.into(), i)));
                true
            }
            RangedVariation::Group21Var0 => {
                false // qualifier 0x06
            }
            RangedVariation::Group21Var1(seq) => {
                handler.handle_frozen_counter(seq.iter().map(|(v,i)| (v.into(), i)));
                true
            }
            RangedVariation::Group21Var2(seq) => {
                handler.handle_frozen_counter(seq.iter().map(|(v,i)| (v.into(), i)));
                true
            }
            RangedVariation::Group21Var5(seq) => {
                handler.handle_frozen_counter(seq.iter().map(|(v,i)| (v.into(), i)));
                true
            }
            RangedVariation::Group21Var6(seq) => {
                handler.handle_frozen_counter(seq.iter().map(|(v,i)| (v.into(), i)));
                true
            }
            RangedVariation::Group21Var9(seq) => {
                handler.handle_frozen_counter(seq.iter().map(|(v,i)| (v.into(), i)));
                true
            }
            RangedVariation::Group21Var10(seq) => {
                handler.handle_frozen_counter(seq.iter().map(|(v,i)| (v.into(), i)));
                true
            }
            RangedVariation::Group30Var0 => {
                false // qualifier 0x06
            }
            RangedVariation::Group30Var1(seq) => {
                handler.handle_analog(seq.iter().map(|(v,i)| (v.into(), i)));
                true
            }
            RangedVariation::Group30Var2(seq) => {
                handler.handle_analog(seq.iter().map(|(v,i)| (v.into(), i)));
                true
            }
            RangedVariation::Group30Var3(seq) => {
                handler.handle_analog(seq.iter().map(|(v,i)| (v.into(), i)));
                true
            }
            RangedVariation::Group30Var4(seq) => {
                handler.handle_analog(seq.iter().map(|(v,i)| (v.into(), i)));
                true
            }
            RangedVariation::Group30Var5(seq) => {
                handler.handle_analog(seq.iter().map(|(v,i)| (v.into(), i)));
                true
            }
            RangedVariation::Group30Var6(seq) => {
                handler.handle_analog(seq.iter().map(|(v,i)| (v.into(), i)));
                true
            }
            RangedVariation::Group40Var0 => {
                false // qualifier 0x06
            }
            RangedVariation::Group40Var1(seq) => {
                handler.handle_analog_output_status(seq.iter().map(|(v,i)| (v.into(), i)));
                true
            }
            RangedVariation::Group40Var2(seq) => {
                handler.handle_analog_output_status(seq.iter().map(|(v,i)| (v.into(), i)));
                true
            }
            RangedVariation::Group40Var3(seq) => {
                handler.handle_analog_output_status(seq.iter().map(|(v,i)| (v.into(), i)));
                true
            }
            RangedVariation::Group40Var4(seq) => {
                handler.handle_analog_output_status(seq.iter().map(|(v,i)| (v.into(), i)));
                true
            }
            RangedVariation::Group80Var1(_) => {
                false // internal indications
            }
            RangedVariation::Group110Var0 => {
                false
            }
            RangedVariation::Group110VarX(_,seq) => {
                handler.handle_octet_string(seq.iter());
                true
            }
        }
    }
}
