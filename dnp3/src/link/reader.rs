use crate::decode::DecodeLevel;
use crate::link::display::LinkDisplay;
use crate::link::error::{LinkError, LogicError};
use crate::link::header::Header;
use crate::link::parser::{FramePayload, Parser};
use crate::link::{LinkErrorMode, LinkReadMode};
use crate::util::phys::{PhysAddr, PhysLayer};

use crate::link;
use scursor::{ReadCursor, WriteCursor};

/// Given a fragment size, how should we size our read buffer
const fn read_buffer_size(fragment_size: usize) -> usize {
    // the link frames needed to carry a fragment of this size, and at least one
    let num_frames = fragment_size.div_ceil(link::constant::MAX_APP_BYTES_PER_FRAME);
    let num_frames = if num_frames == 0 { 1 } else { num_frames };

    // we add 1 to this number for transports like UDP to detect under-sized reads
    num_frames
        .saturating_mul(link::constant::MAX_LINK_FRAME_LENGTH)
        .saturating_add(1)
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

    pub(crate) const fn datagram(error_mode: LinkErrorMode) -> Self {
        Self {
            error_mode,
            read_mode: LinkReadMode::Datagram,
        }
    }

    #[cfg(feature = "serial")]
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

pub(crate) struct Reader {
    read_mode: LinkReadMode,
    parser: Parser,
    buffer: ReadBuffer,
    /// where the data in `buffer` came from, for datagrams that contain more than one frame
    addr: PhysAddr,
}

/// Bytes read from the physical layer: `begin..end` is unread, and `begin <= end <= buffer.len()`
struct ReadBuffer {
    begin: usize,
    end: usize,
    buffer: Box<[u8]>,
}

impl ReadBuffer {
    fn new(buffer_size: usize) -> Self {
        Self {
            begin: 0,
            end: 0,
            buffer: vec![0; buffer_size].into_boxed_slice(),
        }
    }
    /// move the unread bytes to the start of the buffer
    fn shift_unread_bytes(&mut self) -> Result<(), LogicError> {
        if self.begin > self.end || self.end > self.buffer.len() {
            return Err(LogicError::BadSize);
        }
        self.buffer.copy_within(self.begin..self.end, 0);
        self.end = self.num_bytes_unread();
        self.begin = 0;
        Ok(())
    }

    fn writable(&mut self) -> Result<&mut [u8], LogicError> {
        self.buffer.get_mut(self.end..).ok_or(LogicError::BadSize)
    }

    fn readable(&self) -> Result<&[u8], LogicError> {
        self.buffer
            .get(self.begin..self.end)
            .ok_or(LogicError::BadSize)
    }

    fn advance_write(&mut self, count: usize) -> Result<(), LogicError> {
        self.end = self
            .end
            .checked_add(count)
            .filter(|end| *end <= self.buffer.len())
            .ok_or(LogicError::BadSize)?;
        Ok(())
    }

    fn advance_read(&mut self, count: usize) -> Result<(), LogicError> {
        self.begin = self
            .begin
            .checked_add(count)
            .filter(|begin| *begin <= self.end)
            .ok_or(LogicError::BadSize)?;
        Ok(())
    }

    fn is_full(&self) -> bool {
        self.end == self.buffer.len()
    }

    fn reset(&mut self) {
        self.begin = 0;
        self.end = 0;
    }

    fn num_bytes_unread(&self) -> usize {
        self.end.saturating_sub(self.begin)
    }
}

impl Reader {
    pub(crate) fn new(link_modes: LinkModes, max_fragment_size: usize) -> Self {
        let buffer_size = read_buffer_size(max_fragment_size);

        Self {
            read_mode: link_modes.read_mode,
            parser: Parser::new(link_modes.error_mode),
            buffer: ReadBuffer::new(buffer_size),
            addr: PhysAddr::None,
        }
    }

    pub(crate) fn seed(&mut self, seed_data: &[u8]) -> Result<(), LinkError> {
        let mut cursor = WriteCursor::new(self.buffer.writable()?);
        cursor.write_bytes(seed_data)?;
        self.buffer.advance_write(seed_data.len())?;
        Ok(())
    }

    pub(crate) fn reset(&mut self) {
        self.buffer.reset();
        self.parser.reset();
        self.addr = PhysAddr::None;
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
    ) -> Result<(Header, PhysAddr), LinkError> {
        loop {
            // how much data is currently in the buffer?
            let length = self.buffer.num_bytes_unread();

            if length == 0 {
                self.buffer.reset();
                self.addr = self.read_more_data(io, level).await?;
            } else {
                match self.parse_buffer(payload, level)? {
                    None => {
                        if self.read_mode == LinkReadMode::Datagram {
                            // We didn't read a frame this iteration even though there was data in the buffer.
                            // This means that our datagram didn't contain a complete frame
                            tracing::warn!("Partial datagram of length {length} did not contain a full link-layer frame. Resetting link-layer parser.");
                            self.buffer.reset();
                            self.parser.reset();
                        }
                        self.addr = self.read_more_data(io, level).await?;
                    }
                    Some(header) => return Ok((header, self.addr)),
                }
            }
        }
    }

    pub(crate) async fn read_more_data(
        &mut self,
        io: &mut PhysLayer,
        level: DecodeLevel,
    ) -> Result<PhysAddr, LinkError> {
        // if we've consumed all the data, we need to shift contents
        if self.buffer.is_full() {
            self.buffer.shift_unread_bytes()?;
        }

        // now we can read more data
        let (count, addr) = io.read(self.buffer.writable()?, level.physical).await?;

        self.buffer.advance_write(count)?;
        Ok(addr)
    }

    /// Parse data currently available in the buffer
    pub(crate) fn parse_buffer(
        &mut self,
        payload: &mut FramePayload,
        level: DecodeLevel,
    ) -> Result<Option<Header>, LinkError> {
        // the readable portion of the buffer
        let mut cursor = ReadCursor::new(self.buffer.readable()?);
        let result = self.parser.parse(&mut cursor, payload)?;
        let consumed = cursor.position();
        self.buffer.advance_read(consumed)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_buffer_rejects_advances_past_its_bounds() {
        let mut buffer = ReadBuffer::new(4);

        assert_eq!(buffer.advance_write(5), Err(LogicError::BadSize));
        assert_eq!((buffer.begin, buffer.end), (0, 0));

        buffer.advance_write(3).unwrap();
        assert_eq!(buffer.advance_read(4), Err(LogicError::BadSize));
        assert_eq!((buffer.begin, buffer.end), (0, 3));

        buffer.advance_read(3).unwrap();
        assert_eq!(buffer.num_bytes_unread(), 0);
    }
    use crate::link::test_data::{ACK, RESET_LINK};
    use crate::udp::layer::UdpLayer;

    async fn bound() -> (PhysLayer, std::net::SocketAddr) {
        let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let local = socket.local_addr().unwrap();
        (PhysLayer::Udp(UdpLayer::bound(socket)), local)
    }

    async fn sender() -> (tokio::net::UdpSocket, PhysAddr) {
        let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr = PhysAddr::Udp(socket.local_addr().unwrap());
        (socket, addr)
    }

    async fn read(reader: &mut Reader, io: &mut PhysLayer) -> (Header, PhysAddr) {
        let mut payload = FramePayload::new();
        reader
            .read_frame(io, &mut payload, DecodeLevel::nothing())
            .await
            .unwrap()
    }

    fn datagram_reader() -> Reader {
        Reader::new(LinkModes::datagram(LinkErrorMode::Discard), 2048)
    }

    #[tokio::test]
    async fn later_datagram_from_another_sender_has_its_address() {
        let (mut io, local) = bound().await;
        let (first, first_addr) = sender().await;
        let (second, second_addr) = sender().await;
        first.send_to(RESET_LINK.bytes, local).await.unwrap();
        second.send_to(ACK.bytes, local).await.unwrap();

        let mut reader = datagram_reader();
        assert_eq!(
            read(&mut reader, &mut io).await,
            (RESET_LINK.header, first_addr)
        );
        assert_eq!(read(&mut reader, &mut io).await, (ACK.header, second_addr));
    }

    #[tokio::test]
    async fn partial_frame_is_not_joined_with_the_next_datagram() {
        let (mut io, local) = bound().await;
        let (first, first_addr) = sender().await;
        let (second, second_addr) = sender().await;
        let partial = ACK.bytes.get(..4).unwrap();
        first
            .send_to(&[RESET_LINK.bytes, partial].concat(), local)
            .await
            .unwrap();
        second.send_to(ACK.bytes, local).await.unwrap();

        let mut reader = datagram_reader();
        assert_eq!(
            read(&mut reader, &mut io).await,
            (RESET_LINK.header, first_addr)
        );
        assert_eq!(read(&mut reader, &mut io).await, (ACK.header, second_addr));
    }

    #[tokio::test]
    async fn frames_from_one_datagram_keep_the_sender_address() {
        let (mut io, local) = bound().await;
        let (sender, addr) = sender().await;
        sender
            .send_to(&[RESET_LINK.bytes, ACK.bytes].concat(), local)
            .await
            .unwrap();

        let mut reader = datagram_reader();
        assert_eq!(read(&mut reader, &mut io).await, (RESET_LINK.header, addr));
        assert_eq!(read(&mut reader, &mut io).await, (ACK.header, addr));
    }
}
