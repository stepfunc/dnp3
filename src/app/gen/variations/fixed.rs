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

use crate::app::gen::enums::CommandStatus;
use crate::app::parse::traits::FixedSize;
use crate::app::types::{ControlCode, Timestamp};
use crate::util::cursor::*;

/// Time Delay - Fine
#[derive(Debug, PartialEq)]
pub struct Group52Var2 {
    pub time: u16,
}

/// Time Delay - Coarse
#[derive(Debug, PartialEq)]
pub struct Group52Var1 {
    pub time: u16,
}

/// Time and Date CTO - Absolute time, unsynchronized
#[derive(Debug, PartialEq)]
pub struct Group51Var2 {
    pub time: Timestamp,
}

/// Time and Date CTO - Absolute time, synchronized
#[derive(Debug, PartialEq)]
pub struct Group51Var1 {
    pub time: Timestamp,
}

/// Time and Date - Indexed absolute time and long interval
#[derive(Debug, PartialEq)]
pub struct Group50Var4 {
    pub time: Timestamp,
    pub interval: u32,
    pub units: u8,
}

/// Time and Date - Absolute Time at last recorded time
#[derive(Debug, PartialEq)]
pub struct Group50Var3 {
    pub time: Timestamp,
}

/// Time and Date - Absolute Time
#[derive(Debug, PartialEq)]
pub struct Group50Var1 {
    pub time: Timestamp,
}

/// Analog Command Event - Double-precision With Time
#[derive(Debug, PartialEq)]
pub struct Group43Var8 {
    pub status: CommandStatus,
    pub value: f64,
    pub time: Timestamp,
}

/// Analog Command Event - Single-precision With Time
#[derive(Debug, PartialEq)]
pub struct Group43Var7 {
    pub status: CommandStatus,
    pub value: f32,
    pub time: Timestamp,
}

/// Analog Command Event - Double-precision
#[derive(Debug, PartialEq)]
pub struct Group43Var6 {
    pub status: CommandStatus,
    pub value: f64,
}

/// Analog Command Event - Single-precision
#[derive(Debug, PartialEq)]
pub struct Group43Var5 {
    pub status: CommandStatus,
    pub value: f32,
}

/// Analog Command Event - 16-bit With Time
#[derive(Debug, PartialEq)]
pub struct Group43Var4 {
    pub status: CommandStatus,
    pub value: i16,
    pub time: Timestamp,
}

/// Analog Command Event - 32-bit With Time
#[derive(Debug, PartialEq)]
pub struct Group43Var3 {
    pub status: CommandStatus,
    pub value: i32,
    pub time: Timestamp,
}

/// Analog Command Event - 16-bit
#[derive(Debug, PartialEq)]
pub struct Group43Var2 {
    pub status: CommandStatus,
    pub value: i16,
}

/// Analog Command Event - 32-bit
#[derive(Debug, PartialEq)]
pub struct Group43Var1 {
    pub status: CommandStatus,
    pub value: i32,
}

/// Analog Output Event - Double-precision With Flag and Time
#[derive(Debug, PartialEq)]
pub struct Group42Var8 {
    pub flags: u8,
    pub value: f64,
    pub time: Timestamp,
}

/// Analog Output Event - Single-precision With Flag and Time
#[derive(Debug, PartialEq)]
pub struct Group42Var7 {
    pub flags: u8,
    pub value: f32,
    pub time: Timestamp,
}

/// Analog Output Event - Double-precision With Flag
#[derive(Debug, PartialEq)]
pub struct Group42Var6 {
    pub flags: u8,
    pub value: f64,
}

/// Analog Output Event - Single-precision With Flag
#[derive(Debug, PartialEq)]
pub struct Group42Var5 {
    pub flags: u8,
    pub value: f32,
}

/// Analog Output Event - 16-bit With Flag and Time
#[derive(Debug, PartialEq)]
pub struct Group42Var4 {
    pub flags: u8,
    pub value: i16,
    pub time: Timestamp,
}

