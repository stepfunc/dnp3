use crate::app::parse::parser::{FragmentDisplay, ParsedFragment};
use crate::app::EndpointType;
use crate::decode::DecodeLevel;
use crate::link::error::LinkError;
use crate::link::EndpointAddress;
use crate::transport::FragmentAddr;
use crate::util::phys::PhysLayer;

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

    pub(crate) async fn write(
        &mut self,
        io: &mut PhysLayer,
        level: DecodeLevel,
        destination: FragmentAddr,
        fragment: &[u8],
    ) -> Result<(), LinkError> {
        if level.application.enabled() {
            if let Ok(fragment) = ParsedFragment::parse(fragment) {
                let x: FragmentDisplay = fragment.display(level.application);
                tracing::info!("APP TX - {}", x);
            }
        }
        self.inner.write(io, level, destination, fragment).await
    }

    pub(crate) async fn send_link_status_request(
        &mut self,
        io: &mut PhysLayer,
        level: DecodeLevel,
        dest: FragmentAddr,
    ) -> Result<(), LinkError> {
        self.inner.write_link_status_request(io, dest, level).await
    }
}
