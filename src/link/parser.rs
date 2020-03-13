use crate::error::*;
use crate::link::constant;
use crate::link::header::{Address, ControlField, Header};
use crate::util::cursor::{ReadCursor, ReadError};
use crate::util::slice_ext::*;

#[derive(Copy, Clone)]
enum ParseState {
    FindSync1,
    FindSync2,
    ReadHeader,
    ReadBody(Header, usize), // the header + calculated trailer length
}

#[derive(Debug, PartialEq)]
pub struct Frame<'a> {
    pub header: Header,
    pub payload: &'a [u8],
}

impl<'a> Frame<'a> {
    pub fn new(header: Header, payload: &'a [u8]) -> Self {
        Frame { header, payload }
    }
}

pub struct Parser {
    state: ParseState,
    buffer: [u8; 250], // where the payload of the frame is placed after removing the CRCs
}

impl std::convert::From<ReadError> for ParseError {
    fn from(_: ReadError) -> Self {
        ParseError::BadLogic(LogicError::BadRead)
    }
}

impl std::convert::From<FrameError> for ParseError {
    fn from(err: FrameError) -> Self {
        ParseError::BadFrame(err)
    }
}

impl std::convert::From<LogicError> for ParseError {
    fn from(err: LogicError) -> Self {
        ParseError::BadLogic(err)
    }
}

impl Parser {
    pub fn new() -> Parser {
        Parser {
            state: ParseState::FindSync1,
            buffer: [0; constant::MAX_FRAME_PAYLOAD_LENGTH],
        }
    }

    pub fn reset(&mut self) {
        self.state = ParseState::FindSync1;
    }

    pub fn parse<'a>(
        &'a mut self,
        cursor: &mut ReadCursor,
    ) -> Result<Option<Frame<'a>>, ParseError> {
        loop {
            let start = cursor.len();

            match self.state {
                ParseState::FindSync1 => self.parse_sync1(cursor)?,
                ParseState::FindSync2 => self.parse_sync2(cursor)?,
                ParseState::ReadHeader => self.parse_header(cursor)?,
                ParseState::ReadBody(header, length) => {
                    if let Some(length) = self.parse_body(length, cursor)? {
                        return Ok(Some(Frame::new(header, &self.buffer[0..length])));
                    }
                }
            }

            let end = cursor.len();

            if start == end {
                // no progress
                return Ok(None);
            }
        }
    }

    fn calc_trailer_length(data_length: u8) -> usize {
        let div16: usize = data_length as usize / constant::MAX_BLOCK_SIZE;
        let mod16: usize = data_length as usize % constant::MAX_BLOCK_SIZE;

        if mod16 == 0 {
            div16 * constant::MAX_BLOCK_SIZE_WITH_CRC
        } else {
            (div16 * constant::MAX_BLOCK_SIZE_WITH_CRC) + mod16 + constant::CRC_LENGTH
        }
    }

    fn parse_sync1(&mut self, cursor: &mut ReadCursor) -> Result<(), ParseError> {
        if cursor.is_empty() {
            return Ok(());
        }

        let x = cursor.read_u8()?;

        if x != 0x05 {
            return Err(FrameError::UnexpectedStart1(x).into());
        }

        self.state = ParseState::FindSync2;
        Ok(())
    }

    fn parse_sync2(&mut self, cursor: &mut ReadCursor) -> Result<(), ParseError> {
        if cursor.is_empty() {
            return Ok(());
        }

        let x = cursor.read_u8()?;

        if x != 0x64 {
            return Err(FrameError::UnexpectedStart2(x).into());
        }

        self.state = ParseState::ReadHeader;
        Ok(())
    }

    fn parse_header(&mut self, cursor: &mut ReadCursor) -> Result<(), ParseError> {
        if cursor.len() < 8 {
            return Ok(());
        }

        let crc_bytes = cursor.read_bytes(6)?;
        let crc_value = cursor.read_u16_le()?;

        let mut cursor = ReadCursor::new(crc_bytes);
        let len = cursor.read_u8()?;

        let header = Header::new(
            ControlField::from(cursor.read_u8()?),
            Address::new(cursor.read_u16_le()?, cursor.read_u16_le()?),
        );

        if len < 5 {
            return Err(FrameError::BadLength(len).into());
        }

        let expected_crc = super::crc::calc_crc_with_0564(crc_bytes);
        if crc_value != expected_crc {
            return Err(FrameError::BadHeaderCRC.into());
        }

        let trailer_length = Self::calc_trailer_length(len - 5); // ok b/c len >= 5 above

        self.state = ParseState::ReadBody(header, trailer_length);
        Ok(())
    }

    fn parse_body(
        &mut self,
        trailer_length: usize,
        cursor: &mut ReadCursor,
    ) -> Result<Option<usize>, ParseError> {
        if cursor.len() < trailer_length {
            return Ok(None);
        }

        let body = cursor.read_bytes(trailer_length)?;

        let mut pos = 0;

        for block in body.chunks(18) {
            if block.len() < 3 {
                // can't be a valid block
                return Err(LogicError::BadSize.into());
            }

            let data_len = block.len() - 2;

            let (data, crc) = block.np_split_at(data_len)?;
            let crc_value = ReadCursor::new(crc).read_u16_le()?;
            let calc_crc = super::crc::calc_crc(data);

            if crc_value != calc_crc {
                return Err(FrameError::BadBodyCRC.into());
            }

            // copy the data and advance the position
            self.buffer
                .as_mut()
                .np_get_mut(pos..(pos + data_len))?
                .clone_from_slice(data);

            pos += data_len;
        }

        self.state = ParseState::FindSync1;
        Ok(Some(pos))
    }
}

