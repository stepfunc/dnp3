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

use crate::app::header::FixedSizeVariation;
use crate::util::cursor::{ReadCursor, ReadError};

#[derive(Debug, PartialEq)]
pub struct Group1Var2 {
    pub flags: u8,
}
#[derive(Debug, PartialEq)]
pub struct Group2Var1 {
    pub flags: u8,
}
#[derive(Debug, PartialEq)]
pub struct Group2Var2 {
    pub flags: u8,
    pub time: u64,
}
#[derive(Debug, PartialEq)]
pub struct Group2Var3 {
    pub flags: u8,
    pub time: u16,
}
#[derive(Debug, PartialEq)]
pub struct Group3Var2 {
    pub flags: u8,
}
#[derive(Debug, PartialEq)]
pub struct Group4Var1 {
    pub flags: u8,
}
#[derive(Debug, PartialEq)]
pub struct Group4Var2 {
    pub flags: u8,
    pub time: u64,
}
#[derive(Debug, PartialEq)]
pub struct Group4Var3 {
    pub flags: u8,
    pub time: u16,
}
#[derive(Debug, PartialEq)]
pub struct Group10Var2 {
    pub flags: u8,
}
#[derive(Debug, PartialEq)]
pub struct Group11Var1 {
    pub flags: u8,
}
#[derive(Debug, PartialEq)]
pub struct Group11Var2 {
    pub flags: u8,
    pub time: u64,
}
#[derive(Debug, PartialEq)]
pub struct Group12Var1 {
    pub code: u8,
    pub count: u8,
    pub on_time: u32,
    pub off_time: u32,
    pub status: u8,
}
#[derive(Debug, PartialEq)]
pub struct Group13Var1 {
    pub flags: u8,
}
#[derive(Debug, PartialEq)]
pub struct Group13Var2 {
    pub flags: u8,
    pub time: u64,
}
#[derive(Debug, PartialEq)]
pub struct Group20Var1 {
    pub flags: u8,
    pub value: u32,
}
#[derive(Debug, PartialEq)]
pub struct Group20Var2 {
    pub flags: u8,
    pub value: u16,
}
#[derive(Debug, PartialEq)]
pub struct Group20Var5 {
    pub value: u32,
}
#[derive(Debug, PartialEq)]
pub struct Group20Var6 {
    pub value: u16,
}
#[derive(Debug, PartialEq)]
pub struct Group21Var1 {
    pub flags: u8,
    pub value: u32,
}
#[derive(Debug, PartialEq)]
pub struct Group21Var2 {
    pub flags: u8,
    pub value: u16,
}
#[derive(Debug, PartialEq)]
pub struct Group21Var5 {
    pub flags: u8,
    pub value: u32,
    pub time: u64,
}
#[derive(Debug, PartialEq)]
pub struct Group21Var6 {
    pub flags: u8,
    pub value: u16,
    pub time: u64,
}
#[derive(Debug, PartialEq)]
pub struct Group21Var9 {
    pub value: u32,
}
#[derive(Debug, PartialEq)]
pub struct Group21Var10 {
    pub value: u16,
}
#[derive(Debug, PartialEq)]
pub struct Group22Var1 {
    pub flags: u8,
    pub value: u32,
}
#[derive(Debug, PartialEq)]
pub struct Group22Var2 {
    pub flags: u8,
    pub value: u16,
}
#[derive(Debug, PartialEq)]
pub struct Group22Var5 {
    pub flags: u8,
    pub value: u32,
    pub time: u64,
}
#[derive(Debug, PartialEq)]
pub struct Group22Var6 {
    pub flags: u8,
    pub value: u16,
    pub time: u64,
}
#[derive(Debug, PartialEq)]
pub struct Group23Var1 {
    pub flags: u8,
    pub value: u32,
}
#[derive(Debug, PartialEq)]
pub struct Group23Var2 {
    pub flags: u8,
    pub value: u16,
}
#[derive(Debug, PartialEq)]
pub struct Group23Var5 {
    pub flags: u8,
    pub value: u32,
    pub time: u64,
}
#[derive(Debug, PartialEq)]
pub struct Group23Var6 {
    pub flags: u8,
    pub value: u16,
    pub time: u64,
}
#[derive(Debug, PartialEq)]
pub struct Group30Var1 {
    pub flags: u8,
    pub value: i32,
}
#[derive(Debug, PartialEq)]
pub struct Group30Var2 {
    pub flags: u8,
    pub value: i16,
}
#[derive(Debug, PartialEq)]
pub struct Group30Var3 {
    pub value: i32,
}
#[derive(Debug, PartialEq)]
pub struct Group30Var4 {
    pub value: i16,
}
#[derive(Debug, PartialEq)]
pub struct Group30Var5 {
    pub flags: u8,
    pub value: f32,
}
#[derive(Debug, PartialEq)]
pub struct Group30Var6 {
    pub flags: u8,
    pub value: f64,
}
#[derive(Debug, PartialEq)]
pub struct Group32Var1 {
    pub flags: u8,
    pub value: i32,
}
#[derive(Debug, PartialEq)]
pub struct Group32Var2 {
    pub flags: u8,
    pub value: i16,
}
#[derive(Debug, PartialEq)]
pub struct Group32Var3 {
    pub flags: u8,
    pub value: i32,
    pub time: u64,
}
#[derive(Debug, PartialEq)]
pub struct Group32Var4 {
    pub flags: u8,
    pub value: i16,
    pub time: u64,
}
#[derive(Debug, PartialEq)]
pub struct Group32Var5 {
    pub flags: u8,
    pub value: f32,
}
#[derive(Debug, PartialEq)]
pub struct Group32Var6 {
    pub flags: u8,
    pub value: f64,
}
#[derive(Debug, PartialEq)]
pub struct Group32Var7 {
    pub flags: u8,
    pub value: f32,
    pub time: u64,
}
#[derive(Debug, PartialEq)]
pub struct Group32Var8 {
    pub flags: u8,
    pub value: f64,
    pub time: u64,
}
#[derive(Debug, PartialEq)]
pub struct Group40Var1 {
    pub flags: u8,
    pub value: i32,
}
#[derive(Debug, PartialEq)]
pub struct Group40Var2 {
    pub flags: u8,
    pub value: i16,
}
#[derive(Debug, PartialEq)]
pub struct Group40Var3 {
    pub flags: u8,
    pub value: f32,
}
#[derive(Debug, PartialEq)]
pub struct Group40Var4 {
    pub flags: u8,
    pub value: f64,
}
#[derive(Debug, PartialEq)]
pub struct Group41Var1 {
    pub value: i32,
    pub status: u8,
}
#[derive(Debug, PartialEq)]
pub struct Group41Var2 {
    pub value: i16,
    pub status: u8,
}
#[derive(Debug, PartialEq)]
pub struct Group41Var3 {
    pub value: f32,
    pub status: u8,
}
#[derive(Debug, PartialEq)]
pub struct Group41Var4 {
    pub value: f64,
    pub status: u8,
}
#[derive(Debug, PartialEq)]
pub struct Group42Var1 {
    pub flags: u8,
    pub value: i32,
}
#[derive(Debug, PartialEq)]
pub struct Group42Var2 {
    pub flags: u8,
    pub value: i16,
}
#[derive(Debug, PartialEq)]
pub struct Group42Var3 {
    pub flags: u8,
    pub value: i32,
    pub time: u64,
}
#[derive(Debug, PartialEq)]
pub struct Group42Var4 {
    pub flags: u8,
    pub value: i16,
    pub time: u64,
}
#[derive(Debug, PartialEq)]
pub struct Group42Var5 {
    pub flags: u8,
    pub value: f32,
}
#[derive(Debug, PartialEq)]
pub struct Group42Var6 {
    pub flags: u8,
    pub value: f64,
}
#[derive(Debug, PartialEq)]
pub struct Group42Var7 {
    pub flags: u8,
    pub value: f32,
    pub time: u64,
}
#[derive(Debug, PartialEq)]
pub struct Group42Var8 {
    pub flags: u8,
    pub value: f64,
    pub time: u64,
}
#[derive(Debug, PartialEq)]
pub struct Group43Var1 {
    pub status: u8,
    pub value: i32,
}
#[derive(Debug, PartialEq)]
pub struct Group43Var2 {
    pub status: u8,
    pub value: i16,
}
#[derive(Debug, PartialEq)]
pub struct Group43Var3 {
    pub status: u8,
    pub value: i32,
    pub time: u64,
}
#[derive(Debug, PartialEq)]
pub struct Group43Var4 {
    pub status: u8,
    pub value: i16,
    pub time: u64,
}
#[derive(Debug, PartialEq)]
pub struct Group43Var5 {
    pub status: u8,
    pub value: f32,
}
#[derive(Debug, PartialEq)]
pub struct Group43Var6 {
    pub status: u8,
    pub value: f64,
}
#[derive(Debug, PartialEq)]
pub struct Group43Var7 {
    pub status: u8,
    pub value: f32,
    pub time: u64,
}
#[derive(Debug, PartialEq)]
pub struct Group43Var8 {
    pub status: u8,
    pub value: f64,
    pub time: u64,
}
#[derive(Debug, PartialEq)]
pub struct Group50Var1 {
    pub time: u64,
}
#[derive(Debug, PartialEq)]
pub struct Group50Var3 {
    pub time: u64,
}
#[derive(Debug, PartialEq)]
pub struct Group50Var4 {
    pub time: u64,
    pub interval: u32,
    pub units: u8,
}
#[derive(Debug, PartialEq)]
pub struct Group51Var1 {
    pub time: u64,
}
#[derive(Debug, PartialEq)]
pub struct Group51Var2 {
    pub time: u64,
}
#[derive(Debug, PartialEq)]
pub struct Group52Var1 {
    pub time: u16,
}
#[derive(Debug, PartialEq)]
pub struct Group52Var2 {
    pub time: u16,
}
impl FixedSizeVariation for Group1Var2 {
    const SIZE: u8 = 1;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group1Var2 {
                flags: cursor.read_u8()?,
            }
        )
    }
}
impl FixedSizeVariation for Group2Var1 {
    const SIZE: u8 = 1;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group2Var1 {
                flags: cursor.read_u8()?,
            }
        )
    }
}
impl FixedSizeVariation for Group2Var2 {
    const SIZE: u8 = 7;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group2Var2 {
                flags: cursor.read_u8()?,
                time: cursor.read_u48_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group2Var3 {
    const SIZE: u8 = 3;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group2Var3 {
                flags: cursor.read_u8()?,
                time: cursor.read_u16_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group3Var2 {
    const SIZE: u8 = 1;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group3Var2 {
                flags: cursor.read_u8()?,
            }
        )
    }
}
impl FixedSizeVariation for Group4Var1 {
    const SIZE: u8 = 1;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group4Var1 {
                flags: cursor.read_u8()?,
            }
        )
    }
}
impl FixedSizeVariation for Group4Var2 {
    const SIZE: u8 = 7;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group4Var2 {
                flags: cursor.read_u8()?,
                time: cursor.read_u48_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group4Var3 {
    const SIZE: u8 = 3;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group4Var3 {
                flags: cursor.read_u8()?,
                time: cursor.read_u16_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group10Var2 {
    const SIZE: u8 = 1;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group10Var2 {
                flags: cursor.read_u8()?,
            }
        )
    }
}
impl FixedSizeVariation for Group11Var1 {
    const SIZE: u8 = 1;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group11Var1 {
                flags: cursor.read_u8()?,
            }
        )
    }
}
impl FixedSizeVariation for Group11Var2 {
    const SIZE: u8 = 7;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group11Var2 {
                flags: cursor.read_u8()?,
                time: cursor.read_u48_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group12Var1 {
    const SIZE: u8 = 11;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group12Var1 {
                code: cursor.read_u8()?,
                count: cursor.read_u8()?,
                on_time: cursor.read_u32_le()?,
                off_time: cursor.read_u32_le()?,
                status: cursor.read_u8()?,
            }
        )
    }
}
impl FixedSizeVariation for Group13Var1 {
    const SIZE: u8 = 1;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group13Var1 {
                flags: cursor.read_u8()?,
            }
        )
    }
}
impl FixedSizeVariation for Group13Var2 {
    const SIZE: u8 = 7;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group13Var2 {
                flags: cursor.read_u8()?,
                time: cursor.read_u48_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group20Var1 {
    const SIZE: u8 = 5;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group20Var1 {
                flags: cursor.read_u8()?,
                value: cursor.read_u32_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group20Var2 {
    const SIZE: u8 = 3;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group20Var2 {
                flags: cursor.read_u8()?,
                value: cursor.read_u16_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group20Var5 {
    const SIZE: u8 = 4;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group20Var5 {
                value: cursor.read_u32_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group20Var6 {
    const SIZE: u8 = 2;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group20Var6 {
                value: cursor.read_u16_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group21Var1 {
    const SIZE: u8 = 5;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group21Var1 {
                flags: cursor.read_u8()?,
                value: cursor.read_u32_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group21Var2 {
    const SIZE: u8 = 3;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group21Var2 {
                flags: cursor.read_u8()?,
                value: cursor.read_u16_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group21Var5 {
    const SIZE: u8 = 11;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group21Var5 {
                flags: cursor.read_u8()?,
                value: cursor.read_u32_le()?,
                time: cursor.read_u48_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group21Var6 {
    const SIZE: u8 = 9;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group21Var6 {
                flags: cursor.read_u8()?,
                value: cursor.read_u16_le()?,
                time: cursor.read_u48_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group21Var9 {
    const SIZE: u8 = 4;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group21Var9 {
                value: cursor.read_u32_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group21Var10 {
    const SIZE: u8 = 2;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group21Var10 {
                value: cursor.read_u16_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group22Var1 {
    const SIZE: u8 = 5;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group22Var1 {
                flags: cursor.read_u8()?,
                value: cursor.read_u32_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group22Var2 {
    const SIZE: u8 = 3;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group22Var2 {
                flags: cursor.read_u8()?,
                value: cursor.read_u16_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group22Var5 {
    const SIZE: u8 = 11;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group22Var5 {
                flags: cursor.read_u8()?,
                value: cursor.read_u32_le()?,
                time: cursor.read_u48_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group22Var6 {
    const SIZE: u8 = 9;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group22Var6 {
                flags: cursor.read_u8()?,
                value: cursor.read_u16_le()?,
                time: cursor.read_u48_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group23Var1 {
    const SIZE: u8 = 5;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group23Var1 {
                flags: cursor.read_u8()?,
                value: cursor.read_u32_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group23Var2 {
    const SIZE: u8 = 3;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group23Var2 {
                flags: cursor.read_u8()?,
                value: cursor.read_u16_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group23Var5 {
    const SIZE: u8 = 11;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group23Var5 {
                flags: cursor.read_u8()?,
                value: cursor.read_u32_le()?,
                time: cursor.read_u48_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group23Var6 {
    const SIZE: u8 = 9;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group23Var6 {
                flags: cursor.read_u8()?,
                value: cursor.read_u16_le()?,
                time: cursor.read_u48_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group30Var1 {
    const SIZE: u8 = 5;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group30Var1 {
                flags: cursor.read_u8()?,
                value: cursor.read_i32_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group30Var2 {
    const SIZE: u8 = 3;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group30Var2 {
                flags: cursor.read_u8()?,
                value: cursor.read_i16_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group30Var3 {
    const SIZE: u8 = 4;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group30Var3 {
                value: cursor.read_i32_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group30Var4 {
    const SIZE: u8 = 2;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group30Var4 {
                value: cursor.read_i16_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group30Var5 {
    const SIZE: u8 = 5;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group30Var5 {
                flags: cursor.read_u8()?,
                value: cursor.read_f32_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group30Var6 {
    const SIZE: u8 = 9;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group30Var6 {
                flags: cursor.read_u8()?,
                value: cursor.read_f64_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group32Var1 {
    const SIZE: u8 = 5;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group32Var1 {
                flags: cursor.read_u8()?,
                value: cursor.read_i32_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group32Var2 {
    const SIZE: u8 = 3;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group32Var2 {
                flags: cursor.read_u8()?,
                value: cursor.read_i16_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group32Var3 {
    const SIZE: u8 = 11;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group32Var3 {
                flags: cursor.read_u8()?,
                value: cursor.read_i32_le()?,
                time: cursor.read_u48_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group32Var4 {
    const SIZE: u8 = 9;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group32Var4 {
                flags: cursor.read_u8()?,
                value: cursor.read_i16_le()?,
                time: cursor.read_u48_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group32Var5 {
    const SIZE: u8 = 5;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group32Var5 {
                flags: cursor.read_u8()?,
                value: cursor.read_f32_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group32Var6 {
    const SIZE: u8 = 9;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group32Var6 {
                flags: cursor.read_u8()?,
                value: cursor.read_f64_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group32Var7 {
    const SIZE: u8 = 11;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group32Var7 {
                flags: cursor.read_u8()?,
                value: cursor.read_f32_le()?,
                time: cursor.read_u48_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group32Var8 {
    const SIZE: u8 = 15;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group32Var8 {
                flags: cursor.read_u8()?,
                value: cursor.read_f64_le()?,
                time: cursor.read_u48_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group40Var1 {
    const SIZE: u8 = 5;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group40Var1 {
                flags: cursor.read_u8()?,
                value: cursor.read_i32_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group40Var2 {
    const SIZE: u8 = 3;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group40Var2 {
                flags: cursor.read_u8()?,
                value: cursor.read_i16_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group40Var3 {
    const SIZE: u8 = 5;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group40Var3 {
                flags: cursor.read_u8()?,
                value: cursor.read_f32_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group40Var4 {
    const SIZE: u8 = 9;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group40Var4 {
                flags: cursor.read_u8()?,
                value: cursor.read_f64_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group41Var1 {
    const SIZE: u8 = 5;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group41Var1 {
                value: cursor.read_i32_le()?,
                status: cursor.read_u8()?,
            }
        )
    }
}
impl FixedSizeVariation for Group41Var2 {
    const SIZE: u8 = 3;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group41Var2 {
                value: cursor.read_i16_le()?,
                status: cursor.read_u8()?,
            }
        )
    }
}
impl FixedSizeVariation for Group41Var3 {
    const SIZE: u8 = 5;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group41Var3 {
                value: cursor.read_f32_le()?,
                status: cursor.read_u8()?,
            }
        )
    }
}
impl FixedSizeVariation for Group41Var4 {
    const SIZE: u8 = 9;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group41Var4 {
                value: cursor.read_f64_le()?,
                status: cursor.read_u8()?,
            }
        )
    }
}
impl FixedSizeVariation for Group42Var1 {
    const SIZE: u8 = 5;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group42Var1 {
                flags: cursor.read_u8()?,
                value: cursor.read_i32_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group42Var2 {
    const SIZE: u8 = 3;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group42Var2 {
                flags: cursor.read_u8()?,
                value: cursor.read_i16_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group42Var3 {
    const SIZE: u8 = 11;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group42Var3 {
                flags: cursor.read_u8()?,
                value: cursor.read_i32_le()?,
                time: cursor.read_u48_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group42Var4 {
    const SIZE: u8 = 9;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group42Var4 {
                flags: cursor.read_u8()?,
                value: cursor.read_i16_le()?,
                time: cursor.read_u48_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group42Var5 {
    const SIZE: u8 = 5;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group42Var5 {
                flags: cursor.read_u8()?,
                value: cursor.read_f32_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group42Var6 {
    const SIZE: u8 = 9;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group42Var6 {
                flags: cursor.read_u8()?,
                value: cursor.read_f64_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group42Var7 {
    const SIZE: u8 = 11;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group42Var7 {
                flags: cursor.read_u8()?,
                value: cursor.read_f32_le()?,
                time: cursor.read_u48_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group42Var8 {
    const SIZE: u8 = 15;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group42Var8 {
                flags: cursor.read_u8()?,
                value: cursor.read_f64_le()?,
                time: cursor.read_u48_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group43Var1 {
    const SIZE: u8 = 5;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group43Var1 {
                status: cursor.read_u8()?,
                value: cursor.read_i32_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group43Var2 {
    const SIZE: u8 = 3;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group43Var2 {
                status: cursor.read_u8()?,
                value: cursor.read_i16_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group43Var3 {
    const SIZE: u8 = 11;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group43Var3 {
                status: cursor.read_u8()?,
                value: cursor.read_i32_le()?,
                time: cursor.read_u48_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group43Var4 {
    const SIZE: u8 = 9;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group43Var4 {
                status: cursor.read_u8()?,
                value: cursor.read_i16_le()?,
                time: cursor.read_u48_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group43Var5 {
    const SIZE: u8 = 5;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group43Var5 {
                status: cursor.read_u8()?,
                value: cursor.read_f32_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group43Var6 {
    const SIZE: u8 = 9;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group43Var6 {
                status: cursor.read_u8()?,
                value: cursor.read_f64_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group43Var7 {
    const SIZE: u8 = 11;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group43Var7 {
                status: cursor.read_u8()?,
                value: cursor.read_f32_le()?,
                time: cursor.read_u48_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group43Var8 {
    const SIZE: u8 = 15;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group43Var8 {
                status: cursor.read_u8()?,
                value: cursor.read_f64_le()?,
                time: cursor.read_u48_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group50Var1 {
    const SIZE: u8 = 6;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group50Var1 {
                time: cursor.read_u48_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group50Var3 {
    const SIZE: u8 = 6;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group50Var3 {
                time: cursor.read_u48_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group50Var4 {
    const SIZE: u8 = 11;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group50Var4 {
                time: cursor.read_u48_le()?,
                interval: cursor.read_u32_le()?,
                units: cursor.read_u8()?,
            }
        )
    }
}
impl FixedSizeVariation for Group51Var1 {
    const SIZE: u8 = 6;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group51Var1 {
                time: cursor.read_u48_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group51Var2 {
    const SIZE: u8 = 6;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group51Var2 {
                time: cursor.read_u48_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group52Var1 {
    const SIZE: u8 = 2;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group52Var1 {
                time: cursor.read_u16_le()?,
            }
        )
    }
}
impl FixedSizeVariation for Group52Var2 {
    const SIZE: u8 = 2;
    #[rustfmt::skip]
    fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(
            Group52Var2 {
                time: cursor.read_u16_le()?,
            }
        )
    }
}
