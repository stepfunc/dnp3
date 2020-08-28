use crate::link::error::LinkError;
use crate::link::header::Header;
use crate::link::parser::{FramePayload, Parser};
use crate::util::cursor::ReadCursor;
use std::io::ErrorKind;
use tokio::io::{AsyncRead, AsyncReadExt};

pub(crate) struct Reader {
    parser: Parser,
    begin: usize,
    end: usize,
    buffer: [u8; super::constant::MAX_LINK_FRAME_LENGTH],
}

impl Reader {
    pub(crate) fn new() -> Self {
        Self {
            parser: Parser::new(),
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
    pub(crate) async fn read<R>(
        &mut self,
        io: &mut R,
        payload: &mut FramePayload,
    ) -> Result<Header, LinkError>
    where
        R: AsyncRead + Unpin,
    {
        loop {
            // if all bytes are consumed, ensure these are set back to zero
            if self.begin == self.end {
                self.begin = 0;
                self.end = 0;
            }

            // the readable portion of the buffer
            let mut cursor = ReadCursor::new(&self.buffer[self.begin..self.end]);
            let start = cursor.len();
            let result = self.parser.parse(&mut cursor, payload)?;
            self.begin += start - cursor.len();
            match result {
                // complete frame
                Some(header) => return Ok(header),
                // parser can't make progress without more bytes
                None => {
                    // if the buffer is full, we need to shift its contents
                    if self.end == super::constant::MAX_LINK_FRAME_LENGTH {
                        self.buffer.copy_within(self.begin..self.end, 0);
                        self.end -= self.begin;
                        self.begin = 0;
                    }

                    // now we can read more data
                    let count = io.read(&mut self.buffer[self.end..]).await?;
                    if count == 0 {
                        return Err(LinkError::IO(ErrorKind::UnexpectedEof));
                    }

                    self.end += count;
                }
            }
        }
    }
}

impl Default for Reader {
    fn default() -> Self {
        Self::new()
    }
}
