//! Links the parsers and the transport assembler into the test binary with input the compiler can't know, so that the
//! functions marked `no_panic` here and in the parsers are checked for every input. The link
//! fails if the optimizer can't remove every path to a panic from one of them.
//!
//! Built by the `no-panic` CI job with `--cfg dnp3_no_panic`, in release with LTO and one
//! codegen unit, which the optimizer needs to see through calls into other crates

use scursor::ReadCursor;

use crate::app::attr::{AnyAttribute, AttrValue, Attribute, KnownAttribute};
use crate::app::measurement::*;
use crate::app::parse::options::ParseOptions;
use crate::app::parse::parser::{HeaderCollection, ParsedFragment};
use crate::link::header::{
    AnyAddress, BroadcastConfirmMode, ControlField, FrameInfo, FrameType, Header as LinkHeader,
};
use crate::link::layer::Layer;
use crate::link::parser::{FramePayload, Parser};
use crate::link::reader::{LinkModes, Reader, Step};
use crate::link::{EndpointAddress, LinkErrorMode, LinkRole};
use crate::master::extract::{extract_header, validate_response, Extracted};
use crate::master::{HeaderInfo, ReadHandler};
use crate::outstation::Feature;
use crate::transport::real::assembler::{Assembler, AssemblyState};
use crate::transport::real::header::Header;
use crate::util::phys::PhysAddr;

fn input() -> Vec<u8> {
    match std::env::var_os("DNP3_NO_PANIC_INPUT") {
        Some(path) => std::fs::read(path).unwrap_or_default(),
        None => Vec::new(),
    }
}

/// a handler that reads every value it's given
struct Drain;

fn drain<T>(iter: &mut dyn Iterator<Item = T>) {
    for x in iter {
        std::hint::black_box(x);
    }
}

impl ReadHandler for Drain {
    fn handle_binary_input(
        &mut self,
        _: HeaderInfo,
        iter: &mut dyn Iterator<Item = (BinaryInput, u16)>,
    ) {
        drain(iter)
    }

    fn handle_double_bit_binary_input(
        &mut self,
        _: HeaderInfo,
        iter: &mut dyn Iterator<Item = (DoubleBitBinaryInput, u16)>,
    ) {
        drain(iter)
    }

    fn handle_binary_output_status(
        &mut self,
        _: HeaderInfo,
        iter: &mut dyn Iterator<Item = (BinaryOutputStatus, u16)>,
    ) {
        drain(iter)
    }

    fn handle_counter(&mut self, _: HeaderInfo, iter: &mut dyn Iterator<Item = (Counter, u16)>) {
        drain(iter)
    }

    fn handle_frozen_counter(
        &mut self,
        _: HeaderInfo,
        iter: &mut dyn Iterator<Item = (FrozenCounter, u16)>,
    ) {
        drain(iter)
    }

    fn handle_analog_input(
        &mut self,
        _: HeaderInfo,
        iter: &mut dyn Iterator<Item = (AnalogInput, u16)>,
    ) {
        drain(iter)
    }

    fn handle_frozen_analog_input(
        &mut self,
        _: HeaderInfo,
        iter: &mut dyn Iterator<Item = (FrozenAnalogInput, u16)>,
    ) {
        drain(iter)
    }

    fn handle_analog_input_dead_band(
        &mut self,
        _: HeaderInfo,
        iter: &mut dyn Iterator<Item = (AnalogInputDeadBand, u16)>,
    ) {
        drain(iter)
    }

    fn handle_analog_output_status(
        &mut self,
        _: HeaderInfo,
        iter: &mut dyn Iterator<Item = (AnalogOutputStatus, u16)>,
    ) {
        drain(iter)
    }

    fn handle_analog_output_command_event(
        &mut self,
        _: HeaderInfo,
        iter: &mut dyn Iterator<Item = (AnalogOutputCommandEvent, u16)>,
    ) {
        drain(iter)
    }

