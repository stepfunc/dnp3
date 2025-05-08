pub(crate) use reader::*;
pub(crate) use types::*;
pub(crate) use writer::*;

use crate::app::parse::options::ParseOptions;
use crate::app::{BufferSize, EndpointType};
use crate::link::reader::LinkModes;
use crate::link::EndpointAddress;
use crate::outstation::Feature;

#[cfg(test)]
pub(crate) mod mock;
#[cfg(not(test))]
pub(crate) mod real;

mod reader;
mod types;
mod writer;

pub(crate) fn create_master_transport_layer(
    link_modes: LinkModes,
    parse_options: ParseOptions,
    address: EndpointAddress,
    rx_buffer_size: BufferSize<2048, 2048>,
) -> (TransportReader, TransportWriter) {
    (
        TransportReader::master(link_modes, parse_options, address, rx_buffer_size.value()),
        TransportWriter::new(EndpointType::Master, address),
    )
}

pub(crate) fn create_outstation_transport_layer(
    link_modes: LinkModes,
    parse_options: ParseOptions,
    address: EndpointAddress,
    self_address: Feature,
    rx_buffer_size: BufferSize,
) -> (TransportReader, TransportWriter) {
    (
        TransportReader::outstation(
            link_modes,
            parse_options,
            address,
            self_address,
            rx_buffer_size.value(),
        ),
        TransportWriter::new(EndpointType::Outstation, address),
    )
}
