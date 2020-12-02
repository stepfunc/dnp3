use crate::app::parse::parser::ParsedFragment;
use crate::app::parse::DecodeLogLevel;
use crate::app::EndpointType;
use crate::entry::EndpointAddress;
use crate::link::error::LinkError;
use crate::link::header::AnyAddress;
use crate::tokio::io::{AsyncWrite, AsyncWriteExt};

pub(crate) struct MockWriter {
    num_writes: usize,
}

// same signature as the real transport writer
impl MockWriter {
    pub(crate) fn new(_: EndpointType, _: EndpointAddress) -> Self {
        Self { num_writes: 0 }
    }

    pub(crate) fn reset(&mut self) {}

    pub(crate) fn num_writes(&self) -> usize {
        self.num_writes
    }

    // just write the fragment directly to the I/O
    pub(crate) async fn write<W>(
        &mut self,
        io: &mut W,
        level: DecodeLogLevel,
        _: AnyAddress,
        fragment: &[u8],
    ) -> Result<(), LinkError>
    where
        W: AsyncWrite + Unpin,
    {
        if level != DecodeLogLevel::Nothing {
            let _ = ParsedFragment::parse(level.transmit(), fragment);
        }
        io.write(fragment).await?;
        self.num_writes += 1;
        Ok(())
    }
}
