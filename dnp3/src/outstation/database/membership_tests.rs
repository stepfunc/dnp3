// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! A READ response reports the static points it selected, even if points change between its
//! fragments
//!
//! For generated binary inputs and analog inputs, READ headers and small fragment sizes, the
//! database is written one fragment at a time, and points are added, removed and updated between
//! fragments. What master extraction reports across the fragments is each point the READ
//! selected, once per header that selected it, in header and index order, with the value it had
//! when the READ selected it; except that a point removed before its turn is left out, and isn't
//! reported again if its index is added back. A point added after the selection is never
//! reported. The series completes within a bound, with nothing left that should have been
//! reported.
//!
//! The model is independent of the database: it records the ordered occurrences when the READ
//! selects them, with the values it gave the points, and gives every added point a new identity,
//! so that a removal ends the pending occurrences of exactly the point it removed.
//!
//! Binary inputs are read in ranges and by class 0 (the class 0 configuration includes only
//! them), through the static value writer, as g1v2. Analog inputs are read as g34v1 deadbands,
//! through the deadband writer. Indices are dense enough that additions often land inside a range
//! still to be written, and fragments are small enough that most responses take several. Not
//! generated: events, attributes, other point types and variations, which the fragmenting
//! property covers without changes between fragments.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "the model numbers points and counts fragments"
)]

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use proptest::prelude::*;
use scursor::WriteCursor;

use crate::app::parse::options::ParseOptions;
use crate::app::parse::parser::HeaderCollection;
use crate::app::FunctionCode;
use crate::master::extract_recorder::{extract, Value};
use crate::outstation::config::OutstationConfig;
use crate::outstation::database::*;

/// Indices of the generated points
const INDICES: std::ops::Range<u16> = 0..24;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Binary,
    Analog,
}

/// What a point reports: a binary input's state and flags, or an analog input's deadband
#[derive(Copy, Clone, Debug, PartialEq)]
enum Reported {
    Binary(bool, u8),
    DeadBand(u16),
}

#[derive(Copy, Clone, Debug)]
enum Range {
    All,
    Some(u16, u16),
}

