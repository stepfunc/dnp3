use super::permissions::*;
use super::*;
use crate::app::format::WriteError;
use crate::app::Timestamp;
use crate::master::FileMode;
use scursor::{ReadCursor, WriteCursor};

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

    pub(crate) fn format(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "\nfile name: {}", self.file_name)?;
        write!(
            f,
            "\ntime of creation: {}",
            self.time_of_creation.raw_value()
        )?;
        write!(f, "\npermissions: {}", self.permissions)?;
        write!(f, "\nauth key: {}", self.auth_key)?;
        write!(f, "\nfile size: {}", self.file_size)?;
        write!(f, "\noperational mode: {:?}", self.mode)?;
        write!(f, "\nmax block size: {}", self.max_block_size)?;
        write!(f, "\nrequest id: {}", self.request_id)?;
        Ok(())
    }

    pub(crate) fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u16_le(Self::FILE_NAME_OFFSET)?;
        cursor.write_u16_le(byte_length(self.file_name)?)?;
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

    const OBJECT: Group70Var3 = Group70Var3 {
        time_of_creation: Timestamp::new(0xAABBCCDDEEFF),
        permissions: Permissions {
            world: PermissionSet {
                execute: true,
                write: false,
                read: true,
            },
            group: PermissionSet {
                execute: false,
                write: true,
                read: false,
            },
            owner: PermissionSet {
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

    const DATA: &[u8] = &[
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
    fn writes_valid_object() {
        let mut buffer = [0; 64];

        let mut cursor = WriteCursor::new(&mut buffer);
        OBJECT.write(&mut cursor).unwrap();

        assert_eq!(cursor.written(), DATA)
    }

    #[test]
    fn parses_valid_object() {
        let mut cursor = ReadCursor::new(DATA);
        let obj = Group70Var3::read(&mut cursor).unwrap();

        assert_eq!(obj, OBJECT);
        assert!(cursor.is_empty());
    }
}
