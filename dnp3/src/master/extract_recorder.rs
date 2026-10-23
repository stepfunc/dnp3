// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! A `ReadHandler` that records each value an outstation database can produce, with the header
//! information it came with, so that tests can compare what master extraction reports
//!
//! Attributes of the default set (0) aren't a supported input: the handler panics on them.

use crate::app::attr::{AnyAttribute, AttrItem, AttrSet, AttrValue, OwnedAttribute};
use crate::app::measurement::*;
use crate::master::{HeaderInfo, ReadHandler};

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
    AnalogDeadBand(AnalogInputDeadBand, u16),
    AnalogOutputStatus(AnalogOutputStatus, u16),
    OctetString(Vec<u8>, u16),
    Attribute(OwnedAttribute),
    AttributeList(AttrSet, Vec<AttrItem>),
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
            (Value::AnalogDeadBand(x, i), Value::AnalogDeadBand(y, j)) => {
                same_dead_band(*x, *y) && i == j
            }
            (Value::AnalogOutputStatus(x, i), Value::AnalogOutputStatus(y, j)) => {
                same_float(x.value, y.value) && (x.flags, x.time, i) == (y.flags, y.time, j)
            }
            (Value::OctetString(x, i), Value::OctetString(y, j)) => (x, i) == (y, j),
            (Value::Attribute(x), Value::Attribute(y)) => x == y,
            (Value::AttributeList(x, i), Value::AttributeList(y, j)) => (x, i) == (y, j),
            _ => false,
        }
    }
}

/// Each value, in the order the handler was passed them, with the information of its header
#[derive(Default)]
pub(crate) struct Recorder {
    pub(crate) values: Vec<(HeaderInfo, Value)>,
}

impl Recorder {
    fn push<T>(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (T, u16)>,
        value: fn(T, u16) -> Value,
    ) {
        self.values
            .extend(iter.map(|(x, index)| (info, value(x, index))));
    }
}

impl ReadHandler for Recorder {
    fn handle_binary_input(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (BinaryInput, u16)>,
    ) {
        self.push(info, iter, Value::Binary)
    }

    fn handle_double_bit_binary_input(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (DoubleBitBinaryInput, u16)>,
    ) {
        self.push(info, iter, Value::DoubleBit)
    }

    fn handle_binary_output_status(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (BinaryOutputStatus, u16)>,
    ) {
        self.push(info, iter, Value::BinaryOutputStatus)
    }

    fn handle_counter(&mut self, info: HeaderInfo, iter: &mut dyn Iterator<Item = (Counter, u16)>) {
        self.push(info, iter, Value::Counter)
    }

    fn handle_frozen_counter(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (FrozenCounter, u16)>,
    ) {
        self.push(info, iter, Value::FrozenCounter)
    }

    fn handle_analog_input(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (AnalogInput, u16)>,
    ) {
        self.push(info, iter, Value::Analog)
    }

    fn handle_analog_input_dead_band(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (AnalogInputDeadBand, u16)>,
    ) {
        self.push(info, iter, Value::AnalogDeadBand)
    }

    fn handle_analog_output_status(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (AnalogOutputStatus, u16)>,
    ) {
        self.push(info, iter, Value::AnalogOutputStatus)
    }

    fn handle_octet_string<'a>(
        &mut self,
        info: HeaderInfo,
        iter: &'a mut dyn Iterator<Item = (&'a [u8], u16)>,
    ) {
        self.values
            .extend(iter.map(|(bytes, index)| (info, Value::OctetString(bytes.to_vec(), index))));
    }

    fn handle_device_attribute(&mut self, info: HeaderInfo, attr: AnyAttribute) {
        let AnyAttribute::Other(attr) = attr else {
            panic!("the default set is not used by these tests");
        };
        let value = match attr.value {
            AttrValue::AttrList(list) => Value::AttributeList(attr.set, list.iter().collect()),
            _ => Value::Attribute(attr.to_owned().unwrap()),
        };
        self.values.push((info, value));
    }
}
