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

use crate::app::parse::traits::{FixedSize, FixedSizeVariation};
use crate::app::control::{CommandStatus, ControlCode};
use crate::app::Timestamp;
use crate::app::measurement::*;

use scursor::*;

/// All variations supported by the library
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Variation {
    /// Device Attributes - Non-Specific All Attributes Request
    Group0Var254,
    /// Device Attributes - Specific Attribute
    Group0(u8),
    /// Binary Input - Any Variation
    Group1Var0,
    /// Binary Input - Packed Format
    Group1Var1,
    /// Binary Input - With Flags
    Group1Var2,
    /// Binary Input Event - Any Variation
    Group2Var0,
    /// Binary Input Event - Without Time
    Group2Var1,
    /// Binary Input Event - With Absolute Time
    Group2Var2,
    /// Binary Input Event - With Relative Time
    Group2Var3,
    /// Double-bit Binary Input - Any Variation
    Group3Var0,
    /// Double-bit Binary Input - Packed Format
    Group3Var1,
    /// Double-bit Binary Input - With Flags
    Group3Var2,
    /// Double-bit Binary Input Event - Any Variation
    Group4Var0,
    /// Double-bit Binary Input Event - Without Time
    Group4Var1,
    /// Double-bit Binary Input Event - With Absolute Time
    Group4Var2,
    /// Double-bit Binary Input Event - With Relative Time
    Group4Var3,
    /// Binary Output - Any Variation
    Group10Var0,
    /// Binary Output - Packed Format
    Group10Var1,
    /// Binary Output - Output Status With Flags
    Group10Var2,
    /// Binary Output Event - Any Variation
    Group11Var0,
    /// Binary Output Event - Output Status Without Time
    Group11Var1,
    /// Binary Output Event - Output Status With Time
    Group11Var2,
    /// Binary Command - Control Relay Output Block
    Group12Var1,
    /// Binary Output Command Event - Without Time
    Group13Var1,
    /// Binary Output Command Event - With Time
    Group13Var2,
    /// Counter - Any Variation
    Group20Var0,
    /// Counter - 32-bit With Flag
    Group20Var1,
    /// Counter - 16-bit With Flag
    Group20Var2,
    /// Counter - 32-bit Without Flag
    Group20Var5,
    /// Counter - 16-bit Without Flag
    Group20Var6,
    /// Frozen Counter - Any Variation
    Group21Var0,
    /// Frozen Counter - 32-bit With Flag
    Group21Var1,
    /// Frozen Counter - 16-bit With Flag
    Group21Var2,
    /// Frozen Counter - 32-bit With Flag and Time
    Group21Var5,
    /// Frozen Counter - 16-bit With Flag and Time
    Group21Var6,
    /// Frozen Counter - 32-bit Without Flag
    Group21Var9,
    /// Frozen Counter - 16-bit Without Flag
    Group21Var10,
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
    /// Analog Input - Any Variation
    Group30Var0,
    /// Analog Input - 32-bit With Flag
    Group30Var1,
    /// Analog Input - 16-bit With Flag
    Group30Var2,
    /// Analog Input - 32-bit Without Flag
    Group30Var3,
    /// Analog Input - 16-bit Without Flag
    Group30Var4,
    /// Analog Input - Single-precision With Flag
    Group30Var5,
    /// Analog Input - Double-precision With Flag
    Group30Var6,
    /// Frozen Analog Input - Any Variation
    Group31Var0,
    /// Frozen Analog Input - 32-bit With Flag
    Group31Var1,
    /// Frozen Analog Input - 16-bit With Flag
    Group31Var2,
    /// Frozen Analog Input - 32-bit with Flag and Time-of-Freeze
    Group31Var3,
    /// Frozen Analog Input - 16-bit with Flag and Time-of-Freeze
    Group31Var4,
    /// Frozen Analog Input - 32-bit Without Flag
    Group31Var5,
    /// Frozen Analog Input - 16-bit Without Flag
    Group31Var6,
    /// Frozen Analog Input - Single-precision With Flag
    Group31Var7,
    /// Frozen Analog Input - Double-precision With Flag
    Group31Var8,
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
    /// Frozen Analog Input Event - Any Variation
    Group33Var0,
    /// Frozen Analog Input Event - 32-bit With Flag
    Group33Var1,
    /// Frozen Analog Input Event - 16-bit With Flag
    Group33Var2,
    /// Frozen Analog Input Event - 32-bit with Flag and Time-of-Freeze
    Group33Var3,
    /// Frozen Analog Input Event - 16-bit with Flag and Time-of-Freeze
    Group33Var4,
    /// Frozen Analog Input Event - Single-precision With Flag
    Group33Var5,
    /// Frozen Analog Input Event - Double-precision With Flag
    Group33Var6,
    /// Frozen Analog Input Event - Single-precision With Flag and Time
    Group33Var7,
    /// Frozen Analog Input Event - Double-precision With Flag and Time
    Group33Var8,
    /// Analog Input Reporting Deadband - Any Variation
    Group34Var0,
    /// Analog Input Reporting Deadband - 16-bit
    Group34Var1,
    /// Analog Input Reporting Deadband - 32-bit
    Group34Var2,
    /// Analog Input Reporting Deadband - Single-precision
    Group34Var3,
    /// Analog Output Status - Any Variation
    Group40Var0,
    /// Analog Output Status - 32-bit With Flag
    Group40Var1,
    /// Analog Output Status - 16-bit With Flag
    Group40Var2,
    /// Analog Output Status - Single-precision With Flag
    Group40Var3,
    /// Analog Output Status - Double-precision With Flag
    Group40Var4,
    /// Analog Output - 32-bit With Flag
    Group41Var1,
    /// Analog Output - 16-bit With Flag
    Group41Var2,
    /// Analog Output - Single-precision
    Group41Var3,
    /// Analog Output - Double-precision
    Group41Var4,
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
    /// Analog Output Command Event - 32-bit
    Group43Var1,
    /// Analog Output Command Event - 16-bit
    Group43Var2,
    /// Analog Output Command Event - 32-bit With Time
    Group43Var3,
    /// Analog Output Command Event - 16-bit With Time
    Group43Var4,
    /// Analog Output Command Event - Single-precision
    Group43Var5,
    /// Analog Output Command Event - Double-precision
    Group43Var6,
    /// Analog Output Command Event - Single-precision With Time
    Group43Var7,
    /// Analog Output Command Event - Double-precision With Time
    Group43Var8,
    /// Time and Date - Absolute Time
    Group50Var1,
    /// Time and Date - Absolute time and interval
    Group50Var2,
    /// Time and Date - Absolute Time at last recorded time
    Group50Var3,
    /// Time and Date - Indexed absolute time and long interval
    Group50Var4,
    /// Time and Date CTO - Absolute time, synchronized
    Group51Var1,
    /// Time and Date CTO - Absolute time, unsynchronized
    Group51Var2,
    /// Time Delay - Coarse
    Group52Var1,
    /// Time Delay - Fine
    Group52Var2,
    /// Class Data - Class 0
    Group60Var1,
    /// Class Data - Class 1
    Group60Var2,
    /// Class Data - Class 2
    Group60Var3,
    /// Class Data - Class 3
    Group60Var4,
    /// File-control - authentication
    Group70Var2,
    /// File-control - file command
    Group70Var3,
    /// File-control - file command status
    Group70Var4,
    /// File-control - file transport
    Group70Var5,
    /// File-control - file transport status
    Group70Var6,
    /// File-control - file descriptor
    Group70Var7,
    /// File-control - file specification string
    Group70Var8,
    /// Internal Indications - Packed Format
    Group80Var1,
    /// Unsigned Integer - Any Variation
    Group102Var0,
    /// Unsigned Integer - 8-bit
    Group102Var1,
    /// Octet String - Sized by variation
    Group110(u8),
    /// Octet String Event - Sized by variation
    Group111(u8),
}

