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

/// Combines the error and read modes
#[derive(Copy, Clone, Debug)]
pub(crate) struct LinkModes {
    pub(crate) error_mode: LinkErrorMode,
    pub(crate) read_mode: LinkReadMode,
}

impl LinkModes {
    pub(crate) const fn stream(error_mode: LinkErrorMode) -> Self {
        Self {
            error_mode,
            read_mode: LinkReadMode::Stream,
        }
    }

    pub(crate) const fn serial() -> Self {
        Self {
            error_mode: LinkErrorMode::Discard,
            read_mode: LinkReadMode::Stream,
        }
    }

    #[cfg(test)]
    pub(crate) const fn test() -> Self {
        Self {
            error_mode: LinkErrorMode::Close,
            read_mode: LinkReadMode::Stream,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum LinkReadMode {
    /// Reading from a stream (TCP, serial, etc.) where link-layer frames MAY span separate calls to read
    Stream,
    /// Reading datagrams (UDP) where link-layer frames MAY NOT span separate calls to read
    Datagram,
}

pub(crate) struct Reader {
    read_mode: LinkReadMode,
    parser: Parser,
    begin: usize,
    end: usize,
    buffer: Box<[u8]>,
}

impl Reader {
    pub(crate) fn new(link_modes: LinkModes, max_fragment_size: usize) -> Self {
        let buffer_size = read_buffer_size(max_fragment_size);

        Self {
            read_mode: link_modes.read_mode,
            parser: Parser::new(link_modes.error_mode),
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
            // how much data is currently in the buffer?
            let length = self.end - self.begin;

            if length == 0 {
                // anytime we've consumed
                self.begin = 0;
                self.end = 0;
                self.read_more_data(io, level).await?;
            } else {
                match self.parse_buffer(payload, level)? {
                    None => {
                        if self.read_mode == LinkReadMode::Datagram {
                            // We didn't read a frame this iteration even though there was data in the buffer.
                            // This means that our datagram didn't contain a complete frame
                            tracing::warn!("Partial datagram of length {length} did not contain a full link-layer frame. Resetting link-layer parser.");
                            self.begin = 0;
                            self.end = 0;
                            self.parser.reset();
                            self.read_more_data(io, level).await?;
                        }
                    }
                    Some(header) => return Ok(header),
                }
            }
        }
    }

    pub(crate) async fn read_more_data(
        &mut self,
        io: &mut PhysLayer,
        level: DecodeLevel,
    ) -> Result<(), LinkError> {
        // if we've consumed all the data, we need to shift contents
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
        Ok(())
    }

    /// Read from the physical layer no more than once
    pub(crate) fn parse_buffer(
        &mut self,
        payload: &mut FramePayload,
        level: DecodeLevel,
    ) -> Result<Option<Header>, LinkError> {
        // the readable portion of the buffer
        let mut cursor = ReadCursor::new(&self.buffer[self.begin..self.end]);
        let start = cursor.remaining();
        let result = self.parser.parse(&mut cursor, payload)?;
        let consumed = start - cursor.remaining();
        self.begin += consumed;
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
            None => Ok(None),
        }
    }
}
