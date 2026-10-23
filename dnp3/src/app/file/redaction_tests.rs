// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Credential redaction (#520)
//!
//! User decision D6 allows these tests, and only these, to format values into text: each one
//! asserts that a secret is absent from the formatted output and asserts nothing else about it.

use crate::app::file::{Group70Var2, Group70Var3};
use crate::app::format::free_format::FreeFormat;
use crate::app::format::write::start_request;
use crate::app::parse::free_format::FreeFormatVariation;
use crate::app::parse::options::ParseOptions;
use crate::app::parse::parser::{ObjectHeader, ParsedFragment};
use crate::app::{ControlField, FunctionCode, Permissions, Sequence, Timestamp};
use crate::decode::AppDecodeLevel;
use crate::master::{AuthKey, FileCredentials, FileMode};
use scursor::WriteCursor;

const PASSWORD: &str = "pa55-hunter2";
const AUTH_KEY: u32 = 0xDEAD_CAFE;

fn renderings_of(key: u32) -> [String; 3] {
    [key.to_string(), format!("{key:x}"), format!("{key:X}")]
}

fn write_request<T: FreeFormat>(function: FunctionCode, object: &T) -> Vec<u8> {
    let mut buffer = [0; 256];
    let mut cursor = WriteCursor::new(&mut buffer);
    let mut writer = start_request(
        ControlField::request(Sequence::default()),
        function,
        &mut cursor,
    )
    .unwrap();
    writer.write_free_format(object).unwrap();
    cursor.written().to_vec()
}

fn parse(fragment: &[u8]) -> ParsedFragment<'_> {
    ParsedFragment::parse(ParseOptions::default(), fragment).unwrap()
}

fn only_object<'a>(fragment: &ParsedFragment<'a>) -> FreeFormatVariation<'a> {
    let mut headers = fragment.objects.unwrap().iter();
    let header = headers.next().unwrap();
    assert!(headers.next().is_none());
    match header {
        ObjectHeader::FreeFormat(object) => object,
        _ => panic!("expected a free-format header"),
    }
}

fn authenticate() -> Group70Var2<'static> {
    Group70Var2 {
        auth_key: AUTH_KEY,
        user_name: "operator",
        password: PASSWORD,
    }
}

fn open() -> Group70Var3<'static> {
    Group70Var3 {
        time_of_creation: Timestamp::zero(),
        permissions: Permissions::default(),
        auth_key: AUTH_KEY,
        file_size: 0,
        mode: FileMode::Read,
        max_block_size: 1024,
        request_id: 0,
        file_name: "data.txt",
    }
}

fn assert_omits_key(text: &str) {
    for key in renderings_of(AUTH_KEY) {
        assert!(!text.contains(&key));
    }
}

fn decode_log(fragment: &ParsedFragment) -> String {
    fragment.display(AppDecodeLevel::ObjectValues).to_string()
}

#[test]
fn authenticate_omits_password_and_key() {
    let object = authenticate();
    let bytes = write_request(FunctionCode::AuthenticateFile, &object);
    let fragment = parse(&bytes);
    assert!(matches!(
        only_object(&fragment),
        FreeFormatVariation::Group70Var2(_)
    ));

    for text in [format!("{object:?}"), decode_log(&fragment)] {
        assert!(!text.contains(PASSWORD));
        assert_omits_key(&text);
    }
}

#[test]
fn open_omits_key() {
    let object = open();
    let bytes = write_request(FunctionCode::OpenFile, &object);
    let fragment = parse(&bytes);
    assert!(matches!(
        only_object(&fragment),
        FreeFormatVariation::Group70Var3(_)
    ));

    for text in [format!("{object:?}"), decode_log(&fragment)] {
        assert_omits_key(&text);
    }
}

#[test]
fn file_credentials_debug_omits_password() {
    let credentials = FileCredentials {
        user_name: "operator".to_string(),
        password: PASSWORD.to_string(),
    };
    assert!(!format!("{credentials:?}").contains(PASSWORD));
}

#[test]
fn auth_key_debug_omits_key() {
    assert_omits_key(&format!("{:?}", AuthKey::new(AUTH_KEY)));
}