impl Variation {
    pub(crate) fn lookup(group: u8, var: u8) -> Option<Variation> {
        match group {
            0 => match var {
                0 => None,
                254 => Some(Variation::Group0Var254),
                _ => Some(Variation::Group0(var)),
            },
            1 => match var {
                0 => Some(Variation::Group1Var0),
                1 => Some(Variation::Group1Var1),
                2 => Some(Variation::Group1Var2),
                _ => None,
            },
            2 => match var {
                0 => Some(Variation::Group2Var0),
                1 => Some(Variation::Group2Var1),
                2 => Some(Variation::Group2Var2),
                3 => Some(Variation::Group2Var3),
                _ => None,
            },
            3 => match var {
                0 => Some(Variation::Group3Var0),
                1 => Some(Variation::Group3Var1),
                2 => Some(Variation::Group3Var2),
                _ => None,
            },
            4 => match var {
                0 => Some(Variation::Group4Var0),
                1 => Some(Variation::Group4Var1),
                2 => Some(Variation::Group4Var2),
                3 => Some(Variation::Group4Var3),
                _ => None,
            },
            10 => match var {
                0 => Some(Variation::Group10Var0),
                1 => Some(Variation::Group10Var1),
                2 => Some(Variation::Group10Var2),
                _ => None,
            },
            11 => match var {
                0 => Some(Variation::Group11Var0),
                1 => Some(Variation::Group11Var1),
                2 => Some(Variation::Group11Var2),
                _ => None,
            },
            12 => match var {
                1 => Some(Variation::Group12Var1),
                _ => None,
            },
            13 => match var {
                1 => Some(Variation::Group13Var1),
                2 => Some(Variation::Group13Var2),
                _ => None,
            },
            20 => match var {
                0 => Some(Variation::Group20Var0),
                1 => Some(Variation::Group20Var1),
                2 => Some(Variation::Group20Var2),
                5 => Some(Variation::Group20Var5),
                6 => Some(Variation::Group20Var6),
                _ => None,
            },
            21 => match var {
                0 => Some(Variation::Group21Var0),
                1 => Some(Variation::Group21Var1),
                2 => Some(Variation::Group21Var2),
                5 => Some(Variation::Group21Var5),
                6 => Some(Variation::Group21Var6),
                9 => Some(Variation::Group21Var9),
                10 => Some(Variation::Group21Var10),
                _ => None,
            },
            22 => match var {
                0 => Some(Variation::Group22Var0),
                1 => Some(Variation::Group22Var1),
                2 => Some(Variation::Group22Var2),
                5 => Some(Variation::Group22Var5),
                6 => Some(Variation::Group22Var6),
                _ => None,
            },
            23 => match var {
                0 => Some(Variation::Group23Var0),
                1 => Some(Variation::Group23Var1),
                2 => Some(Variation::Group23Var2),
                5 => Some(Variation::Group23Var5),
                6 => Some(Variation::Group23Var6),
                _ => None,
            },
            30 => match var {
                0 => Some(Variation::Group30Var0),
                1 => Some(Variation::Group30Var1),
                2 => Some(Variation::Group30Var2),
                3 => Some(Variation::Group30Var3),
                4 => Some(Variation::Group30Var4),
                5 => Some(Variation::Group30Var5),
                6 => Some(Variation::Group30Var6),
                _ => None,
            },
            31 => match var {
                0 => Some(Variation::Group31Var0),
                1 => Some(Variation::Group31Var1),
                2 => Some(Variation::Group31Var2),
                3 => Some(Variation::Group31Var3),
                4 => Some(Variation::Group31Var4),
                5 => Some(Variation::Group31Var5),
                6 => Some(Variation::Group31Var6),
                7 => Some(Variation::Group31Var7),
                8 => Some(Variation::Group31Var8),
                _ => None,
            },
            32 => match var {
                0 => Some(Variation::Group32Var0),
                1 => Some(Variation::Group32Var1),
                2 => Some(Variation::Group32Var2),
                3 => Some(Variation::Group32Var3),
                4 => Some(Variation::Group32Var4),
                5 => Some(Variation::Group32Var5),
                6 => Some(Variation::Group32Var6),
                7 => Some(Variation::Group32Var7),
                8 => Some(Variation::Group32Var8),
                _ => None,
            },
            33 => match var {
                0 => Some(Variation::Group33Var0),
                1 => Some(Variation::Group33Var1),
                2 => Some(Variation::Group33Var2),
                3 => Some(Variation::Group33Var3),
                4 => Some(Variation::Group33Var4),
                5 => Some(Variation::Group33Var5),
                6 => Some(Variation::Group33Var6),
                7 => Some(Variation::Group33Var7),
                8 => Some(Variation::Group33Var8),
                _ => None,
            },
            34 => match var {
                0 => Some(Variation::Group34Var0),
                1 => Some(Variation::Group34Var1),
                2 => Some(Variation::Group34Var2),
                3 => Some(Variation::Group34Var3),
                _ => None,
            },
            40 => match var {
                0 => Some(Variation::Group40Var0),
                1 => Some(Variation::Group40Var1),
                2 => Some(Variation::Group40Var2),
                3 => Some(Variation::Group40Var3),
                4 => Some(Variation::Group40Var4),
                _ => None,
            },
            41 => match var {
                1 => Some(Variation::Group41Var1),
                2 => Some(Variation::Group41Var2),
                3 => Some(Variation::Group41Var3),
                4 => Some(Variation::Group41Var4),
                _ => None,
            },
            42 => match var {
                0 => Some(Variation::Group42Var0),
                1 => Some(Variation::Group42Var1),
                2 => Some(Variation::Group42Var2),
                3 => Some(Variation::Group42Var3),
                4 => Some(Variation::Group42Var4),
                5 => Some(Variation::Group42Var5),
                6 => Some(Variation::Group42Var6),
                7 => Some(Variation::Group42Var7),
                8 => Some(Variation::Group42Var8),
                _ => None,
            },
            43 => match var {
                1 => Some(Variation::Group43Var1),
                2 => Some(Variation::Group43Var2),
                3 => Some(Variation::Group43Var3),
                4 => Some(Variation::Group43Var4),
                5 => Some(Variation::Group43Var5),
                6 => Some(Variation::Group43Var6),
                7 => Some(Variation::Group43Var7),
                8 => Some(Variation::Group43Var8),
                _ => None,
            },
            50 => match var {
                1 => Some(Variation::Group50Var1),
                2 => Some(Variation::Group50Var2),
                3 => Some(Variation::Group50Var3),
                4 => Some(Variation::Group50Var4),
                _ => None,
            },
            51 => match var {
                1 => Some(Variation::Group51Var1),
                2 => Some(Variation::Group51Var2),
                _ => None,
            },
            52 => match var {
                1 => Some(Variation::Group52Var1),
                2 => Some(Variation::Group52Var2),
                _ => None,
            },
            60 => match var {
                1 => Some(Variation::Group60Var1),
                2 => Some(Variation::Group60Var2),
                3 => Some(Variation::Group60Var3),
                4 => Some(Variation::Group60Var4),
                _ => None,
            },
            70 => match var {
                2 => Some(Variation::Group70Var2),
                3 => Some(Variation::Group70Var3),
                4 => Some(Variation::Group70Var4),
                5 => Some(Variation::Group70Var5),
                6 => Some(Variation::Group70Var6),
                7 => Some(Variation::Group70Var7),
                8 => Some(Variation::Group70Var8),
                _ => None,
            },
            80 => match var {
                1 => Some(Variation::Group80Var1),
                _ => None,
            },
            102 => match var {
                0 => Some(Variation::Group102Var0),
                1 => Some(Variation::Group102Var1),
                _ => None,
            },
            110 => Some(Variation::Group110(var)),
            111 => Some(Variation::Group111(var)),
            _ => None,
        }
    }
    
    pub(crate) fn to_group_and_var(self) -> (u8, u8) {
        match self {
            Variation::Group0Var254 => (0, 254),
            Variation::Group0(x) => (0, x),
            Variation::Group1Var0 => (1, 0),
            Variation::Group1Var1 => (1, 1),
            Variation::Group1Var2 => (1, 2),
            Variation::Group2Var0 => (2, 0),
            Variation::Group2Var1 => (2, 1),
            Variation::Group2Var2 => (2, 2),
            Variation::Group2Var3 => (2, 3),
            Variation::Group3Var0 => (3, 0),
            Variation::Group3Var1 => (3, 1),
            Variation::Group3Var2 => (3, 2),
            Variation::Group4Var0 => (4, 0),
            Variation::Group4Var1 => (4, 1),
            Variation::Group4Var2 => (4, 2),
            Variation::Group4Var3 => (4, 3),
            Variation::Group10Var0 => (10, 0),
            Variation::Group10Var1 => (10, 1),
            Variation::Group10Var2 => (10, 2),
            Variation::Group11Var0 => (11, 0),
            Variation::Group11Var1 => (11, 1),
            Variation::Group11Var2 => (11, 2),
            Variation::Group12Var1 => (12, 1),
            Variation::Group13Var1 => (13, 1),
            Variation::Group13Var2 => (13, 2),
            Variation::Group20Var0 => (20, 0),
            Variation::Group20Var1 => (20, 1),
            Variation::Group20Var2 => (20, 2),
            Variation::Group20Var5 => (20, 5),
            Variation::Group20Var6 => (20, 6),
            Variation::Group21Var0 => (21, 0),
            Variation::Group21Var1 => (21, 1),
            Variation::Group21Var2 => (21, 2),
            Variation::Group21Var5 => (21, 5),
            Variation::Group21Var6 => (21, 6),
            Variation::Group21Var9 => (21, 9),
            Variation::Group21Var10 => (21, 10),
            Variation::Group22Var0 => (22, 0),
            Variation::Group22Var1 => (22, 1),
            Variation::Group22Var2 => (22, 2),
            Variation::Group22Var5 => (22, 5),
            Variation::Group22Var6 => (22, 6),
            Variation::Group23Var0 => (23, 0),
            Variation::Group23Var1 => (23, 1),
            Variation::Group23Var2 => (23, 2),
            Variation::Group23Var5 => (23, 5),
            Variation::Group23Var6 => (23, 6),
            Variation::Group30Var0 => (30, 0),
            Variation::Group30Var1 => (30, 1),
            Variation::Group30Var2 => (30, 2),
            Variation::Group30Var3 => (30, 3),
            Variation::Group30Var4 => (30, 4),
            Variation::Group30Var5 => (30, 5),
            Variation::Group30Var6 => (30, 6),
            Variation::Group31Var0 => (31, 0),
            Variation::Group31Var1 => (31, 1),
            Variation::Group31Var2 => (31, 2),
            Variation::Group31Var3 => (31, 3),
            Variation::Group31Var4 => (31, 4),
            Variation::Group31Var5 => (31, 5),
            Variation::Group31Var6 => (31, 6),
            Variation::Group31Var7 => (31, 7),
            Variation::Group31Var8 => (31, 8),
            Variation::Group32Var0 => (32, 0),
            Variation::Group32Var1 => (32, 1),
            Variation::Group32Var2 => (32, 2),
            Variation::Group32Var3 => (32, 3),
            Variation::Group32Var4 => (32, 4),
            Variation::Group32Var5 => (32, 5),
            Variation::Group32Var6 => (32, 6),
            Variation::Group32Var7 => (32, 7),
            Variation::Group32Var8 => (32, 8),
            Variation::Group33Var0 => (33, 0),
            Variation::Group33Var1 => (33, 1),
            Variation::Group33Var2 => (33, 2),
            Variation::Group33Var3 => (33, 3),
            Variation::Group33Var4 => (33, 4),
            Variation::Group33Var5 => (33, 5),
            Variation::Group33Var6 => (33, 6),
            Variation::Group33Var7 => (33, 7),
            Variation::Group33Var8 => (33, 8),
            Variation::Group34Var0 => (34, 0),
            Variation::Group34Var1 => (34, 1),
            Variation::Group34Var2 => (34, 2),
            Variation::Group34Var3 => (34, 3),
            Variation::Group40Var0 => (40, 0),
            Variation::Group40Var1 => (40, 1),
            Variation::Group40Var2 => (40, 2),
            Variation::Group40Var3 => (40, 3),
            Variation::Group40Var4 => (40, 4),
            Variation::Group41Var1 => (41, 1),
            Variation::Group41Var2 => (41, 2),
            Variation::Group41Var3 => (41, 3),
            Variation::Group41Var4 => (41, 4),
            Variation::Group42Var0 => (42, 0),
            Variation::Group42Var1 => (42, 1),
            Variation::Group42Var2 => (42, 2),
            Variation::Group42Var3 => (42, 3),
            Variation::Group42Var4 => (42, 4),
            Variation::Group42Var5 => (42, 5),
            Variation::Group42Var6 => (42, 6),
            Variation::Group42Var7 => (42, 7),
            Variation::Group42Var8 => (42, 8),
            Variation::Group43Var1 => (43, 1),
            Variation::Group43Var2 => (43, 2),
            Variation::Group43Var3 => (43, 3),
            Variation::Group43Var4 => (43, 4),
            Variation::Group43Var5 => (43, 5),
            Variation::Group43Var6 => (43, 6),
            Variation::Group43Var7 => (43, 7),
            Variation::Group43Var8 => (43, 8),
            Variation::Group50Var1 => (50, 1),
            Variation::Group50Var2 => (50, 2),
            Variation::Group50Var3 => (50, 3),
            Variation::Group50Var4 => (50, 4),
            Variation::Group51Var1 => (51, 1),
            Variation::Group51Var2 => (51, 2),
            Variation::Group52Var1 => (52, 1),
            Variation::Group52Var2 => (52, 2),
            Variation::Group60Var1 => (60, 1),
            Variation::Group60Var2 => (60, 2),
            Variation::Group60Var3 => (60, 3),
            Variation::Group60Var4 => (60, 4),
            Variation::Group70Var2 => (70, 2),
            Variation::Group70Var3 => (70, 3),
            Variation::Group70Var4 => (70, 4),
            Variation::Group70Var5 => (70, 5),
            Variation::Group70Var6 => (70, 6),
            Variation::Group70Var7 => (70, 7),
            Variation::Group70Var8 => (70, 8),
            Variation::Group80Var1 => (80, 1),
            Variation::Group102Var0 => (102, 0),
            Variation::Group102Var1 => (102, 1),
            Variation::Group110(x) => (110, x),
            Variation::Group111(x) => (111, x),
        }
    }
    
