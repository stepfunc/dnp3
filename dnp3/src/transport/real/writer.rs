use crate::app::EndpointType;
use crate::decode::DecodeLevel;
use crate::link::error::LinkError;
use crate::link::format::{format_data_frame, format_header_only, Payload};
use crate::link::EndpointAddress;
use crate::transport::real::display::SegmentDisplay;
use crate::transport::real::header::Header;
use crate::transport::real::sequence::Sequence;
use crate::util::phys::PhysLayer;

use crate::transport::FragmentAddr;
use scursor::WriteCursor;

pub(crate) struct Writer {
    endpoint_type: EndpointType,
    local_address: EndpointAddress,
    seq: Sequence,
    buffer: [u8; crate::link::constant::MAX_LINK_FRAME_LENGTH],
}

impl Writer {
    pub(crate) fn new(endpoint_type: EndpointType, local_address: EndpointAddress) -> Self {
        Self {
            endpoint_type,
            local_address,
            seq: Sequence::default(),
            buffer: [0; crate::link::constant::MAX_LINK_FRAME_LENGTH],
        }
    }

    pub(crate) fn reset(&mut self) {
        self.seq.reset();
    }

    pub(crate) async fn write(
        &mut self,
        io: &mut PhysLayer,
        level: DecodeLevel,
        destination: FragmentAddr,
        fragment: &[u8],
    ) -> Result<(), LinkError> {
        let chunks = fragment.chunks(crate::link::constant::MAX_APP_BYTES_PER_FRAME);

        let last = if chunks.len() == 0 {
            0
        } else {
            chunks.len() - 1
        };

        for (count, chunk) in chunks.enumerate() {
            let mut cursor = WriteCursor::new(&mut self.buffer);
            let header = Header::new(count == last, count == 0, self.seq.increment());
            if level.transport.enabled() {
                tracing::info!(
                    "TRANSPORT TX - {}",
                    SegmentDisplay::new(header, chunk, level.transport)
                );
            }
            let link_header = crate::link::header::Header::unconfirmed_user_data(
                self.endpoint_type.dir_bit(),
                destination.link.wrap(),
                self.local_address.wrap(),
            );
            let data = format_data_frame(
                link_header,
                Payload::new(header.to_u8(), chunk),
                &mut cursor,
            )?;
            if level.link.header_enabled() {
                tracing::info!("LINK TX - {}", data.to_link_display(level.link));
            }
            io.write(data.frame, destination.phys, level.physical)
                .await?;
        }

        Ok(())
    }

    pub(crate) async fn write_link_status_request(
        &mut self,
        io: &mut PhysLayer,
        destination: FragmentAddr,
        level: DecodeLevel,
    ) -> Result<(), LinkError> {
        let mut cursor = WriteCursor::new(&mut self.buffer);
        let header = crate::link::header::Header::request_link_status(
            self.endpoint_type.dir_bit(),
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
