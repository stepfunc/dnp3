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
    /// Binary Input Event - Any Variation
    Group2Var0,
    /// Binary Input Event - Without Time
    Group2Var1,
    /// Binary Input Event - With Absolute Time
    Group2Var2,
    /// Binary Input Event - With Relative Time
    Group2Var3,
    /// Double-bit Binary Input Event - Any Variation
    Group4Var0,
    /// Double-bit Binary Input Event - Without Time
    Group4Var1,
    /// Double-bit Binary Input Event - With Absolute Time
    Group4Var2,
    /// Double-bit Binary Input Event - With Relative Time
    Group4Var3,
    /// Binary Output Event - Any Variation
    Group11Var0,
    /// Binary Output Event - Output Status Without Time
    Group11Var1,
    /// Binary Output Event - Output Status With Time
    Group11Var2,
    /// Counter Event - Any Variation
    Group22Var0,
    /// Counter Event - 32-bit With Flag
    Group22Var1,
    /// Counter Event - 16-bit With Flag
    Group22Var2,
    /// Counter Event - 32-bit With Flag and Time
    Group22Var5,
    /// Counter Event - 16-bit With Flag and Time
    Group22Var6,
    /// Frozen Counter Event - Any Variation
    Group23Var0,
    /// Frozen Counter Event - 32-bit With Flag
    Group23Var1,
    /// Frozen Counter Event - 16-bit With Flag
    Group23Var2,
    /// Frozen Counter Event - 32-bit With Flag and Time
    Group23Var5,
    /// Frozen Counter Event - 16-bit With Flag and Time
    Group23Var6,
    /// Analog Input Event - Any Variation
    Group32Var0,
    /// Analog Input Event - 32-bit With Flag
    Group32Var1,
    /// Analog Input Event - 16-bit With Flag
    Group32Var2,
    /// Analog Input Event - 32-bit With Flag and Time
    Group32Var3,
    /// Analog Input Event - 16-bit With Flag and Time
    Group32Var4,
    /// Analog Input Event - Single-precision With Flag
    Group32Var5,
    /// Analog Input Event - Double-precision With Flag
    Group32Var6,
    /// Analog Input Event - Single-precision With Flag and Time
    Group32Var7,
    /// Analog Input Event - Double-precision With Flag and Time
    Group32Var8,
    /// Analog Output Event - Any Variation
    Group42Var0,
    /// Analog Output Event - 32-bit With Flag
    Group42Var1,
    /// Analog Output Event - 16-bit With Flag
    Group42Var2,
    /// Analog Output Event - 32-bit With Flag and Time
    Group42Var3,
    /// Analog Output Event - 16-bit With Flag and Time
    Group42Var4,
    /// Analog Output Event - Single-precision With Flag
    Group42Var5,
    /// Analog Output Event - Double-precision With Flag
    Group42Var6,
    /// Analog Output Event - Single-precision With Flag and Time
    Group42Var7,
    /// Analog Output Event - Double-precision With Flag and Time
    Group42Var8,
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
    /// Class Data - Class 1
    Group60Var2,
    /// Class Data - Class 2
    Group60Var3,
    /// Class Data - Class 3
    Group60Var4,
    /// Octet String Event - Sized by variation
    Group111Var0,
    Group111VarX(u8),
}

