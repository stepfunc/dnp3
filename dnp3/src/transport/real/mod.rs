use crate::link::reader::LinkModes;
use crate::link::EndpointAddress;
use crate::outstation::Feature;
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

    fn master(
        link_modes: LinkModes,
        address: EndpointAddress,
        rx_buffer_size: usize,
    ) -> (Self::Reader, Self::Writer) {
        (
            reader::Reader::master(link_modes, address, rx_buffer_size),
            writer::Writer::new(crate::app::EndpointType::Master, address),
        )
    }

    fn outstation(
        link_modes: LinkModes,
        address: EndpointAddress,
        self_address: Feature,
        rx_buffer_size: usize,
    ) -> (Self::Reader, Self::Writer) {
        (
            reader::Reader::outstation(link_modes, address, self_address, rx_buffer_size),
            writer::Writer::new(crate::app::EndpointType::Outstation, address),
        )
    }
}
