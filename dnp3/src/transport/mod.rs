use crate::app::EndpointType;
use crate::link::EndpointAddress;
use crate::master::session::MasterSession;

#[cfg(test)]
pub(crate) mod mock;
#[cfg(not(test))]
pub(crate) mod real;

mod reader;
mod types;
mod writer;

use crate::link::LinkErrorMode;
use crate::outstation::config::Feature;

pub(crate) use reader::*;
pub(crate) use types::*;
pub(crate) use writer::*;

pub(crate) fn create_master_transport_layer(
    link_error_mode: LinkErrorMode,
    address: EndpointAddress,
    rx_buffer_size: usize,
) -> (TransportReader, TransportWriter) {
    let rx_buffer_size = if rx_buffer_size < MasterSession::MIN_RX_BUFFER_SIZE {
        tracing::warn!("Minimum RX buffer size is {}. Defaulting to this value because the provided value ({}) is too low.", MasterSession::MIN_RX_BUFFER_SIZE, rx_buffer_size);
        MasterSession::MIN_RX_BUFFER_SIZE
    } else {
        rx_buffer_size
    };

    (
        TransportReader::master(link_error_mode, address, rx_buffer_size),
        TransportWriter::new(EndpointType::Master, address),
    )
}

pub(crate) fn create_outstation_transport_layer(
    link_error_mode: LinkErrorMode,
    address: EndpointAddress,
    self_address: Feature,
    rx_buffer_size: crate::outstation::config::BufferSize,
) -> (TransportReader, TransportWriter) {
    (
        TransportReader::outstation(
            link_error_mode,
            address,
            self_address,
            rx_buffer_size.value(),
        ),
        TransportWriter::new(EndpointType::Outstation, address),
    )
}
