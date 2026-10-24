// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! For each measurement family and each variation a READ can ask for, what master extraction
//! reports from what the outstation database writes equals a reference projection of the
//! database's values, written independently of the production conversions
//!
//! The reference conversions: integers clamp to their range with OVER_RANGE, and NaN is 0 with
//! OVER_RANGE (the model in [`analog_model`], shared with the conversion properties, so a mistake
//! there would hide from both); finite values past the f32 range clamp with OVER_RANGE, and NaN
//! and infinities are kept; 16-bit counters keep the low 16 bits; variations without flags report
//! ONLINE, and a variation without flags is promoted to the one with them when the flags it would
//! report aren't just ONLINE; events without time are reported in the variation without time at
//! the same precision (the 3.1r1 IED certification procedures 8.14.1 require this for binary
//! inputs; for the other families it's our deliberate departure from reporting a requested
//! variation as is, TB2017-003a 5.1.3), and binary and double-bit events with unsynchronized time
//! relative to an unsynchronized CTO (TB2017-003a)
//!
//! The times of the other variations with absolute time pin the current API convention rather
//! than a standards-backed oracle: see [`absolute`].
//!
//! Event times are generated in sequence, so that events share a CTO with offsets from 0 to its
//! reach of 65,535 and just past it, change synchronization, and step backward.
//!
//! Flags are ONLINE, ONLINE with bit 6, bit 7 or both (the bits that carry the state of a
//! double-bit point, of which a binary point uses bit 7), or any byte, so that the promotion of
//! g1v1, g3v1 and g10v1 meets both sides of its rule: the state bits alone keep the variation
//! without flags, and any other bit promotes it.
//!
//! Not generated: READs other than all objects, responses of more than one fragment, and events
//! in classes 2 and 3.

use std::collections::BTreeSet;
use std::fmt::Debug;

use proptest::prelude::*;
use scursor::WriteCursor;

use crate::app::measurement::*;
use crate::app::parse::options::ParseOptions;
use crate::app::parse::parser::HeaderCollection;
use crate::app::{FunctionCode, Iin2, QualifierCode, Timestamp, Variation as V};
use crate::master::extract_recorder::{extract, Value};
use crate::master::HeaderInfo;
use crate::outstation::database::*;
use crate::outstation::OutstationConfig;

/// The most points of a family in a case
const MAX_POINTS: usize = 16;

/// Points of one family at distinct indices, in the order their events are created
#[derive(Clone, Debug)]
enum Points {
    Binary(Vec<(u16, BinaryInput)>),
    DoubleBit(Vec<(u16, DoubleBitBinaryInput)>),
    BinaryOutputStatus(Vec<(u16, BinaryOutputStatus)>),
    Counter(Vec<(u16, Counter)>),
    FrozenCounter(Vec<(u16, FrozenCounter)>),
    Analog(Vec<(u16, AnalogInput)>),
    AnalogOutputStatus(Vec<(u16, AnalogOutputStatus)>),
    OctetString(Vec<(u16, OctetString)>),
    /// analog inputs with these deadbands, read in group 34
    DeadBand(Vec<(u16, f64)>),
}

/// The variation a READ asks for, and whether it's an event variation
type Read = (V, bool);

const OCTET_STRING: &[Read] = &[(V::Group110(0), false), (V::Group111(0), true)];

/// The specific variations in a family's static and event READ lists, which start with variation
/// 0, and whether each is an event variation
fn specific(static_reads: &[V], event_reads: &[V]) -> Vec<Read> {
    fn after_v0(reads: &[V]) -> &[V] {
        reads.split_first().map_or(&[], |(_, rest)| rest)
    }
    let statics = after_v0(static_reads).iter().map(|&v| (v, false));
    let events = after_v0(event_reads).iter().map(|&v| (v, true));
    statics.chain(events).collect()
}

// ---------- the outstation writes, the master extracts ----------

fn load<T, C>(db: &mut Database, points: &[(u16, T)], config: C, event: bool)
where
    Database: Add<C> + Update<T>,
    C: Copy,
{
    let options = if event {
        UpdateOptions::new(true, EventMode::Force)
    } else {
        UpdateOptions::no_event()
    };
    for (index, value) in points {
        assert!(db.add(*index, Some(EventClass::Class1), config));
        assert!(db.update(*index, value, options));
    }
}

