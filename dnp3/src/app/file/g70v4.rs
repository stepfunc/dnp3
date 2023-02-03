use super::*;
use crate::app::format::WriteError;
use scursor::{ReadCursor, WriteCursor};

/// Group 70 Variation 4 - file command status
///
/// This representation is borrowed from the underlying ASDU
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Group70Var4<'a> {
    pub(crate) file_handle: u32,
    pub(crate) file_size: u32,
    pub(crate) max_block_size: u16,
    pub(crate) request_id: u16,
    pub(crate) status_code: FileStatus,
    pub(crate) text: &'a str,
}

impl<'a> Group70Var4<'a> {
    pub(crate) fn format(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "\nfile handle: {}", self.file_handle)?;
        write!(f, "\nfile size: {}", self.file_size)?;
        write!(f, "\nmax block size: {}", self.max_block_size)?;
        write!(f, "\nrequest id: {}", self.request_id)?;
        write!(f, "\nstatus code: {:?}", self.status_code)?;
        write!(f, "\noptional text: {:?}", self.text)?;
        Ok(())
    }

    pub(crate) fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u32_le(self.file_handle)?;
        cursor.write_u32_le(self.file_size)?;
        cursor.write_u16_le(self.max_block_size)?;
        cursor.write_u16_le(self.request_id)?;
        cursor.write_u8(self.status_code.to_u8())?;
        cursor.write_bytes(self.text.as_bytes())?;
        Ok(())
    }

    pub(crate) fn read(cursor: &mut ReadCursor<'a>) -> Result<Self, ReadError> {
        let file_handle = cursor.read_u32_le()?;
        let file_size = cursor.read_u32_le()?;
        let max_block_size = cursor.read_u16_le()?;
        let request_id = cursor.read_u16_le()?;
        let status_code = FileStatus::new(cursor.read_u8()?);
        let text = std::str::from_utf8(cursor.read_all())?;

        Ok(Self {
            file_handle,
            file_size,
            max_block_size,
            request_id,
            status_code,
            text,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const OBJECT: Group70Var4 = Group70Var4 {
        file_handle: 0x01020304,
        file_size: 0xAABBCCDD,
        max_block_size: 1024,
        request_id: 42,
        status_code: FileStatus::FileNotFound,
        text: "wat",
    };

    const DATA: &[u8] = &[
        4, // file handle
        3, 2, 1, 0xDD, // file size
        0xCC, 0xBB, 0xAA, 00, // max block size
        04, 42, // request id
        00, 3,    // status code
        b'w', // text
        b'a', b't',
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
        let obj = Group70Var4::read(&mut cursor).unwrap();

        assert_eq!(obj, OBJECT);
        assert!(cursor.is_empty());
    }
}
