use scursor::ReadCursor;
use std::str::Utf8Error;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum FileField {
    Username,
    Password,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum FileParseError {
    /// No more data
    ReadError,
    /// Field has a bad offset in the encoding
    BadOffset {
        field: FileField,
        expected: u16,
        actual: u16,
    },
    /// The encoding is bad because it requires that a value overflows the u16 representation
    Overflow,
    /// A string is not UTF8 encoded
    BadString(FileField, Utf8Error),
}

/// Group 70 Variation2 - File-control - authentication
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

    pub(crate) fn parse(cursor: &mut ReadCursor<'a>) -> Result<Self, FileParseError> {
        let user_name_offset = cursor.read_u16_le()?;

        // since this is fixed length, we can validate it early
        if user_name_offset != Self::USER_NAME_OFFSET {
            return Err(FileParseError::BadOffset {
                field: FileField::Username,
                expected: Self::USER_NAME_OFFSET,
                actual: user_name_offset,
            });
        }

        let user_name_length: u16 = cursor.read_u16_le()?;

        let implied_password_offset = match Self::USER_NAME_OFFSET.checked_add(user_name_length) {
            None => return Err(FileParseError::Overflow),
            Some(x) => x,
        };

        let password_offset = cursor.read_u16_le()?;

        if password_offset != implied_password_offset {
            return Err(FileParseError::BadOffset {
                field: FileField::Password,
                expected: implied_password_offset,
                actual: password_offset,
            });
        }

        let password_length: u16 = cursor.read_u16_le()?;
        let auth_key = cursor.read_u32_le()?;

        let user_name_bytes = cursor.read_bytes(user_name_length as usize)?;
        let password_bytes = cursor.read_bytes(password_length as usize)?;

        let user_name = std::str::from_utf8(user_name_bytes)
            .map_err(|err| FileParseError::BadString(FileField::Username, err))?;
        let password = std::str::from_utf8(password_bytes)
            .map_err(|err| FileParseError::BadString(FileField::Password, err))?;

        Ok(Self {
            auth_key,
            user_name,
            password,
        })
    }
}

impl From<scursor::ReadError> for FileParseError {
    fn from(_: scursor::ReadError) -> Self {
        Self::ReadError
    }
}

#[cfg(test)]
mod test {
    use crate::app::file::Group70Var2;
    use scursor::ReadCursor;

    #[test]
    fn parses_valid_g70v2() {
        let input: &[u8] = &[
            12,
            0, // username string offset - always 12
            4,
            0, // username string size
            12 + 4,
            0, // password string offset
            3,
            0, // password string size
            0xDE,
            0xAD,
            0xCA,
            0xFE, // authentication key
            b'r',
            b'o',
            b'o',
            b't', // username
            b'f',
            b'o',
            b'o', // password
        ];

        let mut cursor = ReadCursor::new(input);
        let obj = Group70Var2::parse(&mut cursor).unwrap();

        assert_eq!(
            obj,
            Group70Var2 {
                auth_key: 0xFECAADDE,
                user_name: "root",
                password: "foo",
            }
        )
    }
}