impl Points {
    fn load(&self, db: &mut Database, event: bool) {
        match self {
            Points::Binary(x) => load(db, x, BinaryInputConfig::default(), event),
            Points::DoubleBit(x) => load(db, x, DoubleBitBinaryInputConfig::default(), event),
            Points::BinaryOutputStatus(x) => {
                load(db, x, BinaryOutputStatusConfig::default(), event)
            }
            Points::Counter(x) => load(db, x, CounterConfig::default(), event),
            Points::FrozenCounter(x) => load(db, x, FrozenCounterConfig::default(), event),
            Points::Analog(x) => load(db, x, AnalogInputConfig::default(), event),
            Points::AnalogOutputStatus(x) => {
                load(db, x, AnalogOutputStatusConfig::default(), event)
            }
            Points::OctetString(x) => load(db, x, OctetStringConfig, event),
            Points::DeadBand(x) => {
                for (index, deadband) in x {
                    let config = AnalogInputConfig {
                        deadband: *deadband,
                        ..Default::default()
                    };
                    assert!(db.add(*index, None, config));
                }
            }
        }
    }
}

/// READ every object of the variation, write the response in one fragment, then extract it
fn write_and_extract(points: &Points, (variation, event): Read) -> Vec<(HeaderInfo, Value)> {
    let mut handle = DatabaseHandle::new(
        OutstationConfig::DEFAULT_MAX_READ_REQUEST_HEADERS,
        ClassZeroConfig::default(),
        EventBufferConfig::all_types(MAX_POINTS.try_into().unwrap()),
    );
    handle.transaction(|db| points.load(db, event));

    let (group, var) = variation.to_group_and_var();
    let request = [group, var, QualifierCode::AllObjects.as_u8()];
    let headers =
        HeaderCollection::parse(ParseOptions::default(), FunctionCode::Read, &request).unwrap();
    assert_eq!(handle.select(&headers), Iin2::default());

    let mut buffer = vec![0; 8192];
    let mut cursor = WriteCursor::new(&mut buffer);
    assert!(handle.write_response_headers(&mut cursor).complete);

    let (result, recorder) = extract(cursor.written());
    result.unwrap();
    recorder.values().collect()
}

// ---------- the reference projection ----------

/// What the master reports for one point: the variation it was written in, whether that
/// variation has flags, and the value
type Reported = (V, bool, Value);

fn binary_flags(flags: Flags, state: bool) -> Flags {
    Flags::new((flags.value & 0x7F) | if state { 0x80 } else { 0 })
}

fn double_bit_flags(flags: Flags, state: DoubleBit) -> Flags {
    let bits = match state {
        DoubleBit::Intermediate => 0x00,
        DoubleBit::DeterminedOff => 0x40,
        DoubleBit::DeterminedOn => 0x80,
        DoubleBit::Indeterminate => 0xC0,
    };
    Flags::new((flags.value & 0x3F) | bits)
}

/// The time the master reports for a variation with absolute time: static frozen counters
/// (g21v5/6), and binary output status (g11v2), counter (g22v5/6), frozen counter (g23v5/6),
/// analog (g32v3/4/7/8) and analog output status (g42v3/4/7/8) events
///
/// This pins the current API convention, not a standards-backed oracle. These objects carry no
/// synchronization, so the master reports the time as synchronized whatever the outstation
/// recorded. Events recorded without time are reported in the variation without time instead,
/// so only a static frozen counter recorded without time is written with time 0 (a known
/// limitation, absent time -> 0). Only binary and double-bit events fall back to a variation
/// that keeps unsynchronized time; see [`binary`] and [`double_bit`].
fn absolute(time: Option<Time>) -> Option<Time> {
    let timestamp = match time {
        None => Timestamp::zero(),
        Some(Time::Synchronized(x) | Time::Unsynchronized(x)) => x,
    };
    Some(Time::Synchronized(timestamp))
}

