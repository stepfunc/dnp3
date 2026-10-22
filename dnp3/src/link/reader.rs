use crate::decode::DecodeLevel;
use crate::link::display::LinkDisplay;
use crate::link::error::{LinkError, LogicError};
use crate::link::header::Header;
use crate::link::parser::{FramePayload, Parser};
use crate::link::{LinkErrorMode, LinkReadMode};
use crate::util::phys::{PhysAddr, PhysLayer};

use crate::link;
use scursor::{ReadCursor, WriteCursor};

/// The largest read for a fragment size: the frames that carry a fragment of this size
const fn max_read_size(fragment_size: usize) -> usize {
    // the link frames needed to carry a fragment of this size, and at least one
    let num_frames = fragment_size.div_ceil(link::constant::MAX_APP_BYTES_PER_FRAME);
    let num_frames = if num_frames == 0 { 1 } else { num_frames };

    // we add 1 to this number for transports like UDP to detect under-sized reads
    num_frames
        .saturating_mul(link::constant::MAX_LINK_FRAME_LENGTH)
        .saturating_add(1)
}

/// The most bytes of an incomplete frame a stream leaves unread between reads
const MAX_INCOMPLETE_FRAME: usize = link::constant::MAX_LINK_FRAME_LENGTH - 1;

/// The read buffer for a read mode and fragment size
///
/// A datagram reader discards what it can't parse before each read. A stream reader keeps an
/// incomplete frame, and still has room for the largest read after it.
const fn read_buffer_size(read_mode: LinkReadMode, fragment_size: usize) -> usize {
    let read_size = max_read_size(fragment_size);
    match read_mode {
        LinkReadMode::Datagram => read_size,
        LinkReadMode::Stream => read_size.saturating_add(MAX_INCOMPLETE_FRAME),
    }
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
        let buffer_size = read_buffer_size(link_modes.read_mode, max_fragment_size);

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
            match self.step(payload)? {
                Step::Frame(header) => {
                    if level.link.enabled() {
                        tracing::info!(
                            "LINK RX - {}",
                            LinkDisplay::new(header, payload.get(), level.link)
                        );
                    }
                    return Ok((header, self.addr));
                }
                Step::NeedData => {}
                Step::PartialDatagram(length) => {
                    tracing::warn!("Partial datagram of length {length} did not contain a full link-layer frame. Resetting link-layer parser.");
                }
            }
            let space = if io.is_datagram_socket() {
                self.datagram_space()?
            } else {
                self.write_space()?
            };
            let (count, addr) = io.read(space, level.physical).await?;
            self.wrote(count)?;
            self.addr = addr;
        }
    }

    /// Parse the data in the buffer, and report what happened
    #[cfg_attr(all(test, dnp3_no_panic), no_panic::no_panic)]
    pub(crate) fn step(&mut self, payload: &mut FramePayload) -> Result<Step, LinkError> {
        // how much data is currently in the buffer?
        let length = self.buffer.num_bytes_unread();

        if length == 0 {
            self.buffer.reset();
            return Ok(Step::NeedData);
        }

        // the readable portion of the buffer
        let mut cursor = ReadCursor::new(self.buffer.readable()?);
        let result = self.parser.parse(&mut cursor, payload)?;
        let consumed = cursor.position();
        self.buffer.advance_read(consumed)?;
        match result {
            Some(header) => Ok(Step::Frame(header)),
            // the parser can't make progress without more bytes
            None => {
                if self.read_mode == LinkReadMode::Datagram {
                    // the datagram didn't contain a complete frame
                    self.buffer.reset();
                    Ok(Step::PartialDatagram(length))
                } else {
                    Ok(Step::NeedData)
                }
            }
        }
    }

    /// The space to read more data into, after moving the unread bytes to the start of the
    /// buffer if it's full
    #[cfg_attr(all(test, dnp3_no_panic), no_panic::no_panic)]
    pub(crate) fn write_space(&mut self) -> Result<&mut [u8], LogicError> {
        if self.buffer.is_full() {
            self.buffer.shift_unread_bytes()?;
        }
        self.buffer.writable()
    }

    /// The space to read a datagram into, after moving the unread bytes to the start of the buffer
    ///
    /// A datagram that doesn't fit is truncated, so this always leaves room for the largest read:
    /// a stream reader's unread bytes are at most one incomplete frame, and a datagram reader's
    /// buffer is empty.
    #[cfg_attr(all(test, dnp3_no_panic), no_panic::no_panic)]
    pub(crate) fn datagram_space(&mut self) -> Result<&mut [u8], LogicError> {
        self.buffer.shift_unread_bytes()?;
        self.buffer.writable()
    }

    /// Record that `count` bytes were read into [`Self::write_space`] or [`Self::datagram_space`]
    #[cfg_attr(all(test, dnp3_no_panic), no_panic::no_panic)]
    pub(crate) fn wrote(&mut self, count: usize) -> Result<(), LogicError> {
        self.buffer.advance_write(count)
    }
}

