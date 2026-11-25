// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use scursor::{WriteCursor, WriteError};

use super::context::SessionContext;
use super::phase::SolTx;
use super::state::ResponseKind;
use crate::app::*;
use crate::outstation::config::OutstationConfig;
use crate::transport::FragmentAddr;
use crate::util::clock::MonotonicClock;

/// Pre-allocated response buffers owned separately from [`SessionContext`] so response builders
/// can borrow a slot while calling `&mut self` methods on the context.
pub(super) struct Slots {
    /// Stored response to the last valid request (for repeats and solicited confirm-wait echoes).
    pub(super) sol: ResponseSlot,
    /// Stored last unsolicited response (for retries).
    pub(super) unsol: ResponseSlot,
    /// Ephemeral 4-byte response header for unknown function codes (does not overwrite `sol`).
    pub(super) transient: [u8; ResponseHeader::LENGTH],
}

impl Slots {
    pub(super) fn new(config: &OutstationConfig) -> Self {
        Self {
            sol: ResponseSlot::new(config.solicited_buffer_size),
            unsol: ResponseSlot::new(config.unsolicited_buffer_size),
            transient: [0; ResponseHeader::LENGTH],
        }
    }

    pub(super) fn clear(&mut self) {
        self.sol.clear();
        self.unsol.clear();
    }
}

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
pub(super) struct ResponseSlot {
    bytes: Box<[u8]>,
    /// length of the response in `bytes`, header included, never less than the header
    len: usize,
    /// header of the published response, or `None` if there isn't one to send
    header: Option<ResponseHeader>,
}

/// The objects a build left in its slot, waiting for their header: all of them, or none if the
/// build failed
pub(super) struct Staged<'a> {
    slot: &'a mut ResponseSlot,
}

impl<'a> Staged<'a> {
    pub(super) fn has_objects(&self) -> bool {
        self.slot.len > ResponseHeader::LENGTH
    }

    pub(super) fn with_header(self, header: ResponseHeader) -> Prepared<'a> {
        Prepared {
            slot: self.slot,
            header,
        }
    }
}

/// A response prepared in a slot, ready to publish there
pub(super) struct Prepared<'a> {
    slot: &'a mut ResponseSlot,
    header: ResponseHeader,
}

impl<'a> Prepared<'a> {
    pub(super) fn header(&self) -> ResponseHeader {
        self.header
    }

    pub(super) fn header_mut(&mut self) -> &mut ResponseHeader {
        &mut self.header
    }

    /// Publish the response in its slot and return the bytes to send
    pub(super) fn publish(self) -> &'a [u8] {
        let Self { slot, header } = self;
        for (dest, src) in slot.bytes.iter_mut().zip(header.to_bytes()) {
            *dest = src;
        }
        slot.header = Some(header);
        slot.view()
    }
}

impl ResponseSlot {
    pub(super) fn new(size: BufferSize<249, 2048>) -> Self {
        Self {
            bytes: vec![0; size.value()].into_boxed_slice(),
            len: ResponseHeader::LENGTH,
            header: None,
        }
    }

    /// Write objects after the header, for a response to prepare from the result. The response
    /// published before is gone.
    pub(super) fn build<R>(
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
    pub(super) fn empty(&mut self, header: ResponseHeader) -> Prepared<'_> {
        self.header = None;
        self.len = ResponseHeader::LENGTH;
        Prepared { slot: self, header }
    }

    /// The published response, to send again exactly as it was
    pub(super) fn ready(&self) -> Option<&[u8]> {
        self.header.map(|_| self.view())
    }

    /// The header of the published response
    pub(super) fn header(&self) -> Option<ResponseHeader> {
        self.header
    }

    pub(super) fn clear(&mut self) {
        self.header = None;
    }

    fn view(&self) -> &[u8] {
        self.bytes.get(..self.len).unwrap_or_default()
    }
}

impl<C: MonotonicClock> SessionContext<C> {
    pub(super) fn publish_solicited(&mut self, to: FragmentAddr, response: Prepared<'_>) -> SolTx {
        let header = self.publish(response, ResponseKind::Solicited);
        SolTx::new(to, header)
    }

    pub(super) fn publish_unsolicited(&mut self, response: Prepared<'_>) -> Sequence {
        self.publish(response, ResponseKind::Unsolicited)
            .control
            .seq
    }

    fn publish(&mut self, mut response: Prepared<'_>, kind: ResponseKind) -> ResponseHeader {
        self.finish_header(response.header_mut(), kind);
        let header = response.header();
        response.publish();
        header
    }

    /// Build a transient 4-byte solicited response header that doesn't overwrite `sol`.
    pub(super) fn build_transient_solicited(
        &mut self,
        transient: &mut [u8; ResponseHeader::LENGTH],
        mut header: ResponseHeader,
    ) {
        self.finish_header(&mut header, ResponseKind::Solicited);
        *transient = header.to_bytes();
    }

    fn finish_header(&mut self, header: &mut ResponseHeader, kind: ResponseKind) {
        header.iin |= self.get_response_iin();
        self.state.broadcast.report(header, kind);
    }

    fn get_response_iin(&self) -> Iin {
        let mut iin = Iin::default();

        // Restart IIN bit
        if self.state.restart_iin_asserted {
            iin |= Iin1::RESTART
        }

        // Events available & buffer overflow (short DB lock, released before application callback)
        let events_info = self.db.get_events_info();
        if events_info.unwritten_classes.class1 {
            iin |= Iin1::CLASS_1_EVENTS;
        }
        if events_info.unwritten_classes.class2 {
            iin |= Iin1::CLASS_2_EVENTS;
        }
        if events_info.unwritten_classes.class3 {
            iin |= Iin1::CLASS_3_EVENTS;
        }

        if events_info.is_overflown {
            iin |= Iin2::EVENT_BUFFER_OVERFLOW;
        }

        // Application-controlled IIN bits (outside DB lock)
        iin |= self.application.get_application_iin();

        iin
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
