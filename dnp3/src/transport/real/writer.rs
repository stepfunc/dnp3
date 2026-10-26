// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::decode::DecodeLevel;
use crate::link::error::LinkError;
use crate::link::format::{format_data_frame, format_header_only, FrameData, Payload};
use crate::link::{EndpointAddress, LinkRole};
use crate::phys::PhysLayer;
use crate::transport::real::display::SegmentDisplay;
use crate::transport::real::header::Header;
use crate::transport::real::sequence::Sequence;

use crate::transport::{FragmentAddr, TransportWrite};
use scursor::WriteCursor;

pub(crate) struct Writer {
    role: LinkRole,
    local_address: EndpointAddress,
    seq: Sequence,
    buffer: [u8; crate::link::constant::MAX_LINK_FRAME_LENGTH],
}

impl Writer {
    pub(crate) fn new(role: LinkRole, local_address: EndpointAddress) -> Self {
        Self {
            role,
            local_address,
            seq: Sequence::default(),
            buffer: [0; crate::link::constant::MAX_LINK_FRAME_LENGTH],
        }
    }
}

/// where a segment falls in its fragment
#[derive(Copy, Clone)]
pub(crate) struct Position {
    pub(crate) first: bool,
    pub(crate) last: bool,
}

/// Format one segment of a fragment as an unconfirmed user data frame from `local` to
/// `destination`, taking the next sequence number from `seq`
pub(crate) fn format_segment<'a>(
    role: LinkRole,
    local: EndpointAddress,
    destination: EndpointAddress,
    seq: &mut Sequence,
    position: Position,
    chunk: &[u8],
    cursor: &'a mut WriteCursor,
) -> Result<(Header, FrameData<'a>), LinkError> {
    let header = Header::new(position.last, position.first, seq.increment());
    let link_header = crate::link::header::Header::unconfirmed_user_data(
        role.dir_bit(),
        destination.wrap(),
        local.wrap(),
    );
    let data = format_data_frame(link_header, Payload::new(header.to_u8(), chunk), cursor)?;
    Ok((header, data))
}

impl TransportWrite for Writer {
    fn reset(&mut self) {
        self.seq.reset();
    }

    async fn write(
        &mut self,
        io: &mut PhysLayer,
        level: DecodeLevel,
        destination: FragmentAddr,
        fragment: &[u8],
    ) -> Result<(), LinkError> {
        let mut chunks =
            crate::util::chunks::<_, { crate::link::constant::MAX_APP_BYTES_PER_FRAME }>(fragment)
                .peekable();

        let mut first = true;
        while let Some(chunk) = chunks.next() {
            let mut cursor = WriteCursor::new(&mut self.buffer);
            let last = chunks.peek().is_none();
            let (header, data) = format_segment(
                self.role,
                self.local_address,
                destination.link,
                &mut self.seq,
                Position { first, last },
                chunk,
                &mut cursor,
            )?;
            first = false;
            if level.transport.enabled() {
                tracing::info!(
                    "TRANSPORT TX - {}",
                    SegmentDisplay::new(header, chunk, level.transport)
                );
            }
            if level.link.enabled() {
                tracing::info!("LINK TX - {}", data.to_link_display(level.link));
            }
            io.write(data.frame, destination.phys, level.physical)
                .await?;
        }

        Ok(())
    }

    async fn write_link_status_request(
        &mut self,
        io: &mut PhysLayer,
        destination: FragmentAddr,
        level: DecodeLevel,
    ) -> Result<(), LinkError> {
        let mut cursor = WriteCursor::new(&mut self.buffer);
        let header = crate::link::header::Header::request_link_status(
            self.role.dir_bit(),
            destination.link,
            self.local_address,
        );

        let data = format_header_only(header, &mut cursor)?;
        if level.link.enabled() {
            tracing::info!("LINK TX - {}", data.to_link_display(level.link));
        }
        io.write(data.frame, destination.phys, level.physical)
            .await?;

        Ok(())
    }
}