/// a value in an integer variation with the range `[min, max]`
fn integer(value: f64, flags: Flags, min: f64, max: f64) -> (Flags, f64) {
    let (value, over_range) = analog_model::integer(value, min, max);
    let flags = if over_range {
        flags | Flags::OVER_RANGE
    } else {
        flags
    };
    // an integer has no negative zero
    (flags, if value == 0.0 { 0.0 } else { value })
}

fn i16_value(value: f64, flags: Flags) -> (Flags, f64) {
    integer(value, flags, i16::MIN.into(), i16::MAX.into())
}

fn i32_value(value: f64, flags: Flags) -> (Flags, f64) {
    integer(value, flags, i32::MIN.into(), i32::MAX.into())
}

#[expect(clippy::as_conversions, reason = "as rounds an f64 to the nearest f32")]
fn nearest_f32(value: f64) -> f32 {
    value as f32
}

fn f32_value(value: f64, flags: Flags) -> (Flags, f64) {
    let (min, max) = (f64::from(f32::MIN), f64::from(f32::MAX));
    let over_range = flags | Flags::OVER_RANGE;
    if value.is_finite() && value < min {
        (over_range, min)
    } else if value.is_finite() && value > max {
        (over_range, max)
    } else {
        (flags, nearest_f32(value).into())
    }
}

fn binary(read: V, index: u16, x: &BinaryInput) -> Reported {
    let flags = binary_flags(x.flags, x.value);
    let point = |flags, time| {
        let value = x.value;
        Value::Binary(BinaryInput { value, flags, time }, index)
    };
    match (read, x.time) {
        (V::Group1Var1, _) if x.flags.value & 0x7F == 0x01 => {
            (V::Group1Var1, false, point(Flags::ONLINE, None))
        }
        (V::Group1Var1 | V::Group1Var2, _) => (V::Group1Var2, true, point(flags, None)),
        (V::Group2Var1 | V::Group2Var2 | V::Group2Var3, None) | (V::Group2Var1, _) => {
            (V::Group2Var1, true, point(flags, None))
        }
        (V::Group2Var2, Some(Time::Synchronized(_))) => (V::Group2Var2, true, point(flags, x.time)),
        (V::Group2Var2 | V::Group2Var3, Some(_)) => (V::Group2Var3, true, point(flags, x.time)),
        _ => panic!("{read} is not a binary input variation"),
    }
}

fn double_bit(read: V, index: u16, x: &DoubleBitBinaryInput) -> Reported {
    let flags = double_bit_flags(x.flags, x.value);
    let point = |flags, time| {
        let value = x.value;
        Value::DoubleBit(DoubleBitBinaryInput { value, flags, time }, index)
    };
    match (read, x.time) {
        (V::Group3Var1, _) if x.flags.value & 0x3F == 0x01 => {
            (V::Group3Var1, false, point(Flags::ONLINE, None))
        }
        (V::Group3Var1 | V::Group3Var2, _) => (V::Group3Var2, true, point(flags, None)),
        (V::Group4Var1 | V::Group4Var2 | V::Group4Var3, None) | (V::Group4Var1, _) => {
            (V::Group4Var1, true, point(flags, None))
        }
        (V::Group4Var2, Some(Time::Synchronized(_))) => (V::Group4Var2, true, point(flags, x.time)),
        (V::Group4Var2 | V::Group4Var3, Some(_)) => (V::Group4Var3, true, point(flags, x.time)),
        _ => panic!("{read} is not a double-bit binary input variation"),
    }
}

fn binary_output_status(read: V, index: u16, x: &BinaryOutputStatus) -> Reported {
    let flags = binary_flags(x.flags, x.value);
    let point = |flags, time| {
        let value = x.value;
        Value::BinaryOutputStatus(BinaryOutputStatus { value, flags, time }, index)
    };
    match read {
        V::Group10Var1 if x.flags.value & 0x7F == 0x01 => {
            (V::Group10Var1, false, point(Flags::ONLINE, None))
        }
        V::Group10Var1 | V::Group10Var2 => (V::Group10Var2, true, point(flags, None)),
        V::Group11Var1 | V::Group11Var2 if x.time.is_none() => {
            (V::Group11Var1, true, point(flags, None))
        }
        V::Group11Var1 => (read, true, point(flags, None)),
        V::Group11Var2 => (read, true, point(flags, absolute(x.time))),
        _ => panic!("{read} is not a binary output status variation"),
    }
}

