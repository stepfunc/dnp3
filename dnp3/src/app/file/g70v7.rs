use super::*;
use crate::app::format::WriteError;
use crate::app::Timestamp;

/// Group 70 Variation 7 - file descriptor
///
/// This representation is borrowed from the underlying ASDU
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Group70Var7<'a> {
    pub(crate) file_type: FileType,
    pub(crate) file_size: u32,
    pub(crate) time_of_creation: Timestamp,
    pub(crate) permissions: Permissions,
    pub(crate) request_id: u16,
    pub(crate) file_name: &'a str,
}

impl<'a> Group70Var7<'a> {
    // why on earth have these constants in the protocol?
    const FILE_NAME_OFFSET: u16 = 20;

    pub(crate) fn format(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "\nfile name: {}", self.file_name)?;
        write!(f, "\nfile type: {:?}", self.file_type)?;
        write!(f, "\nfile size: {}", self.file_size)?;
        write!(
            f,
            "\ntime of creation: {}",
            self.time_of_creation.raw_value()
        )?;
        write!(f, "\npermissions: {}", self.permissions)?;
        write!(f, "\nrequest id: {}", self.request_id)?;
        Ok(())
    }

    pub(crate) fn write(&self, cursor: &mut scursor::WriteCursor) -> Result<(), WriteError> {
        cursor.write_u16_le(Self::FILE_NAME_OFFSET)?;
        cursor.write_u16_le(byte_length(self.file_name)?)?;
        cursor.write_u16_le(self.file_type.to_u16())?;
        cursor.write_u32_le(self.file_size)?;
        self.time_of_creation.write(cursor)?;
        self.permissions.write(cursor)?;
        cursor.write_u16_le(self.request_id)?;
        cursor.write_bytes(self.file_name.as_bytes())?;
        Ok(())
    }

    pub(crate) fn read(cursor: &mut scursor::ReadCursor<'a>) -> Result<Self, ReadError> {
        let file_name_offset = cursor.read_u16_le()?;
        if file_name_offset != Self::FILE_NAME_OFFSET {
            return Err(ReadError::BadOffset {
                expected: Self::FILE_NAME_OFFSET,
                actual: file_name_offset,
            });
        }
        let file_name_length = cursor.read_u16_le()?;
        let file_type = FileType::new(cursor.read_u16_le()?);
        let file_size = cursor.read_u32_le()?;
        let time_of_creation = Timestamp::read(cursor)?;
        let permissions = Permissions::read(cursor)?;
        let request_id = cursor.read_u16_le()?;
        let file_name_bytes = cursor.read_bytes(file_name_length as usize)?;
        let file_name = std::str::from_utf8(file_name_bytes)?;

        Ok(Self {
            file_type,
            file_size,
            time_of_creation,
            permissions,
            request_id,
            file_name,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const OBJECT: Group70Var7 = Group70Var7 {
        file_type: FileType::File,
        file_size: 0xAABBCCDD,
        time_of_creation: Timestamp::new(0xAABBCCDDEEFF),
        permissions: Permissions {
            world: PermissionSet::all(),
            group: PermissionSet::all(),
            owner: PermissionSet::all(),
        },
        request_id: 0xEEFF,
        file_name: "foo.dat",
    };

    const DATA: &[u8] = &[
        20, // filename string offset - always 20
        0, 7, // filename string size
        0, 0x01, //  file type
        0x00, 0xDD, // file size
        0xCC, 0xBB, 0xAA, 0xFF, // time of creation
        0xEE, 0xDD, 0xCC, 0xBB, 0xAA, 0b11111111, // permissions
        0b00000001, 0xFF, // request id
        0xEE, b'f', // filename
        b'o', b'o', b'.', b'd', b'a', b't',
    ];

    #[test]
    fn writes_valid_object() {
        let mut buffer = [0; 64];

        let mut cursor = scursor::WriteCursor::new(&mut buffer);
        OBJECT.write(&mut cursor).unwrap();

        assert_eq!(cursor.written(), DATA)
    }

    #[test]
    fn parses_valid_object() {
        let mut cursor = scursor::ReadCursor::new(DATA);
        let obj = Group70Var7::read(&mut cursor).unwrap();

        assert_eq!(obj, OBJECT);
        assert!(cursor.is_empty());
    }
}
