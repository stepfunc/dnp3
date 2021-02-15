use crate::app::EndpointType;
use crate::config::DecodeLevel;
use crate::link::error::LinkError;
use crate::link::header::AnyAddress;
use crate::link::EndpointAddress;
use crate::util::phys::PhysLayer;

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
    pub(crate) async fn write(
        &mut self,
        io: &mut PhysLayer,
        level: DecodeLevel,
        _: AnyAddress,
        fragment: &[u8],
    ) -> Result<(), LinkError> {
        io.write(fragment, level.physical).await?;
        self.num_writes += 1;
        Ok(())
    }

    pub(crate) async fn write_link_status_request(
        &mut self,
        _: &mut PhysLayer,
        _: DecodeLevel,
        _: AnyAddress,
    ) -> Result<(), LinkError> {
        // ignore this yet
        Ok(())
    }
}
