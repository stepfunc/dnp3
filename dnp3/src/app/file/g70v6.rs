use super::*;
use scursor::ReadCursor;

/// Group 70 Variation 6 - file transport status
///
/// This representation is borrowed from the underlying ASDU
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Group70Var6<'a> {
    pub(crate) file_handle: u32,
    pub(crate) block_number: u32,
    pub(crate) status_code: FileStatus,
    pub(crate) text: &'a str,
}

impl<'a> Group70Var6<'a> {
    pub(crate) fn format(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "\nfile handle: {}", self.file_handle)?;
        write!(f, "\nblock number: {}", self.block_number)?;
        write!(f, "\nstatus code: {:?}", self.status_code)?;
        write!(f, "\noptional text: {:?}", self.text)?;
        Ok(())
    }

    // we aren't using this in production yet but will be needed for the outstation
    #[cfg(test)]
    pub(crate) fn write(
        &self,
        cursor: &mut scursor::WriteCursor,
    ) -> Result<(), crate::app::format::WriteError> {
        cursor.write_u32_le(self.file_handle)?;
        cursor.write_u32_le(self.block_number)?;
        cursor.write_u8(self.status_code.to_u8())?;
        cursor.write_bytes(self.text.as_bytes())?;
        Ok(())
    }

    pub(crate) fn read(cursor: &mut ReadCursor<'a>) -> Result<Self, ReadError> {
        let file_handle = cursor.read_u32_le()?;
        let block_number = cursor.read_u32_le()?;
        let status_code = FileStatus::new(cursor.read_u8()?);
        let text = std::str::from_utf8(cursor.read_all())?;

        Ok(Self {
            file_handle,
            block_number,
            status_code,
            text,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const OBJECT: Group70Var6 = Group70Var6 {
        file_handle: 0x01020304,
        block_number: 3,
        status_code: FileStatus::CommLost,
        text: "wat",
    };

    const DATA: &[u8] = &[
        4, // file handle
        3, 2, 1, 03, // block number
        00, 00, 00, 8,    // status code
        b'w', // text
        b'a', b't',
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
        let mut cursor = ReadCursor::new(DATA);
        let obj = Group70Var6::read(&mut cursor).unwrap();

        assert_eq!(obj, OBJECT);
        assert!(cursor.is_empty());
    }
}
