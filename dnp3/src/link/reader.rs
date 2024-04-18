use std::io::ErrorKind;

use crate::decode::DecodeLevel;
use crate::link::display::LinkDisplay;
use crate::link::error::LinkError;
use crate::link::header::Header;
use crate::link::parser::{FramePayload, Parser};
use crate::link::LinkErrorMode;
use crate::util::phys::PhysLayer;

use crate::link;
use scursor::ReadCursor;

/// How many link frames might be required to transport this much application data?
const fn num_link_frames(fragment_size: usize) -> usize {
    let full_link_frames = fragment_size / link::constant::MAX_APP_BYTES_PER_FRAME;

    if fragment_size % link::constant::MAX_APP_BYTES_PER_FRAME == 0 {
        full_link_frames
    } else {
        full_link_frames + 1
    }
}

/// Given a fragment size, how should we size our read buffer
const fn read_buffer_size(fragment_size: usize) -> usize {
    let num_frames = num_link_frames(fragment_size);

    let size = if num_frames == 0 {
        link::constant::MAX_LINK_FRAME_LENGTH
    } else {
        num_frames * link::constant::MAX_LINK_FRAME_LENGTH
    };

    // we add 1 to this number for transports like UDP to detect under-sized reads
    size + 1
}

pub(crate) struct Reader {
    parser: Parser,
    begin: usize,
    end: usize,
    buffer: Box<[u8]>,
}

impl Reader {
    pub(crate) fn new(mode: LinkErrorMode, max_fragment_size: usize) -> Self {
        let buffer_size = read_buffer_size(max_fragment_size);

        tracing::info!("link read buffer size: {buffer_size}");

        Self {
            parser: Parser::new(mode),
            begin: 0,
            end: 0,
            buffer: vec![0; buffer_size].into_boxed_slice(),
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
    pub(crate) async fn read_frame(
        &mut self,
        io: &mut PhysLayer,
        payload: &mut FramePayload,
        level: DecodeLevel,
    ) -> Result<Header, LinkError> {
        loop {
            if let Some(header) = self.read_partial(io, payload, level).await? {
                return Ok(header);
            }
        }
    }

    pub(crate) async fn read_partial(
        &mut self,
        io: &mut PhysLayer,
        payload: &mut FramePayload,
        level: DecodeLevel,
    ) -> Result<Option<Header>, LinkError> {

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
                Ok(Some(header))
            }
            // parser can't make progress without more bytes
            None => {
                // if the buffer is full, we need to shift its contents
                if self.end == self.buffer.len() {
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
                Ok(None)
            }
        }
    }
}
