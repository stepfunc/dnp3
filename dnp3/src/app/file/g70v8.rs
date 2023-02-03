use super::*;

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

    // not using this in production yet
    #[cfg(test)]
    pub(crate) fn write(
        &self,
        cursor: &mut scursor::WriteCursor,
    ) -> Result<(), scursor::WriteError> {
        cursor.write_bytes(self.file_specification.as_bytes())
    }

    pub(crate) fn read(cursor: &mut scursor::ReadCursor<'a>) -> Result<Self, ReadError> {
        let file_specification_bytes = cursor.read_all();
        let file_specification = std::str::from_utf8(file_specification_bytes)?;
        Ok(Self { file_specification })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const OBJECT: Group70Var8 = Group70Var8 {
        file_specification: "test",
    };

    const DATA: &[u8] = &[b't', b'e', b's', b't'];

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
        let obj = Group70Var8::read(&mut cursor).unwrap();

        assert_eq!(obj, OBJECT);
        assert!(cursor.is_empty());
    }
}