/// Analog Output Event - 32-bit With Flag and Time
#[derive(Debug, PartialEq)]
pub struct Group42Var3 {
    pub flags: u8,
    pub value: i32,
    pub time: Timestamp,
}

/// Analog Output Event - 16-bit With Flag
#[derive(Debug, PartialEq)]
pub struct Group42Var2 {
    pub flags: u8,
    pub value: i16,
}

/// Analog Output Event - 32-bit With Flag
#[derive(Debug, PartialEq)]
pub struct Group42Var1 {
    pub flags: u8,
    pub value: i32,
}

/// Analog Output - Double-precision
#[derive(Debug, PartialEq)]
pub struct Group41Var4 {
    pub value: f64,
    pub status: CommandStatus,
}

/// Analog Output - Single-precision
#[derive(Debug, PartialEq)]
pub struct Group41Var3 {
    pub value: f32,
    pub status: CommandStatus,
}

/// Analog Output - 16-bit With Flag
#[derive(Debug, PartialEq)]
pub struct Group41Var2 {
    pub value: i16,
    pub status: CommandStatus,
}

/// Analog Output - 32-bit With Flag
#[derive(Debug, PartialEq)]
pub struct Group41Var1 {
    pub value: i32,
    pub status: CommandStatus,
}

/// Analog Output Status - Double-precision With Flag
#[derive(Debug, PartialEq)]
pub struct Group40Var4 {
    pub flags: u8,
    pub value: f64,
}

/// Analog Output Status - Single-precision With Flag
#[derive(Debug, PartialEq)]
pub struct Group40Var3 {
    pub flags: u8,
    pub value: f32,
}

/// Analog Output Status - 16-bit With Flag
#[derive(Debug, PartialEq)]
pub struct Group40Var2 {
    pub flags: u8,
    pub value: i16,
}

/// Analog Output Status - 32-bit With Flag
#[derive(Debug, PartialEq)]
pub struct Group40Var1 {
    pub flags: u8,
    pub value: i32,
}

/// Analog Input Event - Double-precision With Flag and Time
#[derive(Debug, PartialEq)]
pub struct Group32Var8 {
    pub flags: u8,
    pub value: f64,
    pub time: Timestamp,
}

/// Analog Input Event - Single-precision With Flag and Time
#[derive(Debug, PartialEq)]
pub struct Group32Var7 {
    pub flags: u8,
    pub value: f32,
    pub time: Timestamp,
}

/// Analog Input Event - Double-precision With Flag
#[derive(Debug, PartialEq)]
pub struct Group32Var6 {
    pub flags: u8,
    pub value: f64,
}

/// Analog Input Event - Single-precision With Flag
#[derive(Debug, PartialEq)]
pub struct Group32Var5 {
    pub flags: u8,
    pub value: f32,
}

/// Analog Input Event - 16-bit With Flag and Time
#[derive(Debug, PartialEq)]
pub struct Group32Var4 {
    pub flags: u8,
    pub value: i16,
    pub time: Timestamp,
}

/// Analog Input Event - 32-bit With Flag and Time
#[derive(Debug, PartialEq)]
pub struct Group32Var3 {
    pub flags: u8,
    pub value: i32,
    pub time: Timestamp,
}

/// Analog Input Event - 16-bit With Flag
#[derive(Debug, PartialEq)]
pub struct Group32Var2 {
    pub flags: u8,
    pub value: i16,
}

/// Analog Input Event - 32-bit With Flag
#[derive(Debug, PartialEq)]
pub struct Group32Var1 {
    pub flags: u8,
    pub value: i32,
}

/// Analog Input - Double-precision With Flag
#[derive(Debug, PartialEq)]
pub struct Group30Var6 {
    pub flags: u8,
    pub value: f64,
}

