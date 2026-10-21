pub(crate) use reader::*;
pub(crate) use types::*;
pub(crate) use writer::*;

use std::future::Future;

use crate::app::parse::options::ParseOptions;
use crate::app::BufferSize;
use crate::decode::DecodeLevel;
use crate::link::error::LinkError;
use crate::link::reader::LinkModes;
use crate::link::EndpointAddress;
use crate::outstation::Feature;
use crate::util::phys::PhysLayer;

#[cfg(test)]
pub(crate) mod mock;
pub(crate) mod real;

pub(crate) use real::RealTransport;

mod reader;
mod types;
mod writer;

/// A transport layer: the matching reader and writer that move fragments over a physical layer
pub(crate) trait Transport: Send + 'static {
    type Reader: TransportRead;
    type Writer: TransportWrite;

    fn master(
        link_modes: LinkModes,
        address: EndpointAddress,
        rx_buffer_size: usize,
    ) -> (Self::Reader, Self::Writer);

    fn outstation(
        link_modes: LinkModes,
        address: EndpointAddress,
        self_address: Feature,
        rx_buffer_size: usize,
    ) -> (Self::Reader, Self::Writer);
}

/// Reads fragments and link-layer messages
pub(crate) trait TransportRead: Send {
    /// Read until a fragment or link-layer message is available, returning at once if one already
    /// is. Callers race this against timers and drop it, so cancelling it must not lose data.
    fn read(
        &mut self,
        io: &mut PhysLayer,
        level: DecodeLevel,
    ) -> impl Future<Output = Result<(), LinkError>> + Send;

    /// The available fragment or link-layer message, without consuming it
    fn peek(&self) -> Option<TransportData<'_>>;

    /// Consume the available fragment or link-layer message
    fn pop(&mut self) -> Option<TransportData<'_>>;

    /// Discard everything received, between communication sessions
    fn reset(&mut self);
}

/// Writes fragments and link status requests
pub(crate) trait TransportWrite: Send {
    fn write(
        &mut self,
        io: &mut PhysLayer,
        level: DecodeLevel,
        destination: FragmentAddr,
        fragment: &[u8],
    ) -> impl Future<Output = Result<(), LinkError>> + Send;

    fn write_link_status_request(
        &mut self,
        io: &mut PhysLayer,
        destination: FragmentAddr,
        level: DecodeLevel,
    ) -> impl Future<Output = Result<(), LinkError>> + Send;

    fn reset(&mut self);
}

pub(crate) fn create_master_transport_layer<T: Transport>(
    link_modes: LinkModes,
    parse_options: ParseOptions,
    address: EndpointAddress,
    rx_buffer_size: BufferSize<2048, 2048>,
) -> (TransportReader<T>, TransportWriter<T>) {
    let (reader, writer) = T::master(link_modes, address, rx_buffer_size.value());
    (
        TransportReader::new(parse_options, reader),
        TransportWriter::new(writer),
    )
}

pub(crate) fn create_outstation_transport_layer<T: Transport>(
    link_modes: LinkModes,
    parse_options: ParseOptions,
    address: EndpointAddress,
    self_address: Feature,
    rx_buffer_size: BufferSize,
) -> (TransportReader<T>, TransportWriter<T>) {
    let (reader, writer) = T::outstation(link_modes, address, self_address, rx_buffer_size.value());
    (
        TransportReader::new(parse_options, reader),
        TransportWriter::new(writer),
    )
}
