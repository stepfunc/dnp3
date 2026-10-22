use scursor::{WriteCursor, WriteError};

use crate::app::*;

/// Holds an outstation response, from when it's written until it's replaced.
///
/// The outstation has to keep a response across other I/O so it can send it again: to answer a
/// repeated request, during a confirm wait, or as an unsolicited retry. The slot owns the complete
/// response, header and objects, so a resend is always the bytes of the response it repeats. No
/// caller supplies a length. The slot records the length itself, from the cursor it wrote with.
///
/// A response is prepared in the slot that sends it: [`ResponseSlot::build`] and
/// [`ResponseSlot::empty`] borrow the slot until the [`Prepared`] response is published or
/// dropped. So a header can only be published with the objects of the slot it was prepared in,
/// and nothing else writes to that slot in between.
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

/// The objects a build left in its slot, waiting for their header: all of them, or none if the
/// build failed
pub(crate) struct Staged<'a> {
    slot: &'a mut ResponseSlot,
}

impl<'a> Staged<'a> {
    pub(crate) fn with_header(self, header: ResponseHeader) -> Prepared<'a> {
        Prepared {
            slot: self.slot,
            header,
        }
    }
}

/// A response prepared in a slot, ready to publish there
pub(crate) struct Prepared<'a> {
    slot: &'a mut ResponseSlot,
    header: ResponseHeader,
}

impl<'a> Prepared<'a> {
    pub(crate) fn header(&self) -> ResponseHeader {
        self.header
    }

    pub(crate) fn header_mut(&mut self) -> &mut ResponseHeader {
        &mut self.header
    }

    /// Publish the response in its slot and return the bytes to send
    pub(crate) fn publish(self) -> &'a [u8] {
        let Self { slot, header } = self;
        for (dest, src) in slot.bytes.iter_mut().zip(header.to_bytes()) {
            *dest = src;
        }
        slot.header = Some(header);
        slot.view()
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

    /// Write objects after the header, for a response to prepare from the result. The response
    /// published before is gone.
    pub(crate) fn build<R>(
        &mut self,
        write: impl FnOnce(&mut WriteCursor) -> Result<R, WriteError>,
    ) -> (Result<R, WriteError>, Staged<'_>) {
        self.header = None;
        self.len = ResponseHeader::LENGTH;
        let result = self.write_objects(write);
        (result, Staged { slot: self })
    }

    fn write_objects<R>(
        &mut self,
        write: impl FnOnce(&mut WriteCursor) -> Result<R, WriteError>,
    ) -> Result<R, WriteError> {
        let mut cursor = WriteCursor::new(&mut self.bytes);
        cursor.skip(ResponseHeader::LENGTH)?;
        let result = write(&mut cursor)?;
        self.len = cursor.position().max(ResponseHeader::LENGTH);
        Ok(result)
    }

    /// Prepare a response with no objects. The response published before is gone.
    pub(crate) fn empty(&mut self, header: ResponseHeader) -> Prepared<'_> {
        self.header = None;
        self.len = ResponseHeader::LENGTH;
        Prepared { slot: self, header }
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
        slot.empty(header(1)).publish();

        let (result, _) = slot.build(|cursor| cursor.write_u8(0xAA));
        result.unwrap();
        assert!(slot.ready().is_none());

        slot.empty(header(2)).publish();
        let (result, _) = slot.build(|cursor| {
            cursor.write_u8(0xAA)?;
            cursor.write_bytes(&[0; 300])
        });
        result.unwrap_err();
        assert!(slot.ready().is_none());
        assert!(slot.header().is_none());
    }

    #[test]
    fn publishes_the_header_and_objects() {
        let mut slot = ResponseSlot::new(BufferSize::min());
        let (result, staged) = slot.build(|cursor| cursor.write_bytes(&[0xAA, 0xBB]));
        result.unwrap();

        let expected = [0xC1, 0x81, 0x00, 0x00, 0xAA, 0xBB];
        assert_eq!(staged.with_header(header(1)).publish(), expected);
        assert_eq!(slot.ready(), Some(expected.as_slice()));
    }

    #[test]
    fn failed_build_leaves_no_objects() {
        let mut slot = ResponseSlot::new(BufferSize::min());
        let (result, staged) = slot.build(|cursor| {
            cursor.write_u8(0xAA)?;
            cursor.write_bytes(&[0; 300])
        });
        result.unwrap_err();

        let expected = [0xC1, 0x81, 0x00, 0x00];
        assert_eq!(staged.with_header(header(1)).publish(), expected);
        assert_eq!(slot.ready(), Some(expected.as_slice()));
    }

    #[test]
    fn empty_response_drops_the_previous_objects() {
        let mut slot = ResponseSlot::new(BufferSize::min());
        let (result, staged) = slot.build(|cursor| cursor.write_bytes(&[0xAA, 0xBB]));
        result.unwrap();
        staged.with_header(header(1)).publish();

        let expected = [0xC2, 0x81, 0x00, 0x00];
        assert_eq!(slot.empty(header(2)).publish(), expected);
        assert_eq!(slot.ready(), Some(expected.as_slice()));
    }

    #[test]
    fn dropped_empty_response_leaves_nothing_published() {
        let mut slot = ResponseSlot::new(BufferSize::min());
        slot.empty(header(1)).publish();

        {
            let _unpublished = slot.empty(header(2));
        }
        assert!(slot.ready().is_none());
        assert!(slot.header().is_none());
    }

    #[test]
    fn clear_unpublishes() {
        let mut slot = ResponseSlot::new(BufferSize::min());
        slot.empty(header(1)).publish();
        slot.clear();
        assert!(slot.ready().is_none());
        assert!(slot.header().is_none());
    }
}