/// Analog Input - Single-precision With Flag
#[derive(Debug, PartialEq)]
pub struct Group30Var5 {
    pub flags: u8,
    pub value: f32,
}

/// Analog Input - 16-bit Without Flag
#[derive(Debug, PartialEq)]
pub struct Group30Var4 {
    pub value: i16,
}

/// Analog Input - 32-bit Without Flag
#[derive(Debug, PartialEq)]
pub struct Group30Var3 {
    pub value: i32,
}

/// Analog Input - 16-bit With Flag
#[derive(Debug, PartialEq)]
pub struct Group30Var2 {
    pub flags: u8,
    pub value: i16,
}

/// Analog Input - 32-bit With Flag
#[derive(Debug, PartialEq)]
pub struct Group30Var1 {
    pub flags: u8,
    pub value: i32,
}

/// Frozen Counter Event - 16-bit With Flag and Time
#[derive(Debug, PartialEq)]
pub struct Group23Var6 {
    pub flags: u8,
    pub value: u16,
    pub time: Timestamp,
}

/// Frozen Counter Event - 32-bit With Flag and Time
#[derive(Debug, PartialEq)]
pub struct Group23Var5 {
    pub flags: u8,
    pub value: u32,
    pub time: Timestamp,
}

/// Frozen Counter Event - 16-bit With Flag
#[derive(Debug, PartialEq)]
pub struct Group23Var2 {
    pub flags: u8,
    pub value: u16,
}

/// Frozen Counter Event - 32-bit With Flag
#[derive(Debug, PartialEq)]
pub struct Group23Var1 {
    pub flags: u8,
    pub value: u32,
}

/// Counter Event - 16-bit With Flag and Time
#[derive(Debug, PartialEq)]
pub struct Group22Var6 {
    pub flags: u8,
    pub value: u16,
    pub time: Timestamp,
}

/// Counter Event - 32-bit With Flag and Time
#[derive(Debug, PartialEq)]
pub struct Group22Var5 {
    pub flags: u8,
    pub value: u32,
    pub time: Timestamp,
}

/// Counter Event - 16-bit With Flag
#[derive(Debug, PartialEq)]
pub struct Group22Var2 {
    pub flags: u8,
    pub value: u16,
}

/// Counter Event - 32-bit With Flag
#[derive(Debug, PartialEq)]
pub struct Group22Var1 {
    pub flags: u8,
    pub value: u32,
}

/// Frozen Counter - 16-bit Without Flag
#[derive(Debug, PartialEq)]
pub struct Group21Var10 {
    pub value: u16,
}

/// Frozen Counter - 32-bit Without Flag
#[derive(Debug, PartialEq)]
pub struct Group21Var9 {
    pub value: u32,
}

/// Frozen Counter - 16-bit With Flag and Time
#[derive(Debug, PartialEq)]
pub struct Group21Var6 {
    pub flags: u8,
    pub value: u16,
    pub time: Timestamp,
}

/// Frozen Counter - 32-bit With Flag and Time
#[derive(Debug, PartialEq)]
pub struct Group21Var5 {
    pub flags: u8,
    pub value: u32,
    pub time: Timestamp,
}

/// Frozen Counter - 16-bit With Flag
#[derive(Debug, PartialEq)]
pub struct Group21Var2 {
    pub flags: u8,
    pub value: u16,
}

/// Frozen Counter - 32-bit With Flag
#[derive(Debug, PartialEq)]
pub struct Group21Var1 {
    pub flags: u8,
    pub value: u32,
}

/// Counter - 16-bit Without Flag
#[derive(Debug, PartialEq)]
pub struct Group20Var6 {
    pub value: u16,
}

/// Counter - 32-bit Without Flag
#[derive(Debug, PartialEq)]
pub struct Group20Var5 {
    pub value: u32,
}

/// Counter - 16-bit With Flag
#[derive(Debug, PartialEq)]
pub struct Group20Var2 {
    pub flags: u8,
    pub value: u16,
}

