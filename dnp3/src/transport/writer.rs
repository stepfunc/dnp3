use crate::app::parse::parser::ParsedFragment;
use crate::app::parse::DecodeLogLevel;
use crate::entry::NormalAddress;
use crate::link::error::LinkError;
use crate::link::formatter::{LinkFormatter, Payload};
use crate::link::header::{Address, AddressPair};
use crate::transport::sequence::Sequence;
use crate::transport::TransportType;
use crate::util::cursor::WriteCursor;
use tokio::io::{AsyncWrite, AsyncWriteExt};

pub(crate) struct Writer {
    local_address: NormalAddress,
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

    pub(crate) fn new(tt: TransportType, local_address: NormalAddress) -> Self {
        Self {
            local_address,
            formatter: LinkFormatter::new(tt.is_master()),
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
        level: DecodeLogLevel,
        destination: Address,
        fragment: &[u8],
    ) -> Result<(), LinkError>
    where
        W: AsyncWrite + Unpin,
    {
        if level != DecodeLogLevel::Nothing {
            let _ = ParsedFragment::parse(level.transmit(), fragment);
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
            let addresses = AddressPair::new(destination, self.local_address.wrap());
            self.formatter.format_unconfirmed_user_data(
                addresses,
                Payload::new(transport_byte, chunk),
                &mut cursor,
            )?;
            let written = cursor.written_since(mark)?;
            io.write_all(written).await?;
        }

        Ok(())
    }
}
