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

use crate::app::gen::variations::variation::Variation;
use crate::app::parse::count::CountSequence;
use crate::app::gen::variations::fixed::*;
use crate::util::cursor::ReadCursor;
use crate::app::parse::parser::*;
use crate::app::parse::traits::{FixedSize, Index};
use crate::app::parse::prefix::Prefix;
use crate::app::parse::bytes::PrefixedBytesSequence;
use crate::app::measurement::Time;
use crate::master::handle::ReadHandler;
use crate::app::parse::error::ObjectParseError;

#[derive(Debug, PartialEq)]
pub(crate) enum PrefixedVariation<'a, I> where I : FixedSize + Index + std::fmt::Display {
    /// Binary Input Event - Without Time
    Group2Var1(CountSequence<'a, Prefix<I, Group2Var1>>),
    /// Binary Input Event - With Absolute Time
    Group2Var2(CountSequence<'a, Prefix<I, Group2Var2>>),
    /// Binary Input Event - With Relative Time
    Group2Var3(CountSequence<'a, Prefix<I, Group2Var3>>),
    /// Double-bit Binary Input Event - Without Time
    Group4Var1(CountSequence<'a, Prefix<I, Group4Var1>>),
    /// Double-bit Binary Input Event - With Absolute Time
    Group4Var2(CountSequence<'a, Prefix<I, Group4Var2>>),
    /// Double-bit Binary Input Event - With Relative Time
    Group4Var3(CountSequence<'a, Prefix<I, Group4Var3>>),
    /// Binary Output Event - Output Status Without Time
    Group11Var1(CountSequence<'a, Prefix<I, Group11Var1>>),
    /// Binary Output Event - Output Status With Time
    Group11Var2(CountSequence<'a, Prefix<I, Group11Var2>>),
    /// Binary Command - Control Relay Output Block
    Group12Var1(CountSequence<'a, Prefix<I, Group12Var1>>),
    /// Binary Command Event - Without Time
    Group13Var1(CountSequence<'a, Prefix<I, Group13Var1>>),
    /// Binary Command Event - With Time
    Group13Var2(CountSequence<'a, Prefix<I, Group13Var2>>),
    /// Counter Event - 32-bit With Flag
    Group22Var1(CountSequence<'a, Prefix<I, Group22Var1>>),
    /// Counter Event - 16-bit With Flag
    Group22Var2(CountSequence<'a, Prefix<I, Group22Var2>>),
    /// Counter Event - 32-bit With Flag and Time
    Group22Var5(CountSequence<'a, Prefix<I, Group22Var5>>),
    /// Counter Event - 16-bit With Flag and Time
    Group22Var6(CountSequence<'a, Prefix<I, Group22Var6>>),
    /// Frozen Counter Event - 32-bit With Flag
    Group23Var1(CountSequence<'a, Prefix<I, Group23Var1>>),
    /// Frozen Counter Event - 16-bit With Flag
    Group23Var2(CountSequence<'a, Prefix<I, Group23Var2>>),
    /// Frozen Counter Event - 32-bit With Flag and Time
    Group23Var5(CountSequence<'a, Prefix<I, Group23Var5>>),
    /// Frozen Counter Event - 16-bit With Flag and Time
    Group23Var6(CountSequence<'a, Prefix<I, Group23Var6>>),
    /// Analog Input Event - 32-bit With Flag
    Group32Var1(CountSequence<'a, Prefix<I, Group32Var1>>),
    /// Analog Input Event - 16-bit With Flag
    Group32Var2(CountSequence<'a, Prefix<I, Group32Var2>>),
    /// Analog Input Event - 32-bit With Flag and Time
    Group32Var3(CountSequence<'a, Prefix<I, Group32Var3>>),
    /// Analog Input Event - 16-bit With Flag and Time
    Group32Var4(CountSequence<'a, Prefix<I, Group32Var4>>),
    /// Analog Input Event - Single-precision With Flag
    Group32Var5(CountSequence<'a, Prefix<I, Group32Var5>>),
    /// Analog Input Event - Double-precision With Flag
    Group32Var6(CountSequence<'a, Prefix<I, Group32Var6>>),
    /// Analog Input Event - Single-precision With Flag and Time
    Group32Var7(CountSequence<'a, Prefix<I, Group32Var7>>),
    /// Analog Input Event - Double-precision With Flag and Time
    Group32Var8(CountSequence<'a, Prefix<I, Group32Var8>>),
    /// Analog Output - 32-bit With Flag
    Group41Var1(CountSequence<'a, Prefix<I, Group41Var1>>),
    /// Analog Output - 16-bit With Flag
    Group41Var2(CountSequence<'a, Prefix<I, Group41Var2>>),
    /// Analog Output - Single-precision
    Group41Var3(CountSequence<'a, Prefix<I, Group41Var3>>),
    /// Analog Output - Double-precision
    Group41Var4(CountSequence<'a, Prefix<I, Group41Var4>>),
    /// Analog Output Event - 32-bit With Flag
    Group42Var1(CountSequence<'a, Prefix<I, Group42Var1>>),
    /// Analog Output Event - 16-bit With Flag
    Group42Var2(CountSequence<'a, Prefix<I, Group42Var2>>),
    /// Analog Output Event - 32-bit With Flag and Time
    Group42Var3(CountSequence<'a, Prefix<I, Group42Var3>>),
    /// Analog Output Event - 16-bit With Flag and Time
    Group42Var4(CountSequence<'a, Prefix<I, Group42Var4>>),
    /// Analog Output Event - Single-precision With Flag
    Group42Var5(CountSequence<'a, Prefix<I, Group42Var5>>),
    /// Analog Output Event - Double-precision With Flag
    Group42Var6(CountSequence<'a, Prefix<I, Group42Var6>>),
    /// Analog Output Event - Single-precision With Flag and Time
    Group42Var7(CountSequence<'a, Prefix<I, Group42Var7>>),
    /// Analog Output Event - Double-precision With Flag and Time
    Group42Var8(CountSequence<'a, Prefix<I, Group42Var8>>),
    /// Analog Command Event - 32-bit
    Group43Var1(CountSequence<'a, Prefix<I, Group43Var1>>),
    /// Analog Command Event - 16-bit
    Group43Var2(CountSequence<'a, Prefix<I, Group43Var2>>),
    /// Analog Command Event - 32-bit With Time
    Group43Var3(CountSequence<'a, Prefix<I, Group43Var3>>),
    /// Analog Command Event - 16-bit With Time
    Group43Var4(CountSequence<'a, Prefix<I, Group43Var4>>),
    /// Analog Command Event - Single-precision
    Group43Var5(CountSequence<'a, Prefix<I, Group43Var5>>),
    /// Analog Command Event - Double-precision
    Group43Var6(CountSequence<'a, Prefix<I, Group43Var6>>),
    /// Analog Command Event - Single-precision With Time
    Group43Var7(CountSequence<'a, Prefix<I, Group43Var7>>),
    /// Analog Command Event - Double-precision With Time
    Group43Var8(CountSequence<'a, Prefix<I, Group43Var8>>),
    /// Octet String Event - Sized by variation
    Group111VarX(u8, PrefixedBytesSequence<'a, I>),
}

impl<'a, I> PrefixedVariation<'a, I> where I : FixedSize + Index + std::fmt::Display {
    pub(crate) fn parse(v: Variation, count: u16, cursor: &mut ReadCursor<'a>) -> Result<PrefixedVariation<'a, I>, ObjectParseError> {
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
            _ => Err(ObjectParseError::InvalidQualifierForVariation(v, I::COUNT_AND_PREFIX_QUALIFIER)),
        }
    }
    
    pub(crate) fn format_objects(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
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
            PrefixedVariation::Group111VarX(_,seq) =>  format_indexed_items(f, seq.iter()),
        }
    }
    
    pub(crate) fn extract_measurements_to(&self, cto: Time, handler: &mut dyn ReadHandler) -> bool {
        match self {
            PrefixedVariation::Group2Var1(seq) => {
                handler.handle_binary(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group2Var2(seq) => {
                handler.handle_binary(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group2Var3(seq) => {
                handler.handle_binary(&mut seq.iter().map( |x| (x.value.to_measurement(cto), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group4Var1(seq) => {
                handler.handle_double_bit_binary(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group4Var2(seq) => {
                handler.handle_double_bit_binary(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group4Var3(seq) => {
                handler.handle_double_bit_binary(&mut seq.iter().map( |x| (x.value.to_measurement(cto), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group11Var1(seq) => {
                handler.handle_binary_output_status(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group11Var2(seq) => {
                handler.handle_binary_output_status(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group12Var1(_) => {
                false // command
            }
            PrefixedVariation::Group13Var1(seq) => {
                handler.handle_binary_output_status(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group13Var2(seq) => {
                handler.handle_binary_output_status(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group22Var1(seq) => {
                handler.handle_counter(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group22Var2(seq) => {
                handler.handle_counter(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group22Var5(seq) => {
                handler.handle_counter(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group22Var6(seq) => {
                handler.handle_counter(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group23Var1(seq) => {
                handler.handle_frozen_counter(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group23Var2(seq) => {
                handler.handle_frozen_counter(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group23Var5(seq) => {
                handler.handle_frozen_counter(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group23Var6(seq) => {
                handler.handle_frozen_counter(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group32Var1(seq) => {
                handler.handle_analog(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group32Var2(seq) => {
                handler.handle_analog(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group32Var3(seq) => {
                handler.handle_analog(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group32Var4(seq) => {
                handler.handle_analog(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group32Var5(seq) => {
                handler.handle_analog(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group32Var6(seq) => {
                handler.handle_analog(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group32Var7(seq) => {
                handler.handle_analog(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group32Var8(seq) => {
                handler.handle_analog(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
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
                handler.handle_analog_output_status(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group42Var2(seq) => {
                handler.handle_analog_output_status(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group42Var3(seq) => {
                handler.handle_analog_output_status(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group42Var4(seq) => {
                handler.handle_analog_output_status(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group42Var5(seq) => {
                handler.handle_analog_output_status(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group42Var6(seq) => {
                handler.handle_analog_output_status(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group42Var7(seq) => {
                handler.handle_analog_output_status(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group42Var8(seq) => {
                handler.handle_analog_output_status(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group43Var1(seq) => {
                handler.handle_analog_output_status(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group43Var2(seq) => {
                handler.handle_analog_output_status(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group43Var3(seq) => {
                handler.handle_analog_output_status(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group43Var4(seq) => {
                handler.handle_analog_output_status(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group43Var5(seq) => {
                handler.handle_analog_output_status(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group43Var6(seq) => {
                handler.handle_analog_output_status(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group43Var7(seq) => {
                handler.handle_analog_output_status(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group43Var8(seq) => {
                handler.handle_analog_output_status(&mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16())));
                true
            }
            PrefixedVariation::Group111VarX(_, seq) => {
                handler.handle_octet_string(&mut seq.iter().map(|x| (x.0, x.1.widen_to_u16())));
                true
            }
        }
    }
}
