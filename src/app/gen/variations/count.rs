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
use crate::app::parse::count::CountSequence;
use crate::app::parse::parser::ObjectParseError;
use crate::util::cursor::ReadCursor;
use crate::util::logging::*;

#[derive(Debug, PartialEq)]
pub enum CountVariation<'a> {
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

#[rustfmt::skip]
impl<'a> CountVariation<'a> {
    pub(crate) fn parse(v: Variation, count: u16, cursor: &mut ReadCursor<'a>) -> Result<CountVariation<'a>, ObjectParseError> {
        match v {
            Variation::Group50Var1 => Ok(CountVariation::Group50Var1(CountSequence::parse(count, cursor)?)),
            Variation::Group50Var3 => Ok(CountVariation::Group50Var3(CountSequence::parse(count, cursor)?)),
            Variation::Group50Var4 => Ok(CountVariation::Group50Var4(CountSequence::parse(count, cursor)?)),
            Variation::Group51Var1 => Ok(CountVariation::Group51Var1(CountSequence::parse(count, cursor)?)),
            Variation::Group51Var2 => Ok(CountVariation::Group51Var2(CountSequence::parse(count, cursor)?)),
            Variation::Group52Var1 => Ok(CountVariation::Group52Var1(CountSequence::parse(count, cursor)?)),
            Variation::Group52Var2 => Ok(CountVariation::Group52Var2(CountSequence::parse(count, cursor)?)),
            _ => Err(ObjectParseError::InvalidQualifierForVariation(v)),
        }
    }
    
    pub(crate) fn log_objects(&self, level : log::Level) {
        match self {
            CountVariation::Group50Var1(seq) => log_count_of_items(level, seq.iter()),
            CountVariation::Group50Var3(seq) => log_count_of_items(level, seq.iter()),
            CountVariation::Group50Var4(seq) => log_count_of_items(level, seq.iter()),
            CountVariation::Group51Var1(seq) => log_count_of_items(level, seq.iter()),
            CountVariation::Group51Var2(seq) => log_count_of_items(level, seq.iter()),
            CountVariation::Group52Var1(seq) => log_count_of_items(level, seq.iter()),
            CountVariation::Group52Var2(seq) => log_count_of_items(level, seq.iter()),
        }
    }
}