    pub(crate) fn description(self) -> &'static str {
        match self {
            Variation::Group0Var254 => "Device Attributes - Non-Specific All Attributes Request",
            Variation::Group0(_) => "Device Attributes - Specific Attribute",
            Variation::Group1Var0 => "Binary Input - Any Variation",
            Variation::Group1Var1 => "Binary Input - Packed Format",
            Variation::Group1Var2 => "Binary Input - With Flags",
            Variation::Group2Var0 => "Binary Input Event - Any Variation",
            Variation::Group2Var1 => "Binary Input Event - Without Time",
            Variation::Group2Var2 => "Binary Input Event - With Absolute Time",
            Variation::Group2Var3 => "Binary Input Event - With Relative Time",
            Variation::Group3Var0 => "Double-bit Binary Input - Any Variation",
            Variation::Group3Var1 => "Double-bit Binary Input - Packed Format",
            Variation::Group3Var2 => "Double-bit Binary Input - With Flags",
            Variation::Group4Var0 => "Double-bit Binary Input Event - Any Variation",
            Variation::Group4Var1 => "Double-bit Binary Input Event - Without Time",
            Variation::Group4Var2 => "Double-bit Binary Input Event - With Absolute Time",
            Variation::Group4Var3 => "Double-bit Binary Input Event - With Relative Time",
            Variation::Group10Var0 => "Binary Output - Any Variation",
            Variation::Group10Var1 => "Binary Output - Packed Format",
            Variation::Group10Var2 => "Binary Output - Output Status With Flags",
            Variation::Group11Var0 => "Binary Output Event - Any Variation",
            Variation::Group11Var1 => "Binary Output Event - Output Status Without Time",
            Variation::Group11Var2 => "Binary Output Event - Output Status With Time",
            Variation::Group12Var1 => "Binary Command - Control Relay Output Block",
            Variation::Group13Var1 => "Binary Output Command Event - Without Time",
            Variation::Group13Var2 => "Binary Output Command Event - With Time",
            Variation::Group20Var0 => "Counter - Any Variation",
            Variation::Group20Var1 => "Counter - 32-bit With Flag",
            Variation::Group20Var2 => "Counter - 16-bit With Flag",
            Variation::Group20Var5 => "Counter - 32-bit Without Flag",
            Variation::Group20Var6 => "Counter - 16-bit Without Flag",
            Variation::Group21Var0 => "Frozen Counter - Any Variation",
            Variation::Group21Var1 => "Frozen Counter - 32-bit With Flag",
            Variation::Group21Var2 => "Frozen Counter - 16-bit With Flag",
            Variation::Group21Var5 => "Frozen Counter - 32-bit With Flag and Time",
            Variation::Group21Var6 => "Frozen Counter - 16-bit With Flag and Time",
            Variation::Group21Var9 => "Frozen Counter - 32-bit Without Flag",
            Variation::Group21Var10 => "Frozen Counter - 16-bit Without Flag",
            Variation::Group22Var0 => "Counter Event - Any Variation",
            Variation::Group22Var1 => "Counter Event - 32-bit With Flag",
            Variation::Group22Var2 => "Counter Event - 16-bit With Flag",
            Variation::Group22Var5 => "Counter Event - 32-bit With Flag and Time",
            Variation::Group22Var6 => "Counter Event - 16-bit With Flag and Time",
            Variation::Group23Var0 => "Frozen Counter Event - Any Variation",
            Variation::Group23Var1 => "Frozen Counter Event - 32-bit With Flag",
            Variation::Group23Var2 => "Frozen Counter Event - 16-bit With Flag",
            Variation::Group23Var5 => "Frozen Counter Event - 32-bit With Flag and Time",
            Variation::Group23Var6 => "Frozen Counter Event - 16-bit With Flag and Time",
            Variation::Group30Var0 => "Analog Input - Any Variation",
            Variation::Group30Var1 => "Analog Input - 32-bit With Flag",
            Variation::Group30Var2 => "Analog Input - 16-bit With Flag",
            Variation::Group30Var3 => "Analog Input - 32-bit Without Flag",
            Variation::Group30Var4 => "Analog Input - 16-bit Without Flag",
            Variation::Group30Var5 => "Analog Input - Single-precision With Flag",
            Variation::Group30Var6 => "Analog Input - Double-precision With Flag",
            Variation::Group31Var0 => "Frozen Analog Input - Any Variation",
            Variation::Group31Var1 => "Frozen Analog Input - 32-bit With Flag",
            Variation::Group31Var2 => "Frozen Analog Input - 16-bit With Flag",
            Variation::Group31Var3 => "Frozen Analog Input - 32-bit with Flag and Time-of-Freeze",
            Variation::Group31Var4 => "Frozen Analog Input - 16-bit with Flag and Time-of-Freeze",
            Variation::Group31Var5 => "Frozen Analog Input - 32-bit Without Flag",
            Variation::Group31Var6 => "Frozen Analog Input - 16-bit Without Flag",
            Variation::Group31Var7 => "Frozen Analog Input - Single-precision With Flag",
            Variation::Group31Var8 => "Frozen Analog Input - Double-precision With Flag",
            Variation::Group32Var0 => "Analog Input Event - Any Variation",
            Variation::Group32Var1 => "Analog Input Event - 32-bit With Flag",
            Variation::Group32Var2 => "Analog Input Event - 16-bit With Flag",
            Variation::Group32Var3 => "Analog Input Event - 32-bit With Flag and Time",
            Variation::Group32Var4 => "Analog Input Event - 16-bit With Flag and Time",
            Variation::Group32Var5 => "Analog Input Event - Single-precision With Flag",
            Variation::Group32Var6 => "Analog Input Event - Double-precision With Flag",
            Variation::Group32Var7 => "Analog Input Event - Single-precision With Flag and Time",
            Variation::Group32Var8 => "Analog Input Event - Double-precision With Flag and Time",
            Variation::Group33Var0 => "Frozen Analog Input Event - Any Variation",
            Variation::Group33Var1 => "Frozen Analog Input Event - 32-bit With Flag",
            Variation::Group33Var2 => "Frozen Analog Input Event - 16-bit With Flag",
            Variation::Group33Var3 => "Frozen Analog Input Event - 32-bit with Flag and Time-of-Freeze",
            Variation::Group33Var4 => "Frozen Analog Input Event - 16-bit with Flag and Time-of-Freeze",
            Variation::Group33Var5 => "Frozen Analog Input Event - Single-precision With Flag",
            Variation::Group33Var6 => "Frozen Analog Input Event - Double-precision With Flag",
            Variation::Group33Var7 => "Frozen Analog Input Event - Single-precision With Flag and Time",
            Variation::Group33Var8 => "Frozen Analog Input Event - Double-precision With Flag and Time",
            Variation::Group34Var0 => "Analog Input Reporting Deadband - Any Variation",
            Variation::Group34Var1 => "Analog Input Reporting Deadband - 16-bit",
            Variation::Group34Var2 => "Analog Input Reporting Deadband - 32-bit",
            Variation::Group34Var3 => "Analog Input Reporting Deadband - Single-precision",
            Variation::Group40Var0 => "Analog Output Status - Any Variation",
            Variation::Group40Var1 => "Analog Output Status - 32-bit With Flag",
            Variation::Group40Var2 => "Analog Output Status - 16-bit With Flag",
            Variation::Group40Var3 => "Analog Output Status - Single-precision With Flag",
            Variation::Group40Var4 => "Analog Output Status - Double-precision With Flag",
            Variation::Group41Var1 => "Analog Output - 32-bit With Flag",
            Variation::Group41Var2 => "Analog Output - 16-bit With Flag",
            Variation::Group41Var3 => "Analog Output - Single-precision",
            Variation::Group41Var4 => "Analog Output - Double-precision",
            Variation::Group42Var0 => "Analog Output Event - Any Variation",
            Variation::Group42Var1 => "Analog Output Event - 32-bit With Flag",
            Variation::Group42Var2 => "Analog Output Event - 16-bit With Flag",
            Variation::Group42Var3 => "Analog Output Event - 32-bit With Flag and Time",
            Variation::Group42Var4 => "Analog Output Event - 16-bit With Flag and Time",
            Variation::Group42Var5 => "Analog Output Event - Single-precision With Flag",
            Variation::Group42Var6 => "Analog Output Event - Double-precision With Flag",
            Variation::Group42Var7 => "Analog Output Event - Single-precision With Flag and Time",
            Variation::Group42Var8 => "Analog Output Event - Double-precision With Flag and Time",
            Variation::Group43Var1 => "Analog Output Command Event - 32-bit",
            Variation::Group43Var2 => "Analog Output Command Event - 16-bit",
            Variation::Group43Var3 => "Analog Output Command Event - 32-bit With Time",
            Variation::Group43Var4 => "Analog Output Command Event - 16-bit With Time",
            Variation::Group43Var5 => "Analog Output Command Event - Single-precision",
            Variation::Group43Var6 => "Analog Output Command Event - Double-precision",
            Variation::Group43Var7 => "Analog Output Command Event - Single-precision With Time",
            Variation::Group43Var8 => "Analog Output Command Event - Double-precision With Time",
            Variation::Group50Var1 => "Time and Date - Absolute Time",
            Variation::Group50Var2 => "Time and Date - Absolute time and interval",
            Variation::Group50Var3 => "Time and Date - Absolute Time at last recorded time",
            Variation::Group50Var4 => "Time and Date - Indexed absolute time and long interval",
            Variation::Group51Var1 => "Time and Date CTO - Absolute time, synchronized",
            Variation::Group51Var2 => "Time and Date CTO - Absolute time, unsynchronized",
            Variation::Group52Var1 => "Time Delay - Coarse",
            Variation::Group52Var2 => "Time Delay - Fine",
            Variation::Group60Var1 => "Class Data - Class 0",
            Variation::Group60Var2 => "Class Data - Class 1",
            Variation::Group60Var3 => "Class Data - Class 2",
            Variation::Group60Var4 => "Class Data - Class 3",
            Variation::Group70Var2 => "File-control - authentication",
            Variation::Group70Var3 => "File-control - file command",
            Variation::Group70Var4 => "File-control - file command status",
            Variation::Group70Var5 => "File-control - file transport",
            Variation::Group70Var6 => "File-control - file transport status",
            Variation::Group70Var7 => "File-control - file descriptor",
            Variation::Group70Var8 => "File-control - file specification string",
            Variation::Group80Var1 => "Internal Indications - Packed Format",
            Variation::Group102Var0 => "Unsigned Integer - Any Variation",
            Variation::Group102Var1 => "Unsigned Integer - 8-bit",
            Variation::Group110(_) => "Octet String - Sized by variation",
            Variation::Group111(_) => "Octet String Event - Sized by variation",
        }
    }
}

