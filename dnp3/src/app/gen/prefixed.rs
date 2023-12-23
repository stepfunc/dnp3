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
use crate::app::parse::traits::{FixedSize, Index};
use crate::app::parse::prefix::Prefix;
use crate::app::parse::bytes::*;
use crate::app::measurement::Time;
use crate::master::{ReadHandler, HeaderInfo};
use crate::app::ObjectParseError;

use scursor::ReadCursor;

#[derive(Debug)]
pub(crate) enum PrefixedVariation<'a, I> where I : FixedSize + Index + std::fmt::Display {
    /// Device Attributes - Specific Attribute
    Group0(crate::app::attr::Attribute<'a>),
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
    /// Binary Output Command Event - Without Time
    Group13Var1(CountSequence<'a, Prefix<I, Group13Var1>>),
    /// Binary Output Command Event - With Time
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
    /// Frozen Analog Input Event - 32-bit With Flag
    Group33Var1(CountSequence<'a, Prefix<I, Group33Var1>>),
    /// Frozen Analog Input Event - 16-bit With Flag
    Group33Var2(CountSequence<'a, Prefix<I, Group33Var2>>),
    /// Frozen Analog Input Event - 32-bit with Flag and Time-of-Freeze
    Group33Var3(CountSequence<'a, Prefix<I, Group33Var3>>),
    /// Frozen Analog Input Event - 16-bit with Flag and Time-of-Freeze
    Group33Var4(CountSequence<'a, Prefix<I, Group33Var4>>),
    /// Frozen Analog Input Event - Single-precision With Flag
    Group33Var5(CountSequence<'a, Prefix<I, Group33Var5>>),
    /// Frozen Analog Input Event - Double-precision With Flag
    Group33Var6(CountSequence<'a, Prefix<I, Group33Var6>>),
    /// Frozen Analog Input Event - Single-precision With Flag and Time
    Group33Var7(CountSequence<'a, Prefix<I, Group33Var7>>),
    /// Frozen Analog Input Event - Double-precision With Flag and Time
    Group33Var8(CountSequence<'a, Prefix<I, Group33Var8>>),
    /// Analog Input Reporting Deadband - 16-bit
    Group34Var1(CountSequence<'a, Prefix<I, Group34Var1>>),
    /// Analog Input Reporting Deadband - 32-bit
    Group34Var2(CountSequence<'a, Prefix<I, Group34Var2>>),
    /// Analog Input Reporting Deadband - Single-precision
    Group34Var3(CountSequence<'a, Prefix<I, Group34Var3>>),
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
    /// Analog Output Command Event - 32-bit
    Group43Var1(CountSequence<'a, Prefix<I, Group43Var1>>),
    /// Analog Output Command Event - 16-bit
    Group43Var2(CountSequence<'a, Prefix<I, Group43Var2>>),
    /// Analog Output Command Event - 32-bit With Time
    Group43Var3(CountSequence<'a, Prefix<I, Group43Var3>>),
    /// Analog Output Command Event - 16-bit With Time
    Group43Var4(CountSequence<'a, Prefix<I, Group43Var4>>),
    /// Analog Output Command Event - Single-precision
    Group43Var5(CountSequence<'a, Prefix<I, Group43Var5>>),
    /// Analog Output Command Event - Double-precision
    Group43Var6(CountSequence<'a, Prefix<I, Group43Var6>>),
    /// Analog Output Command Event - Single-precision With Time
    Group43Var7(CountSequence<'a, Prefix<I, Group43Var7>>),
    /// Analog Output Command Event - Double-precision With Time
    Group43Var8(CountSequence<'a, Prefix<I, Group43Var8>>),
    /// Octet String Event - Sized by variation
    Group111VarX(u8, PrefixedBytesSequence<'a, I>),
}

impl<'a, I> PrefixedVariation<'a, I> where I : FixedSize + Index + std::fmt::Display {
    pub(crate) fn parse(v: Variation, count: u16, cursor: &mut ReadCursor<'a>) -> Result<PrefixedVariation<'a, I>, ObjectParseError> {
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
            Variation::Group111(0) => Err(ObjectParseError::ZeroLengthOctetData),
            Variation::Group111(x) => Ok(PrefixedVariation::Group111VarX(x, PrefixedBytesSequence::parse(x, count, cursor)?)),
            _ => Err(ObjectParseError::InvalidQualifierForVariation(v, I::COUNT_AND_PREFIX_QUALIFIER)),
        }
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
    
    pub(crate) fn extract_measurements_to(&self, cto: Option<Time>, handler: &mut dyn ReadHandler) -> bool {
        match self {
            PrefixedVariation::Group0(attr) => {
                let info = self.get_header_info();
                crate::master::handle_attribute(info.variation, info.qualifier, &Some(*attr), handler);
                true
            }
            PrefixedVariation::Group2Var1(seq) => {
                handler.handle_binary_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group2Var2(seq) => {
                handler.handle_binary_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group2Var3(seq) => {
                handler.handle_binary_input(
                    self.get_header_info(),
                    &mut seq.iter().map( |x| (x.value.to_measurement(cto), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group4Var1(seq) => {
                handler.handle_double_bit_binary_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group4Var2(seq) => {
                handler.handle_double_bit_binary_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group4Var3(seq) => {
                handler.handle_double_bit_binary_input(
                    self.get_header_info(),
                    &mut seq.iter().map( |x| (x.value.to_measurement(cto), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group11Var1(seq) => {
                handler.handle_binary_output_status(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group11Var2(seq) => {
                handler.handle_binary_output_status(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group12Var1(_) => {
                false // command
            }
            PrefixedVariation::Group13Var1(seq) => {
                handler.handle_binary_output_command_event(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group13Var2(seq) => {
                handler.handle_binary_output_command_event(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group22Var1(seq) => {
                handler.handle_counter(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group22Var2(seq) => {
                handler.handle_counter(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group22Var5(seq) => {
                handler.handle_counter(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group22Var6(seq) => {
                handler.handle_counter(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group23Var1(seq) => {
                handler.handle_frozen_counter(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group23Var2(seq) => {
                handler.handle_frozen_counter(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group23Var5(seq) => {
                handler.handle_frozen_counter(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group23Var6(seq) => {
                handler.handle_frozen_counter(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group32Var1(seq) => {
                handler.handle_analog_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group32Var2(seq) => {
                handler.handle_analog_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group32Var3(seq) => {
                handler.handle_analog_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group32Var4(seq) => {
                handler.handle_analog_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group32Var5(seq) => {
                handler.handle_analog_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group32Var6(seq) => {
                handler.handle_analog_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group32Var7(seq) => {
                handler.handle_analog_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group32Var8(seq) => {
                handler.handle_analog_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group33Var1(seq) => {
                handler.handle_frozen_analog_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group33Var2(seq) => {
                handler.handle_frozen_analog_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group33Var3(seq) => {
                handler.handle_frozen_analog_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group33Var4(seq) => {
                handler.handle_frozen_analog_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group33Var5(seq) => {
                handler.handle_frozen_analog_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group33Var6(seq) => {
                handler.handle_frozen_analog_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group33Var7(seq) => {
                handler.handle_frozen_analog_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group33Var8(seq) => {
                handler.handle_frozen_analog_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group34Var1(_) => {
                false // dead-band
            }
            PrefixedVariation::Group34Var2(_) => {
                false // dead-band
            }
            PrefixedVariation::Group34Var3(_) => {
                false // dead-band
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
                handler.handle_analog_output_status(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group42Var2(seq) => {
                handler.handle_analog_output_status(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group42Var3(seq) => {
                handler.handle_analog_output_status(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group42Var4(seq) => {
                handler.handle_analog_output_status(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group42Var5(seq) => {
                handler.handle_analog_output_status(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group42Var6(seq) => {
                handler.handle_analog_output_status(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group42Var7(seq) => {
                handler.handle_analog_output_status(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group42Var8(seq) => {
                handler.handle_analog_output_status(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group43Var1(seq) => {
                handler.handle_analog_output_command_event(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group43Var2(seq) => {
                handler.handle_analog_output_command_event(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group43Var3(seq) => {
                handler.handle_analog_output_command_event(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group43Var4(seq) => {
                handler.handle_analog_output_command_event(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group43Var5(seq) => {
                handler.handle_analog_output_command_event(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group43Var6(seq) => {
                handler.handle_analog_output_command_event(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group43Var7(seq) => {
                handler.handle_analog_output_command_event(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group43Var8(seq) => {
                handler.handle_analog_output_command_event(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group111VarX(_, seq) => {
                handler.handle_octet_string(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.0, x.1.widen_to_u16()))
                );
                true
            }
        }
    }
    
    pub(crate) fn get_header_info(&self) -> HeaderInfo {
        match self {
            PrefixedVariation::Group0(attr) => HeaderInfo::new(Variation::Group0(attr.variation), I::COUNT_AND_PREFIX_QUALIFIER, false, false),
            PrefixedVariation::Group2Var1(_) => HeaderInfo::new(Variation::Group2Var1, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group2Var2(_) => HeaderInfo::new(Variation::Group2Var2, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group2Var3(_) => HeaderInfo::new(Variation::Group2Var3, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group4Var1(_) => HeaderInfo::new(Variation::Group4Var1, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group4Var2(_) => HeaderInfo::new(Variation::Group4Var2, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group4Var3(_) => HeaderInfo::new(Variation::Group4Var3, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group11Var1(_) => HeaderInfo::new(Variation::Group11Var1, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group11Var2(_) => HeaderInfo::new(Variation::Group11Var2, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group12Var1(_) => HeaderInfo::new(Variation::Group12Var1, I::COUNT_AND_PREFIX_QUALIFIER, false, false),
            PrefixedVariation::Group13Var1(_) => HeaderInfo::new(Variation::Group13Var1, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group13Var2(_) => HeaderInfo::new(Variation::Group13Var2, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group22Var1(_) => HeaderInfo::new(Variation::Group22Var1, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group22Var2(_) => HeaderInfo::new(Variation::Group22Var2, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group22Var5(_) => HeaderInfo::new(Variation::Group22Var5, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group22Var6(_) => HeaderInfo::new(Variation::Group22Var6, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group23Var1(_) => HeaderInfo::new(Variation::Group23Var1, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group23Var2(_) => HeaderInfo::new(Variation::Group23Var2, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group23Var5(_) => HeaderInfo::new(Variation::Group23Var5, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group23Var6(_) => HeaderInfo::new(Variation::Group23Var6, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group32Var1(_) => HeaderInfo::new(Variation::Group32Var1, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group32Var2(_) => HeaderInfo::new(Variation::Group32Var2, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group32Var3(_) => HeaderInfo::new(Variation::Group32Var3, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group32Var4(_) => HeaderInfo::new(Variation::Group32Var4, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group32Var5(_) => HeaderInfo::new(Variation::Group32Var5, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group32Var6(_) => HeaderInfo::new(Variation::Group32Var6, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group32Var7(_) => HeaderInfo::new(Variation::Group32Var7, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group32Var8(_) => HeaderInfo::new(Variation::Group32Var8, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group33Var1(_) => HeaderInfo::new(Variation::Group33Var1, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group33Var2(_) => HeaderInfo::new(Variation::Group33Var2, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group33Var3(_) => HeaderInfo::new(Variation::Group33Var3, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group33Var4(_) => HeaderInfo::new(Variation::Group33Var4, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group33Var5(_) => HeaderInfo::new(Variation::Group33Var5, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group33Var6(_) => HeaderInfo::new(Variation::Group33Var6, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group33Var7(_) => HeaderInfo::new(Variation::Group33Var7, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group33Var8(_) => HeaderInfo::new(Variation::Group33Var8, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group34Var1(_) => HeaderInfo::new(Variation::Group34Var1, I::COUNT_AND_PREFIX_QUALIFIER, false, false),
            PrefixedVariation::Group34Var2(_) => HeaderInfo::new(Variation::Group34Var2, I::COUNT_AND_PREFIX_QUALIFIER, false, false),
            PrefixedVariation::Group34Var3(_) => HeaderInfo::new(Variation::Group34Var3, I::COUNT_AND_PREFIX_QUALIFIER, false, false),
            PrefixedVariation::Group41Var1(_) => HeaderInfo::new(Variation::Group41Var1, I::COUNT_AND_PREFIX_QUALIFIER, false, false),
            PrefixedVariation::Group41Var2(_) => HeaderInfo::new(Variation::Group41Var2, I::COUNT_AND_PREFIX_QUALIFIER, false, false),
            PrefixedVariation::Group41Var3(_) => HeaderInfo::new(Variation::Group41Var3, I::COUNT_AND_PREFIX_QUALIFIER, false, false),
            PrefixedVariation::Group41Var4(_) => HeaderInfo::new(Variation::Group41Var4, I::COUNT_AND_PREFIX_QUALIFIER, false, false),
            PrefixedVariation::Group42Var1(_) => HeaderInfo::new(Variation::Group42Var1, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group42Var2(_) => HeaderInfo::new(Variation::Group42Var2, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group42Var3(_) => HeaderInfo::new(Variation::Group42Var3, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group42Var4(_) => HeaderInfo::new(Variation::Group42Var4, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group42Var5(_) => HeaderInfo::new(Variation::Group42Var5, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group42Var6(_) => HeaderInfo::new(Variation::Group42Var6, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group42Var7(_) => HeaderInfo::new(Variation::Group42Var7, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group42Var8(_) => HeaderInfo::new(Variation::Group42Var8, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group43Var1(_) => HeaderInfo::new(Variation::Group43Var1, I::COUNT_AND_PREFIX_QUALIFIER, true, false),
            PrefixedVariation::Group43Var2(_) => HeaderInfo::new(Variation::Group43Var2, I::COUNT_AND_PREFIX_QUALIFIER, true, false),
            PrefixedVariation::Group43Var3(_) => HeaderInfo::new(Variation::Group43Var3, I::COUNT_AND_PREFIX_QUALIFIER, true, false),
            PrefixedVariation::Group43Var4(_) => HeaderInfo::new(Variation::Group43Var4, I::COUNT_AND_PREFIX_QUALIFIER, true, false),
            PrefixedVariation::Group43Var5(_) => HeaderInfo::new(Variation::Group43Var5, I::COUNT_AND_PREFIX_QUALIFIER, true, false),
            PrefixedVariation::Group43Var6(_) => HeaderInfo::new(Variation::Group43Var6, I::COUNT_AND_PREFIX_QUALIFIER, true, false),
            PrefixedVariation::Group43Var7(_) => HeaderInfo::new(Variation::Group43Var7, I::COUNT_AND_PREFIX_QUALIFIER, true, false),
            PrefixedVariation::Group43Var8(_) => HeaderInfo::new(Variation::Group43Var8, I::COUNT_AND_PREFIX_QUALIFIER, true, false),
            PrefixedVariation::Group111VarX(x, _) =>  HeaderInfo::new(Variation::Group111(*x), I::COUNT_AND_PREFIX_QUALIFIER, true, false),
        }
    }
}
