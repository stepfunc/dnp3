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

use crate::app::gen::variations::fixed::*;
use crate::app::gen::variations::gv::Variation;
use crate::app::parse::bytes::PrefixedBytesSequence;
use crate::app::parse::count::CountSequence;
use crate::app::parse::parser::HeaderParseError;
use crate::app::parse::prefix::Prefix;
use crate::app::parse::traits::FixedSize;
use crate::util::cursor::ReadCursor;

#[derive(Debug, PartialEq)]
pub enum PrefixedVariation<'a, I>
where
    I: FixedSize,
{
    Group2Var1(CountSequence<'a, Prefix<I, Group2Var1>>),
    Group2Var2(CountSequence<'a, Prefix<I, Group2Var2>>),
    Group2Var3(CountSequence<'a, Prefix<I, Group2Var3>>),
    Group4Var1(CountSequence<'a, Prefix<I, Group4Var1>>),
    Group4Var2(CountSequence<'a, Prefix<I, Group4Var2>>),
    Group4Var3(CountSequence<'a, Prefix<I, Group4Var3>>),
    Group11Var1(CountSequence<'a, Prefix<I, Group11Var1>>),
    Group11Var2(CountSequence<'a, Prefix<I, Group11Var2>>),
    Group12Var1(CountSequence<'a, Prefix<I, Group12Var1>>),
    Group13Var1(CountSequence<'a, Prefix<I, Group13Var1>>),
    Group13Var2(CountSequence<'a, Prefix<I, Group13Var2>>),
    Group22Var1(CountSequence<'a, Prefix<I, Group22Var1>>),
    Group22Var2(CountSequence<'a, Prefix<I, Group22Var2>>),
    Group22Var5(CountSequence<'a, Prefix<I, Group22Var5>>),
    Group22Var6(CountSequence<'a, Prefix<I, Group22Var6>>),
    Group23Var1(CountSequence<'a, Prefix<I, Group23Var1>>),
    Group23Var2(CountSequence<'a, Prefix<I, Group23Var2>>),
    Group23Var5(CountSequence<'a, Prefix<I, Group23Var5>>),
    Group23Var6(CountSequence<'a, Prefix<I, Group23Var6>>),
    Group32Var1(CountSequence<'a, Prefix<I, Group32Var1>>),
    Group32Var2(CountSequence<'a, Prefix<I, Group32Var2>>),
    Group32Var3(CountSequence<'a, Prefix<I, Group32Var3>>),
    Group32Var4(CountSequence<'a, Prefix<I, Group32Var4>>),
    Group32Var5(CountSequence<'a, Prefix<I, Group32Var5>>),
    Group32Var6(CountSequence<'a, Prefix<I, Group32Var6>>),
    Group32Var7(CountSequence<'a, Prefix<I, Group32Var7>>),
    Group32Var8(CountSequence<'a, Prefix<I, Group32Var8>>),
    Group40Var1(CountSequence<'a, Prefix<I, Group40Var1>>),
    Group40Var2(CountSequence<'a, Prefix<I, Group40Var2>>),
    Group40Var3(CountSequence<'a, Prefix<I, Group40Var3>>),
    Group40Var4(CountSequence<'a, Prefix<I, Group40Var4>>),
    Group41Var1(CountSequence<'a, Prefix<I, Group41Var1>>),
    Group41Var2(CountSequence<'a, Prefix<I, Group41Var2>>),
    Group41Var3(CountSequence<'a, Prefix<I, Group41Var3>>),
    Group41Var4(CountSequence<'a, Prefix<I, Group41Var4>>),
    Group42Var1(CountSequence<'a, Prefix<I, Group42Var1>>),
    Group42Var2(CountSequence<'a, Prefix<I, Group42Var2>>),
    Group42Var3(CountSequence<'a, Prefix<I, Group42Var3>>),
    Group42Var4(CountSequence<'a, Prefix<I, Group42Var4>>),
    Group42Var5(CountSequence<'a, Prefix<I, Group42Var5>>),
    Group42Var6(CountSequence<'a, Prefix<I, Group42Var6>>),
    Group42Var7(CountSequence<'a, Prefix<I, Group42Var7>>),
    Group42Var8(CountSequence<'a, Prefix<I, Group42Var8>>),
    Group43Var1(CountSequence<'a, Prefix<I, Group43Var1>>),
    Group43Var2(CountSequence<'a, Prefix<I, Group43Var2>>),
    Group43Var3(CountSequence<'a, Prefix<I, Group43Var3>>),
    Group43Var4(CountSequence<'a, Prefix<I, Group43Var4>>),
    Group43Var5(CountSequence<'a, Prefix<I, Group43Var5>>),
    Group43Var6(CountSequence<'a, Prefix<I, Group43Var6>>),
    Group43Var7(CountSequence<'a, Prefix<I, Group43Var7>>),
    Group43Var8(CountSequence<'a, Prefix<I, Group43Var8>>),
    Group111VarX(u8, PrefixedBytesSequence<'a, I>),
}

impl<'a, I> PrefixedVariation<'a, I>
where
    I: FixedSize,
{
    #[rustfmt::skip]
    pub fn parse(v: Variation, count: u16, cursor: &mut ReadCursor<'a>) -> Result<PrefixedVariation<'a, I>, HeaderParseError> {
        match v {
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
            Variation::Group40Var1 => Ok(PrefixedVariation::Group40Var1(CountSequence::parse(count, cursor)?)),
            Variation::Group40Var2 => Ok(PrefixedVariation::Group40Var2(CountSequence::parse(count, cursor)?)),
            Variation::Group40Var3 => Ok(PrefixedVariation::Group40Var3(CountSequence::parse(count, cursor)?)),
            Variation::Group40Var4 => Ok(PrefixedVariation::Group40Var4(CountSequence::parse(count, cursor)?)),
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
            Variation::Group111(0) => Err(HeaderParseError::ZeroLengthOctetData),
            Variation::Group111(x) => Ok(PrefixedVariation::Group111VarX(x, PrefixedBytesSequence::parse(x, count, cursor)?)),
            _ => Err(HeaderParseError::InvalidQualifierForVariation(v)),
        }
    }
}
