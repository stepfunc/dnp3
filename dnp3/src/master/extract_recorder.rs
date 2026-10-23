// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! A `ReadHandler` that records each callback of master extraction, with the header information
//! and the values it was passed, and helpers that extract response objects into one

use crate::app::attr::{
    AnyAttribute, AttrItem, AttrSet, AttrValue, KnownAttribute, OwnedAttribute, VariationListAttr,
};
use crate::app::measurement::*;
use crate::app::parse::options::ParseOptions;
use crate::app::parse::parser::HeaderCollection;
use crate::app::{FunctionCode, Timestamp};
use crate::master::extract::{
    extract_header, extract_measurements_inner, Delivered, Extracted, InvalidResponse,
    ProcessableResponse,
};
use crate::master::{HeaderInfo, ReadHandler};

/// The callbacks of a `ReadHandler` that carry values
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    BinaryInput,
    DoubleBitBinaryInput,
    BinaryOutputStatus,
    Counter,
    FrozenCounter,
    AnalogInput,
    FrozenAnalogInput,
    AnalogInputDeadBand,
    AnalogOutputStatus,
    BinaryOutputCommandEvent,
    AnalogOutputCommandEvent,
    UnsignedInteger,
    OctetString,
    DeviceAttribute,
    AbsTime,
}

/// A value passed to the handler, with its index where it has one
///
/// Floating-point values are equal if their bits are, or if both are NaN
#[derive(Clone, Debug)]
pub(crate) enum Value {
    Binary(BinaryInput, u16),
    DoubleBit(DoubleBitBinaryInput, u16),
    BinaryOutputStatus(BinaryOutputStatus, u16),
    Counter(Counter, u16),
    FrozenCounter(FrozenCounter, u16),
    Analog(AnalogInput, u16),
    FrozenAnalog(FrozenAnalogInput, u16),
    AnalogDeadBand(AnalogInputDeadBand, u16),
    AnalogOutputStatus(AnalogOutputStatus, u16),
    BinaryCommandEvent(BinaryOutputCommandEvent, u16),
    AnalogCommandEvent(AnalogOutputCommandEvent, u16),
    UnsignedInteger(UnsignedInteger, u16),
    OctetString(Vec<u8>, u16),
    /// an attribute of a private set, or of the default set with a variation it doesn't define
    Attribute(OwnedAttribute),
    /// a list of attribute variations of a private set: the set and the variation
    AttributeList(AttrSet, u8, Vec<AttrItem>),
    /// an attribute the default set defines
    KnownAttribute(OwnedAttribute),
    /// a list of attribute variations the default set defines
    KnownAttributeList(VariationListAttr, Vec<AttrItem>),
    AbsTime(Timestamp),
}

fn same_float(lhs: f64, rhs: f64) -> bool {
    lhs.to_bits() == rhs.to_bits() || (lhs.is_nan() && rhs.is_nan())
}

fn same_dead_band(lhs: AnalogInputDeadBand, rhs: AnalogInputDeadBand) -> bool {
    match (lhs, rhs) {
        (AnalogInputDeadBand::U16(x), AnalogInputDeadBand::U16(y)) => x == y,
        (AnalogInputDeadBand::U32(x), AnalogInputDeadBand::U32(y)) => x == y,
        (AnalogInputDeadBand::F32(x), AnalogInputDeadBand::F32(y)) => {
            same_float(x.into(), y.into())
        }
        _ => false,
    }
}

