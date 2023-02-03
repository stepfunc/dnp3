use super::*;
use crate::app::format::WriteError;
use scursor::{ReadCursor, WriteCursor};

/// Group 70 Variation 5 - file transport
///
/// This representation is borrowed from the underlying ASDU
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Group70Var5<'a> {
    pub(crate) file_handle: u32,
    pub(crate) block_number: u32,
    pub(crate) file_data: &'a [u8],
}

impl<'a> Group70Var5<'a> {
    pub(crate) fn format(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "\nfile handle: {}", self.file_handle)?;
        write!(f, "\nblock number: {}", self.block_number)?;
        write!(f, "\nfile data length: {}", self.file_data.len())?;
        Ok(())
    }

    pub(crate) fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u32_le(self.file_handle)?;
        cursor.write_u32_le(self.block_number)?;
        cursor.write_bytes(self.file_data)?;
        Ok(())
    }

    pub(crate) fn read(cursor: &mut ReadCursor<'a>) -> Result<Self, ReadError> {
        let file_handle = cursor.read_u32_le()?;
        let block_number = cursor.read_u32_le()?;
        let file_data = cursor.read_all();

        Ok(Self {
            file_handle,
            block_number,
            file_data,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const OBJECT: Group70Var5 = Group70Var5 {
        file_handle: 0x01020304,
        block_number: 0xFECAADDE,
        file_data: &[b'd', b'a', b't', b'a'],
    };

    const DATA: &[u8] = &[
        4, // file handle
        3, 2, 1, 0xDE, // block number
        0xAD, 0xCA, 0xFE, b'd', // data
        b'a', b't', b'a',
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
        let obj = Group70Var5::read(&mut cursor).unwrap();

        assert_eq!(obj, OBJECT);
        assert!(cursor.is_empty());
    }
}
