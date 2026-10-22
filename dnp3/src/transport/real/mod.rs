use crate::link::reader::LinkModes;
use crate::link::{EndpointAddress, LinkRole};
use crate::transport::Transport;

pub(crate) mod assembler;
pub(crate) mod display;
pub(crate) mod header;
pub(crate) mod reader;
pub(crate) mod sequence;
pub(crate) mod writer;

pub(crate) mod constants {
    pub(crate) const FIN_MASK: u8 = 0b1000_0000;
    pub(crate) const FIR_MASK: u8 = 0b0100_0000;
}

/// DNP3 link-layer framing with transport segmentation and reassembly
pub(crate) struct RealTransport;

impl Transport for RealTransport {
    type Reader = reader::Reader;
    type Writer = writer::Writer;

    fn new(
        link_modes: LinkModes,
        role: LinkRole,
        address: EndpointAddress,
        rx_buffer_size: usize,
    ) -> (Self::Reader, Self::Writer) {
        (
            reader::Reader::new(link_modes, role, address, rx_buffer_size),
            writer::Writer::new(role, address),
        )
    }
}