/// Unsigned Integer - 8-bit
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group102Var1 {
    /// value field of the variation
    pub(crate) value: u8,
}

/// Time Delay - Fine
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group52Var2 {
    /// time field of the variation
    pub(crate) time: u16,
}

/// Time Delay - Coarse
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group52Var1 {
    /// time field of the variation
    pub(crate) time: u16,
}

/// Time and Date CTO - Absolute time, unsynchronized
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group51Var2 {
    /// time field of the variation
    pub(crate) time: Timestamp,
}

/// Time and Date CTO - Absolute time, synchronized
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group51Var1 {
    /// time field of the variation
    pub(crate) time: Timestamp,
}

/// Time and Date - Indexed absolute time and long interval
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group50Var4 {
    /// time field of the variation
    pub(crate) time: Timestamp,
    /// interval field of the variation
    pub(crate) interval: u32,
    /// units field of the variation
    pub(crate) units: u8,
}

/// Time and Date - Absolute Time at last recorded time
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group50Var3 {
    /// time field of the variation
    pub(crate) time: Timestamp,
}

/// Time and Date - Absolute time and interval
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group50Var2 {
    /// time field of the variation
    pub(crate) time: Timestamp,
    /// interval field of the variation
    pub(crate) interval: u32,
}

/// Time and Date - Absolute Time
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group50Var1 {
    /// time field of the variation
    pub(crate) time: Timestamp,
}

/// Analog Output Command Event - Double-precision With Time
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Group43Var8 {
    /// status field of the variation
    pub(crate) status: CommandStatus,
    /// value field of the variation
    pub(crate) value: f64,
    /// time field of the variation
    pub(crate) time: Timestamp,
}

/// Analog Output Command Event - Single-precision With Time
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Group43Var7 {
    /// status field of the variation
    pub(crate) status: CommandStatus,
    /// value field of the variation
    pub(crate) value: f32,
    /// time field of the variation
    pub(crate) time: Timestamp,
}

/// Analog Output Command Event - Double-precision
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Group43Var6 {
    /// status field of the variation
    pub(crate) status: CommandStatus,
    /// value field of the variation
    pub(crate) value: f64,
}

/// Analog Output Command Event - Single-precision
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Group43Var5 {
    /// status field of the variation
    pub(crate) status: CommandStatus,
    /// value field of the variation
    pub(crate) value: f32,
}

/// Analog Output Command Event - 16-bit With Time
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group43Var4 {
    /// status field of the variation
    pub(crate) status: CommandStatus,
    /// value field of the variation
    pub(crate) value: i16,
    /// time field of the variation
    pub(crate) time: Timestamp,
}

/// Analog Output Command Event - 32-bit With Time
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group43Var3 {
    /// status field of the variation
    pub(crate) status: CommandStatus,
    /// value field of the variation
    pub(crate) value: i32,
    /// time field of the variation
    pub(crate) time: Timestamp,
}

/// Analog Output Command Event - 16-bit
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group43Var2 {
    /// status field of the variation
    pub(crate) status: CommandStatus,
    /// value field of the variation
    pub(crate) value: i16,
}

/// Analog Output Command Event - 32-bit
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group43Var1 {
    /// status field of the variation
    pub(crate) status: CommandStatus,
    /// value field of the variation
    pub(crate) value: i32,
}

/// Analog Output Event - Double-precision With Flag and Time
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Group42Var8 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: f64,
    /// time field of the variation
    pub(crate) time: Timestamp,
}

/// Analog Output Event - Single-precision With Flag and Time
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Group42Var7 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: f32,
    /// time field of the variation
    pub(crate) time: Timestamp,
}

/// Analog Output Event - Double-precision With Flag
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Group42Var6 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: f64,
}

/// Analog Output Event - Single-precision With Flag
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Group42Var5 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: f32,
}

/// Analog Output Event - 16-bit With Flag and Time
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group42Var4 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: i16,
    /// time field of the variation
    pub(crate) time: Timestamp,
}

/// Analog Output Event - 32-bit With Flag and Time
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group42Var3 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: i32,
    /// time field of the variation
    pub(crate) time: Timestamp,
}

/// Analog Output Event - 16-bit With Flag
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group42Var2 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: i16,
}

/// Analog Output Event - 32-bit With Flag
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group42Var1 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: i32,
}

/// Analog Output - Double-precision
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Group41Var4 {
    /// value field of the variation
    pub value: f64,
    /// status field of the variation
    pub status: CommandStatus,
}

/// Analog Output - Single-precision
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Group41Var3 {
    /// value field of the variation
    pub value: f32,
    /// status field of the variation
    pub status: CommandStatus,
}

/// Analog Output - 16-bit With Flag
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Group41Var2 {
    /// value field of the variation
    pub value: i16,
    /// status field of the variation
    pub status: CommandStatus,
}

/// Analog Output - 32-bit With Flag
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Group41Var1 {
    /// value field of the variation
    pub value: i32,
    /// status field of the variation
    pub status: CommandStatus,
}

/// Analog Output Status - Double-precision With Flag
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Group40Var4 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: f64,
}

/// Analog Output Status - Single-precision With Flag
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Group40Var3 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: f32,
}

/// Analog Output Status - 16-bit With Flag
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group40Var2 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: i16,
}

/// Analog Output Status - 32-bit With Flag
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group40Var1 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: i32,
}

/// Analog Input Reporting Deadband - Single-precision
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Group34Var3 {
    /// value field of the variation
    pub(crate) value: f32,
}

/// Analog Input Reporting Deadband - 32-bit
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group34Var2 {
    /// value field of the variation
    pub(crate) value: u32,
}

/// Analog Input Reporting Deadband - 16-bit
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group34Var1 {
    /// value field of the variation
    pub(crate) value: u16,
}

/// Frozen Analog Input Event - Double-precision With Flag and Time
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Group33Var8 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: f64,
    /// time field of the variation
    pub(crate) time: Timestamp,
}

/// Frozen Analog Input Event - Single-precision With Flag and Time
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Group33Var7 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: f32,
    /// time field of the variation
    pub(crate) time: Timestamp,
}

/// Frozen Analog Input Event - Double-precision With Flag
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Group33Var6 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: f64,
}

/// Frozen Analog Input Event - Single-precision With Flag
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Group33Var5 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: f32,
}

/// Frozen Analog Input Event - 16-bit with Flag and Time-of-Freeze
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group33Var4 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: i16,
    /// time field of the variation
    pub(crate) time: Timestamp,
}

/// Frozen Analog Input Event - 32-bit with Flag and Time-of-Freeze
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group33Var3 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: i32,
    /// time field of the variation
    pub(crate) time: Timestamp,
}

/// Frozen Analog Input Event - 16-bit With Flag
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group33Var2 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: i16,
}

/// Frozen Analog Input Event - 32-bit With Flag
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group33Var1 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: i32,
}

/// Analog Input Event - Double-precision With Flag and Time
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Group32Var8 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: f64,
    /// time field of the variation
    pub(crate) time: Timestamp,
}

/// Analog Input Event - Single-precision With Flag and Time
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Group32Var7 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: f32,
    /// time field of the variation
    pub(crate) time: Timestamp,
}

/// Analog Input Event - Double-precision With Flag
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Group32Var6 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: f64,
}

/// Analog Input Event - Single-precision With Flag
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Group32Var5 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: f32,
}

/// Analog Input Event - 16-bit With Flag and Time
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group32Var4 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: i16,
    /// time field of the variation
    pub(crate) time: Timestamp,
}

/// Analog Input Event - 32-bit With Flag and Time
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group32Var3 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: i32,
    /// time field of the variation
    pub(crate) time: Timestamp,
}

/// Analog Input Event - 16-bit With Flag
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group32Var2 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: i16,
}

/// Analog Input Event - 32-bit With Flag
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group32Var1 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: i32,
}

/// Frozen Analog Input - Double-precision With Flag
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Group31Var8 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: f64,
}

/// Frozen Analog Input - Single-precision With Flag
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Group31Var7 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: f32,
}

/// Frozen Analog Input - 16-bit Without Flag
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group31Var6 {
    /// value field of the variation
    pub(crate) value: i16,
}

/// Frozen Analog Input - 32-bit Without Flag
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group31Var5 {
    /// value field of the variation
    pub(crate) value: i32,
}

/// Frozen Analog Input - 16-bit with Flag and Time-of-Freeze
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group31Var4 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: i16,
    /// time field of the variation
    pub(crate) time: Timestamp,
}

/// Frozen Analog Input - 32-bit with Flag and Time-of-Freeze
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group31Var3 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: i32,
    /// time field of the variation
    pub(crate) time: Timestamp,
}

/// Frozen Analog Input - 16-bit With Flag
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group31Var2 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: i16,
}

/// Frozen Analog Input - 32-bit With Flag
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group31Var1 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: i32,
}

/// Analog Input - Double-precision With Flag
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Group30Var6 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: f64,
}

/// Analog Input - Single-precision With Flag
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Group30Var5 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: f32,
}

/// Analog Input - 16-bit Without Flag
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group30Var4 {
    /// value field of the variation
    pub(crate) value: i16,
}

/// Analog Input - 32-bit Without Flag
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group30Var3 {
    /// value field of the variation
    pub(crate) value: i32,
}

/// Analog Input - 16-bit With Flag
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group30Var2 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: i16,
}

/// Analog Input - 32-bit With Flag
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group30Var1 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: i32,
}

/// Frozen Counter Event - 16-bit With Flag and Time
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group23Var6 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: u16,
    /// time field of the variation
    pub(crate) time: Timestamp,
}

/// Frozen Counter Event - 32-bit With Flag and Time
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group23Var5 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: u32,
    /// time field of the variation
    pub(crate) time: Timestamp,
}

/// Frozen Counter Event - 16-bit With Flag
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group23Var2 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: u16,
}

/// Frozen Counter Event - 32-bit With Flag
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group23Var1 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: u32,
}

/// Counter Event - 16-bit With Flag and Time
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group22Var6 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: u16,
    /// time field of the variation
    pub(crate) time: Timestamp,
}

/// Counter Event - 32-bit With Flag and Time
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group22Var5 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: u32,
    /// time field of the variation
    pub(crate) time: Timestamp,
}

/// Counter Event - 16-bit With Flag
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group22Var2 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: u16,
}

/// Counter Event - 32-bit With Flag
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group22Var1 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: u32,
}

/// Frozen Counter - 16-bit Without Flag
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group21Var10 {
    /// value field of the variation
    pub(crate) value: u16,
}

/// Frozen Counter - 32-bit Without Flag
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group21Var9 {
    /// value field of the variation
    pub(crate) value: u32,
}

