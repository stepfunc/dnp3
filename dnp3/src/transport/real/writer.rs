use crate::app::EndpointType;
use crate::config::{DecodeLevel, EndpointAddress};
use crate::link::error::LinkError;
use crate::link::format::{format_link_status_request, format_unconfirmed_user_data, Payload};
use crate::link::header::AnyAddress;
use crate::tokio::io::{AsyncWrite, AsyncWriteExt};
use crate::transport::real::display::SegmentDisplay;
use crate::transport::real::header::Header;
use crate::transport::real::sequence::Sequence;
use crate::util::cursor::WriteCursor;

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

    pub(crate) async fn write<W>(
        &mut self,
        io: &mut W,
        level: DecodeLevel,
        destination: AnyAddress,
        fragment: &[u8],
    ) -> Result<(), LinkError>
    where
        W: AsyncWrite + Unpin,
    {
        let chunks = fragment.chunks(crate::link::constant::MAX_APP_BYTES_PER_FRAME);

        let last = if chunks.len() == 0 {
            0
        } else {
            chunks.len() - 1
        };

        for (count, chunk) in chunks.enumerate() {
            let mut cursor = WriteCursor::new(&mut self.buffer);
            let header = Header::new(count == last, count == 0, self.seq.increment());
            let segment = SegmentDisplay::new(header, chunk, level.transport);
            let mark = cursor.position();
            format_unconfirmed_user_data(
                self.endpoint_type.dir_bit(),
                destination.value(),
                self.local_address.raw_value(),
                Payload::new(header.to_u8(), chunk),
                &mut cursor,
            )?;
            let written = cursor.written_since(mark)?;
            if level.transport.enabled() {
                tracing::info!("TRANSPORT TX - {}", segment);
            }
            io.write_all(written).await?;
        }

        Ok(())
    }

    pub(crate) async fn write_link_status_request<W>(
        &mut self,
        io: &mut W,
        destination: AnyAddress,
    ) -> Result<(), LinkError>
    where
        W: AsyncWrite + Unpin,
    {
        let mut cursor = WriteCursor::new(&mut self.buffer);
        format_link_status_request(
            self.endpoint_type.dir_bit(),
            destination.value(),
            self.local_address.raw_value(),
            &mut cursor,
        )?;
        io.write_all(cursor.written()).await?;

        Ok(())
    }
}
