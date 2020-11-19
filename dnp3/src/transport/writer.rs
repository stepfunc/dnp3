use crate::app::parse::parser::ParsedFragment;
use crate::app::parse::DecodeLogLevel;
use crate::link::error::LinkError;
use crate::link::formatter::{LinkFormatter, Payload};
use crate::transport::sequence::Sequence;
use crate::transport::TransportType;
use crate::util::cursor::WriteCursor;
use tokio::io::{AsyncWrite, AsyncWriteExt};

pub(crate) struct Writer {
    formatter: LinkFormatter,
    seq: Sequence,
    buffer: [u8; crate::link::constant::MAX_LINK_FRAME_LENGTH],
}

impl Writer {
    fn get_header(fin: bool, fir: bool, seq: Sequence) -> u8 {
        let mut acc: u8 = 0;

        if fin {
            acc |= super::constants::FIN_MASK;
        }
        if fir {
            acc |= super::constants::FIR_MASK;
        }

        acc | seq.value()
    }

    pub(crate) fn new(tt: TransportType, address: u16) -> Self {
        Self {
            formatter: LinkFormatter::new(tt.is_master(), address),
            seq: Sequence::default(),
            buffer: [0; crate::link::constant::MAX_LINK_FRAME_LENGTH],
        }
    }

    pub(crate) fn reset(&mut self) {
        self.seq.reset();
    }

    pub(crate) async fn write<W>(
        &mut self,
        level: DecodeLogLevel,
        io: &mut W,
        destination: u16,
        fragment: &[u8],
    ) -> Result<(), LinkError>
    where
        W: AsyncWrite + Unpin,
    {
        if level != DecodeLogLevel::Nothing {
            ParsedFragment::parse(level.transmit(), fragment).ok();
        }

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
            self.formatter.format_unconfirmed_user_data(
                destination,
                Payload::new(transport_byte, chunk),
                &mut cursor,
            )?;
            let written = cursor.written_since(mark)?;
            io.write_all(written).await?;
        }

        Ok(())
    }
}