/// Frozen Counter - 16-bit With Flag and Time
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group21Var6 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: u16,
    /// time field of the variation
    pub(crate) time: Timestamp,
}

/// Frozen Counter - 32-bit With Flag and Time
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group21Var5 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: u32,
    /// time field of the variation
    pub(crate) time: Timestamp,
}

/// Frozen Counter - 16-bit With Flag
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group21Var2 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: u16,
}

/// Frozen Counter - 32-bit With Flag
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group21Var1 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: u32,
}

/// Counter - 16-bit Without Flag
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group20Var6 {
    /// value field of the variation
    pub(crate) value: u16,
}

/// Counter - 32-bit Without Flag
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group20Var5 {
    /// value field of the variation
    pub(crate) value: u32,
}

/// Counter - 16-bit With Flag
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group20Var2 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: u16,
}

/// Counter - 32-bit With Flag
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group20Var1 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// value field of the variation
    pub(crate) value: u32,
}

/// Binary Output Command Event - With Time
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group13Var2 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// time field of the variation
    pub(crate) time: Timestamp,
}

/// Binary Output Command Event - Without Time
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group13Var1 {
    /// flags field of the variation
    pub(crate) flags: u8,
}

/// Binary Command - Control Relay Output Block
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Group12Var1 {
    /// code field of the variation
    pub code: ControlCode,
    /// count field of the variation
    pub count: u8,
    /// on_time field of the variation
    pub on_time: u32,
    /// off_time field of the variation
    pub off_time: u32,
    /// status field of the variation
    pub status: CommandStatus,
}

/// Binary Output Event - Output Status With Time
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group11Var2 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// time field of the variation
    pub(crate) time: Timestamp,
}

/// Binary Output Event - Output Status Without Time
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group11Var1 {
    /// flags field of the variation
    pub(crate) flags: u8,
}

/// Binary Output - Output Status With Flags
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group10Var2 {
    /// flags field of the variation
    pub(crate) flags: u8,
}

/// Double-bit Binary Input Event - With Relative Time
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group4Var3 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// time field of the variation
    pub(crate) time: u16,
}

/// Double-bit Binary Input Event - With Absolute Time
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group4Var2 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// time field of the variation
    pub(crate) time: Timestamp,
}

/// Double-bit Binary Input Event - Without Time
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group4Var1 {
    /// flags field of the variation
    pub(crate) flags: u8,
}

/// Double-bit Binary Input - With Flags
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group3Var2 {
    /// flags field of the variation
    pub(crate) flags: u8,
}

/// Binary Input Event - With Relative Time
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group2Var3 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// time field of the variation
    pub(crate) time: u16,
}

/// Binary Input Event - With Absolute Time
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group2Var2 {
    /// flags field of the variation
    pub(crate) flags: u8,
    /// time field of the variation
    pub(crate) time: Timestamp,
}

/// Binary Input Event - Without Time
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group2Var1 {
    /// flags field of the variation
    pub(crate) flags: u8,
}

/// Binary Input - With Flags
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group1Var2 {
    /// flags field of the variation
    pub(crate) flags: u8,
}


