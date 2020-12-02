use crate::app::EndpointType;
use crate::entry::EndpointAddress;
use crate::link::header::BroadcastConfirmMode;
use crate::master::session::MasterSession;
use crate::outstation::task::OutstationSession;
use crate::outstation::SelfAddressSupport;
use crate::transport::reader::TransportReader;

#[cfg(test)]
pub(crate) mod mock;
#[cfg(not(test))]
pub(crate) mod real;

pub(crate) mod reader;

#[cfg(not(test))]
/// This type definition is used so that we can mock the transport writer during testing.
/// If Rust eventually allows `async fn` in traits, this can be removed
pub(crate) type TransportWriter = real::writer::Writer;
#[cfg(test)]
pub(crate) type TransportWriter = crate::transport::mock::writer::MockWriter;

#[derive(Debug, Copy, Clone)]
pub(crate) struct FragmentInfo {
    pub(crate) id: u32,
    pub(crate) source: EndpointAddress,
    pub(crate) broadcast: Option<BroadcastConfirmMode>,
}

impl FragmentInfo {
    pub(crate) fn new(
        id: u32,
        source: EndpointAddress,
        broadcast: Option<BroadcastConfirmMode>,
    ) -> Self {
        FragmentInfo {
            id,
            source,
            broadcast,
        }
    }
}

#[derive(Debug)]
pub(crate) struct Fragment<'a> {
    pub(crate) info: FragmentInfo,
    pub(crate) data: &'a [u8],
}

pub(crate) fn create_master_transport_layer(
    address: EndpointAddress,
    rx_buffer_size: usize,
) -> (TransportReader, TransportWriter) {
    let rx_buffer_size = if rx_buffer_size < MasterSession::MIN_RX_BUFFER_SIZE {
        log::warn!("Minimum RX buffer size is {}. Defaulting to this value because the provided value ({}) is too low.", MasterSession::MIN_RX_BUFFER_SIZE, rx_buffer_size);
        MasterSession::MIN_RX_BUFFER_SIZE
    } else {
        rx_buffer_size
    };

    (
        TransportReader::master(address, rx_buffer_size),
        TransportWriter::new(EndpointType::Master, address),
    )
}

pub(crate) fn create_outstation_transport_layer(
    address: EndpointAddress,
    self_address_support: SelfAddressSupport,
    rx_buffer_size: usize,
) -> (TransportReader, TransportWriter) {
    let rx_buffer_size = if rx_buffer_size < OutstationSession::MIN_RX_BUFFER_SIZE {
        log::warn!("Minimum RX buffer size is {}. Defaulting to this value because the provided value ({}) is too low.", OutstationSession::MIN_RX_BUFFER_SIZE, rx_buffer_size);
        OutstationSession::MIN_RX_BUFFER_SIZE
    } else {
        rx_buffer_size
    };

    (
        TransportReader::outstation(address, self_address_support, rx_buffer_size),
        TransportWriter::new(EndpointType::Outstation, address),
    )
}
