use std::io::ErrorKind;

use crate::decode::DecodeLevel;
use crate::link::display::LinkDisplay;
use crate::link::error::LinkError;
use crate::link::header::Header;
use crate::link::parser::{FramePayload, Parser};
use crate::link::LinkErrorMode;
use crate::util::cursor::ReadCursor;
use crate::util::phys::PhysLayer;

pub(crate) struct Reader {
    parser: Parser,
    begin: usize,
    end: usize,
    buffer: [u8; super::constant::MAX_LINK_FRAME_LENGTH],
}

impl Reader {
    pub(crate) fn new(mode: LinkErrorMode) -> Self {
        Self {
            parser: Parser::new(mode),
            begin: 0,
            end: 0,
            buffer: [0; super::constant::MAX_LINK_FRAME_LENGTH],
        }
    }

    pub(crate) fn reset(&mut self) {
        self.begin = 0;
        self.end = 0;
        self.parser.reset();
    }

    /**
    Returns a future that keeps reading until a frame is received or an error is returned
    This future can be dropped without losing any state.
    */
    pub(crate) async fn read(
        &mut self,
        io: &mut PhysLayer,
        payload: &mut FramePayload,
        level: DecodeLevel,
    ) -> Result<Header, LinkError> {
        loop {
            // if all bytes are consumed, ensure these are set back to zero
            if self.begin == self.end {
                self.begin = 0;
                self.end = 0;
            }

            // the readable portion of the buffer
            let mut cursor = ReadCursor::new(&self.buffer[self.begin..self.end]);
            let start = cursor.remaining();
            let result = self.parser.parse(&mut cursor, payload)?;
            self.begin += start - cursor.remaining();
            match result {
                // complete frame
                Some(header) => {
                    if level.link.enabled() {
                        tracing::info!(
                            "LINK RX - {}",
                            LinkDisplay::new(header, payload.get(), level.link)
                        );
                    }
                    return Ok(header);
                }
                // parser can't make progress without more bytes
                None => {
                    // if the buffer is full, we need to shift its contents
                    if self.end == super::constant::MAX_LINK_FRAME_LENGTH {
                        self.buffer.copy_within(self.begin..self.end, 0);
                        self.end -= self.begin;
                        self.begin = 0;
                    }

                    // now we can read more data
                    let count = io
                        .read(&mut self.buffer[self.end..], level.physical)
                        .await?;
                    if count == 0 {
                        return Err(LinkError::Stdio(ErrorKind::UnexpectedEof));
                    }

                    self.end += count;
                }
            }
        }
    }
}