impl FixedSize for Group102Var1 {
    const SIZE: u8 = 1;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group102Var1 {
                value: cursor.read_u8()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group52Var2 {
    const SIZE: u8 = 2;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group52Var2 {
                time: cursor.read_u16_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u16_le(self.time)?;
        Ok(())
    }
}

impl FixedSize for Group52Var1 {
    const SIZE: u8 = 2;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group52Var1 {
                time: cursor.read_u16_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u16_le(self.time)?;
        Ok(())
    }
}

impl FixedSize for Group51Var2 {
    const SIZE: u8 = 6;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group51Var2 {
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        self.time.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group51Var1 {
    const SIZE: u8 = 6;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group51Var1 {
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        self.time.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group50Var4 {
    const SIZE: u8 = 11;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group50Var4 {
                time: Timestamp::new(cursor.read_u48_le()?),
                interval: cursor.read_u32_le()?,
                units: cursor.read_u8()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        self.time.write(cursor)?;
        cursor.write_u32_le(self.interval)?;
        cursor.write_u8(self.units)?;
        Ok(())
    }
}

impl FixedSize for Group50Var3 {
    const SIZE: u8 = 6;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group50Var3 {
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        self.time.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group50Var2 {
    const SIZE: u8 = 10;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group50Var2 {
                time: Timestamp::new(cursor.read_u48_le()?),
                interval: cursor.read_u32_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        self.time.write(cursor)?;
        cursor.write_u32_le(self.interval)?;
        Ok(())
    }
}

impl FixedSize for Group50Var1 {
    const SIZE: u8 = 6;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group50Var1 {
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        self.time.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group43Var8 {
    const SIZE: u8 = 15;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group43Var8 {
                status: CommandStatus::from(cursor.read_u8()?),
                value: cursor.read_f64_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        self.status.write(cursor)?;
        cursor.write_f64_le(self.value)?;
        self.time.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group43Var7 {
    const SIZE: u8 = 11;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group43Var7 {
                status: CommandStatus::from(cursor.read_u8()?),
                value: cursor.read_f32_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        self.status.write(cursor)?;
        cursor.write_f32_le(self.value)?;
        self.time.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group43Var6 {
    const SIZE: u8 = 9;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group43Var6 {
                status: CommandStatus::from(cursor.read_u8()?),
                value: cursor.read_f64_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        self.status.write(cursor)?;
        cursor.write_f64_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group43Var5 {
    const SIZE: u8 = 5;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group43Var5 {
                status: CommandStatus::from(cursor.read_u8()?),
                value: cursor.read_f32_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        self.status.write(cursor)?;
        cursor.write_f32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group43Var4 {
    const SIZE: u8 = 9;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group43Var4 {
                status: CommandStatus::from(cursor.read_u8()?),
                value: cursor.read_i16_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        self.status.write(cursor)?;
        cursor.write_i16_le(self.value)?;
        self.time.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group43Var3 {
    const SIZE: u8 = 11;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group43Var3 {
                status: CommandStatus::from(cursor.read_u8()?),
                value: cursor.read_i32_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        self.status.write(cursor)?;
        cursor.write_i32_le(self.value)?;
        self.time.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group43Var2 {
    const SIZE: u8 = 3;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group43Var2 {
                status: CommandStatus::from(cursor.read_u8()?),
                value: cursor.read_i16_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        self.status.write(cursor)?;
        cursor.write_i16_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group43Var1 {
    const SIZE: u8 = 5;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group43Var1 {
                status: CommandStatus::from(cursor.read_u8()?),
                value: cursor.read_i32_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        self.status.write(cursor)?;
        cursor.write_i32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group42Var8 {
    const SIZE: u8 = 15;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group42Var8 {
                flags: cursor.read_u8()?,
                value: cursor.read_f64_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_f64_le(self.value)?;
        self.time.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group42Var7 {
    const SIZE: u8 = 11;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group42Var7 {
                flags: cursor.read_u8()?,
                value: cursor.read_f32_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_f32_le(self.value)?;
        self.time.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group42Var6 {
    const SIZE: u8 = 9;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group42Var6 {
                flags: cursor.read_u8()?,
                value: cursor.read_f64_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_f64_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group42Var5 {
    const SIZE: u8 = 5;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group42Var5 {
                flags: cursor.read_u8()?,
                value: cursor.read_f32_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_f32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group42Var4 {
    const SIZE: u8 = 9;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group42Var4 {
                flags: cursor.read_u8()?,
                value: cursor.read_i16_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_i16_le(self.value)?;
        self.time.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group42Var3 {
    const SIZE: u8 = 11;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group42Var3 {
                flags: cursor.read_u8()?,
                value: cursor.read_i32_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_i32_le(self.value)?;
        self.time.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group42Var2 {
    const SIZE: u8 = 3;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group42Var2 {
                flags: cursor.read_u8()?,
                value: cursor.read_i16_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_i16_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group42Var1 {
    const SIZE: u8 = 5;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group42Var1 {
                flags: cursor.read_u8()?,
                value: cursor.read_i32_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_i32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group41Var4 {
    const SIZE: u8 = 9;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group41Var4 {
                value: cursor.read_f64_le()?,
                status: CommandStatus::from(cursor.read_u8()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_f64_le(self.value)?;
        self.status.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group41Var3 {
    const SIZE: u8 = 5;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group41Var3 {
                value: cursor.read_f32_le()?,
                status: CommandStatus::from(cursor.read_u8()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_f32_le(self.value)?;
        self.status.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group41Var2 {
    const SIZE: u8 = 3;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group41Var2 {
                value: cursor.read_i16_le()?,
                status: CommandStatus::from(cursor.read_u8()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_i16_le(self.value)?;
        self.status.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group41Var1 {
    const SIZE: u8 = 5;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group41Var1 {
                value: cursor.read_i32_le()?,
                status: CommandStatus::from(cursor.read_u8()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_i32_le(self.value)?;
        self.status.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group40Var4 {
    const SIZE: u8 = 9;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group40Var4 {
                flags: cursor.read_u8()?,
                value: cursor.read_f64_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_f64_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group40Var3 {
    const SIZE: u8 = 5;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group40Var3 {
                flags: cursor.read_u8()?,
                value: cursor.read_f32_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_f32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group40Var2 {
    const SIZE: u8 = 3;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group40Var2 {
                flags: cursor.read_u8()?,
                value: cursor.read_i16_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_i16_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group40Var1 {
    const SIZE: u8 = 5;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group40Var1 {
                flags: cursor.read_u8()?,
                value: cursor.read_i32_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_i32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group34Var3 {
    const SIZE: u8 = 4;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group34Var3 {
                value: cursor.read_f32_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_f32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group34Var2 {
    const SIZE: u8 = 4;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group34Var2 {
                value: cursor.read_u32_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group34Var1 {
    const SIZE: u8 = 2;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group34Var1 {
                value: cursor.read_u16_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u16_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group33Var8 {
    const SIZE: u8 = 15;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group33Var8 {
                flags: cursor.read_u8()?,
                value: cursor.read_f64_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_f64_le(self.value)?;
        self.time.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group33Var7 {
    const SIZE: u8 = 11;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group33Var7 {
                flags: cursor.read_u8()?,
                value: cursor.read_f32_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_f32_le(self.value)?;
        self.time.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group33Var6 {
    const SIZE: u8 = 9;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group33Var6 {
                flags: cursor.read_u8()?,
                value: cursor.read_f64_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_f64_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group33Var5 {
    const SIZE: u8 = 5;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group33Var5 {
                flags: cursor.read_u8()?,
                value: cursor.read_f32_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_f32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group33Var4 {
    const SIZE: u8 = 9;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group33Var4 {
                flags: cursor.read_u8()?,
                value: cursor.read_i16_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_i16_le(self.value)?;
        self.time.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group33Var3 {
    const SIZE: u8 = 11;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group33Var3 {
                flags: cursor.read_u8()?,
                value: cursor.read_i32_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_i32_le(self.value)?;
        self.time.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group33Var2 {
    const SIZE: u8 = 3;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group33Var2 {
                flags: cursor.read_u8()?,
                value: cursor.read_i16_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_i16_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group33Var1 {
    const SIZE: u8 = 5;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group33Var1 {
                flags: cursor.read_u8()?,
                value: cursor.read_i32_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_i32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group32Var8 {
    const SIZE: u8 = 15;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group32Var8 {
                flags: cursor.read_u8()?,
                value: cursor.read_f64_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_f64_le(self.value)?;
        self.time.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group32Var7 {
    const SIZE: u8 = 11;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group32Var7 {
                flags: cursor.read_u8()?,
                value: cursor.read_f32_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_f32_le(self.value)?;
        self.time.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group32Var6 {
    const SIZE: u8 = 9;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group32Var6 {
                flags: cursor.read_u8()?,
                value: cursor.read_f64_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_f64_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group32Var5 {
    const SIZE: u8 = 5;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group32Var5 {
                flags: cursor.read_u8()?,
                value: cursor.read_f32_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_f32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group32Var4 {
    const SIZE: u8 = 9;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group32Var4 {
                flags: cursor.read_u8()?,
                value: cursor.read_i16_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_i16_le(self.value)?;
        self.time.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group32Var3 {
    const SIZE: u8 = 11;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group32Var3 {
                flags: cursor.read_u8()?,
                value: cursor.read_i32_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_i32_le(self.value)?;
        self.time.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group32Var2 {
    const SIZE: u8 = 3;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group32Var2 {
                flags: cursor.read_u8()?,
                value: cursor.read_i16_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_i16_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group32Var1 {
    const SIZE: u8 = 5;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group32Var1 {
                flags: cursor.read_u8()?,
                value: cursor.read_i32_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_i32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group31Var8 {
    const SIZE: u8 = 9;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group31Var8 {
                flags: cursor.read_u8()?,
                value: cursor.read_f64_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_f64_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group31Var7 {
    const SIZE: u8 = 5;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group31Var7 {
                flags: cursor.read_u8()?,
                value: cursor.read_f32_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_f32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group31Var6 {
    const SIZE: u8 = 2;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group31Var6 {
                value: cursor.read_i16_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_i16_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group31Var5 {
    const SIZE: u8 = 4;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group31Var5 {
                value: cursor.read_i32_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_i32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group31Var4 {
    const SIZE: u8 = 9;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group31Var4 {
                flags: cursor.read_u8()?,
                value: cursor.read_i16_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_i16_le(self.value)?;
        self.time.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group31Var3 {
    const SIZE: u8 = 11;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group31Var3 {
                flags: cursor.read_u8()?,
                value: cursor.read_i32_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_i32_le(self.value)?;
        self.time.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group31Var2 {
    const SIZE: u8 = 3;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group31Var2 {
                flags: cursor.read_u8()?,
                value: cursor.read_i16_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_i16_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group31Var1 {
    const SIZE: u8 = 5;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group31Var1 {
                flags: cursor.read_u8()?,
                value: cursor.read_i32_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_i32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group30Var6 {
    const SIZE: u8 = 9;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group30Var6 {
                flags: cursor.read_u8()?,
                value: cursor.read_f64_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_f64_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group30Var5 {
    const SIZE: u8 = 5;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group30Var5 {
                flags: cursor.read_u8()?,
                value: cursor.read_f32_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_f32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group30Var4 {
    const SIZE: u8 = 2;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group30Var4 {
                value: cursor.read_i16_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_i16_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group30Var3 {
    const SIZE: u8 = 4;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group30Var3 {
                value: cursor.read_i32_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_i32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group30Var2 {
    const SIZE: u8 = 3;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group30Var2 {
                flags: cursor.read_u8()?,
                value: cursor.read_i16_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_i16_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group30Var1 {
    const SIZE: u8 = 5;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group30Var1 {
                flags: cursor.read_u8()?,
                value: cursor.read_i32_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_i32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group23Var6 {
    const SIZE: u8 = 9;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group23Var6 {
                flags: cursor.read_u8()?,
                value: cursor.read_u16_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u16_le(self.value)?;
        self.time.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group23Var5 {
    const SIZE: u8 = 11;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group23Var5 {
                flags: cursor.read_u8()?,
                value: cursor.read_u32_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u32_le(self.value)?;
        self.time.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group23Var2 {
    const SIZE: u8 = 3;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group23Var2 {
                flags: cursor.read_u8()?,
                value: cursor.read_u16_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u16_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group23Var1 {
    const SIZE: u8 = 5;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group23Var1 {
                flags: cursor.read_u8()?,
                value: cursor.read_u32_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group22Var6 {
    const SIZE: u8 = 9;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group22Var6 {
                flags: cursor.read_u8()?,
                value: cursor.read_u16_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u16_le(self.value)?;
        self.time.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group22Var5 {
    const SIZE: u8 = 11;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group22Var5 {
                flags: cursor.read_u8()?,
                value: cursor.read_u32_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u32_le(self.value)?;
        self.time.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group22Var2 {
    const SIZE: u8 = 3;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group22Var2 {
                flags: cursor.read_u8()?,
                value: cursor.read_u16_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u16_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group22Var1 {
    const SIZE: u8 = 5;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group22Var1 {
                flags: cursor.read_u8()?,
                value: cursor.read_u32_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group21Var10 {
    const SIZE: u8 = 2;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group21Var10 {
                value: cursor.read_u16_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u16_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group21Var9 {
    const SIZE: u8 = 4;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group21Var9 {
                value: cursor.read_u32_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group21Var6 {
    const SIZE: u8 = 9;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group21Var6 {
                flags: cursor.read_u8()?,
                value: cursor.read_u16_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u16_le(self.value)?;
        self.time.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group21Var5 {
    const SIZE: u8 = 11;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group21Var5 {
                flags: cursor.read_u8()?,
                value: cursor.read_u32_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u32_le(self.value)?;
        self.time.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group21Var2 {
    const SIZE: u8 = 3;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group21Var2 {
                flags: cursor.read_u8()?,
                value: cursor.read_u16_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u16_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group21Var1 {
    const SIZE: u8 = 5;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group21Var1 {
                flags: cursor.read_u8()?,
                value: cursor.read_u32_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group20Var6 {
    const SIZE: u8 = 2;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group20Var6 {
                value: cursor.read_u16_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u16_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group20Var5 {
    const SIZE: u8 = 4;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group20Var5 {
                value: cursor.read_u32_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group20Var2 {
    const SIZE: u8 = 3;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group20Var2 {
                flags: cursor.read_u8()?,
                value: cursor.read_u16_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u16_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group20Var1 {
    const SIZE: u8 = 5;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group20Var1 {
                flags: cursor.read_u8()?,
                value: cursor.read_u32_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group13Var2 {
    const SIZE: u8 = 7;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group13Var2 {
                flags: cursor.read_u8()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        self.time.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group13Var1 {
    const SIZE: u8 = 1;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group13Var1 {
                flags: cursor.read_u8()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        Ok(())
    }
}

impl FixedSize for Group12Var1 {
    const SIZE: u8 = 11;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group12Var1 {
                code: ControlCode::from(cursor.read_u8()?),
                count: cursor.read_u8()?,
                on_time: cursor.read_u32_le()?,
                off_time: cursor.read_u32_le()?,
                status: CommandStatus::from(cursor.read_u8()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.code.as_u8())?;
        cursor.write_u8(self.count)?;
        cursor.write_u32_le(self.on_time)?;
        cursor.write_u32_le(self.off_time)?;
        self.status.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group11Var2 {
    const SIZE: u8 = 7;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group11Var2 {
                flags: cursor.read_u8()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        self.time.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group11Var1 {
    const SIZE: u8 = 1;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group11Var1 {
                flags: cursor.read_u8()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        Ok(())
    }
}

impl FixedSize for Group10Var2 {
    const SIZE: u8 = 1;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group10Var2 {
                flags: cursor.read_u8()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        Ok(())
    }
}

impl FixedSize for Group4Var3 {
    const SIZE: u8 = 3;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group4Var3 {
                flags: cursor.read_u8()?,
                time: cursor.read_u16_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u16_le(self.time)?;
        Ok(())
    }
}

impl FixedSize for Group4Var2 {
    const SIZE: u8 = 7;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group4Var2 {
                flags: cursor.read_u8()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        self.time.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group4Var1 {
    const SIZE: u8 = 1;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group4Var1 {
                flags: cursor.read_u8()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        Ok(())
    }
}

impl FixedSize for Group3Var2 {
    const SIZE: u8 = 1;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group3Var2 {
                flags: cursor.read_u8()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        Ok(())
    }
}

impl FixedSize for Group2Var3 {
    const SIZE: u8 = 3;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group2Var3 {
                flags: cursor.read_u8()?,
                time: cursor.read_u16_le()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u16_le(self.time)?;
        Ok(())
    }
}

impl FixedSize for Group2Var2 {
    const SIZE: u8 = 7;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group2Var2 {
                flags: cursor.read_u8()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        self.time.write(cursor)?;
        Ok(())
    }
}

impl FixedSize for Group2Var1 {
    const SIZE: u8 = 1;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group2Var1 {
                flags: cursor.read_u8()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        Ok(())
    }
}

impl FixedSize for Group1Var2 {
    const SIZE: u8 = 1;
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group1Var2 {
                flags: cursor.read_u8()?,
            }
        )
    }
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        Ok(())
    }
}


impl std::fmt::Display for Group102Var1 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "value: {}", self.value)
    }
}

impl std::fmt::Display for Group52Var2 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "time: {}", self.time)
    }
}

impl std::fmt::Display for Group52Var1 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "time: {}", self.time)
    }
}

impl std::fmt::Display for Group51Var2 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "time: {}", self.time)
    }
}

impl std::fmt::Display for Group51Var1 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "time: {}", self.time)
    }
}

impl std::fmt::Display for Group50Var4 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "time: {} interval: {} units: {}", self.time, self.interval, self.units)
    }
}

impl std::fmt::Display for Group50Var3 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "time: {}", self.time)
    }
}

impl std::fmt::Display for Group50Var2 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "time: {} interval: {}", self.time, self.interval)
    }
}

impl std::fmt::Display for Group50Var1 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "time: {}", self.time)
    }
}

impl std::fmt::Display for Group43Var8 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "status: {:?} value: {} time: {}", self.status, self.value, self.time)
    }
}

impl std::fmt::Display for Group43Var7 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "status: {:?} value: {} time: {}", self.status, self.value, self.time)
    }
}

impl std::fmt::Display for Group43Var6 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "status: {:?} value: {}", self.status, self.value)
    }
}

impl std::fmt::Display for Group43Var5 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "status: {:?} value: {}", self.status, self.value)
    }
}

impl std::fmt::Display for Group43Var4 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "status: {:?} value: {} time: {}", self.status, self.value, self.time)
    }
}

impl std::fmt::Display for Group43Var3 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "status: {:?} value: {} time: {}", self.status, self.value, self.time)
    }
}

impl std::fmt::Display for Group43Var2 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "status: {:?} value: {}", self.status, self.value)
    }
}

impl std::fmt::Display for Group43Var1 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "status: {:?} value: {}", self.status, self.value)
    }
}

impl std::fmt::Display for Group42Var8 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {} time: {}", AnalogFlagFormatter::new(self.flags), self.value, self.time)
    }
}

impl std::fmt::Display for Group42Var7 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {} time: {}", AnalogFlagFormatter::new(self.flags), self.value, self.time)
    }
}

impl std::fmt::Display for Group42Var6 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {}", AnalogFlagFormatter::new(self.flags), self.value)
    }
}

impl std::fmt::Display for Group42Var5 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {}", AnalogFlagFormatter::new(self.flags), self.value)
    }
}

impl std::fmt::Display for Group42Var4 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {} time: {}", AnalogFlagFormatter::new(self.flags), self.value, self.time)
    }
}

impl std::fmt::Display for Group42Var3 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {} time: {}", AnalogFlagFormatter::new(self.flags), self.value, self.time)
    }
}

impl std::fmt::Display for Group42Var2 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {}", AnalogFlagFormatter::new(self.flags), self.value)
    }
}

impl std::fmt::Display for Group42Var1 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {}", AnalogFlagFormatter::new(self.flags), self.value)
    }
}

impl std::fmt::Display for Group41Var4 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "value: {} status: {:?}", self.value, self.status)
    }
}

impl std::fmt::Display for Group41Var3 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "value: {} status: {:?}", self.value, self.status)
    }
}

impl std::fmt::Display for Group41Var2 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "value: {} status: {:?}", self.value, self.status)
    }
}

impl std::fmt::Display for Group41Var1 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "value: {} status: {:?}", self.value, self.status)
    }
}

impl std::fmt::Display for Group40Var4 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {}", AnalogFlagFormatter::new(self.flags), self.value)
    }
}

impl std::fmt::Display for Group40Var3 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {}", AnalogFlagFormatter::new(self.flags), self.value)
    }
}

impl std::fmt::Display for Group40Var2 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {}", AnalogFlagFormatter::new(self.flags), self.value)
    }
}

impl std::fmt::Display for Group40Var1 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {}", AnalogFlagFormatter::new(self.flags), self.value)
    }
}

impl std::fmt::Display for Group34Var3 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "value: {}", self.value)
    }
}

impl std::fmt::Display for Group34Var2 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "value: {}", self.value)
    }
}

impl std::fmt::Display for Group34Var1 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "value: {}", self.value)
    }
}

impl std::fmt::Display for Group33Var8 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {} time: {}", AnalogFlagFormatter::new(self.flags), self.value, self.time)
    }
}

impl std::fmt::Display for Group33Var7 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {} time: {}", AnalogFlagFormatter::new(self.flags), self.value, self.time)
    }
}

impl std::fmt::Display for Group33Var6 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {}", AnalogFlagFormatter::new(self.flags), self.value)
    }
}

impl std::fmt::Display for Group33Var5 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {}", AnalogFlagFormatter::new(self.flags), self.value)
    }
}

impl std::fmt::Display for Group33Var4 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {} time: {}", AnalogFlagFormatter::new(self.flags), self.value, self.time)
    }
}

impl std::fmt::Display for Group33Var3 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {} time: {}", AnalogFlagFormatter::new(self.flags), self.value, self.time)
    }
}

impl std::fmt::Display for Group33Var2 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {}", AnalogFlagFormatter::new(self.flags), self.value)
    }
}

impl std::fmt::Display for Group33Var1 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {}", AnalogFlagFormatter::new(self.flags), self.value)
    }
}

impl std::fmt::Display for Group32Var8 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {} time: {}", AnalogFlagFormatter::new(self.flags), self.value, self.time)
    }
}

impl std::fmt::Display for Group32Var7 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {} time: {}", AnalogFlagFormatter::new(self.flags), self.value, self.time)
    }
}

impl std::fmt::Display for Group32Var6 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {}", AnalogFlagFormatter::new(self.flags), self.value)
    }
}

impl std::fmt::Display for Group32Var5 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {}", AnalogFlagFormatter::new(self.flags), self.value)
    }
}

impl std::fmt::Display for Group32Var4 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {} time: {}", AnalogFlagFormatter::new(self.flags), self.value, self.time)
    }
}

impl std::fmt::Display for Group32Var3 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {} time: {}", AnalogFlagFormatter::new(self.flags), self.value, self.time)
    }
}

impl std::fmt::Display for Group32Var2 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {}", AnalogFlagFormatter::new(self.flags), self.value)
    }
}

impl std::fmt::Display for Group32Var1 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {}", AnalogFlagFormatter::new(self.flags), self.value)
    }
}

impl std::fmt::Display for Group31Var8 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {}", AnalogFlagFormatter::new(self.flags), self.value)
    }
}

impl std::fmt::Display for Group31Var7 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {}", AnalogFlagFormatter::new(self.flags), self.value)
    }
}

impl std::fmt::Display for Group31Var6 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "value: {}", self.value)
    }
}

impl std::fmt::Display for Group31Var5 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "value: {}", self.value)
    }
}

impl std::fmt::Display for Group31Var4 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {} time: {}", AnalogFlagFormatter::new(self.flags), self.value, self.time)
    }
}

impl std::fmt::Display for Group31Var3 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {} time: {}", AnalogFlagFormatter::new(self.flags), self.value, self.time)
    }
}

impl std::fmt::Display for Group31Var2 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {}", AnalogFlagFormatter::new(self.flags), self.value)
    }
}

impl std::fmt::Display for Group31Var1 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {}", AnalogFlagFormatter::new(self.flags), self.value)
    }
}

impl std::fmt::Display for Group30Var6 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {}", AnalogFlagFormatter::new(self.flags), self.value)
    }
}

impl std::fmt::Display for Group30Var5 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {}", AnalogFlagFormatter::new(self.flags), self.value)
    }
}

impl std::fmt::Display for Group30Var4 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "value: {}", self.value)
    }
}

impl std::fmt::Display for Group30Var3 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "value: {}", self.value)
    }
}

impl std::fmt::Display for Group30Var2 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {}", AnalogFlagFormatter::new(self.flags), self.value)
    }
}

impl std::fmt::Display for Group30Var1 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {}", AnalogFlagFormatter::new(self.flags), self.value)
    }
}

impl std::fmt::Display for Group23Var6 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {} time: {}", CounterFlagFormatter::new(self.flags), self.value, self.time)
    }
}

impl std::fmt::Display for Group23Var5 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {} time: {}", CounterFlagFormatter::new(self.flags), self.value, self.time)
    }
}

impl std::fmt::Display for Group23Var2 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {}", CounterFlagFormatter::new(self.flags), self.value)
    }
}

impl std::fmt::Display for Group23Var1 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {}", CounterFlagFormatter::new(self.flags), self.value)
    }
}

impl std::fmt::Display for Group22Var6 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {} time: {}", CounterFlagFormatter::new(self.flags), self.value, self.time)
    }
}

