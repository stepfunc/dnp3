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

use crate::app::gen::variations::gv::Variation;
use crate::app::parse::count::CountSequence;
use crate::app::gen::variations::fixed::*;
use crate::util::cursor::ReadCursor;
use crate::app::parse::parser::ObjectParseError;
use crate::app::parse::traits::{FixedSize, Index};
use crate::app::parse::prefix::Prefix;
use crate::app::parse::bytes::PrefixedBytesSequence;
use crate::app::measurement::Time;
use crate::master::handlers::MeasurementHandler;
use crate::util::logging::*;

#[derive(Debug, PartialEq)]
pub enum PrefixedVariation<'a, I> where I : FixedSize + Index + std::fmt::Display {
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

impl<'a, I> PrefixedVariation<'a, I> where I : FixedSize + Index + std::fmt::Display {
    #[rustfmt::skip]
    pub fn parse(v: Variation, count: u16, cursor: &mut ReadCursor<'a>) -> Result<PrefixedVariation<'a, I>, ObjectParseError> {
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
            Variation::Group111(0) => Err(ObjectParseError::ZeroLengthOctetData),
            Variation::Group111(x) => Ok(PrefixedVariation::Group111VarX(x, PrefixedBytesSequence::parse(x, count, cursor)?)),
            _ => Err(ObjectParseError::InvalidQualifierForVariation(v)),
        }
    }
    
    pub fn log(&self, level : log::Level) {
        match self {
            PrefixedVariation::Group2Var1(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group2Var2(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group2Var3(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group4Var1(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group4Var2(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group4Var3(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group11Var1(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group11Var2(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group12Var1(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group13Var1(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group13Var2(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group22Var1(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group22Var2(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group22Var5(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group22Var6(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group23Var1(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group23Var2(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group23Var5(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group23Var6(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group32Var1(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group32Var2(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group32Var3(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group32Var4(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group32Var5(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group32Var6(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group32Var7(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group32Var8(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group41Var1(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group41Var2(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group41Var3(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group41Var4(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group42Var1(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group42Var2(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group42Var3(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group42Var4(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group42Var5(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group42Var6(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group42Var7(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group42Var8(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group43Var1(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group43Var2(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group43Var3(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group43Var4(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group43Var5(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group43Var6(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group43Var7(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group43Var8(seq) => log_prefixed_items(level, seq.iter()),
            PrefixedVariation::Group111VarX(_,seq) =>  log_indexed_items(level, seq.iter()),
        }
    }
    
    pub fn extract_measurements_to<T>(&self, cto: Time, handler: &mut T) -> bool where T: MeasurementHandler {
        match self {
            PrefixedVariation::Group2Var1(seq) => {
                handler.handle_binary(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group2Var2(seq) => {
                handler.handle_binary(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group2Var3(seq) => {
                handler.handle_binary(seq.iter().map( |x| (x.value.to_measurement(cto), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group4Var1(seq) => {
                handler.handle_double_bit_binary(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group4Var2(seq) => {
                handler.handle_double_bit_binary(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group4Var3(seq) => {
                handler.handle_double_bit_binary(seq.iter().map( |x| (x.value.to_measurement(cto), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group11Var1(seq) => {
                handler.handle_binary_output_status(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group11Var2(seq) => {
                handler.handle_binary_output_status(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group12Var1(_) => {
                false // command
            }
            PrefixedVariation::Group13Var1(seq) => {
                handler.handle_binary_output_status(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group13Var2(seq) => {
                handler.handle_binary_output_status(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group22Var1(seq) => {
                handler.handle_counter(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group22Var2(seq) => {
                handler.handle_counter(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group22Var5(seq) => {
                handler.handle_counter(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group22Var6(seq) => {
                handler.handle_counter(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group23Var1(seq) => {
                handler.handle_frozen_counter(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group23Var2(seq) => {
                handler.handle_frozen_counter(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group23Var5(seq) => {
                handler.handle_frozen_counter(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group23Var6(seq) => {
                handler.handle_frozen_counter(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group32Var1(seq) => {
                handler.handle_analog(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group32Var2(seq) => {
                handler.handle_analog(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group32Var3(seq) => {
                handler.handle_analog(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group32Var4(seq) => {
                handler.handle_analog(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group32Var5(seq) => {
                handler.handle_analog(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group32Var6(seq) => {
                handler.handle_analog(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group32Var7(seq) => {
                handler.handle_analog(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group32Var8(seq) => {
                handler.handle_analog(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group41Var1(_) => {
                false // command
            }
            PrefixedVariation::Group41Var2(_) => {
                false // command
            }
            PrefixedVariation::Group41Var3(_) => {
                false // command
            }
            PrefixedVariation::Group41Var4(_) => {
                false // command
            }
            PrefixedVariation::Group42Var1(seq) => {
                handler.handle_analog_output_status(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group42Var2(seq) => {
                handler.handle_analog_output_status(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group42Var3(seq) => {
                handler.handle_analog_output_status(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group42Var4(seq) => {
                handler.handle_analog_output_status(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group42Var5(seq) => {
                handler.handle_analog_output_status(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group42Var6(seq) => {
                handler.handle_analog_output_status(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group42Var7(seq) => {
                handler.handle_analog_output_status(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group42Var8(seq) => {
                handler.handle_analog_output_status(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group43Var1(seq) => {
                handler.handle_analog_output_status(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group43Var2(seq) => {
                handler.handle_analog_output_status(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group43Var3(seq) => {
                handler.handle_analog_output_status(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group43Var4(seq) => {
                handler.handle_analog_output_status(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group43Var5(seq) => {
                handler.handle_analog_output_status(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group43Var6(seq) => {
                handler.handle_analog_output_status(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group43Var7(seq) => {
                handler.handle_analog_output_status(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group43Var8(seq) => {
                handler.handle_analog_output_status(seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group111VarX(_, seq) => {
                handler.handle_octet_string(seq.iter().map(|x| (x.0, x.1.widen_to_u16())));
                true
            }
        }
    }
}
