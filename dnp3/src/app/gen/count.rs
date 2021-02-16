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
use crate::app::parse::count::CountSequence;
use crate::app::parse::parser::*;
use crate::util::cursor::ReadCursor;
use crate::app::ObjectParseError;

#[derive(Debug, PartialEq)]
pub(crate) enum CountVariation<'a> {
    /// Time and Date - Absolute Time
    Group50Var1(CountSequence<'a, Group50Var1>),
    /// Time and Date - Absolute Time at last recorded time
    Group50Var3(CountSequence<'a, Group50Var3>),
    /// Time and Date - Indexed absolute time and long interval
    Group50Var4(CountSequence<'a, Group50Var4>),
    /// Time and Date CTO - Absolute time, synchronized
    Group51Var1(CountSequence<'a, Group51Var1>),
    /// Time and Date CTO - Absolute time, unsynchronized
    Group51Var2(CountSequence<'a, Group51Var2>),
    /// Time Delay - Coarse
    Group52Var1(CountSequence<'a, Group52Var1>),
    /// Time Delay - Fine
    Group52Var2(CountSequence<'a, Group52Var2>),
}

impl<'a> CountVariation<'a> {
    pub(crate) fn parse(v: Variation, qualifier: QualifierCode, count: u16, cursor: &mut ReadCursor<'a>) -> Result<CountVariation<'a>, ObjectParseError> {
        match v {
            Variation::Group50Var1 => Ok(CountVariation::Group50Var1(CountSequence::parse(count, cursor)?)),
            Variation::Group50Var3 => Ok(CountVariation::Group50Var3(CountSequence::parse(count, cursor)?)),
            Variation::Group50Var4 => Ok(CountVariation::Group50Var4(CountSequence::parse(count, cursor)?)),
            Variation::Group51Var1 => Ok(CountVariation::Group51Var1(CountSequence::parse(count, cursor)?)),
            Variation::Group51Var2 => Ok(CountVariation::Group51Var2(CountSequence::parse(count, cursor)?)),
            Variation::Group52Var1 => Ok(CountVariation::Group52Var1(CountSequence::parse(count, cursor)?)),
            Variation::Group52Var2 => Ok(CountVariation::Group52Var2(CountSequence::parse(count, cursor)?)),
            _ => Err(ObjectParseError::InvalidQualifierForVariation(v, qualifier)),
        }
    }
    
    pub(crate) fn format_objects(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CountVariation::Group50Var1(seq) => format_count_of_items(f, seq.iter()),
            CountVariation::Group50Var3(seq) => format_count_of_items(f, seq.iter()),
            CountVariation::Group50Var4(seq) => format_count_of_items(f, seq.iter()),
            CountVariation::Group51Var1(seq) => format_count_of_items(f, seq.iter()),
            CountVariation::Group51Var2(seq) => format_count_of_items(f, seq.iter()),
            CountVariation::Group52Var1(seq) => format_count_of_items(f, seq.iter()),
            CountVariation::Group52Var2(seq) => format_count_of_items(f, seq.iter()),
        }
    }
}