impl<'a> CountVariation<'a> {
    pub(crate) fn parse(v: Variation, qualifier: QualifierCode, count: u16, cursor: &mut ReadCursor<'a>) -> Result<CountVariation<'a>, ObjectParseError> {
        match v {
            Variation::Group2Var0 => Ok(CountVariation::Group2Var0),
            Variation::Group2Var1 => Ok(CountVariation::Group2Var1),
            Variation::Group2Var2 => Ok(CountVariation::Group2Var2),
            Variation::Group2Var3 => Ok(CountVariation::Group2Var3),
            Variation::Group4Var0 => Ok(CountVariation::Group4Var0),
            Variation::Group4Var1 => Ok(CountVariation::Group4Var1),
            Variation::Group4Var2 => Ok(CountVariation::Group4Var2),
            Variation::Group4Var3 => Ok(CountVariation::Group4Var3),
            Variation::Group11Var0 => Ok(CountVariation::Group11Var0),
            Variation::Group11Var1 => Ok(CountVariation::Group11Var1),
            Variation::Group11Var2 => Ok(CountVariation::Group11Var2),
            Variation::Group22Var0 => Ok(CountVariation::Group22Var0),
            Variation::Group22Var1 => Ok(CountVariation::Group22Var1),
            Variation::Group22Var2 => Ok(CountVariation::Group22Var2),
            Variation::Group22Var5 => Ok(CountVariation::Group22Var5),
            Variation::Group22Var6 => Ok(CountVariation::Group22Var6),
            Variation::Group23Var0 => Ok(CountVariation::Group23Var0),
            Variation::Group23Var1 => Ok(CountVariation::Group23Var1),
            Variation::Group23Var2 => Ok(CountVariation::Group23Var2),
            Variation::Group23Var5 => Ok(CountVariation::Group23Var5),
            Variation::Group23Var6 => Ok(CountVariation::Group23Var6),
            Variation::Group32Var0 => Ok(CountVariation::Group32Var0),
            Variation::Group32Var1 => Ok(CountVariation::Group32Var1),
            Variation::Group32Var2 => Ok(CountVariation::Group32Var2),
            Variation::Group32Var3 => Ok(CountVariation::Group32Var3),
            Variation::Group32Var4 => Ok(CountVariation::Group32Var4),
            Variation::Group32Var5 => Ok(CountVariation::Group32Var5),
            Variation::Group32Var6 => Ok(CountVariation::Group32Var6),
            Variation::Group32Var7 => Ok(CountVariation::Group32Var7),
            Variation::Group32Var8 => Ok(CountVariation::Group32Var8),
            Variation::Group42Var0 => Ok(CountVariation::Group42Var0),
            Variation::Group42Var1 => Ok(CountVariation::Group42Var1),
            Variation::Group42Var2 => Ok(CountVariation::Group42Var2),
            Variation::Group42Var3 => Ok(CountVariation::Group42Var3),
            Variation::Group42Var4 => Ok(CountVariation::Group42Var4),
            Variation::Group42Var5 => Ok(CountVariation::Group42Var5),
            Variation::Group42Var6 => Ok(CountVariation::Group42Var6),
            Variation::Group42Var7 => Ok(CountVariation::Group42Var7),
            Variation::Group42Var8 => Ok(CountVariation::Group42Var8),
            Variation::Group50Var1 => Ok(CountVariation::Group50Var1(CountSequence::parse(count, cursor)?)),
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
            Variation::Group111(x) => Ok(CountVariation::Group111VarX(x)),
            _ => Err(ObjectParseError::InvalidQualifierForVariation(v, qualifier)),
        }
    }
    
    pub(crate) fn format_objects(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CountVariation::Group2Var0 => Ok(()),
            CountVariation::Group2Var1 => Ok(()),
            CountVariation::Group2Var2 => Ok(()),
            CountVariation::Group2Var3 => Ok(()),
            CountVariation::Group4Var0 => Ok(()),
            CountVariation::Group4Var1 => Ok(()),
            CountVariation::Group4Var2 => Ok(()),
            CountVariation::Group4Var3 => Ok(()),
            CountVariation::Group11Var0 => Ok(()),
            CountVariation::Group11Var1 => Ok(()),
            CountVariation::Group11Var2 => Ok(()),
            CountVariation::Group22Var0 => Ok(()),
            CountVariation::Group22Var1 => Ok(()),
            CountVariation::Group22Var2 => Ok(()),
            CountVariation::Group22Var5 => Ok(()),
            CountVariation::Group22Var6 => Ok(()),
            CountVariation::Group23Var0 => Ok(()),
            CountVariation::Group23Var1 => Ok(()),
            CountVariation::Group23Var2 => Ok(()),
            CountVariation::Group23Var5 => Ok(()),
            CountVariation::Group23Var6 => Ok(()),
            CountVariation::Group32Var0 => Ok(()),
            CountVariation::Group32Var1 => Ok(()),
            CountVariation::Group32Var2 => Ok(()),
            CountVariation::Group32Var3 => Ok(()),
            CountVariation::Group32Var4 => Ok(()),
            CountVariation::Group32Var5 => Ok(()),
            CountVariation::Group32Var6 => Ok(()),
            CountVariation::Group32Var7 => Ok(()),
            CountVariation::Group32Var8 => Ok(()),
            CountVariation::Group42Var0 => Ok(()),
            CountVariation::Group42Var1 => Ok(()),
            CountVariation::Group42Var2 => Ok(()),
            CountVariation::Group42Var3 => Ok(()),
            CountVariation::Group42Var4 => Ok(()),
            CountVariation::Group42Var5 => Ok(()),
            CountVariation::Group42Var6 => Ok(()),
            CountVariation::Group42Var7 => Ok(()),
            CountVariation::Group42Var8 => Ok(()),
            CountVariation::Group50Var1(seq) => format_count_of_items(f, seq.iter()),
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
            CountVariation::Group111VarX(_) => Ok(()),
        }
    }
}
