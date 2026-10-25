// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use super::*;

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

impl OutstationHandler {
    /// Answer a request that becomes the last valid request, publishing its response. Returns the
    /// confirm that the response waits for.
    pub(super) async fn respond<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        writer: &mut TransportWriter<T>,
        respond_to: FragmentAddr,
        request: LastValidRequest,
        response: Prepared<'_>,
        database: &DatabaseHandle,
    ) -> Result<Option<ResponseSeries>, LinkError> {
        let header = self
            .write_solicited(io, writer, respond_to, response, database)
            .await?;
        self.state.last_valid_request = Some(request);
        Ok(Self::awaited_confirm(header))
    }

    /// Accept a request without a response as the last valid request, so its repeat gets none
    pub(super) fn respond_with_nothing(
        &mut self,
        sol: &mut ResponseSlot,
        request: LastValidRequest,
    ) {
        sol.clear();
        self.state.last_valid_request = Some(request);
    }

    pub(super) async fn repeat_solicited<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        respond_to: FragmentAddr,
        out: &mut Responder<T>,
    ) -> Result<(), LinkError> {
        if let Some(response) = out.sol.ready() {
            out.writer
                .write(io, self.config.decode_level, respond_to, response)
                .await?;
        }
        Ok(())
    }

    pub(super) async fn repeat_unsolicited<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        out: &mut Responder<T>,
    ) -> Result<(), LinkError> {
        if let Some(response) = out.unsol.ready() {
            out.writer
                .write(io, self.config.decode_level, self.destination, response)
                .await?;
        }
        Ok(())
    }

    pub(super) async fn write_solicited<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        writer: &mut TransportWriter<T>,
        respond_to: FragmentAddr,
        mut response: Prepared<'_>,
        database: &DatabaseHandle,
    ) -> Result<ResponseHeader, LinkError> {
        self.finish_solicited_header(response.header_mut(), database);

        let header = response.header();
        writer
            .write(io, self.config.decode_level, respond_to, response.publish())
            .await?;

        Ok(header)
    }

    pub(super) async fn write_unsolicited<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        writer: &mut TransportWriter<T>,
        mut response: Prepared<'_>,
        database: &DatabaseHandle,
    ) -> Result<ResponseHeader, LinkError> {
        response.header_mut().iin |= self.get_response_iin(database);
        self.state
            .broadcast
            .report(response.header_mut(), ResponseKind::Unsolicited);

        let header = response.header();
        writer
            .write(
                io,
                self.config.decode_level,
                self.destination,
                response.publish(),
            )
            .await?;

        Ok(header)
    }

    /// Send a response with no objects that doesn't replace the response to the last valid request
    pub(super) async fn write_transient_solicited<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        writer: &mut TransportWriter<T>,
        respond_to: FragmentAddr,
        mut header: ResponseHeader,
        database: &DatabaseHandle,
    ) -> Result<(), LinkError> {
        self.finish_solicited_header(&mut header, database);

        writer
            .write(io, self.config.decode_level, respond_to, &header.to_bytes())
            .await
    }

    pub(super) fn finish_solicited_header(
        &mut self,
        header: &mut ResponseHeader,
        database: &DatabaseHandle,
    ) {
        header.iin |= self.get_response_iin(database);
        self.state.broadcast.report(header, ResponseKind::Solicited);
    }

    /// The confirm that a written solicited response waits for: its own, or a mandatory
    /// broadcast's
    pub(super) fn awaited_confirm(header: ResponseHeader) -> Option<ResponseSeries> {
        header
            .control
            .con
            .then(|| ResponseSeries::new(header.control.seq, header.control.fin))
    }

    pub(super) fn get_response_iin(&self, database: &DatabaseHandle) -> Iin {
        let mut iin = Iin::default();

        // Restart IIN bit
        if self.state.restart_iin_asserted {
            iin |= Iin1::RESTART
        }

        // Events available
        let events_info = database.get_events_info();
        if events_info.unwritten_classes.class1 {
            iin |= Iin1::CLASS_1_EVENTS;
        }
        if events_info.unwritten_classes.class2 {
            iin |= Iin1::CLASS_2_EVENTS;
        }
        if events_info.unwritten_classes.class3 {
            iin |= Iin1::CLASS_3_EVENTS;
        }

        // Buffer overflow
        if events_info.is_overflown {
            iin |= Iin2::EVENT_BUFFER_OVERFLOW;
        }

        // Application-controlled IIN bits
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
