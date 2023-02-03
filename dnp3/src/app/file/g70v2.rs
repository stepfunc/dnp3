use super::*;
use crate::app::format::WriteError;
use scursor::{ReadCursor, WriteCursor};

/// Group 70 Variation 3 - authentication
///
/// This representation is borrowed from the underlying ASDU
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Group70Var2<'a> {
    pub(crate) auth_key: u32,
    pub(crate) user_name: &'a str,
    pub(crate) password: &'a str,
}

impl<'a> Group70Var2<'a> {
    // the user name offset is always 12 octets because all the fields in front of it are fixed size
    const USER_NAME_OFFSET: u16 = 12;

    pub(crate) fn format(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "\nuser name: {}", self.user_name)?;
        write!(f, "\npassword: {}", self.password)?;
        write!(f, "\nauth key: {}", self.auth_key)?;
        Ok(())
    }

    pub(crate) fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u16_le(Self::USER_NAME_OFFSET)?;
        let user_name_size = byte_length(self.user_name)?;
        cursor.write_u16_le(user_name_size)?;

        let password_offset = match Self::USER_NAME_OFFSET.checked_add(user_name_size) {
            None => return Err(WriteError::Overflow),
            Some(x) => x,
        };

        cursor.write_u16_le(password_offset)?;
        let password_size = byte_length(self.password)?;

        cursor.write_u16_le(password_size)?;
        cursor.write_u32_le(self.auth_key)?;
        cursor.write_bytes(self.user_name.as_bytes())?;
        cursor.write_bytes(self.password.as_bytes())?;

        Ok(())
    }

    pub(crate) fn read(cursor: &mut ReadCursor<'a>) -> Result<Self, ReadError> {
        let user_name_offset = cursor.read_u16_le()?;

        // since this is fixed length, we can validate it early
        if user_name_offset != Self::USER_NAME_OFFSET {
            return Err(ReadError::BadOffset {
                expected: Self::USER_NAME_OFFSET,
                actual: user_name_offset,
            });
        }

        let user_name_length: u16 = cursor.read_u16_le()?;

        let implied_password_offset = match Self::USER_NAME_OFFSET.checked_add(user_name_length) {
            None => return Err(ReadError::Overflow),
            Some(x) => x,
        };

        let password_offset = cursor.read_u16_le()?;

        if password_offset != implied_password_offset {
            return Err(ReadError::BadOffset {
                expected: implied_password_offset,
                actual: password_offset,
            });
        }

        let password_length: u16 = cursor.read_u16_le()?;
        let auth_key = cursor.read_u32_le()?;

        let user_name_bytes = cursor.read_bytes(user_name_length as usize)?;
        let password_bytes = cursor.read_bytes(password_length as usize)?;

        let user_name = std::str::from_utf8(user_name_bytes)?;
        let password = std::str::from_utf8(password_bytes)?;

        Ok(Self {
            auth_key,
            user_name,
            password,
        })
    }
}

#[cfg(test)]
mod test {
    use super::Group70Var2;
    use scursor::{ReadCursor, WriteCursor};

    const OBJECT: Group70Var2 = Group70Var2 {
        auth_key: 0xDEADCAFE,
        user_name: "root",
        password: "foo",
    };

    const DATA: &[u8] = &[
        12, // username string offset - always 12
        0,
        4, // username string size
        0,
        12 + 4, // password string offset
        0,
        3, // password string size
        0,
        0xFE, // authentication key
        0xCA,
        0xAD,
        0xDE,
        b'r', // username
        b'o',
        b'o',
        b't',
        b'f', // password
        b'o',
        b'o',
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
        let obj = Group70Var2::read(&mut cursor).unwrap();

        assert_eq!(obj, OBJECT);
        assert!(cursor.is_empty());
    }
}
