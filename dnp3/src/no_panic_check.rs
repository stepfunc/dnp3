//! Links the parsers into the test binary with input the compiler can't know, so that the
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
use crate::link::parser::{FramePayload, Parser};
use crate::link::LinkErrorMode;
use crate::master::extract::{extract_header, Extracted};
use crate::master::{HeaderInfo, ReadHandler};

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

#[test]
fn parsers_with_unknown_input() {
    let data = std::hint::black_box(input());

    let mut parser = Parser::new(LinkErrorMode::Discard);
    let mut payload = FramePayload::new();
    let mut cursor = ReadCursor::new(&data);
    std::hint::black_box(parser.parse(&mut cursor, &mut payload).is_ok());

    // both settings of the option, chosen by the input
    let options = ParseOptions {
        parse_zero_length_strings: data.first().is_some_and(|x| x & 0x01 != 0),
    };
    if let Ok(fragment) = ParsedFragment::parse_no_logging(options, &data) {
        std::hint::black_box(fragment.to_request().is_ok());
        std::hint::black_box(fragment.to_response().is_ok());
        if let Ok(objects) = fragment.objects {
            extract(objects);
        }
    }
}
