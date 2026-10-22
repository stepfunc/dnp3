use crate::decode::DecodeLevel;
use crate::link::error::LinkError;
use crate::transport::{FragmentAddr, TransportWrite};
use crate::util::phys::PhysLayer;

/// Written in place of a link status request, which the mock transport has no link layer to frame
pub(crate) const LINK_STATUS_REQUEST: &[u8] = b"link status request";

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
        io: &mut PhysLayer,
        destination: FragmentAddr,
        level: DecodeLevel,
    ) -> Result<(), LinkError> {
        io.write(LINK_STATUS_REQUEST, destination.phys, level.physical)
            .await?;
        Ok(())
    }

    fn reset(&mut self) {}
}
