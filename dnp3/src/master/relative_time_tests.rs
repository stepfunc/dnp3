// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! A model of how the master checks and extracts objects whose time is relative to a common time
//! of occurrence (CTO): which responses it can process, the time of each relative-time object,
//! and what extraction reports it delivered

use proptest::prelude::*;

use crate::app::measurement::*;
use crate::app::{QualifierCode, Timestamp, Variation};
use crate::master::extract::{Delivered, InvalidResponse};
use crate::master::extract_recorder::{extract, extract_each_header, Call, Kind, Value};
use crate::master::test_objects::{
    CTOS_WITH_A_COUNT_OTHER_THAN_ONE, OTHER_RELATIVE_TIME_EVENTS, RELATIVE_TIME_EVENT,
    SYNCHRONIZED_CTO,
};
use crate::master::HeaderInfo;

/// One object header of a generated response fragment
#[derive(Clone, Debug)]
enum Spec {
    /// g51v1 (synchronized) or g51v2 with a 1 or 2-byte count of times
    Cto {
        synchronized: bool,
        wide: bool,
        times: Vec<u64>,
    },
    /// g2v3 or g4v3 with a 1 or 2-byte count and prefix: index, flags and relative time
    Relative {
        double_bit: bool,
        wide: bool,
        objects: Vec<(u16, u8, u16)>,
    },
    /// g50v1 with a 1 or 2-byte count of times
    AbsTime { wide: bool, times: Vec<u64> },
    /// g1v2 with a 1-byte start and stop: the flags of each point
    Binary { start: u8, flags: Vec<u8> },
    /// g30v1 with a 1-byte start and stop: the flags and value of each point
    Analog { start: u8, values: Vec<(u8, i32)> },
}

fn write_count(bytes: &mut Vec<u8>, wide: bool, count: usize) {
    if wide {
        bytes.extend(u16::try_from(count).unwrap().to_le_bytes());
    } else {
        bytes.push(u8::try_from(count).unwrap());
    }
}

fn write_u48(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend(&value.to_le_bytes()[..6]);
}

fn count_qualifier(wide: bool) -> QualifierCode {
    if wide {
        QualifierCode::Count16
    } else {
        QualifierCode::Count8
    }
}

fn prefix_qualifier(wide: bool) -> QualifierCode {
    if wide {
        QualifierCode::CountAndPrefix16
    } else {
        QualifierCode::CountAndPrefix8
    }
}

/// the index an object is encoded with: the low byte with a 1-byte prefix
fn prefix_index(wide: bool, index: u16) -> u16 {
    if wide {
        index
    } else {
        index & 0xFF
    }
}

/// the indices of a range of `count` points starting at `start`
fn range_indices(start: u8, count: usize) -> impl Iterator<Item = u16> {
    (u16::from(start)..).take(count)
}

impl Spec {
    fn encode(&self, bytes: &mut Vec<u8>) {
        match self {
            Spec::Cto {
                synchronized,
                wide,
                times,
            } => {
                bytes.extend([51, if *synchronized { 1 } else { 2 }]);
                bytes.push(count_qualifier(*wide).as_u8());
                write_count(bytes, *wide, times.len());
                for time in times {
                    write_u48(bytes, *time);
                }
            }
            Spec::Relative {
                double_bit,
                wide,
                objects,
            } => {
                bytes.extend([if *double_bit { 4 } else { 2 }, 3]);
                bytes.push(prefix_qualifier(*wide).as_u8());
                write_count(bytes, *wide, objects.len());
                for (index, flags, time) in objects {
                    if *wide {
                        bytes.extend(index.to_le_bytes());
                    } else {
                        bytes.push(index.to_le_bytes()[0]);
                    }
                    bytes.push(*flags);
                    bytes.extend(time.to_le_bytes());
                }
            }
            Spec::AbsTime { wide, times } => {
                bytes.extend([50, 1]);
                bytes.push(count_qualifier(*wide).as_u8());
                write_count(bytes, *wide, times.len());
                for time in times {
                    write_u48(bytes, *time);
                }
            }
            Spec::Binary { start, flags } => {
                let stop = range_indices(*start, flags.len()).last().unwrap();
                bytes.extend([1, 2, QualifierCode::Range8.as_u8(), *start]);
                bytes.push(u8::try_from(stop).unwrap());
                bytes.extend(flags);
            }
            Spec::Analog { start, values } => {
                let stop = range_indices(*start, values.len()).last().unwrap();
                bytes.extend([30, 1, QualifierCode::Range8.as_u8(), *start]);
                bytes.push(u8::try_from(stop).unwrap());
                for (flags, value) in values {
                    bytes.push(*flags);
                    bytes.extend(value.to_le_bytes());
                }
            }
        }
    }
}

/// The double-bit state in bits 7 and 6 of the flags
fn double_bit(flags: u8) -> DoubleBit {
    match (flags & 0x80 != 0, flags & 0x40 != 0) {
        (false, false) => DoubleBit::Intermediate,
        (false, true) => DoubleBit::DeterminedOff,
        (true, false) => DoubleBit::DeterminedOn,
        (true, true) => DoubleBit::Indeterminate,
    }
}

