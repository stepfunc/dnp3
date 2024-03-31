use crate::link::constant;
use crate::link::error::*;
use crate::link::header::{AnyAddress, ControlField, Header};
use crate::link::LinkErrorMode;
use crate::util::slice_ext::*;

use scursor::{ReadCursor, ReadError};

#[derive(Copy, Clone)]
enum ParseState {
    FindSync1,
    FindSync2,
    ReadHeader,
    ReadBody(Header, usize), // the header + calculated trailer length
}

pub(crate) struct FramePayload {
    length: usize,
    buffer: [u8; constant::MAX_FRAME_PAYLOAD_LENGTH],
}

impl FramePayload {
    pub(crate) fn new() -> Self {
        Self {
            length: 0,
            buffer: [0; constant::MAX_FRAME_PAYLOAD_LENGTH],
        }
    }

    pub(crate) fn clear(&mut self) {
        self.length = 0;
    }

    pub(crate) fn get(&self) -> &[u8] {
        &self.buffer[0..self.length]
    }

    pub(crate) fn push(&mut self, data: &[u8]) -> Result<(), LogicError> {
        let mut buff = self.buffer.as_mut();
        let dest = buff.np_get_mut(self.length..self.length + data.len())?;
        dest.copy_from_slice(data);
        self.length += data.len();
        Ok(())
    }
}

impl Default for FramePayload {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) struct Parser {
    mode: LinkErrorMode,
    state: ParseState,
}

impl From<ReadError> for ParseError {
    fn from(_: ReadError) -> Self {
        ParseError::BadLogic(LogicError::BadRead)
    }
}

impl From<FrameError> for ParseError {
    fn from(err: FrameError) -> Self {
        ParseError::BadFrame(err)
    }
}

impl From<LogicError> for ParseError {
    fn from(err: LogicError) -> Self {
        ParseError::BadLogic(err)
    }
}

impl Parser {
    pub(crate) fn new(mode: LinkErrorMode) -> Parser {
        Parser {
            mode,
            state: ParseState::FindSync1,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.state = ParseState::FindSync1;
    }

    pub(crate) fn parse(
        &mut self,
        cursor: &mut ReadCursor,
        payload: &mut FramePayload,
    ) -> Result<Option<Header>, ParseError> {
        loop {
            if self.mode == LinkErrorMode::Close {
                return self.parse_impl(cursor, payload);
            }

            let res = cursor.transaction(|cur| self.parse_impl(cur, payload));

            match res {
                Ok(x) => return Ok(x),
                Err(_) => {
                    let _ = cursor.read_u8(); // advance one byte
                    self.reset();
                    // goto next iteration
                }
            }
        }
    }

    fn parse_impl(
        &mut self,
        cursor: &mut ReadCursor,
        payload: &mut FramePayload,
    ) -> Result<Option<Header>, ParseError> {
        loop {
            let start = cursor.remaining();

            match self.state {
                ParseState::FindSync1 => self.parse_sync1(cursor)?,
                ParseState::FindSync2 => self.parse_sync2(cursor)?,
                ParseState::ReadHeader => self.parse_header(cursor)?,
                ParseState::ReadBody(header, length) => {
                    if let Some(()) = self.parse_body(length, cursor, payload)? {
                        return Ok(Some(header));
                    }
                }
            }

            let end = cursor.remaining();

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
        if cursor.remaining() < 8 {
            return Ok(());
        }

        let crc_bytes = cursor.read_bytes(6)?;
        let crc_value = cursor.read_u16_le()?;

        let mut cursor = ReadCursor::new(crc_bytes);
        let len = cursor.read_u8()?;

        let header = Header::new(
            ControlField::from(cursor.read_u8()?),
            AnyAddress::from(cursor.read_u16_le()?),
            AnyAddress::from(cursor.read_u16_le()?),
        );

        if len < 5 {
            return Err(FrameError::BadLength(len).into());
        }

        let expected_crc = super::crc::calc_crc_with_0564(crc_bytes);
        if crc_value != expected_crc {
            return Err(FrameError::BadHeaderCrc.into());
        }

        let trailer_length = Self::calc_trailer_length(len - 5); // ok b/c len >= 5 above

        self.state = ParseState::ReadBody(header, trailer_length);
        Ok(())
    }

    fn parse_body(
        &mut self,
        trailer_length: usize,
        cursor: &mut ReadCursor,
        payload: &mut FramePayload,
    ) -> Result<Option<()>, ParseError> {
        if cursor.remaining() < trailer_length {
            return Ok(None);
        }

        payload.clear();

        let body = cursor.read_bytes(trailer_length)?;

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
                return Err(FrameError::BadBodyCrc.into());
            }

            // copy the data and advance the position
            payload.push(data)?;
        }

        self.state = ParseState::FindSync1;
        Ok(Some(()))
    }
}