    fn handle_binary_output_command_event(
        &mut self,
        _: HeaderInfo,
        iter: &mut dyn Iterator<Item = (BinaryOutputCommandEvent, u16)>,
    ) {
        drain(iter)
    }

    fn handle_unsigned_integer(
        &mut self,
        _: HeaderInfo,
        iter: &mut dyn Iterator<Item = (UnsignedInteger, u16)>,
    ) {
        drain(iter)
    }

    fn handle_octet_string<'a>(
        &mut self,
        _: HeaderInfo,
        iter: &'a mut dyn Iterator<Item = (&'a [u8], u16)>,
    ) {
        drain(iter)
    }

    fn handle_device_attribute(&mut self, _: HeaderInfo, attr: AnyAttribute) {
        match attr {
            AnyAttribute::Known(KnownAttribute::AttributeList(_, list))
            | AnyAttribute::Other(Attribute {
                value: AttrValue::AttrList(list),
                ..
            }) => drain(&mut list.iter()),
            other => {
                std::hint::black_box(other);
            }
        }
    }

    fn handle_abs_time(&mut self, _: HeaderInfo, time: crate::app::Timestamp) {
        std::hint::black_box(time);
    }
}

/// the master's extraction of every header in a response
#[no_panic::no_panic]
fn extract(objects: HeaderCollection) {
    let mut cto = None;
    for header in objects.iter() {
        if let Extracted::Cto(x) = extract_header(&header, cto, &mut Drain) {
            cto = x;
        }
    }
}

/// the master's check that it can process a response
#[no_panic::no_panic]
fn validate(objects: HeaderCollection) -> bool {
    validate_response(Ok(objects)).is_ok()
}

/// one segment given to the transport assembler, returning the length of a completed fragment
#[no_panic::no_panic]
fn assemble(assembler: &mut Assembler, info: FrameInfo, segment: &[u8]) -> usize {
    if let [transport, data @ ..] = segment {
        let header = Header::from_u8(*transport);
        if let AssemblyState::Complete = assembler.assemble(info, header, data, |_| {}) {
            let peeked = assembler.peek().map_or(0, |fragment| fragment.data.len());
            return assembler
                .pop()
                .map_or(peeked, |fragment| fragment.data.len());
        }
    }
    0
}

/// no physical address, or a UDP address, chosen by bits 2 to 7
fn phys_addr(flags: u8) -> PhysAddr {
    if flags & 0x04 == 0 {
        PhysAddr::None
    } else {
        PhysAddr::Udp(std::net::SocketAddr::from((
            [127, 0, 0, 1],
            u16::from(flags >> 3),
        )))
    }
}

/// the link layer's processing of one frame header, and the reply it formats
#[no_panic::no_panic]
fn process_link_header(layer: &mut Layer, header: LinkHeader, addr: PhysAddr) -> usize {
    let (info, reply) = layer.process_header(header, addr, |_| {});
    std::hint::black_box(info);
    match reply {
        Some(reply) => {
            let header = layer.get_header(reply);
            Layer::format_reply(header).len()
        }
        None => 0,
    }
}

/// a link layer configured from the input, given frame headers from the input
fn link_headers(data: &[u8]) {
    let [config, local_0, local_1, headers @ ..] = data else {
        return;
    };
    let mut data = headers;
    let role = if config & 0x01 == 0 {
        LinkRole::Master
    } else if config & 0x02 == 0 {
        LinkRole::Outstation {
            self_address: Feature::Disabled,
        }
    } else {
        LinkRole::Outstation {
            self_address: Feature::Enabled,
        }
    };
    let mut layer = Layer::new(
        LinkModes::stream(LinkErrorMode::Discard),
        2048,
        role,
        EndpointAddress::raw(u16::from_le_bytes([*local_0, *local_1])),
    );
    while let [control, dest_0, dest_1, source_0, source_1, flags, rest @ ..] = data {
        let header = LinkHeader::new(
            ControlField::from(*control),
            AnyAddress::from(u16::from_le_bytes([*dest_0, *dest_1])),
            AnyAddress::from(u16::from_le_bytes([*source_0, *source_1])),
        );
        std::hint::black_box(process_link_header(&mut layer, header, phys_addr(*flags)));
        data = rest;
    }
}

