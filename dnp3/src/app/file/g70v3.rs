use super::*;
use crate::app::Timestamp;
use scursor::{ReadCursor, WriteCursor};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Permission {
    pub(crate) execute: bool,
    pub(crate) write: bool,
    pub(crate) read: bool,
}

impl Permission {
    pub(crate) fn all() -> Self {
        Self {
            execute: true,
            write: true,
            read: true,
        }
    }

    fn value(self) -> u16 {
        let mut x = 0;
        if self.execute {
            x |= 0b001;
        }
        if self.write {
            x |= 0b010;
        }
        if self.read {
            x |= 0b100;
        }
        x
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Permissions {
    pub(crate) world: Permission,
    pub(crate) group: Permission,
    pub(crate) owner: Permission,
}

#[derive(Copy, Clone)]
struct Mask(u16);

impl Mask {
    const fn new(bit: u8) -> Self {
        Self(1 << bit)
    }
    fn is_set(self, value: u16) -> bool {
        self.0 & value != 0
    }
}

impl Permissions {
    const WE: Mask = Mask::new(0);
    const WW: Mask = Mask::new(1);
    const WR: Mask = Mask::new(2);

    const GE: Mask = Mask::new(3);
    const GW: Mask = Mask::new(4);
    const GR: Mask = Mask::new(5);

    const OE: Mask = Mask::new(6);
    const OW: Mask = Mask::new(7);
    const OR: Mask = Mask::new(8);

    fn value(self) -> u16 {
        self.world.value() | self.group.value() << 3 | self.owner.value() << 6
    }

    pub(crate) fn write(&self, cursor: &mut WriteCursor) -> Result<(), scursor::WriteError> {
        cursor.write_u16_le(self.value())
    }

    pub(crate) fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        let bits = cursor.read_u16_le()?;
        Ok(Self {
            world: Permission {
                execute: Self::WE.is_set(bits),
                write: Self::WW.is_set(bits),
                read: Self::WR.is_set(bits),
            },
            group: Permission {
                execute: Self::GE.is_set(bits),
                write: Self::GW.is_set(bits),
                read: Self::GR.is_set(bits),
            },
            owner: Permission {
                execute: Self::OE.is_set(bits),
                write: Self::OW.is_set(bits),
                read: Self::OR.is_set(bits),
            },
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum FileMode {
    /// Code used for non-open command requests
    Null,
    /// Specifies that an existing file is to be opened for reading
    Read,
    /// Specifies that the file is to be opened for writing, truncating any existing file to length 0
    Write,
    /// Specifies that the file is to be opened for writing, appending to the end of the file
    Append,
    /// Used to capture reserved values
    Reserved(u16),
}

impl FileMode {
    fn new(value: u16) -> Self {
        match value {
            0 => Self::Null,
            1 => Self::Read,
            2 => Self::Write,
            3 => Self::Append,
            _ => Self::Reserved(value),
        }
    }

    fn to_u16(self) -> u16 {
        match self {
            FileMode::Null => 0,
            FileMode::Read => 1,
            FileMode::Write => 2,
            FileMode::Append => 3,
            FileMode::Reserved(x) => x,
        }
    }
}

/// Group 70 Variation 3 - file command
///
/// This representation is borrowed from the underlying ASDU
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Group70Var3<'a> {
    pub(crate) time_of_creation: Timestamp,
    pub(crate) permissions: Permissions,
    pub(crate) auth_key: u32,
    pub(crate) file_size: u32,
    pub(crate) mode: FileMode,
    pub(crate) max_block_size: u16,
    pub(crate) request_id: u16,
    pub(crate) file_name: &'a str,
}

impl<'a> Group70Var3<'a> {
    // why on earth have these constants in the protocol?
    const FILE_NAME_OFFSET: u16 = 26;

    pub(crate) fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u16_le(Self::FILE_NAME_OFFSET)?;
        cursor.write_u16_le(length(self.file_name)?)?;
        self.time_of_creation.write(cursor)?;
        self.permissions.write(cursor)?;
        cursor.write_u32_le(self.auth_key)?;
        cursor.write_u32_le(self.file_size)?;
        cursor.write_u16_le(self.mode.to_u16())?;
        cursor.write_u16_le(self.max_block_size)?;
        cursor.write_u16_le(self.request_id)?;
        cursor.write_bytes(self.file_name.as_bytes())?;
        Ok(())
    }

    pub(crate) fn read(cursor: &mut ReadCursor<'a>) -> Result<Self, ReadError> {
        let file_name_offset = cursor.read_u16_le()?;
        if file_name_offset != Self::FILE_NAME_OFFSET {
            return Err(ReadError::BadOffset {
                expected: Self::FILE_NAME_OFFSET,
                actual: file_name_offset,
            });
        }
        let file_name_length = cursor.read_u16_le()?;

        let time_of_creation = Timestamp::read(cursor)?;
        let permissions = Permissions::read(cursor)?;
        let auth_key = cursor.read_u32_le()?;
        let file_size = cursor.read_u32_le()?;
        let mode = FileMode::new(cursor.read_u16_le()?);
        let max_block_size = cursor.read_u16_le()?;
        let request_id = cursor.read_u16_le()?;
        let file_name_bytes = cursor.read_bytes(file_name_length as usize)?;
        let file_name = std::str::from_utf8(file_name_bytes)?;

        Ok(Self {
            time_of_creation,
            permissions,
            auth_key,
            file_size,
            mode,
            max_block_size,
            request_id,
            file_name,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const VALID_G70V3: Group70Var3 = Group70Var3 {
        time_of_creation: Timestamp::new(0xAABBCCDDEEFF),
        permissions: Permissions {
            world: Permission {
                execute: true,
                write: false,
                read: true,
            },
            group: Permission {
                execute: false,
                write: true,
                read: false,
            },
            owner: Permission {
                execute: true,
                write: false,
                read: true,
            },
        },
        auth_key: 0xDEADCAFE,
        file_size: 0xAABBCCDD,
        mode: FileMode::Append,
        max_block_size: 42,
        request_id: 0xEEFF,
        file_name: "secrets.txt",
    };

    const VALID_G70V3_DATA: &[u8] = &[
        26, // filename string offset - always 12
        0, 11, // filename string size
        0, 0xFF, //  time of creation
        0xEE, 0xDD, 0xCC, 0xBB, 0xAA, 0b01010101, // permissions
        0b00000001, 0xFE, // authentication key
        0xCA, 0xAD, 0xDE, 0xDD, // file size
        0xCC, 0xBB, 0xAA, 03, // mode
        00, 42, // max block size
        00, 0xFF, // request id
        0xEE, b's', // filename
        b'e', b'c', b'r', b'e', b't', b's', b'.', b't', b'x', b't',
    ];

    #[test]
    fn calculates_permission_bytes() {
        let all = Permissions {
            world: Permission::all(),
            group: Permission::all(),
            owner: Permission::all(),
        };

        assert_eq!(all.value(), 0x1FF);
    }

    #[test]
    fn writes_valid_g70v3() {
        let mut buffer = [0; 64];

        let mut cursor = WriteCursor::new(&mut buffer);
        VALID_G70V3.write(&mut cursor).unwrap();

        assert_eq!(cursor.written(), VALID_G70V3_DATA)
    }

    #[test]
    fn parses_valid_g70v3() {
        let mut cursor = ReadCursor::new(VALID_G70V3_DATA);
        let obj = Group70Var3::read(&mut cursor).unwrap();

        assert_eq!(obj, VALID_G70V3);
        assert!(cursor.is_empty());
    }
}
