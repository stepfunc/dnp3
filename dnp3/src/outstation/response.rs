use scursor::{WriteCursor, WriteError};

use crate::app::*;

/// Holds an outstation response, from when it's written until it's replaced.
///
/// The outstation has to keep a response across other I/O so it can send it again: to answer a
/// repeated request, during a confirm wait, or as an unsolicited retry. The slot owns the complete
/// response, header and objects, so a resend is always the bytes of the response it repeats. No
/// caller supplies a length. The slot records the length itself, from the cursor it wrote with.
///
/// The master doesn't need this. It sends each request or confirm from the cursor that just wrote
/// it, and never resends one. A retried task formats a new request.
pub(crate) struct ResponseSlot {
    bytes: Box<[u8]>,
    /// length of the response in `bytes`, header included, never less than the header
    len: usize,
    /// header of the published response, or `None` if there isn't one to send
    header: Option<ResponseHeader>,
}

/// A response a handler has prepared, ready to publish
#[derive(Copy, Clone)]
pub(crate) enum Prepared {
    /// a response with no objects
    Empty(ResponseHeader),
    /// a response with the objects already in the slot, from the last successful
    /// [`ResponseSlot::build`] or the response published before, and nothing has built since
    Stored(ResponseHeader),
}

impl Prepared {
    pub(crate) fn empty_solicited(seq: Sequence, iin: Iin) -> Self {
        Self::Empty(ResponseHeader::solicited(seq, iin))
    }

    pub(crate) fn parameter_error(seq: Sequence) -> Self {
        Self::empty_solicited(seq, Iin::default() | Iin2::PARAMETER_ERROR)
    }

    pub(crate) fn header(self) -> ResponseHeader {
        match self {
            Self::Empty(header) | Self::Stored(header) => header,
        }
    }

    pub(crate) fn header_mut(&mut self) -> &mut ResponseHeader {
        match self {
            Self::Empty(header) | Self::Stored(header) => header,
        }
    }
}

impl ResponseSlot {
    pub(crate) fn new(size: BufferSize<249, 2048>) -> Self {
        Self {
            bytes: vec![0; size.value()].into_boxed_slice(),
            len: ResponseHeader::LENGTH,
            header: None,
        }
    }

    /// Write objects after the header. This doesn't publish them, and the response published
    /// before is gone.
    pub(crate) fn build<R>(
        &mut self,
        write: impl FnOnce(&mut WriteCursor) -> Result<R, WriteError>,
    ) -> Result<R, WriteError> {
        self.header = None;
        self.len = ResponseHeader::LENGTH;
        let mut cursor = WriteCursor::new(&mut self.bytes);
        cursor.skip(ResponseHeader::LENGTH)?;
        let result = write(&mut cursor)?;
        self.len = cursor.position().max(ResponseHeader::LENGTH);
        Ok(result)
    }

    /// Publish a response and return the bytes to send
    pub(crate) fn publish(&mut self, response: Prepared) -> &[u8] {
        let header = match response {
            Prepared::Empty(header) => {
                self.len = ResponseHeader::LENGTH;
                header
            }
            Prepared::Stored(header) => header,
        };
        for (dest, src) in self.bytes.iter_mut().zip(header.to_bytes()) {
            *dest = src;
        }
        self.header = Some(header);
        self.view()
    }

    /// The published response, to send again exactly as it was
    pub(crate) fn ready(&self) -> Option<&[u8]> {
        self.header.map(|_| self.view())
    }

    /// The header of the published response
    pub(crate) fn header(&self) -> Option<ResponseHeader> {
        self.header
    }

    pub(crate) fn clear(&mut self) {
        self.header = None;
    }

    fn view(&self) -> &[u8] {
        self.bytes.get(..self.len).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(seq: u8) -> ResponseHeader {
        ResponseHeader::new(
            ControlField::response(Sequence::new(seq), true, true, false),
            ResponseFunction::Response,
            Iin::default(),
        )
    }

    #[test]
    fn builds_are_not_published() {
        let mut slot = ResponseSlot::new(BufferSize::min());
        slot.publish(Prepared::Empty(header(1)));

        slot.build(|cursor| cursor.write_u8(0xAA)).unwrap();
        assert!(slot.ready().is_none());

        slot.publish(Prepared::Empty(header(2)));
        slot.build(|cursor| {
            cursor.write_u8(0xAA)?;
            cursor.write_bytes(&[0; 300])
        })
        .unwrap_err();
        assert!(slot.ready().is_none());
        assert!(slot.header().is_none());
    }

    #[test]
    fn publishes_the_header_and_objects() {
        let mut slot = ResponseSlot::new(BufferSize::min());
        slot.build(|cursor| cursor.write_bytes(&[0xAA, 0xBB]))
            .unwrap();

        let expected = [0xC1, 0x81, 0x00, 0x00, 0xAA, 0xBB];
        assert_eq!(slot.publish(Prepared::Stored(header(1))), expected);
        assert_eq!(slot.ready(), Some(expected.as_slice()));
    }

    #[test]
    fn empty_response_drops_the_previous_objects() {
        let mut slot = ResponseSlot::new(BufferSize::min());
        slot.build(|cursor| cursor.write_bytes(&[0xAA, 0xBB]))
            .unwrap();
        slot.publish(Prepared::Stored(header(1)));

        let expected = [0xC2, 0x81, 0x00, 0x00];
        assert_eq!(slot.publish(Prepared::Empty(header(2))), expected);
        assert_eq!(slot.ready(), Some(expected.as_slice()));
    }

    #[test]
    fn clear_unpublishes() {
        let mut slot = ResponseSlot::new(BufferSize::min());
        slot.publish(Prepared::Empty(header(1)));
        slot.clear();
        assert!(slot.ready().is_none());
        assert!(slot.header().is_none());
    }
}
