use crate::app::EndpointType;
use crate::config::EndpointAddress;
use crate::link::error::LinkError;
use crate::link::format::{format_link_status_request, format_unconfirmed_user_data, Payload};
use crate::link::header::AnyAddress;
use crate::tokio::io::{AsyncWrite, AsyncWriteExt};
use crate::transport::real::constants::{FIN_MASK, FIR_MASK};
use crate::transport::real::sequence::Sequence;
use crate::util::cursor::WriteCursor;

pub(crate) struct Writer {
    endpoint_type: EndpointType,
    local_address: EndpointAddress,
    seq: Sequence,
    buffer: [u8; crate::link::constant::MAX_LINK_FRAME_LENGTH],
}

impl Writer {
    fn get_header(fin: bool, fir: bool, seq: Sequence) -> u8 {
        let mut acc: u8 = 0;

        if fin {
            acc |= FIN_MASK;
        }
        if fir {
            acc |= FIR_MASK;
        }

        acc | seq.value()
    }

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
            let transport_byte = Self::get_header(count == last, count == 0, self.seq.increment());
            let mark = cursor.position();
            format_unconfirmed_user_data(
                self.endpoint_type.dir_bit(),
                destination.value(),
                self.local_address.raw_value(),
                Payload::new(transport_byte, chunk),
                &mut cursor,
            )?;
            let written = cursor.written_since(mark)?;
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