/// The CTO plus a relative time, keeping the CTO's synchronization, unless it overflows 48 bits
fn relative(cto: Option<Time>, offset: u16) -> Option<Time> {
    let (synchronized, base) = match cto? {
        Time::Synchronized(x) => (true, x.raw_value()),
        Time::Unsynchronized(x) => (false, x.raw_value()),
    };
    let sum = base
        .checked_add(offset.into())
        .filter(|x| *x <= Timestamp::MAX_VALUE)?;
    Some(if synchronized {
        Time::Synchronized(Timestamp::new(sum))
    } else {
        Time::Unsynchronized(Timestamp::new(sum))
    })
}

/// What the model predicts for a fragment
#[derive(Debug, PartialEq)]
struct Expected {
    /// the result of the master's check that it can process the fragment
    check: Result<(), InvalidResponse>,
    /// the calls when each header is extracted in turn, whether or not the check passes
    calls: Vec<Call>,
    /// what extraction reports it delivered
    delivered: Delivered,
}

fn model(specs: &[Spec]) -> Expected {
    let mut cto = None;
    let mut violation = None;
    let mut calls = Vec::new();
    let mut delivered = Delivered::default();
    for spec in specs {
        match spec {
            // only a CTO with exactly one time replaces the one before it
            Spec::Cto {
                synchronized,
                times,
                ..
            } => {
                if let [time] = times.as_slice() {
                    cto = Some(if *synchronized {
                        Time::Synchronized(Timestamp::new(*time))
                    } else {
                        Time::Unsynchronized(Timestamp::new(*time))
                    });
                }
            }
            Spec::Relative {
                double_bit: is_double_bit,
                wide,
                objects,
            } => {
                let variation = if *is_double_bit {
                    Variation::Group4Var3
                } else {
                    Variation::Group2Var3
                };
                if !objects.is_empty() {
                    if cto.is_none() && violation.is_none() {
                        violation = Some(variation);
                    }
                    delivered.values = true;
                    delivered.events = true;
                }
                let info = HeaderInfo::new(variation, prefix_qualifier(*wide), true, true);
                let points = objects.iter().map(|(index, flags, offset)| {
                    let index = prefix_index(*wide, *index);
                    (Flags::new(*flags), relative(cto, *offset), index)
                });
                calls.push(if *is_double_bit {
                    Call {
                        kind: Kind::DoubleBitBinaryInput,
                        info,
                        values: points
                            .map(|(flags, time, index)| {
                                let value = double_bit(flags.value);
                                Value::DoubleBit(DoubleBitBinaryInput { value, flags, time }, index)
                            })
                            .collect(),
                    }
                } else {
                    Call {
                        kind: Kind::BinaryInput,
                        info,
                        values: points
                            .map(|(flags, time, index)| {
                                let value = flags.value & 0x80 != 0;
                                Value::Binary(BinaryInput { value, flags, time }, index)
                            })
                            .collect(),
                    }
                });
            }
            // only an absolute time with exactly one time is passed to the handler
            Spec::AbsTime { wide, times } => {
                if let [time] = times.as_slice() {
                    let info = HeaderInfo::new(
                        Variation::Group50Var1,
                        count_qualifier(*wide),
                        false,
                        false,
                    );
                    calls.push(Call {
                        kind: Kind::AbsTime,
                        info,
                        values: vec![Value::AbsTime(Timestamp::new(*time))],
                    });
                    delivered.values = true;
                }
            }
            Spec::Binary { start, flags } => {
                let info =
                    HeaderInfo::new(Variation::Group1Var2, QualifierCode::Range8, false, true);
                let points = flags.iter().zip(range_indices(*start, flags.len()));
                calls.push(Call {
                    kind: Kind::BinaryInput,
                    info,
                    values: points
                        .map(|(flags, index)| {
                            let value = flags & 0x80 != 0;
                            let flags = Flags::new(*flags);
                            let time = None;
                            Value::Binary(BinaryInput { value, flags, time }, index)
                        })
                        .collect(),
                });
                delivered.values = true;
            }
            Spec::Analog { start, values } => {
                let info =
                    HeaderInfo::new(Variation::Group30Var1, QualifierCode::Range8, false, true);
                let points = values.iter().zip(range_indices(*start, values.len()));
                calls.push(Call {
                    kind: Kind::AnalogInput,
                    info,
                    values: points
                        .map(|((flags, value), index)| {
                            let value = AnalogInput {
                                value: (*value).into(),
                                flags: Flags::new(*flags),
                                time: None,
                            };
                            Value::Analog(value, index)
                        })
                        .collect(),
                });
                delivered.values = true;
            }
        }
    }
    Expected {
        check: violation.map_or(Ok(()), |v| Err(InvalidResponse::RelativeTimeWithoutCto(v))),
        calls,
        delivered,
    }
}

