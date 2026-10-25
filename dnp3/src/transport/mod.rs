// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

pub(crate) use reader::*;
pub(crate) use types::*;
pub(crate) use writer::*;

use std::future::Future;

use crate::decode::DecodeLevel;
use crate::link::error::LinkError;
use crate::link::reader::LinkModes;
use crate::link::{EndpointAddress, LinkRole};
use crate::phys::PhysLayer;

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

    fn new(
        link_modes: LinkModes,
        role: LinkRole,
        address: EndpointAddress,
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

pub(crate) fn create_transport_layer<T: Transport>(
    link_modes: LinkModes,
    role: LinkRole,
    address: EndpointAddress,
    rx_buffer_size: usize,
) -> (TransportReader<T>, TransportWriter<T>) {
    let (reader, writer) = T::new(link_modes, role, address, rx_buffer_size);
    (TransportReader::new(reader), TransportWriter::new(writer))
}