/// Counter - 32-bit With Flag
#[derive(Debug, PartialEq)]
pub struct Group20Var1 {
    pub flags: u8,
    pub value: u32,
}

/// Binary Command Event - With Time
#[derive(Debug, PartialEq)]
pub struct Group13Var2 {
    pub flags: u8,
    pub time: Timestamp,
}

/// Binary Command Event - Without Time
#[derive(Debug, PartialEq)]
pub struct Group13Var1 {
    pub flags: u8,
}

/// Binary Command - Control Relay Output Block
#[derive(Debug, PartialEq)]
pub struct Group12Var1 {
    pub code: ControlCode,
    pub count: u8,
    pub on_time: u32,
    pub off_time: u32,
    pub status: CommandStatus,
}

/// Binary Output Event - Output Status With Time
#[derive(Debug, PartialEq)]
pub struct Group11Var2 {
    pub flags: u8,
    pub time: Timestamp,
}

/// Binary Output Event - Output Status Without Time
#[derive(Debug, PartialEq)]
pub struct Group11Var1 {
    pub flags: u8,
}

/// Binary Output - Output Status With Flags
#[derive(Debug, PartialEq)]
pub struct Group10Var2 {
    pub flags: u8,
}

/// Double-bit Binary Input Event - With Relative Time
#[derive(Debug, PartialEq)]
pub struct Group4Var3 {
    pub flags: u8,
    pub time: u16,
}

/// Double-bit Binary Input Event - With Absolute Time
#[derive(Debug, PartialEq)]
pub struct Group4Var2 {
    pub flags: u8,
    pub time: Timestamp,
}

/// Double-bit Binary Input Event - Without Time
#[derive(Debug, PartialEq)]
pub struct Group4Var1 {
    pub flags: u8,
}

/// Double-bit Binary Input - With Flags
#[derive(Debug, PartialEq)]
pub struct Group3Var2 {
    pub flags: u8,
}

/// Binary Input Event - With Relative Time
#[derive(Debug, PartialEq)]
pub struct Group2Var3 {
    pub flags: u8,
    pub time: u16,
}

/// Binary Input Event - With Absolute Time
#[derive(Debug, PartialEq)]
pub struct Group2Var2 {
    pub flags: u8,
    pub time: Timestamp,
}

/// Binary Input Event - Without Time
#[derive(Debug, PartialEq)]
pub struct Group2Var1 {
    pub flags: u8,
}

/// Binary Input - With Flags
#[derive(Debug, PartialEq)]
pub struct Group1Var2 {
    pub flags: u8,
}

impl FixedSize for Group52Var2 {
    const SIZE: u8 = 2;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group52Var2 {
                time: cursor.read_u16_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u16_le(self.time)?;
        Ok(())
    }
}

impl FixedSize for Group52Var1 {
    const SIZE: u8 = 2;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group52Var1 {
                time: cursor.read_u16_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u16_le(self.time)?;
        Ok(())
    }
}

impl FixedSize for Group51Var2 {
    const SIZE: u8 = 6;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group51Var2 {
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u48_le(self.time.value)?;
        Ok(())
    }
}

impl FixedSize for Group51Var1 {
    const SIZE: u8 = 6;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group51Var1 {
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u48_le(self.time.value)?;
        Ok(())
    }
}

impl FixedSize for Group50Var4 {
    const SIZE: u8 = 11;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group50Var4 {
                time: Timestamp::new(cursor.read_u48_le()?),
                interval: cursor.read_u32_le()?,
                units: cursor.read_u8()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u48_le(self.time.value)?;
        cursor.write_u32_le(self.interval)?;
        cursor.write_u8(self.units)?;
        Ok(())
    }
}

impl FixedSize for Group50Var3 {
    const SIZE: u8 = 6;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group50Var3 {
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u48_le(self.time.value)?;
        Ok(())
    }
}