impl std::fmt::Display for Group22Var5 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {} time: {}", CounterFlagFormatter::new(self.flags), self.value, self.time)
    }
}

impl std::fmt::Display for Group22Var2 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {}", CounterFlagFormatter::new(self.flags), self.value)
    }
}

impl std::fmt::Display for Group22Var1 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {}", CounterFlagFormatter::new(self.flags), self.value)
    }
}

impl std::fmt::Display for Group21Var10 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "value: {}", self.value)
    }
}

impl std::fmt::Display for Group21Var9 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "value: {}", self.value)
    }
}

impl std::fmt::Display for Group21Var6 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {} time: {}", CounterFlagFormatter::new(self.flags), self.value, self.time)
    }
}

impl std::fmt::Display for Group21Var5 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {} time: {}", CounterFlagFormatter::new(self.flags), self.value, self.time)
    }
}

impl std::fmt::Display for Group21Var2 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {}", CounterFlagFormatter::new(self.flags), self.value)
    }
}

impl std::fmt::Display for Group21Var1 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {}", CounterFlagFormatter::new(self.flags), self.value)
    }
}

impl std::fmt::Display for Group20Var6 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "value: {}", self.value)
    }
}

impl std::fmt::Display for Group20Var5 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "value: {}", self.value)
    }
}

impl std::fmt::Display for Group20Var2 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {}", CounterFlagFormatter::new(self.flags), self.value)
    }
}

impl std::fmt::Display for Group20Var1 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} value: {}", CounterFlagFormatter::new(self.flags), self.value)
    }
}

impl std::fmt::Display for Group13Var2 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} time: {}", BinaryFlagFormatter::new(self.flags), self.time)
    }
}

impl std::fmt::Display for Group13Var1 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {}", BinaryFlagFormatter::new(self.flags))
    }
}

impl std::fmt::Display for Group12Var1 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "code: {} count: {} on_time: {} off_time: {} status: {:?}", self.code, self.count, self.on_time, self.off_time, self.status)
    }
}

impl std::fmt::Display for Group11Var2 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} time: {}", BinaryOutputStatusFlagFormatter::new(self.flags), self.time)
    }
}

impl std::fmt::Display for Group11Var1 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {}", BinaryOutputStatusFlagFormatter::new(self.flags))
    }
}

impl std::fmt::Display for Group10Var2 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {}", BinaryOutputStatusFlagFormatter::new(self.flags))
    }
}

impl std::fmt::Display for Group4Var3 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} time: {}", DoubleBitBinaryFlagFormatter::new(self.flags), self.time)
    }
}

impl std::fmt::Display for Group4Var2 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} time: {}", DoubleBitBinaryFlagFormatter::new(self.flags), self.time)
    }
}

impl std::fmt::Display for Group4Var1 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {}", DoubleBitBinaryFlagFormatter::new(self.flags))
    }
}

impl std::fmt::Display for Group3Var2 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {}", DoubleBitBinaryFlagFormatter::new(self.flags))
    }
}

impl std::fmt::Display for Group2Var3 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} time: {}", BinaryFlagFormatter::new(self.flags), self.time)
    }
}

impl std::fmt::Display for Group2Var2 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {} time: {}", BinaryFlagFormatter::new(self.flags), self.time)
    }
}

impl std::fmt::Display for Group2Var1 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {}", BinaryFlagFormatter::new(self.flags))
    }
}

impl std::fmt::Display for Group1Var2 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "flags: {}", BinaryFlagFormatter::new(self.flags))
    }
}


impl FixedSizeVariation for Group102Var1 {
    const VARIATION : Variation = Variation::Group102Var1;
}

impl FixedSizeVariation for Group52Var2 {
    const VARIATION : Variation = Variation::Group52Var2;
}

impl FixedSizeVariation for Group52Var1 {
    const VARIATION : Variation = Variation::Group52Var1;
}

impl FixedSizeVariation for Group51Var2 {
    const VARIATION : Variation = Variation::Group51Var2;
}

