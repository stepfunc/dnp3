// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use super::*;
#[cfg(test)]
use crate::app::format::free_format::FreeFormat;
#[cfg(test)]
use crate::app::format::WriteError;
#[cfg(test)]
use crate::app::Variation;
#[cfg(test)]
use scursor::WriteCursor;

/// Group 70 Variation 8 - File specification string
///
/// This representation is borrowed from the underlying ASDU
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Group70Var8<'a> {
    pub(crate) file_specification: &'a str,
}

impl<'a> Group70Var8<'a> {
    pub(crate) fn format(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "\nfile specification: {}", self.file_specification)?;
        Ok(())
    }

    pub(crate) fn read(cursor: &mut scursor::ReadCursor<'a>) -> Result<Self, ReadError> {
        let file_specification_bytes = cursor.read_all();
        let file_specification = std::str::from_utf8(file_specification_bytes)?;
        Ok(Self { file_specification })
    }
}

#[cfg(test)]
impl FreeFormat for Group70Var8<'_> {
    const VARIATION: Variation = Variation::Group70Var8;

    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_bytes(self.file_specification.as_bytes())?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const OBJECT: Group70Var8 = Group70Var8 {
        file_specification: "test",
    };

    const DATA: &[u8] = b"test";

    #[test]
    fn encodes_known_bytes() {
        let mut buffer = [0; 64];
        let mut cursor = scursor::WriteCursor::new(&mut buffer);
        OBJECT.write(&mut cursor).unwrap();
        assert_eq!(cursor.written(), DATA);

        let mut cursor = scursor::ReadCursor::new(DATA);
        assert_eq!(Group70Var8::read(&mut cursor).unwrap(), OBJECT);
        assert!(cursor.is_empty());
    }
}
