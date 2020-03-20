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

use crate::app::parser::ParseError;
use crate::app::range::{Range, RangedSequence};
use crate::app::variations::fixed::*;
use crate::app::variations::gv::Variation;
use crate::util::cursor::ReadCursor;

#[derive(Debug, PartialEq)]
pub enum RangedVariation<'a> {
    Group1Var0,
    Group1Var2(RangedSequence<'a, Group1Var2>),
    Group3Var0,
    Group3Var2(RangedSequence<'a, Group3Var2>),
    Group10Var0,
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
}

impl<'a> RangedVariation<'a> {
    #[rustfmt::skip]
    pub fn parse_non_read(v: Variation, range: Range, cursor: &mut ReadCursor<'a>) -> Result<RangedVariation<'a>, ParseError> {
        match v {
            Variation::Group1Var0 => Ok(RangedVariation::Group1Var0),
            Variation::Group1Var2 => Ok(RangedVariation::Group1Var2(RangedSequence::parse(range, cursor)?)),
            Variation::Group3Var0 => Ok(RangedVariation::Group3Var0),
            Variation::Group3Var2 => Ok(RangedVariation::Group3Var2(RangedSequence::parse(range, cursor)?)),
            Variation::Group10Var0 => Ok(RangedVariation::Group10Var0),
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
            _ => Err(ParseError::InvalidQualifierAndObject),
        }
    }

    pub fn parse_read(v: Variation) -> Result<RangedVariation<'a>, ParseError> {
        match v {
            Variation::Group1Var0 => Ok(RangedVariation::Group1Var0),
            Variation::Group1Var2 => Ok(RangedVariation::Group1Var2(RangedSequence::empty())),
            Variation::Group3Var0 => Ok(RangedVariation::Group3Var0),
            Variation::Group3Var2 => Ok(RangedVariation::Group3Var2(RangedSequence::empty())),
            Variation::Group10Var0 => Ok(RangedVariation::Group10Var0),
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
            _ => Err(ParseError::InvalidQualifierAndObject),
        }
    }
}