fn counter(read: V, index: u16, x: &Counter) -> Reported {
    let online = x.flags == Flags::ONLINE;
    let low = x.value & 0xFFFF;
    let point = |value, flags, time| Value::Counter(Counter { value, flags, time }, index);
    match read {
        V::Group20Var5 if online => (read, false, point(x.value, Flags::ONLINE, None)),
        V::Group20Var6 if online => (read, false, point(low, Flags::ONLINE, None)),
        V::Group20Var1 | V::Group20Var5 => (V::Group20Var1, true, point(x.value, x.flags, None)),
        V::Group20Var2 | V::Group20Var6 => (V::Group20Var2, true, point(low, x.flags, None)),
        V::Group22Var1 => (read, true, point(x.value, x.flags, None)),
        V::Group22Var2 => (read, true, point(low, x.flags, None)),
        V::Group22Var5 if x.time.is_none() => (V::Group22Var1, true, point(x.value, x.flags, None)),
        V::Group22Var6 if x.time.is_none() => (V::Group22Var2, true, point(low, x.flags, None)),
        V::Group22Var5 => (read, true, point(x.value, x.flags, absolute(x.time))),
        V::Group22Var6 => (read, true, point(low, x.flags, absolute(x.time))),
        _ => panic!("{read} is not a counter variation"),
    }
}

fn frozen_counter(read: V, index: u16, x: &FrozenCounter) -> Reported {
    let online = x.flags == Flags::ONLINE;
    let low = x.value & 0xFFFF;
    let point =
        |value, flags, time| Value::FrozenCounter(FrozenCounter { value, flags, time }, index);
    match read {
        V::Group21Var9 if online => (read, false, point(x.value, Flags::ONLINE, None)),
        V::Group21Var10 if online => (read, false, point(low, Flags::ONLINE, None)),
        V::Group21Var1 | V::Group21Var9 => (V::Group21Var1, true, point(x.value, x.flags, None)),
        V::Group21Var2 | V::Group21Var10 => (V::Group21Var2, true, point(low, x.flags, None)),
        V::Group23Var5 if x.time.is_none() => (V::Group23Var1, true, point(x.value, x.flags, None)),
        V::Group23Var6 if x.time.is_none() => (V::Group23Var2, true, point(low, x.flags, None)),
        V::Group21Var5 | V::Group23Var5 => (read, true, point(x.value, x.flags, absolute(x.time))),
        V::Group21Var6 | V::Group23Var6 => (read, true, point(low, x.flags, absolute(x.time))),
        V::Group23Var1 => (read, true, point(x.value, x.flags, None)),
        V::Group23Var2 => (read, true, point(low, x.flags, None)),
        _ => panic!("{read} is not a frozen counter variation"),
    }
}

/// The variation, value and time an analog event or analog output status event reports when
/// `read` is selected for it
fn analog_event(read: V, value: f64, flags: Flags, time: Option<Time>) -> AnalogEvent {
    let (group, variation) = read.to_group_and_var();
    let variation = match (variation, time) {
        (3, None) => 1,
        (4, None) => 2,
        (7, None) => 5,
        (8, None) => 6,
        (variation, _) => variation,
    };
    let (value, time) = match variation {
        1 => (i32_value(value, flags), None),
        2 => (i16_value(value, flags), None),
        3 => (i32_value(value, flags), absolute(time)),
        4 => (i16_value(value, flags), absolute(time)),
        5 => (f32_value(value, flags), None),
        6 => ((flags, value), None),
        7 => (f32_value(value, flags), absolute(time)),
        8 => ((flags, value), absolute(time)),
        _ => panic!("no event variation {variation}"),
    };
    let reported =
        V::lookup(group, variation).unwrap_or_else(|| panic!("no variation g{group}v{variation}"));
    (reported, value, time)
}

/// The variation, value with its flags, and time an analog event is reported with
type AnalogEvent = (V, (Flags, f64), Option<Time>);

