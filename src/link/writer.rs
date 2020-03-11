use crate::error::TransmitError;
use crate::link::formatter::{LinkFormatter, Payload};
use crate::util::cursor::WriteCursor;
use crate::util::sequence::Sequence;
use tokio::io::{AsyncWrite, AsyncWriteExt};

pub mod transport {
    pub const FIN_MASK: u8 = 0b1000_0000;
    pub const FIR_MASK: u8 = 0b0100_0000;
    pub const SEQ_MASK: u8 = 0b0011_1111;

    pub fn get_header(fin: bool, fir: bool, seq: u8) -> u8 {
        let mut acc: u8 = 0;

        if fin {
            acc |= FIN_MASK;
        }
        if fir {
            acc |= FIR_MASK;
        }

        acc | (seq & SEQ_MASK)
    }
}

pub struct Writer {
    formatter: LinkFormatter,
    buffer: [u8; super::constant::MAX_LINK_FRAME_LENGTH],
}

impl Writer {
    pub fn new(source: u16, master: bool) -> Self {
        Self {
            formatter: LinkFormatter::new(master, source),
            buffer: [0; super::constant::MAX_LINK_FRAME_LENGTH],
        }
    }

    pub async fn write<W>(
        &mut self,
        io: &mut W,
        destination: u16,
        seq: &mut Sequence,
        fragment: &[u8],
    ) -> Result<(), TransmitError>
    where
        W: AsyncWrite + Unpin,
    {
        let chunks = fragment.chunks(super::constant::MAX_APP_BYTES_PER_FRAME);

        let last = if chunks.len() == 0 {
            0
        } else {
            chunks.len() - 1
        };

        let mut count = 0;

        for chunk in chunks {
            let mut cursor = WriteCursor::new(&mut self.buffer);
            let transport_byte = transport::get_header(count == last, count == 0, seq.next());
            let mark = cursor.position();
            self.formatter.format_unconfirmed_user_data(
                destination,
                Payload::new(transport_byte, chunk),
                &mut cursor,
            )?;
            let written = cursor.written_since(mark)?;
            io.write_all(written).await?;

            count += 1;
        }

        Ok(())
    }
}