impl FixedSize for Group50Var1 {
    const SIZE: u8 = 6;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group50Var1 {
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u48_le(self.time.value)?;
        Ok(())
    }
}

impl FixedSize for Group43Var8 {
    const SIZE: u8 = 15;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group43Var8 {
                status: CommandStatus::from(cursor.read_u8()?),
                value: cursor.read_f64_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.status.as_u8())?;
        cursor.write_f64_le(self.value)?;
        cursor.write_u48_le(self.time.value)?;
        Ok(())
    }
}

impl FixedSize for Group43Var7 {
    const SIZE: u8 = 11;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group43Var7 {
                status: CommandStatus::from(cursor.read_u8()?),
                value: cursor.read_f32_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.status.as_u8())?;
        cursor.write_f32_le(self.value)?;
        cursor.write_u48_le(self.time.value)?;
        Ok(())
    }
}

impl FixedSize for Group43Var6 {
    const SIZE: u8 = 9;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group43Var6 {
                status: CommandStatus::from(cursor.read_u8()?),
                value: cursor.read_f64_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.status.as_u8())?;
        cursor.write_f64_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group43Var5 {
    const SIZE: u8 = 5;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group43Var5 {
                status: CommandStatus::from(cursor.read_u8()?),
                value: cursor.read_f32_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.status.as_u8())?;
        cursor.write_f32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group43Var4 {
    const SIZE: u8 = 9;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group43Var4 {
                status: CommandStatus::from(cursor.read_u8()?),
                value: cursor.read_i16_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.status.as_u8())?;
        cursor.write_i16_le(self.value)?;
        cursor.write_u48_le(self.time.value)?;
        Ok(())
    }
}

impl FixedSize for Group43Var3 {
    const SIZE: u8 = 11;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group43Var3 {
                status: CommandStatus::from(cursor.read_u8()?),
                value: cursor.read_i32_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.status.as_u8())?;
        cursor.write_i32_le(self.value)?;
        cursor.write_u48_le(self.time.value)?;
        Ok(())
    }
}

