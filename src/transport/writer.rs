use crate::error::Error;
use crate::link::formatter::{LinkFormatter, Payload};
use crate::util::cursor::WriteCursor;
use crate::util::sequence::Sequence;
use tokio::io::{AsyncWrite, AsyncWriteExt};

pub struct Writer {
    formatter: LinkFormatter,
    seq: Sequence,
    buffer: [u8; crate::link::constant::MAX_LINK_FRAME_LENGTH],
}

impl Writer {
    fn get_header(fin: bool, fir: bool, seq: u8) -> u8 {
        let mut acc: u8 = 0;

        if fin {
            acc |= super::constants::FIN_MASK;
        }
        if fir {
            acc |= super::constants::FIR_MASK;
        }

        acc | (seq & super::constants::SEQ_MASK)
    }

    pub fn new(source: u16, master: bool) -> Self {
        Self {
            formatter: LinkFormatter::new(master, source),
            seq: Sequence::transport(),
            buffer: [0; crate::link::constant::MAX_LINK_FRAME_LENGTH],
        }
    }

    pub fn reset(&mut self) {
        self.seq.reset();
    }

    pub async fn write<W>(
        &mut self,
        io: &mut W,
        destination: u16,
        fragment: &[u8],
    ) -> Result<(), Error>
    where
        W: AsyncWrite + Unpin,
    {
        let chunks = fragment.chunks(crate::link::constant::MAX_APP_BYTES_PER_FRAME);

        let last = if chunks.len() == 0 {
            0
        } else {
            chunks.len() - 1
        };

        let mut count = 0;

        for chunk in chunks {
            let mut cursor = WriteCursor::new(&mut self.buffer);
            let transport_byte = Self::get_header(count == last, count == 0, self.seq.next());
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
