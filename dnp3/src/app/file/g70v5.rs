// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use super::*;
use crate::app::format::free_format::FreeFormat;
use crate::app::format::WriteError;
use crate::app::Variation;
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

impl FreeFormat for Group70Var5<'_> {
    const VARIATION: Variation = Variation::Group70Var5;

    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u32_le(self.file_handle)?;
        cursor.write_u32_le(self.block_number)?;
        cursor.write_bytes(self.file_data)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const OBJECT: Group70Var5 = Group70Var5 {
        file_handle: 0x01020304,
        block_number: 0xFECAADDE,
        file_data: b"data",
    };

    const DATA: &[u8] = &[
        4, // file handle
        3, 2, 1, 0xDE, // block number
        0xAD, 0xCA, 0xFE, b'd', // data
        b'a', b't', b'a',
    ];

    #[test]
    fn encodes_known_bytes() {
        let mut buffer = [0; 64];
        let mut cursor = WriteCursor::new(&mut buffer);
        OBJECT.write(&mut cursor).unwrap();
        assert_eq!(cursor.written(), DATA);

        let mut cursor = ReadCursor::new(DATA);
        assert_eq!(Group70Var5::read(&mut cursor).unwrap(), OBJECT);
        assert!(cursor.is_empty());
    }
}
