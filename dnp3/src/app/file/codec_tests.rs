// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Property tests of the group 70 free-format codecs: every object written in a free-format
//! header parses back unchanged, and a wrong offset or a string that isn't UTF-8 makes the whole
//! header a bad encoding

#![expect(
    clippy::arithmetic_side_effects,
    reason = "tests compute positions inside the encodings they wrote"
)]

use super::*;
use crate::app::format::free_format::FreeFormat;
use crate::app::format::write::HeaderWriter;
use crate::app::format::WriteError;
use crate::app::parse::free_format::FreeFormatVariation;
use crate::app::parse::options::ParseOptions;
use crate::app::parse::parser::{HeaderCollection, ObjectHeader};
use crate::app::{FunctionCode, ObjectParseError, Timestamp};
use crate::master::FileMode;
use proptest::prelude::*;
use proptest::sample::Index;
use scursor::WriteCursor;

/// group, variation, qualifier, count and the length of the object
const HEADER_LEN: usize = 6;

fn string() -> impl Strategy<Value = String> {
    prop_oneof![
        prop::collection::vec(0x20u8..0x7F, 0..64)
            .prop_map(|x| x.into_iter().map(char::from).collect()),
        prop::collection::vec(any::<char>(), 0..16).prop_map(String::from_iter),
    ]
}

fn timestamp() -> impl Strategy<Value = Timestamp> {
    (0..=Timestamp::MAX_VALUE).prop_map(Timestamp::new)
}

fn permission_set() -> impl Strategy<Value = PermissionSet> {
    any::<[bool; 3]>().prop_map(|[execute, write, read]| PermissionSet {
        execute,
        write,
        read,
    })
}

fn permissions() -> impl Strategy<Value = Permissions> {
    (permission_set(), permission_set(), permission_set()).prop_map(|(world, group, owner)| {
        Permissions {
            world,
            group,
            owner,
        }
    })
}

fn status() -> impl Strategy<Value = FileStatus> {
    any::<u8>().prop_map(FileStatus::new)
}

fn write<T: FreeFormat>(object: &T) -> Vec<u8> {
    let mut buffer = [0u8; 1024];
    let mut cursor = WriteCursor::new(&mut buffer);
    HeaderWriter::new(&mut cursor)
        .write_free_format(object)
        .unwrap();
    cursor.written().to_vec()
}

/// the only header, if it's a free-format header
fn parse(bytes: &[u8]) -> Result<Option<FreeFormatVariation<'_>>, ObjectParseError> {
    let headers = HeaderCollection::parse(ParseOptions::default(), FunctionCode::Response, bytes)?;
    Ok(match headers.get_only_header() {
        Ok(ObjectHeader::FreeFormat(x)) => Some(x),
        _ => None,
    })
}

fn is_bad_encoding(bytes: &[u8]) -> bool {
    matches!(parse(bytes), Err(ObjectParseError::BadEncoding))
}

/// the encoding with a byte of the string at `start` replaced by one that is never UTF-8
fn with_invalid_utf8(bytes: &[u8], start: usize, string: &str, index: Index) -> Option<Vec<u8>> {
    if string.is_empty() {
        return None;
    }
    let mut bytes = bytes.to_vec();
    bytes[HEADER_LEN + start + index.index(string.len())] = 0xFF;
    Some(bytes)
}

