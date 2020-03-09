use crate::link::header::{Ctrl, Header};

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ParseError {
    BadLength(u8),
    BadHeaderCRC,
    BadBodyCRC,
}

enum ParseState {
    FindSync1,
    FindSync2,
    ReadHeader,
    ReadBody(Header),
}

/*
#[test]
fn header_parse_catches_bad_length() {
    // CRC is the 0x21E9 at the end (little endian)
    let frame: [u8; 10] = [0x05, 0x64, 0x04, 0xC0, 0x01, 0x00, 0x00, 0x04, 0xE9, 0x21];

    let mut parser = Parser::new();
    let mut handler = MockHandler::new();

    handler.expects.push(Expect::Error(ParseError::BadLength(4)));

    parser.decode(&frame[..], &mut handler);

    assert!(handler.expects.is_empty());
}

#[test]
fn header_parse_catches_bad_crc() {
    // CRC is the 0x21E9 at the end (little endian)
    let frame: [u8; 10] = [0x05, 0x64, 0x05, 0xC0, 0x01, 0x00, 0x00, 0x04, 0xE9, 0x20];

    let mut parser = Parser::new();
    let mut handler = MockHandler::new();

    handler.expects.push(Expect::Error(ParseError::BadHeaderCRC));

    parser.decode(&frame[..], &mut handler);

    assert!(handler.expects.is_empty());
}

#[test]
fn returns_frame_for_length_of_five() {
    // CRC is the 0x21E9 at the end (little endian)
    let frame: [u8; 10] = [0x05, 0x64, 0x05, 0xC0, 0x01, 0x00, 0x00, 0x04, 0xE9, 0x21];

    let mut parser = Parser::new();
    let mut handler = MockHandler::new();

    handler.expects.push(
        Expect::Frame(
            Header::from(Ctrl::from(0xC0), 1, 1024),
            0
        )
    );

    parser.decode(&frame[..], &mut handler);

    assert!(handler.expects.is_empty());
}
*/
