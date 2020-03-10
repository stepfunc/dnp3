use crate::error::*;
use crate::link::header::{Ctrl, Header};
use crate::util::cursor::{ReadCursor, ReadError};
use crate::util::slice_ext::{MutSliceExt, SliceExt};

#[derive(Copy, Clone)]
enum ParseState {
    FindSync1,
    FindSync2,
    ReadHeader,
    ReadBody(Header, usize), // the header + calculated trailer length
}

#[derive(Debug, PartialEq)]
pub struct Frame<'a> {
    header: Header,
    payload: &'a [u8],
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
            buffer: [0; 250],
        }
    }

    fn calc_trailer_length(data_length: u8) -> usize {
        let div16: u8 = data_length / 16;
        let mod16: u8 = data_length % 16;

        if mod16 == 0 {
            div16 as usize * 18
        } else {
            (div16 as usize * 18) + mod16 as usize + 2
        }
    }

    pub fn parse<'a>(
        &'a mut self,
        cursor: &mut ReadCursor,
    ) -> Result<Option<Frame<'a>>, ParseError> {
        match self.parse_impl(cursor) {
            Err(err) => {
                self.state = ParseState::FindSync1;
                Err(err)
            }
            Ok(None) => Ok(None),
            Ok(Some((header, length))) => {
                // TODO - use a non-panicking variant
                Ok(Some(Frame::new(header, &self.buffer[0..length])))
            }
        }
    }

    fn parse_impl(
        &mut self,
        cursor: &mut ReadCursor,
    ) -> Result<Option<(Header, usize)>, ParseError> {
        loop {
            let start = cursor.len();

            match self.state {
                ParseState::FindSync1 => self.parse_sync1(cursor)?,
                ParseState::FindSync2 => self.parse_sync2(cursor)?,
                ParseState::ReadHeader => self.parse_header(cursor)?,
                ParseState::ReadBody(header, length) => {
                    if let Some(length) = self.parse_body(length, cursor)? {
                        return Ok(Some((header, length)));
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
            Ctrl::from(cursor.read_u8()?),
            cursor.read_u16_le()?,
            cursor.read_u16_le()?,
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

            let (data, crc) = block.split_at_no_panic(data_len)?;
            let crc_value = ReadCursor::new(crc).read_u16_le()?;
            let calc_crc = super::crc::calc_crc(data);

            if crc_value != calc_crc {
                return Err(FrameError::BadBodyCRC.into());
            }

            // copy the data and advance the position
            self.buffer
                .as_mut()
                .get_mut_no_panic(pos..(pos + data_len))?
                .clone_from_slice(data);

            pos += data_len;
        }

        Ok(Some(pos))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::link::function::Function;

    #[test]
    fn header_parse_catches_bad_length() {
        // CRC is the 0x21E9 at the end (little endian)
        let frame: [u8; 10] = [0x05, 0x64, 0x04, 0xC0, 0x01, 0x00, 0x00, 0x04, 0xE9, 0x21];

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
        // CRC is the 0x21E9 at the end (little endian)
        let frame: [u8; 10] = [0x05, 0x64, 0x05, 0xC0, 0x01, 0x00, 0x00, 0x04, 0xE9, 0x20];

        let mut parser = Parser::new();
        let mut cursor = ReadCursor::new(&frame);

        assert_eq!(
            parser.parse(&mut cursor),
            Err(ParseError::BadFrame(FrameError::BadHeaderCRC))
        );
        assert_eq!(cursor.len(), 0);
    }

    #[test]
    fn returns_frame_for_length_of_five() {
        // CRC is the 0x21E9 at the end (little endian)
        let frame: [u8; 10] = [0x05, 0x64, 0x05, 0xC0, 0x01, 0x00, 0x00, 0x04, 0xE9, 0x21];

        let mut parser = Parser::new();
        let mut cursor = ReadCursor::new(&frame);

        assert_eq!(
            parser.parse(&mut cursor),
            Ok(Some(Frame {
                header: Header {
                    ctrl: Ctrl {
                        func: Function::PriResetLinkStates,
                        master: true,
                        fcb: false,
                        fcv: false
                    },
                    dest: 1,
                    src: 1024
                },
                payload: &[]
            }))
        );
        assert_eq!(cursor.len(), 0);
    }
}
