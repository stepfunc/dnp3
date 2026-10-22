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
use crate::app::ObjectParseError;

use scursor::ReadCursor;

#[derive(Debug, PartialEq)]
pub(crate) enum CountVariation<'a> {
    /// Binary Input Event - Any Variation
    Group2Var0,
    /// Double-bit Binary Input Event - Any Variation
    Group4Var0,
    /// Binary Output Event - Any Variation
    Group11Var0,
    /// Counter Event - Any Variation
    Group22Var0,
    /// Frozen Counter Event - Any Variation
    Group23Var0,
    /// Analog Input Event - Any Variation
    Group32Var0,
    /// Frozen Analog Input Event - Any Variation
    Group33Var0,
    /// Analog Output Event - Any Variation
    Group42Var0,
    /// Time and Date - Absolute Time
    Group50Var1(CountSequence<'a, Group50Var1>),
    /// Time and Date - Absolute time and interval
    Group50Var2(CountSequence<'a, Group50Var2>),
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
    /// Class Data - Class 1
    Group60Var2,
    /// Class Data - Class 2
    Group60Var3,
    /// Class Data - Class 3
    Group60Var4,
    /// Octet String Event - Sized by variation
    Group111Var0,
}

impl<'a> CountVariation<'a> {
    pub(crate) fn parse_non_read(v: Variation, qualifier: QualifierCode, count: u16, cursor: &mut ReadCursor<'a>) -> Result<CountVariation<'a>, ObjectParseError> {
        match v {
            Variation::Group2Var0 => Ok(CountVariation::Group2Var0),
            Variation::Group4Var0 => Ok(CountVariation::Group4Var0),
            Variation::Group11Var0 => Ok(CountVariation::Group11Var0),
            Variation::Group22Var0 => Ok(CountVariation::Group22Var0),
            Variation::Group23Var0 => Ok(CountVariation::Group23Var0),
            Variation::Group32Var0 => Ok(CountVariation::Group32Var0),
            Variation::Group33Var0 => Ok(CountVariation::Group33Var0),
            Variation::Group42Var0 => Ok(CountVariation::Group42Var0),
            Variation::Group50Var1 => Ok(CountVariation::Group50Var1(CountSequence::parse(count, cursor)?)),
            Variation::Group50Var2 => Ok(CountVariation::Group50Var2(CountSequence::parse(count, cursor)?)),
            Variation::Group50Var3 => Ok(CountVariation::Group50Var3(CountSequence::parse(count, cursor)?)),
            Variation::Group50Var4 => Ok(CountVariation::Group50Var4(CountSequence::parse(count, cursor)?)),
            Variation::Group51Var1 => Ok(CountVariation::Group51Var1(CountSequence::parse(count, cursor)?)),
            Variation::Group51Var2 => Ok(CountVariation::Group51Var2(CountSequence::parse(count, cursor)?)),
            Variation::Group52Var1 => Ok(CountVariation::Group52Var1(CountSequence::parse(count, cursor)?)),
            Variation::Group52Var2 => Ok(CountVariation::Group52Var2(CountSequence::parse(count, cursor)?)),
            Variation::Group60Var2 => Ok(CountVariation::Group60Var2),
            Variation::Group60Var3 => Ok(CountVariation::Group60Var3),
            Variation::Group60Var4 => Ok(CountVariation::Group60Var4),
            Variation::Group111(0) => Ok(CountVariation::Group111Var0),
            _ => Err(ObjectParseError::InvalidQualifierForVariation(v, qualifier)),
        }
    }
    
    pub(crate) fn variation(&self) -> Variation {
        match self {
            CountVariation::Group2Var0 => Variation::Group2Var0,
            CountVariation::Group4Var0 => Variation::Group4Var0,
            CountVariation::Group11Var0 => Variation::Group11Var0,
            CountVariation::Group22Var0 => Variation::Group22Var0,
            CountVariation::Group23Var0 => Variation::Group23Var0,
            CountVariation::Group32Var0 => Variation::Group32Var0,
            CountVariation::Group33Var0 => Variation::Group33Var0,
            CountVariation::Group42Var0 => Variation::Group42Var0,
            CountVariation::Group50Var1(_) => Variation::Group50Var1,
            CountVariation::Group50Var2(_) => Variation::Group50Var2,
            CountVariation::Group50Var3(_) => Variation::Group50Var3,
            CountVariation::Group50Var4(_) => Variation::Group50Var4,
            CountVariation::Group51Var1(_) => Variation::Group51Var1,
            CountVariation::Group51Var2(_) => Variation::Group51Var2,
            CountVariation::Group52Var1(_) => Variation::Group52Var1,
            CountVariation::Group52Var2(_) => Variation::Group52Var2,
            CountVariation::Group60Var2 => Variation::Group60Var2,
            CountVariation::Group60Var3 => Variation::Group60Var3,
            CountVariation::Group60Var4 => Variation::Group60Var4,
            CountVariation::Group111Var0 => Variation::Group111(0),
        }
    }
    
    pub(crate) fn format_objects(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CountVariation::Group2Var0 => Ok(()),
            CountVariation::Group4Var0 => Ok(()),
            CountVariation::Group11Var0 => Ok(()),
            CountVariation::Group22Var0 => Ok(()),
            CountVariation::Group23Var0 => Ok(()),
            CountVariation::Group32Var0 => Ok(()),
            CountVariation::Group33Var0 => Ok(()),
            CountVariation::Group42Var0 => Ok(()),
            CountVariation::Group50Var1(seq) => format_count_of_items(f, seq.iter()),
            CountVariation::Group50Var2(seq) => format_count_of_items(f, seq.iter()),
            CountVariation::Group50Var3(seq) => format_count_of_items(f, seq.iter()),
            CountVariation::Group50Var4(seq) => format_count_of_items(f, seq.iter()),
            CountVariation::Group51Var1(seq) => format_count_of_items(f, seq.iter()),
            CountVariation::Group51Var2(seq) => format_count_of_items(f, seq.iter()),
            CountVariation::Group52Var1(seq) => format_count_of_items(f, seq.iter()),
            CountVariation::Group52Var2(seq) => format_count_of_items(f, seq.iter()),
            CountVariation::Group60Var2 => Ok(()),
            CountVariation::Group60Var3 => Ok(()),
            CountVariation::Group60Var4 => Ok(()),
            CountVariation::Group111Var0 => Ok(()),
        }
    }
}