fn analog(read: V, index: u16, x: &AnalogInput) -> Reported {
    let point = |(flags, value), time| Value::Analog(AnalogInput { value, flags, time }, index);
    let (i16, i32) = (i16_value(x.value, x.flags), i32_value(x.value, x.flags));
    match read {
        V::Group30Var3 if i32.0 == Flags::ONLINE => (read, false, point(i32, None)),
        V::Group30Var4 if i16.0 == Flags::ONLINE => (read, false, point(i16, None)),
        V::Group30Var1 | V::Group30Var3 => (V::Group30Var1, true, point(i32, None)),
        V::Group30Var2 | V::Group30Var4 => (V::Group30Var2, true, point(i16, None)),
        V::Group30Var5 => (read, true, point(f32_value(x.value, x.flags), None)),
        V::Group30Var6 => (read, true, point((x.flags, x.value), None)),
        V::Group32Var1
        | V::Group32Var2
        | V::Group32Var3
        | V::Group32Var4
        | V::Group32Var5
        | V::Group32Var6
        | V::Group32Var7
        | V::Group32Var8 => {
            let (variation, value, time) = analog_event(read, x.value, x.flags, x.time);
            (variation, true, point(value, time))
        }
        _ => panic!("{read} is not an analog input variation"),
    }
}

fn analog_output_status(read: V, index: u16, x: &AnalogOutputStatus) -> Reported {
    let point = |(flags, value), time| {
        Value::AnalogOutputStatus(AnalogOutputStatus { value, flags, time }, index)
    };
    match read {
        V::Group40Var1 => (read, true, point(i32_value(x.value, x.flags), None)),
        V::Group40Var2 => (read, true, point(i16_value(x.value, x.flags), None)),
        V::Group40Var3 => (read, true, point(f32_value(x.value, x.flags), None)),
        V::Group40Var4 => (read, true, point((x.flags, x.value), None)),
        V::Group42Var1
        | V::Group42Var2
        | V::Group42Var3
        | V::Group42Var4
        | V::Group42Var5
        | V::Group42Var6
        | V::Group42Var7
        | V::Group42Var8 => {
            let (variation, value, time) = analog_event(read, x.value, x.flags, x.time);
            (variation, true, point(value, time))
        }
        _ => panic!("{read} is not an analog output status variation"),
    }
}

fn octet_string(read: V, index: u16, x: &OctetString) -> Reported {
    let variation = match read {
        V::Group110(0) => V::Group110(x.len()),
        V::Group111(0) => V::Group111(x.len()),
        _ => panic!("{read} is not an octet string variation"),
    };
    (
        variation,
        false,
        Value::OctetString(x.value().to_vec(), index),
    )
}

#[expect(
    clippy::as_conversions,
    reason = "as rounds and saturates an f64 to an integer"
)]
fn dead_band(read: V, index: u16, x: f64) -> Reported {
    let value = match read {
        V::Group34Var1 => AnalogInputDeadBand::U16(x.round() as u16),
        V::Group34Var2 => AnalogInputDeadBand::U32(x.round() as u32),
        V::Group34Var3 => AnalogInputDeadBand::F32(nearest_f32(f32_value(x, Flags::ONLINE).1)),
        _ => panic!("{read} is not a deadband variation"),
    };
    (read, false, Value::AnalogDeadBand(value, index))
}

fn project<T>(
    points: &[(u16, T)],
    event: bool,
    reported: impl Fn(u16, &T) -> Reported,
) -> Vec<(HeaderInfo, Value)> {
    let mut points: Vec<&(u16, T)> = points.iter().collect();
    // events are reported in the order they were created, static values by index
    if !event {
        points.sort_by_key(|(index, _)| *index);
    }
    let qualifier = if event {
        QualifierCode::CountAndPrefix16
    } else {
        QualifierCode::Range16
    };
    points
        .into_iter()
        .map(|(index, x)| {
            let (variation, has_flags, value) = reported(*index, x);
            (
                HeaderInfo::new(variation, qualifier, event, has_flags),
                value,
            )
        })
        .collect()
}

