use crate::app::parse::parser::ParsedFragment;
use crate::app::parse::DecodeLogLevel;
use crate::app::EndpointType;
use crate::entry::EndpointAddress;
use crate::link::error::LinkError;
use crate::link::header::AnyAddress;
use crate::tokio::io::AsyncWrite;

/// This type definition is used so that we can mock the transport writer during testing.
/// If Rust eventually allows `async fn` in traits, this could be removed
#[cfg(not(test))]
pub(crate) type InnerTransportWriter = crate::transport::real::writer::Writer;
#[cfg(test)]
pub(crate) type InnerTransportWriter = crate::transport::mock::writer::MockWriter;

pub(crate) struct TransportWriter {
    inner: InnerTransportWriter,
}

impl TransportWriter {
    pub(crate) fn new(endpoint_type: EndpointType, local_address: EndpointAddress) -> Self {
        Self {
            inner: InnerTransportWriter::new(endpoint_type, local_address),
        }
    }

    pub(crate) fn reset(&mut self) {
        self.inner.reset()
    }

    pub(crate) async fn write<W>(
        &mut self,
        io: &mut W,
        level: DecodeLogLevel,
        destination: AnyAddress,
        fragment: &[u8],
    ) -> Result<(), LinkError>
    where
        W: AsyncWrite + Unpin,
    {
        if level != DecodeLogLevel::Nothing {
            let _ = ParsedFragment::parse(level.transmit(), fragment);
        }
        self.inner.write(io, destination, fragment).await
    }

    pub(crate) async fn write_link_status_request<W>(
        &mut self,
        io: &mut W,
        destination: AnyAddress,
    ) -> Result<(), LinkError>
    where
        W: AsyncWrite + Unpin,
    {
        self.inner.write_link_status_request(io, destination).await
    }
}
