// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Frames, a mock-I/O harness and delivered items shared by the transport reader and writer tests

use std::time::Duration;

use crate::decode::DecodeLevel;
use crate::link::header::{AnyAddress, ControlField, Header};
use crate::link::reader::LinkModes;
use crate::link::test_data;
use crate::link::{EndpointAddress, LinkRole};
use crate::phys::{PhysAddr, PhysLayer};
use crate::transport::real::reader::Reader;
use crate::transport::real::writer::Writer;
use crate::transport::real::RealTransport;
use crate::transport::{
    FragmentAddr, LinkLayerMessageType, Transport, TransportData, TransportRead, TransportWrite,
};

pub(super) const MASTER: EndpointAddress = EndpointAddress::raw(1);
pub(super) const OUTSTATION: EndpointAddress = EndpointAddress::raw(1024);

// link control fields without the DIR bit, which `link_header` adds: the function code with the
// PRM bit, and the FCB and FCV bits where named
pub(super) const UNCONFIRMED_USER_DATA: u8 = 0x44;
pub(super) const CONFIRMED_USER_DATA: u8 = 0x53;
pub(super) const CONFIRMED_USER_DATA_FCB: u8 = 0x73;
pub(super) const RESET_LINK_STATES: u8 = 0x40;
pub(super) const REQUEST_LINK_STATUS: u8 = 0x49;
pub(super) const ACK: u8 = 0x00;
pub(super) const LINK_STATUS: u8 = 0x0B;

pub(super) fn master() -> (Reader, Writer) {
    RealTransport::new(LinkModes::test(), LinkRole::Master, MASTER, 2048)
}

pub(super) fn outstation(self_address: bool) -> (Reader, Writer) {
    RealTransport::new(
        LinkModes::test(),
        LinkRole::Outstation { self_address },
        OUTSTATION,
        2048,
    )
}

pub(super) fn link_header(
    from_master: bool,
    control: u8,
    destination: AnyAddress,
    source: EndpointAddress,
) -> Header {
    Header::new(
        ControlField::from(if from_master { 0x80 } else { 0 } | control),
        destination,
        source.wrap(),
    )
}

pub(super) fn header_only(
    from_master: bool,
    control: u8,
    destination: AnyAddress,
    source: EndpointAddress,
) -> Vec<u8> {
    test_data::header_only(link_header(from_master, control, destination, source))
}

/// a data frame from the master to the outstation carrying one segment of a fragment
pub(super) fn segment(fir: bool, fin: bool, seq: u8, data: &[u8]) -> Vec<u8> {
    let header = link_header(true, UNCONFIRMED_USER_DATA, OUTSTATION.wrap(), MASTER);
    test_data::data_frame(header, test_data::segment(fir, fin, seq), data)
}

/// a link status request from the master to the outstation
pub(super) fn link_status_request() -> Vec<u8> {
    header_only(true, REQUEST_LINK_STATUS, OUTSTATION.wrap(), MASTER)
}

/// the bytes written to the mock since the last call
pub(super) fn drain_writes(handle: &mut sfio_tokio_mock_io::Handle) -> Vec<Vec<u8>> {
    let mut writes = Vec::new();
    while let Some(event) = handle.pop_event() {
        if let sfio_tokio_mock_io::Event::Write(x) = event {
            writes.push(x);
        }
    }
    writes
}

/// true if `read` completed instead of waiting for more data
pub(super) async fn read(reader: &mut Reader, io: &mut PhysLayer) -> bool {
    tokio::time::timeout(
        Duration::from_secs(1),
        reader.read(io, DecodeLevel::nothing()),
    )
    .await
    .map(|res| res.unwrap())
    .is_ok()
}

/// what the reader delivered, with the source of a link-layer message
#[derive(Debug, PartialEq)]
pub(super) enum Item {
    Fragment(Vec<u8>),
    LinkStatusRequest(EndpointAddress),
    LinkStatusResponse(EndpointAddress),
}

impl From<TransportData<'_>> for Item {
    fn from(data: TransportData<'_>) -> Self {
        match data {
            TransportData::Fragment(x) => Item::Fragment(x.data.to_vec()),
            TransportData::LinkLayerMessage(x) => match x.message {
                LinkLayerMessageType::LinkStatusRequest => Item::LinkStatusRequest(x.source),
                LinkLayerMessageType::LinkStatusResponse => Item::LinkStatusResponse(x.source),
            },
        }
    }
}

/// a transport reader and writer on mock I/O
pub(super) struct Harness {
    pub(super) reader: Reader,
    writer: Writer,
    io: PhysLayer,
    handle: sfio_tokio_mock_io::Handle,
}

impl Harness {
    pub(super) fn new((reader, writer): (Reader, Writer)) -> Self {
        let (io, handle) = sfio_tokio_mock_io::mock();
        Self {
            reader,
            writer,
            io: PhysLayer::mock(io),
            handle,
        }
    }

    pub(super) fn queue(&mut self, frames: &[Vec<u8>]) {
        self.handle.read(&frames.concat());
    }

    /// true if `read` completed instead of waiting for more data
    pub(super) async fn read(&mut self) -> bool {
        read(&mut self.reader, &mut self.io).await
    }

    pub(super) fn pop(&mut self) -> Option<Item> {
        self.reader.pop().map(Item::from)
    }

    pub(super) fn peek(&self) -> Option<Item> {
        self.reader.peek().map(Item::from)
    }

    /// queue the frames, then read until something is available
    pub(super) async fn deliver(&mut self, frames: &[Vec<u8>]) -> Option<Item> {
        self.queue(frames);
        if self.read().await {
            self.pop()
        } else {
            None
        }
    }

    pub(super) async fn write(&mut self, destination: EndpointAddress, fragment: &[u8]) {
        let destination = FragmentAddr {
            link: destination,
            phys: PhysAddr::None,
        };
        self.writer
            .write(&mut self.io, DecodeLevel::nothing(), destination, fragment)
            .await
            .unwrap();
    }

    pub(super) async fn write_link_status_request(&mut self, destination: EndpointAddress) {
        let destination = FragmentAddr {
            link: destination,
            phys: PhysAddr::None,
        };
        self.writer
            .write_link_status_request(&mut self.io, destination, DecodeLevel::nothing())
            .await
            .unwrap();
    }

    /// the bytes written since the last call
    pub(super) fn writes(&mut self) -> Vec<Vec<u8>> {
        drain_writes(&mut self.handle)
    }
}