impl FixedSize for Group43Var2 {
    const SIZE: u8 = 3;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group43Var2 {
                status: CommandStatus::from(cursor.read_u8()?),
                value: cursor.read_i16_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.status.as_u8())?;
        cursor.write_i16_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group43Var1 {
    const SIZE: u8 = 5;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group43Var1 {
                status: CommandStatus::from(cursor.read_u8()?),
                value: cursor.read_i32_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.status.as_u8())?;
        cursor.write_i32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group42Var8 {
    const SIZE: u8 = 15;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group42Var8 {
                flags: cursor.read_u8()?,
                value: cursor.read_f64_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_f64_le(self.value)?;
        cursor.write_u48_le(self.time.value)?;
        Ok(())
    }
}

impl FixedSize for Group42Var7 {
    const SIZE: u8 = 11;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group42Var7 {
                flags: cursor.read_u8()?,
                value: cursor.read_f32_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_f32_le(self.value)?;
        cursor.write_u48_le(self.time.value)?;
        Ok(())
    }
}

impl FixedSize for Group42Var6 {
    const SIZE: u8 = 9;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group42Var6 {
                flags: cursor.read_u8()?,
                value: cursor.read_f64_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_f64_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group42Var5 {
    const SIZE: u8 = 5;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group42Var5 {
                flags: cursor.read_u8()?,
                value: cursor.read_f32_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_f32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group42Var4 {
    const SIZE: u8 = 9;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group42Var4 {
                flags: cursor.read_u8()?,
                value: cursor.read_i16_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_i16_le(self.value)?;
        cursor.write_u48_le(self.time.value)?;
        Ok(())
    }
}

impl FixedSize for Group42Var3 {
    const SIZE: u8 = 11;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group42Var3 {
                flags: cursor.read_u8()?,
                value: cursor.read_i32_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_i32_le(self.value)?;
        cursor.write_u48_le(self.time.value)?;
        Ok(())
    }
}

impl FixedSize for Group42Var2 {
    const SIZE: u8 = 3;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group42Var2 {
                flags: cursor.read_u8()?,
                value: cursor.read_i16_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_i16_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group42Var1 {
    const SIZE: u8 = 5;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group42Var1 {
                flags: cursor.read_u8()?,
                value: cursor.read_i32_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_i32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group41Var4 {
    const SIZE: u8 = 9;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group41Var4 {
                value: cursor.read_f64_le()?,
                status: CommandStatus::from(cursor.read_u8()?),
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_f64_le(self.value)?;
        cursor.write_u8(self.status.as_u8())?;
        Ok(())
    }
}

impl FixedSize for Group41Var3 {
    const SIZE: u8 = 5;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group41Var3 {
                value: cursor.read_f32_le()?,
                status: CommandStatus::from(cursor.read_u8()?),
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_f32_le(self.value)?;
        cursor.write_u8(self.status.as_u8())?;
        Ok(())
    }
}

impl FixedSize for Group41Var2 {
    const SIZE: u8 = 3;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group41Var2 {
                value: cursor.read_i16_le()?,
                status: CommandStatus::from(cursor.read_u8()?),
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_i16_le(self.value)?;
        cursor.write_u8(self.status.as_u8())?;
        Ok(())
    }
}

impl FixedSize for Group41Var1 {
    const SIZE: u8 = 5;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group41Var1 {
                value: cursor.read_i32_le()?,
                status: CommandStatus::from(cursor.read_u8()?),
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_i32_le(self.value)?;
        cursor.write_u8(self.status.as_u8())?;
        Ok(())
    }
}

impl FixedSize for Group40Var4 {
    const SIZE: u8 = 9;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group40Var4 {
                flags: cursor.read_u8()?,
                value: cursor.read_f64_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_f64_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group40Var3 {
    const SIZE: u8 = 5;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group40Var3 {
                flags: cursor.read_u8()?,
                value: cursor.read_f32_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_f32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group40Var2 {
    const SIZE: u8 = 3;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group40Var2 {
                flags: cursor.read_u8()?,
                value: cursor.read_i16_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_i16_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group40Var1 {
    const SIZE: u8 = 5;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group40Var1 {
                flags: cursor.read_u8()?,
                value: cursor.read_i32_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_i32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group32Var8 {
    const SIZE: u8 = 15;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group32Var8 {
                flags: cursor.read_u8()?,
                value: cursor.read_f64_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_f64_le(self.value)?;
        cursor.write_u48_le(self.time.value)?;
        Ok(())
    }
}

impl FixedSize for Group32Var7 {
    const SIZE: u8 = 11;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group32Var7 {
                flags: cursor.read_u8()?,
                value: cursor.read_f32_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_f32_le(self.value)?;
        cursor.write_u48_le(self.time.value)?;
        Ok(())
    }
}

impl FixedSize for Group32Var6 {
    const SIZE: u8 = 9;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group32Var6 {
                flags: cursor.read_u8()?,
                value: cursor.read_f64_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_f64_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group32Var5 {
    const SIZE: u8 = 5;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group32Var5 {
                flags: cursor.read_u8()?,
                value: cursor.read_f32_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_f32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group32Var4 {
    const SIZE: u8 = 9;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group32Var4 {
                flags: cursor.read_u8()?,
                value: cursor.read_i16_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_i16_le(self.value)?;
        cursor.write_u48_le(self.time.value)?;
        Ok(())
    }
}

impl FixedSize for Group32Var3 {
    const SIZE: u8 = 11;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group32Var3 {
                flags: cursor.read_u8()?,
                value: cursor.read_i32_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_i32_le(self.value)?;
        cursor.write_u48_le(self.time.value)?;
        Ok(())
    }
}

impl FixedSize for Group32Var2 {
    const SIZE: u8 = 3;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group32Var2 {
                flags: cursor.read_u8()?,
                value: cursor.read_i16_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_i16_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group32Var1 {
    const SIZE: u8 = 5;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group32Var1 {
                flags: cursor.read_u8()?,
                value: cursor.read_i32_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_i32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group30Var6 {
    const SIZE: u8 = 9;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group30Var6 {
                flags: cursor.read_u8()?,
                value: cursor.read_f64_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_f64_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group30Var5 {
    const SIZE: u8 = 5;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group30Var5 {
                flags: cursor.read_u8()?,
                value: cursor.read_f32_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_f32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group30Var4 {
    const SIZE: u8 = 2;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group30Var4 {
                value: cursor.read_i16_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_i16_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group30Var3 {
    const SIZE: u8 = 4;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group30Var3 {
                value: cursor.read_i32_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_i32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group30Var2 {
    const SIZE: u8 = 3;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group30Var2 {
                flags: cursor.read_u8()?,
                value: cursor.read_i16_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_i16_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group30Var1 {
    const SIZE: u8 = 5;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group30Var1 {
                flags: cursor.read_u8()?,
                value: cursor.read_i32_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_i32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group23Var6 {
    const SIZE: u8 = 9;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group23Var6 {
                flags: cursor.read_u8()?,
                value: cursor.read_u16_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u16_le(self.value)?;
        cursor.write_u48_le(self.time.value)?;
        Ok(())
    }
}

impl FixedSize for Group23Var5 {
    const SIZE: u8 = 11;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group23Var5 {
                flags: cursor.read_u8()?,
                value: cursor.read_u32_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u32_le(self.value)?;
        cursor.write_u48_le(self.time.value)?;
        Ok(())
    }
}

impl FixedSize for Group23Var2 {
    const SIZE: u8 = 3;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group23Var2 {
                flags: cursor.read_u8()?,
                value: cursor.read_u16_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u16_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group23Var1 {
    const SIZE: u8 = 5;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group23Var1 {
                flags: cursor.read_u8()?,
                value: cursor.read_u32_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group22Var6 {
    const SIZE: u8 = 9;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group22Var6 {
                flags: cursor.read_u8()?,
                value: cursor.read_u16_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u16_le(self.value)?;
        cursor.write_u48_le(self.time.value)?;
        Ok(())
    }
}

impl FixedSize for Group22Var5 {
    const SIZE: u8 = 11;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group22Var5 {
                flags: cursor.read_u8()?,
                value: cursor.read_u32_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u32_le(self.value)?;
        cursor.write_u48_le(self.time.value)?;
        Ok(())
    }
}

impl FixedSize for Group22Var2 {
    const SIZE: u8 = 3;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group22Var2 {
                flags: cursor.read_u8()?,
                value: cursor.read_u16_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u16_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group22Var1 {
    const SIZE: u8 = 5;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group22Var1 {
                flags: cursor.read_u8()?,
                value: cursor.read_u32_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group21Var10 {
    const SIZE: u8 = 2;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group21Var10 {
                value: cursor.read_u16_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u16_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group21Var9 {
    const SIZE: u8 = 4;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group21Var9 {
                value: cursor.read_u32_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group21Var6 {
    const SIZE: u8 = 9;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group21Var6 {
                flags: cursor.read_u8()?,
                value: cursor.read_u16_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u16_le(self.value)?;
        cursor.write_u48_le(self.time.value)?;
        Ok(())
    }
}

impl FixedSize for Group21Var5 {
    const SIZE: u8 = 11;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group21Var5 {
                flags: cursor.read_u8()?,
                value: cursor.read_u32_le()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u32_le(self.value)?;
        cursor.write_u48_le(self.time.value)?;
        Ok(())
    }
}

impl FixedSize for Group21Var2 {
    const SIZE: u8 = 3;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group21Var2 {
                flags: cursor.read_u8()?,
                value: cursor.read_u16_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u16_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group21Var1 {
    const SIZE: u8 = 5;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group21Var1 {
                flags: cursor.read_u8()?,
                value: cursor.read_u32_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group20Var6 {
    const SIZE: u8 = 2;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group20Var6 {
                value: cursor.read_u16_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u16_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group20Var5 {
    const SIZE: u8 = 4;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group20Var5 {
                value: cursor.read_u32_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group20Var2 {
    const SIZE: u8 = 3;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group20Var2 {
                flags: cursor.read_u8()?,
                value: cursor.read_u16_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u16_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group20Var1 {
    const SIZE: u8 = 5;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group20Var1 {
                flags: cursor.read_u8()?,
                value: cursor.read_u32_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u32_le(self.value)?;
        Ok(())
    }
}

impl FixedSize for Group13Var2 {
    const SIZE: u8 = 7;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group13Var2 {
                flags: cursor.read_u8()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u48_le(self.time.value)?;
        Ok(())
    }
}

impl FixedSize for Group13Var1 {
    const SIZE: u8 = 1;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group13Var1 {
                flags: cursor.read_u8()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        Ok(())
    }
}

impl FixedSize for Group12Var1 {
    const SIZE: u8 = 11;
    #[rustfmt::skip]
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
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.code.as_u8())?;
        cursor.write_u8(self.count)?;
        cursor.write_u32_le(self.on_time)?;
        cursor.write_u32_le(self.off_time)?;
        cursor.write_u8(self.status.as_u8())?;
        Ok(())
    }
}

impl FixedSize for Group11Var2 {
    const SIZE: u8 = 7;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group11Var2 {
                flags: cursor.read_u8()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u48_le(self.time.value)?;
        Ok(())
    }
}

impl FixedSize for Group11Var1 {
    const SIZE: u8 = 1;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group11Var1 {
                flags: cursor.read_u8()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        Ok(())
    }
}

impl FixedSize for Group10Var2 {
    const SIZE: u8 = 1;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group10Var2 {
                flags: cursor.read_u8()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        Ok(())
    }
}

impl FixedSize for Group4Var3 {
    const SIZE: u8 = 3;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group4Var3 {
                flags: cursor.read_u8()?,
                time: cursor.read_u16_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u16_le(self.time)?;
        Ok(())
    }
}

impl FixedSize for Group4Var2 {
    const SIZE: u8 = 7;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group4Var2 {
                flags: cursor.read_u8()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u48_le(self.time.value)?;
        Ok(())
    }
}

impl FixedSize for Group4Var1 {
    const SIZE: u8 = 1;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group4Var1 {
                flags: cursor.read_u8()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        Ok(())
    }
}

impl FixedSize for Group3Var2 {
    const SIZE: u8 = 1;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group3Var2 {
                flags: cursor.read_u8()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        Ok(())
    }
}

impl FixedSize for Group2Var3 {
    const SIZE: u8 = 3;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group2Var3 {
                flags: cursor.read_u8()?,
                time: cursor.read_u16_le()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u16_le(self.time)?;
        Ok(())
    }
}

impl FixedSize for Group2Var2 {
    const SIZE: u8 = 7;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group2Var2 {
                flags: cursor.read_u8()?,
                time: Timestamp::new(cursor.read_u48_le()?),
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        cursor.write_u48_le(self.time.value)?;
        Ok(())
    }
}

impl FixedSize for Group2Var1 {
    const SIZE: u8 = 1;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group2Var1 {
                flags: cursor.read_u8()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        Ok(())
    }
}

impl FixedSize for Group1Var2 {
    const SIZE: u8 = 1;
    #[rustfmt::skip]
    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group1Var2 {
                flags: cursor.read_u8()?,
            }
        )
    }
    #[rustfmt::skip]
    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.flags)?;
        Ok(())
    }
}