fn expected(points: &Points, (read, event): Read) -> Vec<(HeaderInfo, Value)> {
    match points {
        Points::Binary(x) => project(x, event, |i, x| binary(read, i, x)),
        Points::DoubleBit(x) => project(x, event, |i, x| double_bit(read, i, x)),
        Points::BinaryOutputStatus(x) => project(x, event, |i, x| binary_output_status(read, i, x)),
        Points::Counter(x) => project(x, event, |i, x| counter(read, i, x)),
        Points::FrozenCounter(x) => project(x, event, |i, x| frozen_counter(read, i, x)),
        Points::Analog(x) => project(x, event, |i, x| analog(read, i, x)),
        Points::AnalogOutputStatus(x) => project(x, event, |i, x| analog_output_status(read, i, x)),
        Points::OctetString(x) => project(x, event, |i, x| octet_string(read, i, x)),
        Points::DeadBand(x) => project(x, event, |i, x| dead_band(read, i, *x)),
    }
}

// ---------- generators ----------

fn index() -> impl Strategy<Value = u16> {
    prop_oneof![0u16..8, 65528u16..=65535, any::<u16>()]
}

/// up to `MAX_POINTS` points at distinct indices, given times from [`times`] in order
fn points<T: Clone + Debug>(
    value: impl Strategy<Value = T>,
    set_time: fn(&mut T, Option<Time>),
) -> impl Strategy<Value = Vec<(u16, T)>> {
    (
        prop::collection::vec((index(), value), 1..=MAX_POINTS),
        times(),
    )
        .prop_map(move |(points, times)| {
            let mut indices = BTreeSet::new();
            points
                .into_iter()
                .filter(|(index, _)| indices.insert(*index))
                .zip(times)
                .map(|((index, mut value), time)| {
                    set_time(&mut value, time);
                    (index, value)
                })
                .collect()
        })
}

/// How the time of an event relates to the one before it
#[derive(Copy, Clone, Debug)]
enum Quality {
    Same,
    Other,
    Absent,
}

/// ONLINE alone, ONLINE with the bits that carry the state of a binary or double-bit point, or
/// any byte
fn flags() -> impl Strategy<Value = Flags> {
    prop_oneof![
        Just(Flags::ONLINE),
        prop::sample::select(vec![0x41, 0x81, 0xC1]).prop_map(Flags::new),
        any::<u8>().prop_map(Flags::new),
    ]
}

/// Times in the order events are created, each a step from the one before: mostly by 0, 1,
/// 65,535 (the most a CTO reaches) or 65,536, sometimes backward or by any amount, keeping the
/// synchronization of the time before, changing it, or with no time
fn times() -> impl Strategy<Value = Vec<Option<Time>>> {
    let start = prop_oneof![Just(0), Just(0xFFFF_FFFE_0000u64), 0..=Timestamp::MAX_VALUE];
    let step = prop_oneof![
        4 => prop::sample::select(vec![0i64, 1, 65_535, 65_536]),
        1 => prop::sample::select(vec![-1i64, -65_536]),
        1 => -70_000i64..70_000,
    ];
    let quality = prop_oneof![
        4 => Just(Quality::Same),
        1 => Just(Quality::Other),
        1 => Just(Quality::Absent),
    ];
    (
        start,
        any::<bool>(),
        prop::collection::vec((step, quality), MAX_POINTS),
    )
        .prop_map(|(mut timestamp, mut synchronized, steps)| {
            steps
                .into_iter()
                .map(|(step, quality)| {
                    timestamp = timestamp
                        .saturating_add_signed(step)
                        .min(Timestamp::MAX_VALUE);
                    match quality {
                        Quality::Same => {}
                        Quality::Other => synchronized = !synchronized,
                        Quality::Absent => return None,
                    }
                    let timestamp = Timestamp::new(timestamp);
                    Some(if synchronized {
                        Time::Synchronized(timestamp)
                    } else {
                        Time::Unsynchronized(timestamp)
                    })
                })
                .collect()
        })
}

fn counter_value() -> impl Strategy<Value = u32> {
    prop_oneof![
        prop::sample::select(vec![0, 0xFFFF, 0x1_0000, u32::MAX]),
        any::<u32>()
    ]
}

fn dead_band_value() -> impl Strategy<Value = f64> {
    prop_oneof![
        prop::sample::select(vec![
            0.0,
            0.5,
            65534.5,
            65535.0,
            65535.5,
            4_294_967_295.5,
            3.402_823_7e38,
            f64::MAX,
            f64::INFINITY,
        ]),
        0.0..1e10,
    ]
}