/// Check the fragment, and extract each of its headers in turn whatever the result, as
/// `extract_measurements_inner` does once the check passes
fn run(objects: &[u8]) -> Expected {
    let (delivered, recorder) = extract_each_header(objects);

    let (check, processed) = extract(objects);
    let check = check.map(|reported| {
        assert_eq!(processed.calls, recorder.calls);
        assert_eq!(reported, delivered);
    });

    Expected {
        check,
        calls: recorder.calls,
        delivered,
    }
}

fn time() -> impl Strategy<Value = u64> {
    prop_oneof![
        // one below the maximum and the maximum, from which offsets 0 to 2 reach it or overflow it
        Just(Timestamp::MAX_VALUE - 1),
        Just(Timestamp::MAX_VALUE),
        0..=0xFFFFu64,
        0xFFFF_FFFE_FFFFu64..=Timestamp::MAX_VALUE,
        0..=Timestamp::MAX_VALUE,
    ]
}

fn offset() -> impl Strategy<Value = u16> {
    prop_oneof![Just(0), Just(1), Just(2), Just(u16::MAX), any::<u16>()]
}

fn spec() -> impl Strategy<Value = Spec> {
    prop_oneof![
        3 => (any::<bool>(), any::<bool>(), prop::collection::vec(time(), 0..=2)).prop_map(
            |(synchronized, wide, times)| Spec::Cto {
                synchronized,
                wide,
                times,
            }
        ),
        4 => (
            any::<bool>(),
            any::<bool>(),
            prop::collection::vec((any::<u16>(), any::<u8>(), offset()), 0..4),
        )
            .prop_map(|(double_bit, wide, objects)| Spec::Relative {
                double_bit,
                wide,
                objects,
            }),
        1 => (any::<bool>(), prop::collection::vec(time(), 0..=2))
            .prop_map(|(wide, times)| Spec::AbsTime { wide, times }),
        1 => (0u8..=250, prop::collection::vec(any::<u8>(), 1..4))
            .prop_map(|(start, flags)| Spec::Binary { start, flags }),
        1 => (0u8..=250, prop::collection::vec(any::<(u8, i32)>(), 1..4))
            .prop_map(|(start, values)| Spec::Analog { start, values }),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn relative_time_follows_the_cto_model(specs in prop::collection::vec(spec(), 0..8)) {
        let mut objects = Vec::new();
        for spec in &specs {
            spec.encode(&mut objects);
        }
        prop_assert_eq!(run(&objects), model(&specs));
    }
}

/// g2v3 or g4v3 with one event at index 7 with flags 0x01 and relative time 0xFFFF
fn event(double_bit: bool, wide: bool) -> Spec {
    Spec::Relative {
        double_bit,
        wide,
        objects: vec![(7, 0x01, 0xFFFF)],
    }
}

fn cto(synchronized: bool, wide: bool, times: &[u64]) -> Spec {
    Spec::Cto {
        synchronized,
        wide,
        times: times.to_vec(),
    }
}

/// Check that `specs` encode to `objects`, that the model's check decides `check`, and that the
/// master follows the model
fn expect_fixed(specs: &[Spec], objects: &[&[u8]], check: Result<(), InvalidResponse>) {
    let mut encoded = Vec::new();
    for spec in specs {
        spec.encode(&mut encoded);
    }
    assert_eq!(encoded, objects.concat(), "{specs:?}");
    let expected = model(specs);
    assert_eq!(expected.check, check, "{specs:?}");
    assert_eq!(run(&encoded), expected, "{specs:?}");
}

/// The responses that both sites acting on the check, reads and unsolicited responses, are tested
/// with: relative time without a CTO at each variation and width, before a CTO, and after a CTO
/// with a count other than one, with and without a valid CTO before it
#[test]
fn fixed_responses_follow_the_cto_model() {
    let without_cto = |variation| Err(InvalidResponse::RelativeTimeWithoutCto(variation));
    let other_events = [
        (event(true, false), Variation::Group4Var3),
        (event(false, true), Variation::Group2Var3),
        (event(true, true), Variation::Group4Var3),
    ];
    for ((spec, variation), bytes) in other_events.into_iter().zip(OTHER_RELATIVE_TIME_EVENTS) {
        expect_fixed(&[spec], &[bytes], without_cto(variation));
    }
    expect_fixed(
        &[
            event(false, false),
            cto(true, false, &[2]),
            event(false, false),
        ],
        &[RELATIVE_TIME_EVENT, SYNCHRONIZED_CTO, RELATIVE_TIME_EVENT],
        without_cto(Variation::Group2Var3),
    );

    let other_ctos = [
        cto(true, false, &[]),
        cto(false, true, &[]),
        cto(true, false, &[2, 3]),
        cto(false, true, &[2, 3]),
    ];
    for (spec, bytes) in other_ctos.into_iter().zip(CTOS_WITH_A_COUNT_OTHER_THAN_ONE) {
        expect_fixed(
            &[spec.clone(), event(false, false)],
            &[bytes, RELATIVE_TIME_EVENT],
            without_cto(Variation::Group2Var3),
        );
        expect_fixed(
            &[cto(true, false, &[2]), spec, event(false, false)],
            &[SYNCHRONIZED_CTO, bytes, RELATIVE_TIME_EVENT],
            Ok(()),
        );
    }
}