/// one simulated read of `chunk` into the frame reader, returning the bytes it took
#[no_panic::no_panic]
fn link_receive(reader: &mut Reader, chunk: &[u8]) -> usize {
    let Ok(space) = reader.write_space() else {
        return 0;
    };
    let mut count: usize = 0;
    for (dest, src) in space.iter_mut().zip(chunk) {
        *dest = *src;
        count = count.saturating_add(1);
    }
    match reader.wrote(count) {
        Ok(()) => count,
        Err(_) => 0,
    }
}

/// the frame reader's processing of the data it holds, returning the frames it found
#[no_panic::no_panic]
fn link_frames(reader: &mut Reader, payload: &mut FramePayload) -> usize {
    let mut frames: usize = 0;
    while let Ok(Step::Frame(header)) = reader.step(payload) {
        std::hint::black_box((header, payload.get().len()));
        frames = frames.saturating_add(1);
    }
    frames
}

/// a frame reader configured from the input, given reads of input-sized chunks
fn link_reader(data: &[u8]) {
    let [config, size, chunks @ ..] = data else {
        return;
    };
    let error_mode = if config & 0x01 == 0 {
        LinkErrorMode::Discard
    } else {
        LinkErrorMode::Close
    };
    let modes = if config & 0x02 == 0 {
        LinkModes::stream(error_mode)
    } else {
        LinkModes::datagram(error_mode)
    };
    let mut reader = Reader::new(modes, usize::from(*size).saturating_mul(16));
    let mut payload = FramePayload::new();
    let mut data = chunks;
    while let [length, rest @ ..] = data {
        let (chunk, next) = rest.split_at(usize::from(*length).min(rest.len()));
        std::hint::black_box(link_receive(&mut reader, chunk));
        std::hint::black_box(link_frames(&mut reader, &mut payload));
        data = next;
    }
}

/// segments cut from the input, each with a source, broadcast mode and physical address from
/// the input
fn segments(mut data: &[u8]) {
    let mut assembler = Assembler::new(std::hint::black_box(2048));
    while let [length, source, flags, rest @ ..] = data {
        let (segment, next) = rest.split_at(usize::from(*length).min(rest.len()));
        let broadcast = match flags % 4 {
            0 => Some(BroadcastConfirmMode::Optional),
            1 => Some(BroadcastConfirmMode::Mandatory),
            2 => Some(BroadcastConfirmMode::NotRequired),
            _ => None,
        };
        let info = FrameInfo::new(
            EndpointAddress::raw(u16::from(*source)),
            broadcast,
            FrameType::Data,
            phys_addr(*flags),
        );
        std::hint::black_box(assemble(&mut assembler, info, segment));
        data = next;
    }
}

#[test]
fn parsers_with_unknown_input() {
    let data = std::hint::black_box(input());

    segments(&data);
    link_headers(&data);
    link_reader(&data);

    let mut parser = Parser::new(LinkErrorMode::Discard);
    let mut payload = FramePayload::new();
    let mut cursor = ReadCursor::new(&data);
    std::hint::black_box(parser.parse(&mut cursor, &mut payload).is_ok());

    // both settings of the option, chosen by the input
    let options = ParseOptions {
        parse_zero_length_strings: data.first().is_some_and(|x| x & 0x01 != 0),
    };
    if let Ok(fragment) = ParsedFragment::parse(options, &data) {
        std::hint::black_box(fragment.to_request().is_ok());
        std::hint::black_box(fragment.to_response().is_ok());
        if let Ok(objects) = fragment.objects {
            std::hint::black_box(validate(objects));
            extract(objects);
        }
    }
}