#[cfg(test)]
mod test {
    use super::super::test_data::*;
    use super::*;

    fn test_frame_parsing(parser: &mut Parser, frame: &TestFrame) {
        let mut cursor = ReadCursor::new(frame.bytes);
        let mut payload = FramePayload::new();
        let header: Header = parser.parse(&mut cursor, &mut payload).unwrap().unwrap();
        assert_eq!(cursor.remaining(), 0);
        assert_eq!(header, frame.header);
        assert_eq!(payload.get(), frame.payload)
    }

    #[test]
    fn catches_bad_start1() {
        let mut parser = Parser::new(LinkErrorMode::Close);
        let mut cursor = ReadCursor::new(&[0x06]);
        let mut payload = FramePayload::new();

        assert_eq!(
            parser.parse(&mut cursor, &mut payload),
            Err(ParseError::BadFrame(FrameError::UnexpectedStart1(0x06)))
        );

        assert!(cursor.is_empty());
    }

    #[test]
    fn catches_bad_start2() {
        let mut parser = Parser::new(LinkErrorMode::Close);
        let mut cursor = ReadCursor::new(&[0x05, 0x65]);
        let mut payload = FramePayload::new();

        assert_eq!(
            parser.parse(&mut cursor, &mut payload),
            Err(ParseError::BadFrame(FrameError::UnexpectedStart2(0x65)))
        );

        assert!(cursor.is_empty());
    }

    #[test]
    fn catches_bad_length() {
        let mut parser = Parser::new(LinkErrorMode::Close);
        let mut cursor =
            ReadCursor::new(&[0x05, 0x64, 0x04, 0xC0, 0x01, 0x00, 0x00, 0x04, 0xE9, 0x21]);
        let mut payload = FramePayload::new();

        assert_eq!(
            parser.parse(&mut cursor, &mut payload),
            Err(ParseError::BadFrame(FrameError::BadLength(4)))
        );
        assert_eq!(cursor.remaining(), 0);
    }

    #[test]
    fn header_parse_catches_bad_crc() {
        let mut parser = Parser::new(LinkErrorMode::Close);
        let mut cursor =
            ReadCursor::new(&[0x05, 0x64, 0x05, 0xC0, 0x01, 0x00, 0x00, 0x04, 0xE9, 0x20]);
        let mut payload = FramePayload::new();

        assert_eq!(
            parser.parse(&mut cursor, &mut payload),
            Err(ParseError::BadFrame(FrameError::BadHeaderCrc))
        );
        assert_eq!(cursor.remaining(), 0);
    }

    #[test]
    fn catches_bad_crc_in_body() {
        let data: [u8; 27] = [
            // header
            0x05, 0x64, 0x14, 0xF3, 0x01, 0x00, 0x00, 0x04, 0x0A, 0x3B, // body
            0xC0, 0xC3, 0x01, 0x3C, 0x02, 0x06, 0x3C, 0x03, 0x06, 0x3C, 0x04, 0x06, 0x3C, 0x01,
            0x06, 0x9A, 0xFF,
        ];

        let mut parser = Parser::new(LinkErrorMode::Close);
        let mut cursor = ReadCursor::new(&data);
        let mut payload = FramePayload::new();

        assert_eq!(
            parser.parse(&mut cursor, &mut payload),
            Err(ParseError::BadFrame(FrameError::BadBodyCrc)),
        );
        assert_eq!(cursor.remaining(), 0);
    }

    #[test]
    fn can_parse_multiple_different_frames_sequentially() {
        let mut parser = Parser::new(LinkErrorMode::Close);
        test_frame_parsing(&mut parser, &RESET_LINK);
        test_frame_parsing(&mut parser, &ACK);
        test_frame_parsing(&mut parser, &CONFIRM_USER_DATA);
    }

    #[test]
    fn can_consume_leading_garbage_in_discard_mode() {
        let mut parser = Parser::new(LinkErrorMode::Discard);
        // -- ------------ leading garbage ------------- valid frame -----------------------------------------------------
        let data = [
            0x06, 0x05, 0x07, 0x05, 0x64, 0x05, 0x05, 0x64, 0x05, 0xC0, 0x01, 0x00, 0x00, 0x04,
            0xE9, 0x21,
        ];
        let mut cursor = ReadCursor::new(&data);
        let mut payload = FramePayload::new();

        // consume leading garbage until we get to the valid frame
        assert_eq!(
            parser.parse(&mut cursor, &mut payload),
            Ok(Some(RESET_LINK.header)),
        );
    }
}