impl FixedSizeVariation for Group51Var1 {
    const VARIATION : Variation = Variation::Group51Var1;
}

impl FixedSizeVariation for Group50Var4 {
    const VARIATION : Variation = Variation::Group50Var4;
}

impl FixedSizeVariation for Group50Var3 {
    const VARIATION : Variation = Variation::Group50Var3;
}

impl FixedSizeVariation for Group50Var2 {
    const VARIATION : Variation = Variation::Group50Var2;
}

impl FixedSizeVariation for Group50Var1 {
    const VARIATION : Variation = Variation::Group50Var1;
}

impl FixedSizeVariation for Group43Var8 {
    const VARIATION : Variation = Variation::Group43Var8;
}

impl FixedSizeVariation for Group43Var7 {
    const VARIATION : Variation = Variation::Group43Var7;
}

impl FixedSizeVariation for Group43Var6 {
    const VARIATION : Variation = Variation::Group43Var6;
}

impl FixedSizeVariation for Group43Var5 {
    const VARIATION : Variation = Variation::Group43Var5;
}

impl FixedSizeVariation for Group43Var4 {
    const VARIATION : Variation = Variation::Group43Var4;
}

impl FixedSizeVariation for Group43Var3 {
    const VARIATION : Variation = Variation::Group43Var3;
}

impl FixedSizeVariation for Group43Var2 {
    const VARIATION : Variation = Variation::Group43Var2;
}

impl FixedSizeVariation for Group43Var1 {
    const VARIATION : Variation = Variation::Group43Var1;
}

impl FixedSizeVariation for Group42Var8 {
    const VARIATION : Variation = Variation::Group42Var8;
}

impl FixedSizeVariation for Group42Var7 {
    const VARIATION : Variation = Variation::Group42Var7;
}

impl FixedSizeVariation for Group42Var6 {
    const VARIATION : Variation = Variation::Group42Var6;
}

impl FixedSizeVariation for Group42Var5 {
    const VARIATION : Variation = Variation::Group42Var5;
}

impl FixedSizeVariation for Group42Var4 {
    const VARIATION : Variation = Variation::Group42Var4;
}

impl FixedSizeVariation for Group42Var3 {
    const VARIATION : Variation = Variation::Group42Var3;
}

impl FixedSizeVariation for Group42Var2 {
    const VARIATION : Variation = Variation::Group42Var2;
}

impl FixedSizeVariation for Group42Var1 {
    const VARIATION : Variation = Variation::Group42Var1;
}

impl FixedSizeVariation for Group41Var4 {
    const VARIATION : Variation = Variation::Group41Var4;
}

impl FixedSizeVariation for Group41Var3 {
    const VARIATION : Variation = Variation::Group41Var3;
}

impl FixedSizeVariation for Group41Var2 {
    const VARIATION : Variation = Variation::Group41Var2;
}

impl FixedSizeVariation for Group41Var1 {
    const VARIATION : Variation = Variation::Group41Var1;
}

impl FixedSizeVariation for Group40Var4 {
    const VARIATION : Variation = Variation::Group40Var4;
}

impl FixedSizeVariation for Group40Var3 {
    const VARIATION : Variation = Variation::Group40Var3;
}

impl FixedSizeVariation for Group40Var2 {
    const VARIATION : Variation = Variation::Group40Var2;
}

impl FixedSizeVariation for Group40Var1 {
    const VARIATION : Variation = Variation::Group40Var1;
}

impl FixedSizeVariation for Group34Var3 {
    const VARIATION : Variation = Variation::Group34Var3;
}

impl FixedSizeVariation for Group34Var2 {
    const VARIATION : Variation = Variation::Group34Var2;
}

impl FixedSizeVariation for Group34Var1 {
    const VARIATION : Variation = Variation::Group34Var1;
}

impl FixedSizeVariation for Group33Var8 {
    const VARIATION : Variation = Variation::Group33Var8;
}

impl FixedSizeVariation for Group33Var7 {
    const VARIATION : Variation = Variation::Group33Var7;
}

impl FixedSizeVariation for Group33Var6 {
    const VARIATION : Variation = Variation::Group33Var6;
}

impl FixedSizeVariation for Group33Var5 {
    const VARIATION : Variation = Variation::Group33Var5;
}

impl FixedSizeVariation for Group33Var4 {
    const VARIATION : Variation = Variation::Group33Var4;
}

impl FixedSizeVariation for Group33Var3 {
    const VARIATION : Variation = Variation::Group33Var3;
}

impl FixedSizeVariation for Group33Var2 {
    const VARIATION : Variation = Variation::Group33Var2;
}

impl FixedSizeVariation for Group33Var1 {
    const VARIATION : Variation = Variation::Group33Var1;
}

impl FixedSizeVariation for Group32Var8 {
    const VARIATION : Variation = Variation::Group32Var8;
}

impl FixedSizeVariation for Group32Var7 {
    const VARIATION : Variation = Variation::Group32Var7;
}

impl FixedSizeVariation for Group32Var6 {
    const VARIATION : Variation = Variation::Group32Var6;
}

impl FixedSizeVariation for Group32Var5 {
    const VARIATION : Variation = Variation::Group32Var5;
}

impl FixedSizeVariation for Group32Var4 {
    const VARIATION : Variation = Variation::Group32Var4;
}

impl FixedSizeVariation for Group32Var3 {
    const VARIATION : Variation = Variation::Group32Var3;
}

impl FixedSizeVariation for Group32Var2 {
    const VARIATION : Variation = Variation::Group32Var2;
}

impl FixedSizeVariation for Group32Var1 {
    const VARIATION : Variation = Variation::Group32Var1;
}

impl FixedSizeVariation for Group31Var8 {
    const VARIATION : Variation = Variation::Group31Var8;
}

impl FixedSizeVariation for Group31Var7 {
    const VARIATION : Variation = Variation::Group31Var7;
}

impl FixedSizeVariation for Group31Var6 {
    const VARIATION : Variation = Variation::Group31Var6;
}

impl FixedSizeVariation for Group31Var5 {
    const VARIATION : Variation = Variation::Group31Var5;
}

impl FixedSizeVariation for Group31Var4 {
    const VARIATION : Variation = Variation::Group31Var4;
}

impl FixedSizeVariation for Group31Var3 {
    const VARIATION : Variation = Variation::Group31Var3;
}

impl FixedSizeVariation for Group31Var2 {
    const VARIATION : Variation = Variation::Group31Var2;
}

impl FixedSizeVariation for Group31Var1 {
    const VARIATION : Variation = Variation::Group31Var1;
}

impl FixedSizeVariation for Group30Var6 {
    const VARIATION : Variation = Variation::Group30Var6;
}

impl FixedSizeVariation for Group30Var5 {
    const VARIATION : Variation = Variation::Group30Var5;
}

impl FixedSizeVariation for Group30Var4 {
    const VARIATION : Variation = Variation::Group30Var4;
}

impl FixedSizeVariation for Group30Var3 {
    const VARIATION : Variation = Variation::Group30Var3;
}

impl FixedSizeVariation for Group30Var2 {
    const VARIATION : Variation = Variation::Group30Var2;
}

impl FixedSizeVariation for Group30Var1 {
    const VARIATION : Variation = Variation::Group30Var1;
}

impl FixedSizeVariation for Group23Var6 {
    const VARIATION : Variation = Variation::Group23Var6;
}

impl FixedSizeVariation for Group23Var5 {
    const VARIATION : Variation = Variation::Group23Var5;
}

impl FixedSizeVariation for Group23Var2 {
    const VARIATION : Variation = Variation::Group23Var2;
}

impl FixedSizeVariation for Group23Var1 {
    const VARIATION : Variation = Variation::Group23Var1;
}

impl FixedSizeVariation for Group22Var6 {
    const VARIATION : Variation = Variation::Group22Var6;
}

impl FixedSizeVariation for Group22Var5 {
    const VARIATION : Variation = Variation::Group22Var5;
}

impl FixedSizeVariation for Group22Var2 {
    const VARIATION : Variation = Variation::Group22Var2;
}

impl FixedSizeVariation for Group22Var1 {
    const VARIATION : Variation = Variation::Group22Var1;
}

impl FixedSizeVariation for Group21Var10 {
    const VARIATION : Variation = Variation::Group21Var10;
}

impl FixedSizeVariation for Group21Var9 {
    const VARIATION : Variation = Variation::Group21Var9;
}

impl FixedSizeVariation for Group21Var6 {
    const VARIATION : Variation = Variation::Group21Var6;
}

impl FixedSizeVariation for Group21Var5 {
    const VARIATION : Variation = Variation::Group21Var5;
}

impl FixedSizeVariation for Group21Var2 {
    const VARIATION : Variation = Variation::Group21Var2;
}

impl FixedSizeVariation for Group21Var1 {
    const VARIATION : Variation = Variation::Group21Var1;
}

impl FixedSizeVariation for Group20Var6 {
    const VARIATION : Variation = Variation::Group20Var6;
}

impl FixedSizeVariation for Group20Var5 {
    const VARIATION : Variation = Variation::Group20Var5;
}

impl FixedSizeVariation for Group20Var2 {
    const VARIATION : Variation = Variation::Group20Var2;
}

impl FixedSizeVariation for Group20Var1 {
    const VARIATION : Variation = Variation::Group20Var1;
}

impl FixedSizeVariation for Group13Var2 {
    const VARIATION : Variation = Variation::Group13Var2;
}

impl FixedSizeVariation for Group13Var1 {
    const VARIATION : Variation = Variation::Group13Var1;
}

impl FixedSizeVariation for Group12Var1 {
    const VARIATION : Variation = Variation::Group12Var1;
}

impl FixedSizeVariation for Group11Var2 {
    const VARIATION : Variation = Variation::Group11Var2;
}

impl FixedSizeVariation for Group11Var1 {
    const VARIATION : Variation = Variation::Group11Var1;
}

impl FixedSizeVariation for Group10Var2 {
    const VARIATION : Variation = Variation::Group10Var2;
}

impl FixedSizeVariation for Group4Var3 {
    const VARIATION : Variation = Variation::Group4Var3;
}

impl FixedSizeVariation for Group4Var2 {
    const VARIATION : Variation = Variation::Group4Var2;
}

impl FixedSizeVariation for Group4Var1 {
    const VARIATION : Variation = Variation::Group4Var1;
}

impl FixedSizeVariation for Group3Var2 {
    const VARIATION : Variation = Variation::Group3Var2;
}

impl FixedSizeVariation for Group2Var3 {
    const VARIATION : Variation = Variation::Group2Var3;
}

impl FixedSizeVariation for Group2Var2 {
    const VARIATION : Variation = Variation::Group2Var2;
}

impl FixedSizeVariation for Group2Var1 {
    const VARIATION : Variation = Variation::Group2Var1;
}

impl FixedSizeVariation for Group1Var2 {
    const VARIATION : Variation = Variation::Group1Var2;
}