/// Write and extract the points in every variation of their family
fn check(points: &Points, family: &[Read]) -> Result<(), TestCaseError> {
    for &read in family {
        prop_assert_eq!(
            write_and_extract(points, read),
            expected(points, read),
            "read {}",
            read.0
        );
    }
    Ok(())
}

fn double_bit_state() -> impl Strategy<Value = DoubleBit> {
    prop::sample::select(vec![
        DoubleBit::Intermediate,
        DoubleBit::DeterminedOff,
        DoubleBit::DeterminedOn,
        DoubleBit::Indeterminate,
    ])
}

proptest! {
    #![proptest_config(crate::util::test_support::proptest_config(64))]

    #[test]
    fn binary_inputs_are_extracted_as_written(
        points in points((any::<bool>(), flags())
            .prop_map(|(value, flags)| BinaryInput { value, flags, time: None }), |x, time| x.time = time),
    ) {
        check(&Points::Binary(points), &specific(BINARY_INPUT_STATIC_READS, BINARY_INPUT_EVENT_READS))?;
    }

    #[test]
    fn double_bit_binary_inputs_are_extracted_as_written(
        points in points((double_bit_state(), flags())
            .prop_map(|(value, flags)| DoubleBitBinaryInput { value, flags, time: None }), |x, time| x.time = time),
    ) {
        check(&Points::DoubleBit(points), &specific(DOUBLE_BIT_BINARY_INPUT_STATIC_READS, DOUBLE_BIT_BINARY_INPUT_EVENT_READS))?;
    }

    #[test]
    fn binary_output_statuses_are_extracted_as_written(
        points in points((any::<bool>(), flags())
            .prop_map(|(value, flags)| BinaryOutputStatus { value, flags, time: None }), |x, time| x.time = time),
    ) {
        check(&Points::BinaryOutputStatus(points), &specific(BINARY_OUTPUT_STATUS_STATIC_READS, BINARY_OUTPUT_STATUS_EVENT_READS))?;
    }

    #[test]
    fn counters_are_extracted_as_written(
        points in points((counter_value(), flags())
            .prop_map(|(value, flags)| Counter { value, flags, time: None }), |x, time| x.time = time),
    ) {
        check(&Points::Counter(points), &specific(COUNTER_STATIC_READS, COUNTER_EVENT_READS))?;
    }

    #[test]
    fn frozen_counters_are_extracted_as_written(
        points in points((counter_value(), flags())
            .prop_map(|(value, flags)| FrozenCounter { value, flags, time: None }), |x, time| x.time = time),
    ) {
        check(&Points::FrozenCounter(points), &specific(FROZEN_COUNTER_STATIC_READS, FROZEN_COUNTER_EVENT_READS))?;
    }

    #[test]
    fn analog_inputs_are_extracted_as_written(
        points in points((analog_model::value(), flags())
            .prop_map(|(value, flags)| AnalogInput { value, flags, time: None }), |x, time| x.time = time),
    ) {
        check(&Points::Analog(points), &specific(ANALOG_INPUT_STATIC_READS, ANALOG_INPUT_EVENT_READS))?;
    }

    #[test]
    fn analog_output_statuses_are_extracted_as_written(
        points in points((analog_model::value(), flags())
            .prop_map(|(value, flags)| AnalogOutputStatus { value, flags, time: None }), |x, time| x.time = time),
    ) {
        check(&Points::AnalogOutputStatus(points), &specific(ANALOG_OUTPUT_STATUS_STATIC_READS, ANALOG_OUTPUT_STATUS_EVENT_READS))?;
    }

    #[test]
    fn octet_strings_are_extracted_as_written(
        points in points(prop::collection::vec(any::<u8>(), 1..=255)
            .prop_map(|x| OctetString::new(&x).unwrap()), |_, _| {}),
    ) {
        check(&Points::OctetString(points), OCTET_STRING)?;
    }

    #[test]
    fn analog_input_deadbands_are_extracted_as_written(
        points in points(dead_band_value(), |_, _| {}),
    ) {
        check(&Points::DeadBand(points), &specific(ANALOG_INPUT_DEAD_BAND_STATIC_READS, &[]))?;
    }
}
