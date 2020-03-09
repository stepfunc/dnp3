use crate::link::header::{Ctrl, Header};
use crate::util::cursor::{ReadCursor, ReadError};

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ParseError {
    BadLength(u8),
    BadHeaderCRC,
    BadBodyCRC,
    BadRead,
}

enum ParseState {
    FindSync1,
    FindSync2,
    ReadHeader,
    ReadBody(Header, u8, u16), // the header + calculated payload length + length w/ CRCs
}

pub struct Frame<'a> {
    header: Header,
    payload: &'a [u8],
}

pub struct Parser {
    state: ParseState,
    buffer: [u8; 250], // where the payload of the frame is placed after removing the CRCs
}

impl std::convert::From<ReadError> for ParseError {
    fn from(_: ReadError) -> Self {
        ParseError::BadRead
    }
}

impl<'a> Parser {
    pub fn new() -> Parser {
        Parser {
            state: ParseState::FindSync1,
            buffer: [0; 250],
        }
    }

    fn calc_payload_length(data_length: u8) -> u16 {
        let div16: u8 = data_length / 16;
        let mod16: u8 = data_length % 16;

        if mod16 == 0 {
            div16 as u16 * 18
        } else {
            (div16 as u16 * 18) + mod16 as u16 + 2
        }
    }

    pub fn parse_some(&mut self, cursor: &mut ReadCursor) -> Result<Option<Frame<'a>>, ParseError> {
        match self.state {
            ParseState::FindSync1 => self.parse_sync1(cursor),
            ParseState::FindSync2 => self.parse_sync2(cursor),
            ParseState::ReadHeader => self.parse_header(cursor),
            ParseState::ReadBody(header, len, payload_len) => self.parse_body(header, cursor),
        }
    }

    fn parse_sync1(&mut self, cursor: &mut ReadCursor) -> Result<Option<Frame<'a>>, ParseError> {
        if cursor.is_empty() {
            return Ok(None);
        }
        if cursor.read_u8()? != 0x05 {
            return Ok(None);
        }
        self.state = ParseState::FindSync2;
        Ok(None)
    }

    fn parse_sync2(&mut self, cursor: &mut ReadCursor) -> Result<Option<Frame<'a>>, ParseError> {
        if cursor.is_empty() {
            return Ok(None);
        }

        if cursor.read_u8()? != 0x64 {
            self.state = ParseState::FindSync1;
            return Ok(None);
        }

        self.state = ParseState::ReadHeader;
        Ok(None)
    }

    fn parse_header(&mut self, cursor: &mut ReadCursor) -> Result<Option<Frame<'a>>, ParseError> {
        if cursor.len() < 8 {
            return Ok(None);
        }

        let crc_bytes = cursor.read_bytes(6)?;
        let crc_value = cursor.read_u16_le()?;

        let mut cursor = ReadCursor::new(crc_bytes);
        let len = cursor.read_u8()?;
        let header = Header::new(
            Ctrl::from(cursor.read_u8()?),
            cursor.read_u16_le()?,
            cursor.read_u16_le()?,
        );

        if len < 5 {
            return Err(ParseError::BadLength(len));
        }

        let expected_crc = super::crc::calc_crc_with_0564(crc_bytes);
        if crc_value != expected_crc {
            return Err(ParseError::BadHeaderCRC);
        }

        let user_data_length = len - 5; // ok b/c len >= 5 above
        let payload_length = Self::calc_payload_length(user_data_length);

        self.state = ParseState::ReadBody(header, user_data_length, payload_length);

        Ok(None)
    }

    fn parse_body(
        &mut self,
        header: Header,
        cursor: &mut ReadCursor,
    ) -> Result<Option<Frame<'a>>, ParseError> {
        Ok(None)
    }
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