impl Range {
    fn contains(self, index: u16) -> bool {
        match self {
            Range::All => true,
            Range::Some(start, stop) => (start..=stop).contains(&index),
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum Read {
    Binary(Range),
    Class0,
    DeadBand(Range),
}

#[derive(Copy, Clone, Debug)]
enum Op {
    Add(Kind, u16, Reported),
    Remove(Kind, u16),
    /// a new state and flags for a binary input
    Update(u16, bool, u8),
}

#[derive(Clone, Debug)]
struct Spec {
    size: usize,
    initial: Vec<Op>,
    reads: Vec<Read>,
    /// the changes after each fragment but the last
    between: Vec<Vec<Op>>,
}

/// A point that is installed, with the identity the model gave it when it was added
#[derive(Copy, Clone, Debug)]
struct Installed {
    id: u32,
    reported: Reported,
}

/// A point that the READ selected, with what it reports
#[derive(Copy, Clone, Debug)]
struct Occurrence {
    kind: Kind,
    index: u16,
    id: u32,
    reported: Reported,
}

#[derive(Default)]
struct Model {
    next_id: u32,
    installed: BTreeMap<(Kind, u16), Installed>,
    removed: BTreeSet<u32>,
}

impl Model {
    /// apply an operation to the database and the model, which must agree on whether it took
    fn apply(&mut self, db: &mut Database, op: Op) {
        match op {
            Op::Add(kind, index, reported) => {
                let added = match reported {
                    Reported::Binary(state, flags) => {
                        let config = BinaryInputConfig::new(
                            StaticBinaryInputVariation::Group1Var2,
                            EventBinaryInputVariation::Group2Var1,
                        );
                        db.add(index, None, config) && update_binary(db, index, state, flags)
                    }
                    Reported::DeadBand(deadband) => {
                        let config = AnalogInputConfig::new(
                            StaticAnalogInputVariation::Group30Var1,
                            EventAnalogInputVariation::Group32Var1,
                            f64::from(deadband),
                        );
                        db.add(index, None, config)
                    }
                };
                let free = !self.installed.contains_key(&(kind, index));
                assert_eq!(added, free);
                if free {
                    let id = self.next_id;
                    self.next_id += 1;
                    self.installed
                        .insert((kind, index), Installed { id, reported });
                }
            }
            Op::Remove(kind, index) => {
                let removed = match kind {
                    Kind::Binary => Remove::<BinaryInput>::remove(db, index),
                    Kind::Analog => Remove::<AnalogInput>::remove(db, index),
                };
                let installed = self.installed.remove(&(kind, index));
                assert_eq!(removed, installed.is_some());
                if let Some(point) = installed {
                    self.removed.insert(point.id);
                }
            }
            Op::Update(index, state, flags) => {
                let updated = update_binary(db, index, state, flags);
                let point = self.installed.get_mut(&(Kind::Binary, index));
                assert_eq!(updated, point.is_some());
                if let Some(point) = point {
                    point.reported = Reported::Binary(state, flags);
                }
            }
        }
    }

    /// the points each header selects, in order, as they are now
    fn select(&self, reads: &[Read]) -> VecDeque<Occurrence> {
        let points = |kind: Kind, range: Range| {
            self.installed
                .iter()
                .filter(move |((k, index), _)| *k == kind && range.contains(*index))
                .map(|((kind, index), point)| Occurrence {
                    kind: *kind,
                    index: *index,
                    id: point.id,
                    reported: point.reported,
                })
        };
        reads
            .iter()
            .flat_map(|read| -> Vec<Occurrence> {
                match read {
                    Read::Binary(range) => points(Kind::Binary, *range).collect(),
                    Read::Class0 => points(Kind::Binary, Range::All).collect(),
                    Read::DeadBand(range) => points(Kind::Analog, *range).collect(),
                }
            })
            .collect()
    }
}

/// update a binary input's state and flags without an event, returning whether it exists
fn update_binary(db: &mut Database, index: u16, state: bool, flags: u8) -> bool {
    let value = BinaryInput::new(state, Flags::new(flags), Time::synchronized(0));
    db.update(index, &value, UpdateOptions::no_event())
}

fn request(reads: &[Read]) -> Vec<u8> {
    fn range(bytes: &mut Vec<u8>, group: u8, var: u8, range: Range) {
        bytes.extend([group, var]);
        match range {
            Range::All => bytes.push(0x06),
            Range::Some(start, stop) => {
                bytes.push(0x01);
                bytes.extend(start.to_le_bytes());
                bytes.extend(stop.to_le_bytes());
            }
        }
    }

    let mut bytes = Vec::new();
    for read in reads {
        match read {
            Read::Binary(r) => range(&mut bytes, 1, 0, *r),
            Read::Class0 => bytes.extend([60, 1, 0x06]),
            Read::DeadBand(r) => range(&mut bytes, 34, 1, *r),
        }
    }
    bytes
}

/// What a reported value identifies and reports
fn reported(value: &Value) -> (Kind, u16, Reported) {
    match value {
        Value::Binary(x, index) => (
            Kind::Binary,
            *index,
            Reported::Binary(x.value, x.flags.value & 0x7F),
        ),
        Value::AnalogDeadBand(AnalogInputDeadBand::U16(x), index) => {
            (Kind::Analog, *index, Reported::DeadBand(*x))
        }
        other => panic!("unexpected value {other:?}"),
    }
}

fn check(spec: &Spec) -> Result<(), TestCaseError> {
    let handle_config = ClassZeroConfig {
        binary: true,
        double_bit_binary: false,
        binary_output_status: false,
        counter: false,
        frozen_counter: false,
        analog: false,
        analog_output_status: false,
        octet_string: false,
    };
    let mut handle = DatabaseHandle::new(
        OutstationConfig::DEFAULT_MAX_READ_REQUEST_HEADERS,
        handle_config,
        EventBufferConfig::all_types(0),
    );
    let mut model = Model::default();
    handle.transaction(|db| {
        for op in &spec.initial {
            model.apply(db, *op);
        }
    });

    let request = request(&spec.reads);
    let headers = HeaderCollection::parse(ParseOptions::default(), FunctionCode::Read, &request)
        .map_err(|err| TestCaseError::fail(format!("the READ doesn't parse: {err:?}")))?;
    handle.reset();
    handle.select(&headers);
    let mut pending = model.select(&spec.reads);

    // every fragment but the last writes at least one object, since any one fits in an empty
    // fragment
    let max_fragments = pending.len() + 1;
    let mut buffer = vec![0; spec.size];
    let mut between = spec.between.iter();
    for _ in 0..max_fragments {
        let mut cursor = WriteCursor::new(&mut buffer);
        let info = handle.write_response_headers(&mut cursor);
        let (result, recorder) = extract(cursor.written());
        prop_assert!(result.is_ok(), "the fragment doesn't parse");

        for (_, value) in recorder.values() {
            // the next occurrence of a point that hasn't been removed
            while pending
                .front()
                .is_some_and(|x| model.removed.contains(&x.id))
            {
                pending.pop_front();
            }
            let expected = pending.pop_front().map(|x| (x.kind, x.index, x.reported));
            prop_assert_eq!(Some(reported(&value)), expected);
        }

        if info.complete {
            // nothing is left that should have been reported
            prop_assert!(pending.iter().all(|x| model.removed.contains(&x.id)));
            return Ok(());
        }
        if let Some(ops) = between.next() {
            handle.transaction(|db| {
                for op in ops {
                    model.apply(db, *op);
                }
            });
        }
    }
    Err(TestCaseError::fail(format!(
        "the response is not complete after {max_fragments} fragments"
    )))
}

// ---------- generators ----------

fn flags() -> impl Strategy<Value = u8> {
    prop::sample::select(vec![0x00, 0x01, 0x02, 0x03, 0x41])
}

fn add() -> impl Strategy<Value = Op> {
    prop_oneof![
        (INDICES, any::<bool>(), flags()).prop_map(|(index, state, flags)| Op::Add(
            Kind::Binary,
            index,
            Reported::Binary(state, flags)
        )),
        (INDICES, 0u16..1000).prop_map(|(index, deadband)| Op::Add(
            Kind::Analog,
            index,
            Reported::DeadBand(deadband)
        )),
    ]
}

fn op() -> impl Strategy<Value = Op> {
    let kind = prop::sample::select(vec![Kind::Binary, Kind::Analog]);
    prop_oneof![
        3 => add(),
        2 => (kind, INDICES).prop_map(|(kind, index)| Op::Remove(kind, index)),
        2 => (INDICES, any::<bool>(), flags())
            .prop_map(|(index, state, flags)| Op::Update(index, state, flags)),
    ]
}

fn range() -> impl Strategy<Value = Range> {
    prop_oneof![
        Just(Range::All),
        (INDICES, 0u16..12).prop_map(|(start, length)| Range::Some(start, start + length)),
    ]
}

fn read() -> impl Strategy<Value = Read> {
    prop_oneof![
        3 => range().prop_map(Read::Binary),
        1 => Just(Read::Class0),
        2 => range().prop_map(Read::DeadBand),
    ]
}

fn spec() -> impl Strategy<Value = Spec> {
    (
        // a g34v1 header with one object takes 9 bytes
        9usize..=40,
        prop::collection::vec(add(), 1..32),
        prop::collection::vec(read(), 1..=4),
        prop::collection::vec(prop::collection::vec(op(), 0..=3), 0..48),
    )
        .prop_map(|(size, initial, reads, between)| Spec {
            size,
            initial,
            reads,
            between,
        })
}

proptest! {
    #![proptest_config(crate::util::test_support::proptest_config(256))]

    #[test]
    fn a_response_reports_the_points_its_read_selected(spec in spec()) {
        check(&spec)?;
    }
}