fn same_command_value(lhs: AnalogCommandValue, rhs: AnalogCommandValue) -> bool {
    match (lhs, rhs) {
        (AnalogCommandValue::I16(x), AnalogCommandValue::I16(y)) => x == y,
        (AnalogCommandValue::I32(x), AnalogCommandValue::I32(y)) => x == y,
        (AnalogCommandValue::F32(x), AnalogCommandValue::F32(y)) => same_float(x.into(), y.into()),
        (AnalogCommandValue::F64(x), AnalogCommandValue::F64(y)) => same_float(x, y),
        _ => false,
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Binary(x, i), Value::Binary(y, j)) => (x, i) == (y, j),
            (Value::DoubleBit(x, i), Value::DoubleBit(y, j)) => (x, i) == (y, j),
            (Value::BinaryOutputStatus(x, i), Value::BinaryOutputStatus(y, j)) => (x, i) == (y, j),
            (Value::Counter(x, i), Value::Counter(y, j)) => (x, i) == (y, j),
            (Value::FrozenCounter(x, i), Value::FrozenCounter(y, j)) => (x, i) == (y, j),
            (Value::Analog(x, i), Value::Analog(y, j)) => {
                same_float(x.value, y.value) && (x.flags, x.time, i) == (y.flags, y.time, j)
            }
            (Value::FrozenAnalog(x, i), Value::FrozenAnalog(y, j)) => {
                same_float(x.value, y.value) && (x.flags, x.time, i) == (y.flags, y.time, j)
            }
            (Value::AnalogDeadBand(x, i), Value::AnalogDeadBand(y, j)) => {
                same_dead_band(*x, *y) && i == j
            }
            (Value::AnalogOutputStatus(x, i), Value::AnalogOutputStatus(y, j)) => {
                same_float(x.value, y.value) && (x.flags, x.time, i) == (y.flags, y.time, j)
            }
            (Value::BinaryCommandEvent(x, i), Value::BinaryCommandEvent(y, j)) => (x, i) == (y, j),
            (Value::AnalogCommandEvent(x, i), Value::AnalogCommandEvent(y, j)) => {
                same_command_value(x.commanded_value, y.commanded_value)
                    && (x.status, x.time, i) == (y.status, y.time, j)
            }
            (Value::UnsignedInteger(x, i), Value::UnsignedInteger(y, j)) => (x, i) == (y, j),
            (Value::OctetString(x, i), Value::OctetString(y, j)) => (x, i) == (y, j),
            (Value::Attribute(x), Value::Attribute(y)) => x == y,
            (Value::AttributeList(x, i, a), Value::AttributeList(y, j, b)) => {
                (x, i, a) == (y, j, b)
            }
            (Value::KnownAttribute(x), Value::KnownAttribute(y)) => x == y,
            (Value::KnownAttributeList(x, a), Value::KnownAttributeList(y, b)) => (x, a) == (y, b),
            (Value::AbsTime(x), Value::AbsTime(y)) => x == y,
            _ => false,
        }
    }
}

/// One callback: which one, the information of its header, and the values it was passed
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Call {
    pub(crate) kind: Kind,
    pub(crate) info: HeaderInfo,
    pub(crate) values: Vec<Value>,
}

/// Each callback, in the order the handler was passed them
#[derive(Default)]
pub(crate) struct Recorder {
    pub(crate) calls: Vec<Call>,
}

impl Recorder {
    /// Each value, in the order the handler was passed them, with the information of its header
    pub(crate) fn values(self) -> impl Iterator<Item = (HeaderInfo, Value)> {
        self.calls
            .into_iter()
            .flat_map(|call| call.values.into_iter().map(move |value| (call.info, value)))
    }

    fn push<T>(
        &mut self,
        kind: Kind,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (T, u16)>,
        value: impl Fn(T, u16) -> Value,
    ) {
        let values = iter.map(|(x, index)| value(x, index)).collect();
        self.calls.push(Call { kind, info, values });
    }

    fn push_one(&mut self, kind: Kind, info: HeaderInfo, value: Value) {
        let values = vec![value];
        self.calls.push(Call { kind, info, values });
    }
}

fn known_attribute(attr: KnownAttribute) -> Value {
    match attr {
        KnownAttribute::AttributeList(x, list) => {
            Value::KnownAttributeList(x, list.iter().collect())
        }
        KnownAttribute::String(x, v) => Value::KnownAttribute(x.with_value(v)),
        KnownAttribute::Float(x, v) => Value::KnownAttribute(x.with_value(v)),
        KnownAttribute::UInt(x, v) => Value::KnownAttribute(x.with_value(v)),
        KnownAttribute::Bool(x, v) => Value::KnownAttribute(x.with_value(v)),
        KnownAttribute::OctetString(x, v) => Value::KnownAttribute(x.with_value(v.to_vec())),
        KnownAttribute::DNP3Time(x, v) => Value::KnownAttribute(x.with_value(v)),
    }
}

impl ReadHandler for Recorder {
    fn handle_binary_input(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (BinaryInput, u16)>,
    ) {
        self.push(Kind::BinaryInput, info, iter, Value::Binary)
    }

    fn handle_double_bit_binary_input(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (DoubleBitBinaryInput, u16)>,
    ) {
        self.push(Kind::DoubleBitBinaryInput, info, iter, Value::DoubleBit)
    }

