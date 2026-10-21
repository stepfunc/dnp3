use crate::decode::DecodeLevel;
use crate::link::error::LinkError;
use crate::transport::{FragmentAddr, TransportWrite};
use crate::util::phys::PhysLayer;

pub(crate) struct MockWriter;

impl TransportWrite for MockWriter {
    // just write the fragment directly to the I/O
    async fn write(
        &mut self,
        io: &mut PhysLayer,
        level: DecodeLevel,
        destination: FragmentAddr,
        fragment: &[u8],
    ) -> Result<(), LinkError> {
        io.write(fragment, destination.phys, level.physical).await?;
        Ok(())
    }

    async fn write_link_status_request(
        &mut self,
        _: &mut PhysLayer,
        _: FragmentAddr,
        _: DecodeLevel,
    ) -> Result<(), LinkError> {
        Ok(())
    }

    fn reset(&mut self) {}
}
