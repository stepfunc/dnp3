use crate::link::reader::LinkModes;
use crate::link::{EndpointAddress, LinkRole};
use crate::transport::Transport;

pub(crate) mod reader;
pub(crate) mod writer;

/// Passes whole fragments to and from the physical layer, with no link or transport framing
pub(crate) struct MockTransport;

impl Transport for MockTransport {
    type Reader = reader::MockReader;
    type Writer = writer::MockWriter;

    fn new(
        _: LinkModes,
        _: LinkRole,
        _: EndpointAddress,
        rx_buffer_size: usize,
    ) -> (Self::Reader, Self::Writer) {
        (reader::MockReader::new(rx_buffer_size), writer::MockWriter)
    }
}