/// the encoding with the u16 field at `offset` in the object set to `value`
fn with_u16(bytes: &[u8], offset: usize, value: u16) -> Vec<u8> {
    let mut bytes = bytes.to_vec();
    let start = HEADER_LEN + offset;
    bytes[start..start + 2].copy_from_slice(&value.to_le_bytes());
    bytes
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn g70v2_round_trips_and_rejects_bad_encodings(
        auth_key in any::<u32>(),
        user_name in string(),
        password in string(),
        offset_field in 0usize..3,
        offset_value in any::<u16>(),
        corrupt in any::<Index>(),
    ) {
        let object = Group70Var2 {
            auth_key,
            user_name: &user_name,
            password: &password,
        };
        let bytes = write(&object);
        prop_assert!(
            matches!(parse(&bytes), Ok(Some(FreeFormatVariation::Group70Var2(x))) if x == object)
        );

        // the user name offset, the user name length and the password offset must agree
        let field = offset_field * 2;
        prop_assume!(offset_value.to_le_bytes() != bytes[HEADER_LEN + field..HEADER_LEN + field + 2]);
        prop_assert!(is_bad_encoding(&with_u16(&bytes, field, offset_value)));

        let user_name_bad = with_invalid_utf8(&bytes, 12, &user_name, corrupt);
        let password_bad = with_invalid_utf8(&bytes, 12 + user_name.len(), &password, corrupt);
        for bad in [user_name_bad, password_bad].into_iter().flatten() {
            prop_assert!(is_bad_encoding(&bad));
        }
    }

    #[test]
    fn g70v3_round_trips_and_rejects_bad_encodings(
        time_of_creation in timestamp(),
        permissions in permissions(),
        auth_key in any::<u32>(),
        file_size in any::<u32>(),
        mode in any::<u16>(),
        max_block_size in any::<u16>(),
        request_id in any::<u16>(),
        file_name in string(),
        offset in any::<u16>().prop_filter("a wrong offset", |x| *x != 26),
        corrupt in any::<Index>(),
    ) {
        let object = Group70Var3 {
            time_of_creation,
            permissions,
            auth_key,
            file_size,
            mode: FileMode::new(mode),
            max_block_size,
            request_id,
            file_name: &file_name,
        };
        let bytes = write(&object);
        prop_assert!(
            matches!(parse(&bytes), Ok(Some(FreeFormatVariation::Group70Var3(x))) if x == object)
        );
        prop_assert!(is_bad_encoding(&with_u16(&bytes, 0, offset)));
        if let Some(bad) = with_invalid_utf8(&bytes, 26, &file_name, corrupt) {
            prop_assert!(is_bad_encoding(&bad));
        }
    }

    #[test]
    fn g70v4_round_trips_and_rejects_bad_encodings(
        file_handle in any::<u32>(),
        file_size in any::<u32>(),
        max_block_size in any::<u16>(),
        request_id in any::<u16>(),
        status_code in status(),
        text in string(),
        corrupt in any::<Index>(),
    ) {
        let object = Group70Var4 {
            file_handle,
            file_size,
            max_block_size,
            request_id,
            status_code,
            text: &text,
        };
        let bytes = write(&object);
        prop_assert!(
            matches!(parse(&bytes), Ok(Some(FreeFormatVariation::Group70Var4(x))) if x == object)
        );
        if let Some(bad) = with_invalid_utf8(&bytes, 13, &text, corrupt) {
            prop_assert!(is_bad_encoding(&bad));
        }
    }

    #[test]
    fn g70v5_round_trips(
        file_handle in any::<u32>(),
        block_number in any::<u32>(),
        file_data in prop::collection::vec(any::<u8>(), 0..64),
    ) {
        let object = Group70Var5 {
            file_handle,
            block_number,
            file_data: &file_data,
        };
        let bytes = write(&object);
        prop_assert!(
            matches!(parse(&bytes), Ok(Some(FreeFormatVariation::Group70Var5(x))) if x == object)
        );
    }

    #[test]
    fn g70v6_round_trips_and_rejects_bad_encodings(
        file_handle in any::<u32>(),
        block_number in any::<u32>(),
        status_code in status(),
        text in string(),
        corrupt in any::<Index>(),
    ) {
        let object = Group70Var6 {
            file_handle,
            block_number,
            status_code,
            text: &text,
        };
        let bytes = write(&object);
        prop_assert!(
            matches!(parse(&bytes), Ok(Some(FreeFormatVariation::Group70Var6(x))) if x == object)
        );
        if let Some(bad) = with_invalid_utf8(&bytes, 9, &text, corrupt) {
            prop_assert!(is_bad_encoding(&bad));
        }
    }

    #[test]
    fn g70v7_round_trips_and_rejects_bad_encodings(
        file_type in any::<u16>(),
        file_size in any::<u32>(),
        time_of_creation in timestamp(),
        permissions in permissions(),
        request_id in any::<u16>(),
        file_name in string(),
        offset in any::<u16>().prop_filter("a wrong offset", |x| *x != 20),
        corrupt in any::<Index>(),
    ) {
        let object = Group70Var7 {
            file_type: FileType::new(file_type),
            file_size,
            time_of_creation,
            permissions,
            request_id,
            file_name: &file_name,
        };
        let bytes = write(&object);
        prop_assert!(
            matches!(parse(&bytes), Ok(Some(FreeFormatVariation::Group70Var7(x))) if x == object)
        );
        prop_assert!(is_bad_encoding(&with_u16(&bytes, 0, offset)));
        if let Some(bad) = with_invalid_utf8(&bytes, 20, &file_name, corrupt) {
            prop_assert!(is_bad_encoding(&bad));
        }
    }

    #[test]
    fn g70v8_round_trips_and_rejects_bad_encodings(
        file_specification in string(),
        corrupt in any::<Index>(),
    ) {
        let object = Group70Var8 {
            file_specification: &file_specification,
        };
        let bytes = write(&object);
        prop_assert!(
            matches!(parse(&bytes), Ok(Some(FreeFormatVariation::Group70Var8(x))) if x == object)
        );
        if let Some(bad) = with_invalid_utf8(&bytes, 0, &file_specification, corrupt) {
            prop_assert!(is_bad_encoding(&bad));
        }
    }
}

/// the password offset is 12 plus the length of the user name, which must fit in a u16
#[test]
fn g70v2_rejects_a_user_name_that_overflows_the_password_offset() {
    let mut buffer = vec![0u8; 70_000];
    for len in 65_500usize..=65_540 {
        let user_name = "a".repeat(len);
        let object = Group70Var2 {
            auth_key: 0,
            user_name: &user_name,
            password: "",
        };
        let mut cursor = WriteCursor::new(&mut buffer);
        let expected = if len + 12 <= usize::from(u16::MAX) {
            Ok(())
        } else {
            Err(WriteError::Overflow)
        };
        assert_eq!(
            object.write(&mut cursor),
            expected,
            "user name length {len}"
        );
    }
}