/// What [`Reader::step`] did with the data in the buffer
#[derive(Debug, PartialEq)]
pub(crate) enum Step {
    /// a complete frame
    Frame(Header),
    /// more data is needed to make progress
    NeedData,
    /// a datagram of this length held no complete frame, so it was discarded
    PartialDatagram(usize),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link::error::FrameError;
    use crate::link::format::{format_data_frame, Payload};
    use crate::link::header::AnyAddress;
    use crate::link::test_data::{
        TestFrame, ACK, CONFIRM_USER_DATA, MULTI_BLOCK_USER_DATA, RESET_LINK, UNCONFIRMED_USER_DATA,
    };
    use crate::udp::layer::UdpLayer;

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

    async fn bound() -> (PhysLayer, std::net::SocketAddr) {
        let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let local = socket.local_addr().unwrap();
        (PhysLayer::udp(UdpLayer::bound(socket)), local)
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

    /// a frame's header and payload
    type Frame = (Header, Vec<u8>);

    fn frame(frame: &TestFrame) -> Frame {
        (frame.header, frame.payload.to_vec())
    }

    /// read `chunk` into the reader
    fn load(reader: &mut Reader, chunk: &[u8]) -> Result<(), LogicError> {
        reader
            .write_space()?
            .get_mut(..chunk.len())
            .unwrap()
            .copy_from_slice(chunk);
        reader.wrote(chunk.len())
    }

    /// read `chunk` into the reader, then step until it needs more data, adding the frames to
    /// `frames`
    fn feed(
        reader: &mut Reader,
        payload: &mut FramePayload,
        chunk: &[u8],
        frames: &mut Vec<Frame>,
    ) -> Result<(), LinkError> {
        load(reader, chunk)?;
        loop {
            match reader.step(payload)? {
                Step::Frame(header) => frames.push((header, payload.get().to_vec())),
                Step::NeedData | Step::PartialDatagram(_) => return Ok(()),
            }
        }
    }

    /// the frames read from `chunks`, and the error that stopped the reader, if any
    fn receive(modes: LinkModes, chunks: &[&[u8]]) -> (Vec<Frame>, Option<LinkError>) {
        let mut reader = Reader::new(modes, 2048);
        let mut payload = FramePayload::new();
        let mut frames = Vec::new();
        for chunk in chunks {
            if let Err(err) = feed(&mut reader, &mut payload, chunk, &mut frames) {
                return (frames, Some(err));
            }
        }
        (frames, None)
    }

    /// `bytes` with the last one changed
    fn with_last_byte_flipped(bytes: &[u8]) -> Vec<u8> {
        let mut bytes = bytes.to_vec();
        *bytes.last_mut().unwrap() ^= 0xFF;
        bytes
    }

    /// corrupt input, and the error that closes the session on it in `Close` mode
    fn corruptions() -> Vec<(Vec<u8>, FrameError)> {
        let mut bad_length = RESET_LINK.bytes.to_vec();
        bad_length[2] = 4;
        let mut bad_middle_block_crc = MULTI_BLOCK_USER_DATA.bytes.to_vec();
        bad_middle_block_crc[45] ^= 0xFF;
        vec![
            (vec![0x06], FrameError::UnexpectedStart1(0x06)),
            (vec![0x05, 0x65], FrameError::UnexpectedStart2(0x65)),
            // a stray first start byte right before a frame
            (vec![0x05], FrameError::UnexpectedStart2(0x05)),
            // stray start bytes right before a frame
            (vec![0x05, 0x64], FrameError::BadHeaderCrc),
            (bad_length, FrameError::BadLength(4)),
            (
                with_last_byte_flipped(RESET_LINK.bytes),
                FrameError::BadHeaderCrc,
            ),
            (
                with_last_byte_flipped(CONFIRM_USER_DATA.bytes),
                FrameError::BadBodyCrc,
            ),
            (bad_middle_block_crc, FrameError::BadBodyCrc),
            // cut inside the header: the next frame's first bytes complete it
            (
                CONFIRM_USER_DATA.bytes[..6].to_vec(),
                FrameError::BadHeaderCrc,
            ),
            // cut after the header: the next frame's bytes are read as the body
            (
                CONFIRM_USER_DATA.bytes[..10].to_vec(),
                FrameError::BadBodyCrc,
            ),
            // cut inside the body: the next frame's bytes complete it
            (
                CONFIRM_USER_DATA.bytes[..15].to_vec(),
                FrameError::BadBodyCrc,
            ),
        ]
    }

    /// a valid frame, the corrupt input, then two more valid frames
    fn surround(corrupt: &[u8]) -> Vec<u8> {
        [
            RESET_LINK.bytes,
            corrupt,
            UNCONFIRMED_USER_DATA.bytes,
            ACK.bytes,
        ]
        .concat()
    }

    /// the input as one chunk, split in two at every position, and one byte at a time
    fn chunkings(input: &[u8]) -> Vec<Vec<&[u8]>> {
        let mut chunkings: Vec<Vec<&[u8]>> = (0..=input.len())
            .map(|n| {
                let (first, second) = input.split_at(n);
                vec![first, second]
            })
            .collect();
        chunkings.push(input.chunks(1).collect());
        chunkings
    }

    #[test]
    fn discard_mode_recovers_the_frames_after_corrupt_input_however_it_arrives() {
        let expected = vec![
            frame(&RESET_LINK),
            frame(&UNCONFIRMED_USER_DATA),
            frame(&ACK),
        ];
        for (corrupt, _) in corruptions() {
            let input = surround(&corrupt);
            for chunks in chunkings(&input) {
                assert_eq!(
                    receive(LinkModes::stream(LinkErrorMode::Discard), &chunks),
                    (expected.clone(), None),
                    "corrupt input {corrupt:02X?} read as {chunks:02X?}"
                );
            }
        }
    }

    #[test]
    fn discard_mode_recovers_the_frames_after_corrupt_input_in_one_datagram() {
        let expected = vec![
            frame(&RESET_LINK),
            frame(&UNCONFIRMED_USER_DATA),
            frame(&ACK),
        ];
        for (corrupt, _) in corruptions() {
            let input = surround(&corrupt);
            assert_eq!(
                receive(LinkModes::datagram(LinkErrorMode::Discard), &[&input]),
                (expected.clone(), None),
                "corrupt input {corrupt:02X?}"
            );
        }
    }

    #[test]
    fn close_mode_fails_on_corrupt_input_after_the_frames_before_it() {
        for (corrupt, error) in corruptions() {
            let input = surround(&corrupt);
            for chunks in chunkings(&input) {
                assert_eq!(
                    receive(LinkModes::stream(LinkErrorMode::Close), &chunks),
                    (vec![frame(&RESET_LINK)], Some(LinkError::BadFrame(error))),
                    "corrupt input {corrupt:02X?} read as {chunks:02X?}"
                );
            }
        }
    }

    #[test]
    fn discard_mode_holds_frames_after_a_truncated_frame_until_its_length_arrives() {
        let mut reader = Reader::new(LinkModes::stream(LinkErrorMode::Discard), 2048);
        let mut payload = FramePayload::new();
        let mut frames = Vec::new();

        // the truncated frame announces 17 bytes after its header, and only 12 follow it
        let truncated = &CONFIRM_USER_DATA.bytes[..12];
        let first = [truncated, ACK.bytes].concat();
        feed(&mut reader, &mut payload, &first, &mut frames).unwrap();
        assert_eq!(frames, []);
        feed(
            &mut reader,
            &mut payload,
            UNCONFIRMED_USER_DATA.bytes,
            &mut frames,
        )
        .unwrap();
        assert_eq!(frames, [frame(&ACK), frame(&UNCONFIRMED_USER_DATA)]);
    }

    #[test]
    fn only_a_datagram_reader_discards_an_incomplete_frame_and_reports_its_length() {
        let input = [RESET_LINK.bytes, &ACK.bytes[..4]].concat();
        let mut payload = FramePayload::new();

        let mut reader = Reader::new(LinkModes::datagram(LinkErrorMode::Discard), 2048);
        load(&mut reader, &input).unwrap();
        assert_eq!(
            reader.step(&mut payload),
            Ok(Step::Frame(RESET_LINK.header))
        );
        assert_eq!(reader.step(&mut payload), Ok(Step::PartialDatagram(4)));
        assert_eq!(reader.step(&mut payload), Ok(Step::NeedData));
        // the rest of the frame, alone, is no frame either
        load(&mut reader, &ACK.bytes[4..]).unwrap();
        assert_eq!(reader.step(&mut payload), Ok(Step::PartialDatagram(6)));
        assert_eq!(reader.step(&mut payload), Ok(Step::NeedData));

        let mut reader = Reader::new(LinkModes::stream(LinkErrorMode::Discard), 2048);
        load(&mut reader, &input).unwrap();
        assert_eq!(
            reader.step(&mut payload),
            Ok(Step::Frame(RESET_LINK.header))
        );
        assert_eq!(reader.step(&mut payload), Ok(Step::NeedData));
        assert_eq!(reader.step(&mut payload), Ok(Step::NeedData));
        load(&mut reader, &ACK.bytes[4..]).unwrap();
        assert_eq!(reader.step(&mut payload), Ok(Step::Frame(ACK.header)));
        assert_eq!(reader.step(&mut payload), Ok(Step::NeedData));
    }

    /// a frame carrying the most data a frame can, filled with `fill`
    fn full_frame(fill: u8) -> (Vec<u8>, Frame) {
        let header =
            Header::unconfirmed_user_data(true, AnyAddress::from(1), AnyAddress::from(1024));
        let data = [fill; link::constant::MAX_APP_BYTES_PER_FRAME];
        let mut buffer = [0; link::constant::MAX_LINK_FRAME_LENGTH];
        let mut cursor = WriteCursor::new(&mut buffer);
        let bytes = format_data_frame(header, Payload::new(0xC0, &data), &mut cursor)
            .unwrap()
            .frame
            .to_vec();
        assert_eq!(bytes.len(), link::constant::MAX_LINK_FRAME_LENGTH);
        (bytes, (header, [&[0xC0], &data[..]].concat()))
    }

    /// a UDP stream reader for the smallest fragment size, which one frame carries
    fn udp_stream_reader() -> Reader {
        Reader::new(
            LinkModes::stream(LinkErrorMode::Discard),
            link::constant::MAX_APP_BYTES_PER_FRAME,
        )
    }

    /// the next frame, with its payload, and where it came from
    async fn next_frame(reader: &mut Reader, io: &mut PhysLayer) -> (Frame, PhysAddr) {
        let mut payload = FramePayload::new();
        let read = reader.read_frame(io, &mut payload, DecodeLevel::nothing());
        let (header, addr) = tokio::time::timeout(std::time::Duration::from_secs(1), read)
            .await
            .expect("no frame was read")
            .unwrap();
        ((header, payload.get().to_vec()), addr)
    }

    #[tokio::test]
    async fn udp_stream_datagram_that_completes_a_buffered_frame_is_not_truncated() {
        let (mut io, local) = bound().await;
        let (sender, addr) = sender().await;
        let (full, full_frame) = full_frame(0xAA);
        let (start, end) = full.split_at(283);
        sender
            .send_to(&[ACK.bytes, start].concat(), local)
            .await
            .unwrap();
        sender
            .send_to(&[end, ACK.bytes].concat(), local)
            .await
            .unwrap();

        let mut reader = udp_stream_reader();
        assert_eq!(next_frame(&mut reader, &mut io).await, (frame(&ACK), addr));
        assert_eq!(next_frame(&mut reader, &mut io).await, (full_frame, addr));
        assert_eq!(next_frame(&mut reader, &mut io).await, (frame(&ACK), addr));
    }

    #[tokio::test]
    async fn udp_stream_full_size_datagram_after_the_longest_incomplete_frame_is_not_truncated() {
        let (mut io, local) = bound().await;
        let (sender, addr) = sender().await;
        let (truncated, _) = full_frame(0xAA);
        let (full, full_frame) = full_frame(0xBB);
        // all but the last byte of a frame, whose end never arrives
        sender
            .send_to(&truncated[..truncated.len() - 1], local)
            .await
            .unwrap();
        sender.send_to(&full, local).await.unwrap();

        let mut reader = udp_stream_reader();
        assert_eq!(next_frame(&mut reader, &mut io).await, (full_frame, addr));
    }

    #[tokio::test]
    async fn udp_stream_full_size_datagrams_that_each_end_in_an_incomplete_frame_are_not_truncated()
    {
        let (mut io, local) = bound().await;
        let (sender, addr) = sender().await;
        let (first, first_frame) = full_frame(0xAA);
        let (second, second_frame) = full_frame(0xBB);
        let (first_start, first_end) = first.split_at(283);
        let (second_start, second_end) = second.split_at(283);
        let datagrams = [
            [ACK.bytes, first_start].concat(),
            [first_end, second_start].concat(),
            [second_end, ACK.bytes].concat(),
        ];
        assert_eq!(datagrams[1].len(), link::constant::MAX_LINK_FRAME_LENGTH);
        for datagram in datagrams {
            sender.send_to(&datagram, local).await.unwrap();
        }

        let mut reader = udp_stream_reader();
        assert_eq!(next_frame(&mut reader, &mut io).await, (frame(&ACK), addr));
        assert_eq!(next_frame(&mut reader, &mut io).await, (first_frame, addr));
        assert_eq!(next_frame(&mut reader, &mut io).await, (second_frame, addr));
        assert_eq!(next_frame(&mut reader, &mut io).await, (frame(&ACK), addr));
    }

    #[test]
    fn datagram_space_after_the_longest_incomplete_frame_fits_the_largest_datagram_and_one_byte() {
        let (truncated, _) = full_frame(0xAA);
        let mut reader = udp_stream_reader();
        let mut payload = FramePayload::new();
        load(&mut reader, &truncated[..truncated.len() - 1]).unwrap();
        assert_eq!(reader.step(&mut payload), Ok(Step::NeedData));
        // the extra byte shows a datagram that filled the space wasn't truncated
        assert_eq!(
            reader.datagram_space().unwrap().len(),
            link::constant::MAX_LINK_FRAME_LENGTH + 1
        );
    }
}