    fn handle_binary_output_status(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (BinaryOutputStatus, u16)>,
    ) {
        self.push(
            Kind::BinaryOutputStatus,
            info,
            iter,
            Value::BinaryOutputStatus,
        )
    }

    fn handle_counter(&mut self, info: HeaderInfo, iter: &mut dyn Iterator<Item = (Counter, u16)>) {
        self.push(Kind::Counter, info, iter, Value::Counter)
    }

    fn handle_frozen_counter(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (FrozenCounter, u16)>,
    ) {
        self.push(Kind::FrozenCounter, info, iter, Value::FrozenCounter)
    }

    fn handle_analog_input(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (AnalogInput, u16)>,
    ) {
        self.push(Kind::AnalogInput, info, iter, Value::Analog)
    }

    fn handle_frozen_analog_input(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (FrozenAnalogInput, u16)>,
    ) {
        self.push(Kind::FrozenAnalogInput, info, iter, Value::FrozenAnalog)
    }

    fn handle_analog_input_dead_band(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (AnalogInputDeadBand, u16)>,
    ) {
        self.push(Kind::AnalogInputDeadBand, info, iter, Value::AnalogDeadBand)
    }

    fn handle_analog_output_status(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (AnalogOutputStatus, u16)>,
    ) {
        self.push(
            Kind::AnalogOutputStatus,
            info,
            iter,
            Value::AnalogOutputStatus,
        )
    }

    fn handle_binary_output_command_event(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (BinaryOutputCommandEvent, u16)>,
    ) {
        self.push(
            Kind::BinaryOutputCommandEvent,
            info,
            iter,
            Value::BinaryCommandEvent,
        )
    }

    fn handle_analog_output_command_event(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (AnalogOutputCommandEvent, u16)>,
    ) {
        self.push(
            Kind::AnalogOutputCommandEvent,
            info,
            iter,
            Value::AnalogCommandEvent,
        )
    }

    fn handle_unsigned_integer(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (UnsignedInteger, u16)>,
    ) {
        self.push(Kind::UnsignedInteger, info, iter, Value::UnsignedInteger)
    }

    fn handle_octet_string<'a>(
        &mut self,
        info: HeaderInfo,
        iter: &'a mut dyn Iterator<Item = (&'a [u8], u16)>,
    ) {
        self.push(Kind::OctetString, info, iter, |bytes, index| {
            Value::OctetString(bytes.to_vec(), index)
        })
    }

    fn handle_device_attribute(&mut self, info: HeaderInfo, attr: AnyAttribute) {
        let value = match attr {
            AnyAttribute::Known(attr) => known_attribute(attr),
            AnyAttribute::Other(attr) => match attr.value {
                AttrValue::AttrList(list) => {
                    Value::AttributeList(attr.set, attr.variation, list.iter().collect())
                }
                _ => Value::Attribute(attr.to_owned().unwrap()),
            },
        };
        self.push_one(Kind::DeviceAttribute, info, value);
    }

    fn handle_abs_time(&mut self, info: HeaderInfo, time: Timestamp) {
        self.push_one(Kind::AbsTime, info, Value::AbsTime(time));
    }
}

/// Check response objects as the master does, and extract them if the check passes
pub(crate) fn extract(objects: &[u8]) -> (Result<Delivered, InvalidResponse>, Recorder) {
    extract_with_options(ParseOptions::default(), objects)
}

/// [`extract`] with the given parse options
pub(crate) fn extract_with_options(
    options: ParseOptions,
    objects: &[u8],
) -> (Result<Delivered, InvalidResponse>, Recorder) {
    let mut recorder = Recorder::default();
    let result = ProcessableResponse::new(HeaderCollection::parse(
        options,
        FunctionCode::Response,
        objects,
    ))
    .map(|objects| extract_measurements_inner(objects, &mut recorder));
    (result, recorder)
}

/// Extract each header of response objects that parse in turn, without the master's check, and
/// what that delivered
pub(crate) fn extract_each_header(objects: &[u8]) -> (Delivered, Recorder) {
    let headers =
        HeaderCollection::parse(ParseOptions::default(), FunctionCode::Response, objects).unwrap();
    let mut recorder = Recorder::default();
    let mut delivered = Delivered::default();
    let mut cto = None;
    for header in headers.iter() {
        match extract_header(&header, cto, &mut recorder) {
            Extracted::Handled => delivered.add(&header),
            Extracted::Cto(x) => cto = x,
            Extracted::Ignored => {}
        }
    }
    (delivered, recorder)
}
