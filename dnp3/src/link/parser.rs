use crate::link::constant;
use crate::link::error::*;
use crate::link::header::{AnyAddress, ControlField, Header};
use crate::link::LinkErrorMode;

use scursor::{ReadCursor, ReadError};

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
        self.buffer.get(..self.length).unwrap_or_default()
    }

    pub(crate) fn push(&mut self, data: &[u8]) -> Result<(), LogicError> {
        let end = self
            .length
            .checked_add(data.len())
            .ok_or(LogicError::BadSize)?;
        let dest = self
            .buffer
            .get_mut(self.length..end)
            .ok_or(LogicError::BadSize)?;
        for (dest, src) in dest.iter_mut().zip(data) {
            *dest = *src;
        }
        self.length = end;
        Ok(())
    }
}

impl Default for FramePayload {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a complete link header, returning the header and the amount of data it announces
pub(crate) fn parse_header_bytes(
    bytes: [u8; constant::LINK_HEADER_LENGTH],
) -> Result<(Header, u8), FrameError> {
    let [start1, start2, fields @ ..] = bytes;
    if start1 != constant::START1 {
        return Err(FrameError::UnexpectedStart1(start1));
    }
    if start2 != constant::START2 {
        return Err(FrameError::UnexpectedStart2(start2));
    }
    parse_header_fields(fields)
}

/// Parse the header fields that follow the start bytes, returning the header and the amount of
/// data it announces
fn parse_header_fields(bytes: [u8; 8]) -> Result<(Header, u8), FrameError> {
    let [len, control, dest_0, dest_1, src_0, src_1, crc_0, crc_1] = bytes;

    // the length counts the control and address fields, then the data
    let data_length = len
        .checked_sub(constant::MIN_HEADER_LENGTH_VALUE)
        .ok_or(FrameError::BadLength(len))?;

    let expected_crc =
        super::crc::calc_crc_with_0564(&[len, control, dest_0, dest_1, src_0, src_1]);
    if u16::from_le_bytes([crc_0, crc_1]) != expected_crc {
        return Err(FrameError::BadHeaderCrc);
    }

    let header = Header::new(
        ControlField::from(control),
        AnyAddress::from(u16::from_le_bytes([dest_0, dest_1])),
        AnyAddress::from(u16::from_le_bytes([src_0, src_1])),
    );

    Ok((header, data_length))
}

pub(crate) struct Parser {
    mode: LinkErrorMode,
}

/// Why no frame could be read from the start of the unread bytes
enum NoFrame {
    /// the bytes are the start of a frame that needs more of them
    NeedData,
    /// the bytes don't start a valid frame
    Bad(ParseError),
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

impl<T: Into<ParseError>> From<T> for NoFrame {
    fn from(err: T) -> Self {
        NoFrame::Bad(err.into())
    }
}

impl Parser {
    pub(crate) fn new(mode: LinkErrorMode) -> Parser {
        Parser { mode }
    }

    /// Read the next frame, copying its data into `payload`
    ///
    /// The parser keeps nothing between calls: the bytes of an incomplete frame are left unread,
    /// to be parsed again from the frame's first byte once more of them arrive. In
    /// [`LinkErrorMode::Discard`], parsing restarts one byte past where an invalid frame began.
    #[cfg_attr(all(test, dnp3_no_panic), no_panic::no_panic)]
    pub(crate) fn parse(
        &mut self,
        cursor: &mut ReadCursor,
        payload: &mut FramePayload,
    ) -> Result<Option<Header>, ParseError> {
        loop {
            let mut frame = *cursor;
            match Self::read_frame(&mut frame, payload) {
                Ok(header) => {
                    *cursor = frame;
                    return Ok(Some(header));
                }
                Err(NoFrame::NeedData) => return Ok(None),
                Err(NoFrame::Bad(err)) => match self.mode {
                    LinkErrorMode::Close => {
                        *cursor = frame;
                        return Err(err);
                    }
                    LinkErrorMode::Discard => {
                        // skip the byte the invalid frame began on
                        if cursor.read_u8().is_err() {
                            return Ok(None);
                        }
                    }
                },
            }
        }
    }

    /// Read a whole frame from the cursor, copying its data into `payload`
    #[cfg_attr(all(test, dnp3_no_panic), no_panic::no_panic)]
    fn read_frame(cursor: &mut ReadCursor, payload: &mut FramePayload) -> Result<Header, NoFrame> {
        let start1 = cursor.read_u8().map_err(|_| NoFrame::NeedData)?;
        if start1 != constant::START1 {
            return Err(FrameError::UnexpectedStart1(start1).into());
        }

        let start2 = cursor.read_u8().map_err(|_| NoFrame::NeedData)?;
        if start2 != constant::START2 {
            return Err(FrameError::UnexpectedStart2(start2).into());
        }

        let fields = cursor
            .read_bytes(8)
            .ok()
            .and_then(<[u8]>::first_chunk)
            .ok_or(NoFrame::NeedData)?;
        let (header, data_length) = parse_header_fields(*fields)?;

        let trailer = cursor
            .read_bytes(Self::calc_trailer_length(data_length))
            .map_err(|_| NoFrame::NeedData)?;
        Self::read_body(trailer, payload)?;

        Ok(header)
    }

    /// the data and CRC bytes that follow the header, for a frame with this much data
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "a u8 of data needs at most 255 data bytes plus 32 CRC bytes"
    )]
    fn calc_trailer_length(data_length: u8) -> usize {
        let data_length = usize::from(data_length);
        let blocks = data_length.div_ceil(constant::MAX_BLOCK_SIZE);
        data_length + blocks * constant::CRC_LENGTH
    }

    /// Check the CRC of each block of the trailer, copying the data into `payload`
    fn read_body(trailer: &[u8], payload: &mut FramePayload) -> Result<(), ParseError> {
        payload.clear();

        for block in trailer.chunks(constant::MAX_BLOCK_SIZE_WITH_CRC) {
            // a block holds at least one byte of data before its CRC
            let Some(data_len) = block.len().checked_sub(2).filter(|n| *n > 0) else {
                return Err(LogicError::BadSize.into());
            };

            let (data, crc) = block
                .split_at_checked(data_len)
                .ok_or(LogicError::BadSize)?;
            let crc_value = ReadCursor::new(crc).read_u16_le()?;
            let calc_crc = super::crc::calc_crc(data);

            if crc_value != calc_crc {
                return Err(FrameError::BadBodyCrc.into());
            }

            payload.push(data)?;
        }

        Ok(())
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
    fn parses_frame_with_multiple_blocks() {
        let mut parser = Parser::new(LinkErrorMode::Close);
        test_frame_parsing(&mut parser, &MULTI_BLOCK_USER_DATA);
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