impl Default for Parser {
    fn default() -> Self {
        Parser::new()
    }
}

#[cfg(test)]
mod test {
    use super::super::test_data::*;
    use super::*;

    fn test_frame_parsing(parser: &mut Parser, bytes: &[u8], frame: &Frame) {
        let mut cursor = ReadCursor::new(bytes);
        let result: Frame = parser.parse(&mut cursor).unwrap().unwrap();
        assert_eq!(&result, frame);
        assert_eq!(cursor.len(), 0);
    }

    #[test]
    fn can_parse_in_a_loop() {
        let mut parser = Parser::new();
        let mut cursor = ReadCursor::new(&[]);

        loop {
            let result = parser.parse(&mut cursor);
        }
    }

    #[test]
    fn catches_bad_start1() {
        let mut parser = Parser::new();
        let mut cursor = ReadCursor::new(&[0x06]);

        assert_eq!(
            parser.parse(&mut cursor),
            Err(ParseError::BadFrame(FrameError::UnexpectedStart1(0x06)))
        );

        assert!(cursor.is_empty());
    }

    #[test]
    fn catches_bad_start2() {
        let mut parser = Parser::new();
        let mut cursor = ReadCursor::new(&[0x05, 0x65]);

        assert_eq!(
            parser.parse(&mut cursor),
            Err(ParseError::BadFrame(FrameError::UnexpectedStart2(0x65)))
        );

        assert!(cursor.is_empty());
    }

    #[test]
    fn catches_bad_length() {
        let mut frame = RESET_LINK_BYTES.clone();
        *frame.get_mut(2).unwrap() = 0x04;

        let mut parser = Parser::new();
        let mut cursor = ReadCursor::new(&frame);

        assert_eq!(
            parser.parse(&mut cursor),
            Err(ParseError::BadFrame(FrameError::BadLength(4)))
        );
        assert_eq!(cursor.len(), 0);
    }

    #[test]
    fn header_parse_catches_bad_crc() {
        let mut frame = RESET_LINK_BYTES.clone();
        *frame.last_mut().unwrap() = 0xFF;

        let mut parser = Parser::new();
        let mut cursor = ReadCursor::new(&frame);

        assert_eq!(
            parser.parse(&mut cursor),
            Err(ParseError::BadFrame(FrameError::BadHeaderCRC))
        );
        assert_eq!(cursor.len(), 0);
    }

    #[test]
    fn catches_bad_crc_in_body() {
        let mut frame = CONFIRM_USER_DATA_BYTES.clone();
        *frame.last_mut().unwrap() = 0xFF;

        let mut parser = Parser::new();
        let mut cursor = ReadCursor::new(&frame);

        assert_eq!(
            parser.parse(&mut cursor),
            Err(ParseError::BadFrame(FrameError::BadBodyCRC)),
        );
        assert_eq!(cursor.len(), 0);
    }

    #[test]
    fn can_parse_multiple_different_frames_sequentially() {
        let mut parser = Parser::new();
        test_frame_parsing(&mut parser, &RESET_LINK_BYTES, &RESET_LINK_FRAME);
        test_frame_parsing(&mut parser, &ACK_BYTES, &ACK_FRAME);
        test_frame_parsing(
            &mut parser,
            &CONFIRM_USER_DATA_BYTES,
            &CONFIRM_USER_DATA_FRAME,
        );
    }
}
