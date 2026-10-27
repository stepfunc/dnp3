// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! The event buffer against a model
//!
//! `behaves_like_a_list_with_per_type_limits` checks the buffer against a list of records in
//! insertion order, each with its type, class and state (unselected, selected or written), and
//! per-type limits that evict the oldest record of the type, whatever its state. After every
//! generated op, the buffer's class and type counts, unwritten classes, overflow flag and
//! per-type presence must match the model's, and so must the op's result: an insert's id, its
//! eviction (`Overflow { created, discarded }`) or `TypeMaxIsZero`; selection counts and the
//! records each selection visits (all of them, or through the record after the one that
//! reaches its limit, which READ budgets count); the
//! written count, progress and bytes; the cleared ids in order; and discard counts. A final
//! reset, select, write and clear must leave both empty.
//!
//! The oracle is independent: the model writes the bytes itself, a header for each run of one
//! group and variation and then a 2-byte index and the data for each event, with a timestamp
//! whose bytes all differ, and shares nothing with `EventWriter`.
//!
//! The generator uses per-type limits of 0..4, up to 96 ops over the 8 types, any index, class
//! reads with a limit of 0..4 or none, type reads in the default or a specific variation, and
//! cursors of 0..48 bytes or 4096. It is built to reach: eviction of an unselected, a selected
//! and a written record (of the new record's class or another) for every type, and while
//! overflown; a type with no room; selects cut by a limit, including 0; partial writes where
//! nothing fits, in a header and at a new header, and writes resuming after them; clears and
//! discards with other records in flight; overflow persisting and clearing after a clear or a
//! discard; and a reset of selected or written records followed by a discard. Two cases come up
//! in only some runs, so fixed traces through the model below cover them: a new header of the
//! same group in another variation, and every type full at once (which fills the list holding the
//! records) before an insert evicts.
//!
//! Not generated: CTO variations (g2v3, g4v3), events without time, octet strings of other
//! lengths (see `fragment_tests.rs`, `tests/event_time.rs` and `writer.rs`), and a header count
//! past `u16::MAX`.

use proptest::prelude::*;
use scursor::WriteCursor;

use crate::app::measurement::*;
use crate::master::EventClasses;
use crate::outstation::database::config::*;
use crate::outstation::database::details::{Visits, WriteProgress};
use crate::outstation::database::variations::*;
use crate::outstation::database::{EventBufferConfig, EventClass};
use crate::outstation::{BufferState, ClassCount, TypeCount};
use crate::util::test_support::remove_at;

use super::buffer::{EventBuffer, InsertError};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Kind {
    Binary,
    DoubleBitBinary,
    BinaryOutputStatus,
    Counter,
    FrozenCounter,
    Analog,
    AnalogOutputStatus,
    OctetString,
}

const KINDS: [Kind; 8] = [
    Kind::Binary,
    Kind::DoubleBitBinary,
    Kind::BinaryOutputStatus,
    Kind::Counter,
    Kind::FrozenCounter,
    Kind::Analog,
    Kind::AnalogOutputStatus,
    Kind::OctetString,
];

const CLASSES: [EventClass; 3] = [EventClass::Class1, EventClass::Class2, EventClass::Class3];

// the time of every event, distinct in each byte
const TIMESTAMP: u64 = 0x0000_0605_0403_0201;

fn octet_string() -> OctetString {
    OctetString::new(&[0xAA, 0xBB]).unwrap()
}

impl Kind {
    /// the group, variation and data of an event of this type, written independently of the
    /// outstation's encoder, with the default variation or the specific one from `header`
    fn encoding(self, specific: bool) -> (u8, u8, Vec<u8>) {
        const VALUE: [u8; 5] = [0x01, 7, 0, 0, 0];
        let (group, default, other, mut data) = match self {
            Kind::Binary => (2, 1, 2, vec![0x81]),
            Kind::DoubleBitBinary => (4, 1, 2, vec![0x81]),
            Kind::BinaryOutputStatus => (11, 1, 2, vec![0x81]),
            Kind::Counter => (22, 1, 5, VALUE.to_vec()),
            Kind::FrozenCounter => (23, 1, 5, VALUE.to_vec()),
            Kind::Analog => (32, 1, 3, VALUE.to_vec()),
            Kind::AnalogOutputStatus => (42, 1, 3, VALUE.to_vec()),
            // the variation is the length, and a READ can't choose another
            Kind::OctetString => return (111, 2, vec![0xAA, 0xBB]),
        };
        if specific {
            // each specific variation adds the 48-bit time
            data.extend_from_slice(&TIMESTAMP.to_le_bytes()[..6]);
            (group, other, data)
        } else {
            (group, default, data)
        }
    }

    fn max(self, config: &EventBufferConfig) -> usize {
        usize::from(match self {
            Kind::Binary => config.max_binary,
            Kind::DoubleBitBinary => config.max_double_binary,
            Kind::BinaryOutputStatus => config.max_binary_output_status,
            Kind::Counter => config.max_counter,
            Kind::FrozenCounter => config.max_frozen_counter,
            Kind::Analog => config.max_analog,
            Kind::AnalogOutputStatus => config.max_analog_output_status,
            Kind::OctetString => config.max_octet_string,
        })
    }

    fn insert(
        self,
        buffer: &mut EventBuffer,
        index: u16,
        class: EventClass,
    ) -> Result<u64, InsertError> {
        let time = Time::synchronized(TIMESTAMP);
        match self {
            Kind::Binary => buffer.insert(
                index,
                class,
                &BinaryInput::new(true, Flags::ONLINE, time),
                EventBinaryInputVariation::Group2Var1,
            ),
            Kind::DoubleBitBinary => buffer.insert(
                index,
                class,
                &DoubleBitBinaryInput::new(DoubleBit::DeterminedOn, Flags::ONLINE, time),
                EventDoubleBitBinaryInputVariation::Group4Var1,
            ),
            Kind::BinaryOutputStatus => buffer.insert(
                index,
                class,
                &BinaryOutputStatus::new(true, Flags::ONLINE, time),
                EventBinaryOutputStatusVariation::Group11Var1,
            ),
            Kind::Counter => buffer.insert(
                index,
                class,
                &Counter::new(7, Flags::ONLINE, time),
                EventCounterVariation::Group22Var1,
            ),
            Kind::FrozenCounter => buffer.insert(
                index,
                class,
                &FrozenCounter::new(7, Flags::ONLINE, time),
                EventFrozenCounterVariation::Group23Var1,
            ),
            Kind::Analog => buffer.insert(
                index,
                class,
                &AnalogInput::new(7.0, Flags::ONLINE, time),
                EventAnalogInputVariation::Group32Var1,
            ),
            Kind::AnalogOutputStatus => buffer.insert(
                index,
                class,
                &AnalogOutputStatus::new(7.0, Flags::ONLINE, time),
                EventAnalogOutputStatusVariation::Group42Var1,
            ),
            Kind::OctetString => {
                buffer.insert(index, class, &octet_string(), EventOctetStringVariation)
            }
        }
    }

    /// select the events of this type, in the default variation or a specific one
    fn select(
        self,
        buffer: &mut EventBuffer,
        specific: bool,
        limit: Option<usize>,
    ) -> (usize, Visits) {
        match self {
            Kind::Binary => buffer.select_by_type::<BinaryInput>(
                specific.then_some(EventBinaryInputVariation::Group2Var2),
                limit,
            ),
            Kind::DoubleBitBinary => buffer.select_by_type::<DoubleBitBinaryInput>(
                specific.then_some(EventDoubleBitBinaryInputVariation::Group4Var2),
                limit,
            ),
            Kind::BinaryOutputStatus => buffer.select_by_type::<BinaryOutputStatus>(
                specific.then_some(EventBinaryOutputStatusVariation::Group11Var2),
                limit,
            ),
            Kind::Counter => buffer.select_by_type::<Counter>(
                specific.then_some(EventCounterVariation::Group22Var5),
                limit,
            ),
            Kind::FrozenCounter => buffer.select_by_type::<FrozenCounter>(
                specific.then_some(EventFrozenCounterVariation::Group23Var5),
                limit,
            ),
            Kind::Analog => buffer.select_by_type::<AnalogInput>(
                specific.then_some(EventAnalogInputVariation::Group32Var3),
                limit,
            ),
            Kind::AnalogOutputStatus => buffer.select_by_type::<AnalogOutputStatus>(
                specific.then_some(EventAnalogOutputStatusVariation::Group42Var3),
                limit,
            ),
            Kind::OctetString => buffer.select_by_type::<OctetString>(None, limit),
        }
    }

    fn has_events(self, buffer: &EventBuffer) -> bool {
        match self {
            Kind::Binary => buffer.has_events::<BinaryInput>(),
            Kind::DoubleBitBinary => buffer.has_events::<DoubleBitBinaryInput>(),
            Kind::BinaryOutputStatus => buffer.has_events::<BinaryOutputStatus>(),
            Kind::Counter => buffer.has_events::<Counter>(),
            Kind::FrozenCounter => buffer.has_events::<FrozenCounter>(),
            Kind::Analog => buffer.has_events::<AnalogInput>(),
            Kind::AnalogOutputStatus => buffer.has_events::<AnalogOutputStatus>(),
            Kind::OctetString => buffer.has_events::<OctetString>(),
        }
    }

    fn count_mut(self, types: &mut TypeCount) -> &mut usize {
        match self {
            Kind::Binary => &mut types.num_binary_input,
            Kind::DoubleBitBinary => &mut types.num_double_bit_binary_input,
            Kind::BinaryOutputStatus => &mut types.num_binary_output_status,
            Kind::Counter => &mut types.num_counter,
            Kind::FrozenCounter => &mut types.num_frozen_counter,
            Kind::Analog => &mut types.num_analog,
            Kind::AnalogOutputStatus => &mut types.num_analog_output_status,
            Kind::OctetString => &mut types.num_octet_string,
        }
    }
}

fn class_count_mut(count: &mut ClassCount, class: EventClass) -> &mut usize {
    match class {
        EventClass::Class1 => &mut count.num_class_1,
        EventClass::Class2 => &mut count.num_class_2,
        EventClass::Class3 => &mut count.num_class_3,
    }
}

fn matches(classes: EventClasses, class: EventClass) -> bool {
    match class {
        EventClass::Class1 => classes.class1,
        EventClass::Class2 => classes.class2,
        EventClass::Class3 => classes.class3,
    }
}

fn no_types() -> TypeCount {
    TypeCount {
        num_binary_input: 0,
        num_double_bit_binary_input: 0,
        num_binary_output_status: 0,
        num_counter: 0,
        num_frozen_counter: 0,
        num_analog: 0,
        num_analog_output_status: 0,
        num_octet_string: 0,
    }
}

/// write the selected events into a cursor of `size` bytes, returning the result and the bytes
fn write_bytes(buffer: &mut EventBuffer, size: usize) -> ((usize, WriteProgress), Vec<u8>) {
    let mut backing = vec![0u8; size];
    let mut cursor = WriteCursor::new(backing.as_mut_slice());
    let result = buffer.write_events(&mut cursor);
    (result, cursor.written().to_vec())
}

/// write the selected events into a cursor of `size` bytes
fn write(buffer: &mut EventBuffer, size: usize) -> (usize, WriteProgress) {
    write_bytes(buffer, size).0
}

/// clear the written events, returning their ids in order
fn clear(buffer: &mut EventBuffer) -> Vec<u64> {
    let mut ids = Vec::new();
    let count = buffer.clear_written(|id| ids.push(id));
    assert_eq!(count, ids.len());
    ids
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum State {
    Unselected,
    Selected,
    Written,
}

#[derive(Debug)]
struct Record {
    id: u64,
    index: u16,
    kind: Kind,
    class: EventClass,
    state: State,
    // the specific variation from the READ that selected it, rather than the default
    specific: bool,
}

/// the buffer must behave like a list of records with per-type limits, in insertion order
struct Model {
    config: EventBufferConfig,
    records: Vec<Record>,
    overflown: bool,
    next: u64,
}

impl Model {
    fn new(config: EventBufferConfig) -> Self {
        Self {
            config,
            records: Vec::new(),
            overflown: false,
            next: 0,
        }
    }

    fn count(&self, kind: Kind) -> usize {
        self.records.iter().filter(|r| r.kind == kind).count()
    }

    fn is_any_full(&self) -> bool {
        KINDS.iter().any(|kind| {
            let max = kind.max(&self.config);
            max > 0 && self.count(*kind) >= max
        })
    }

    fn insert(&mut self, kind: Kind, index: u16, class: EventClass) -> Result<u64, InsertError> {
        let max = kind.max(&self.config);
        if max == 0 {
            return Err(InsertError::TypeMaxIsZero);
        }

        let id = self.next;
        self.next = self.next.wrapping_add(1);

        let result = if self.count(kind) == max {
            let position = self.records.iter().position(|r| r.kind == kind).unwrap();
            let discarded = remove_at(&mut self.records, position);
            self.overflown = true;
            Err(InsertError::Overflow {
                created: id,
                discarded: discarded.id,
            })
        } else {
            Ok(id)
        };

        self.records.push(Record {
            id,
            index,
            kind,
            class,
            state: State::Unselected,
            specific: false,
        });
        result
    }

    /// select the matching unselected records, returning how many and the records visited:
    /// every record, unless the limit is reached first, which stops at the record after the
    /// last one selected (or the first record, for a limit of 0)
    fn select<F>(&mut self, limit: Option<usize>, specific: bool, matches: F) -> (usize, Visits)
    where
        F: Fn(&Record) -> bool,
    {
        let limit = limit.unwrap_or(usize::MAX);
        let len = self.records.len();
        let mut count: usize = 0;
        let mut last = None;
        for (position, record) in self
            .records
            .iter_mut()
            .enumerate()
            .filter(|(_, r)| r.state == State::Unselected && matches(r))
            .take(limit)
        {
            record.state = State::Selected;
            record.specific = specific;
            count = count.saturating_add(1);
            last = Some(position);
        }
        let visits = if count < limit {
            len
        } else {
            match last {
                Some(last) => last.saturating_add(2).min(len),
                None => len.min(1),
            }
        };
        (count, Visits::new(visits))
    }

    /// write the selected records in order while they fit in `capacity` bytes, returning the
    /// expected result and bytes: a header (group, variation, qualifier 0x28 and a 2-byte count)
    /// for each run of the same group and variation, then a 2-byte index and data per event
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the model adds up a few small sizes"
    )]
    fn write(&mut self, capacity: usize) -> ((usize, WriteProgress), Vec<u8>) {
        let mut bytes: Vec<u8> = Vec::new();
        // the group, variation, count position and count of the current header
        let mut header: Option<(u8, u8, usize, u16)> = None;
        let mut count: usize = 0;
        for record in self
            .records
            .iter_mut()
            .filter(|r| r.state == State::Selected)
        {
            let (group, variation, data) = record.kind.encoding(record.specific);
            let header_size = match header {
                Some((g, v, _, _)) if (g, v) == (group, variation) => 0,
                _ => 5,
            };
            if bytes.len() + header_size + 2 + data.len() > capacity {
                return ((count, WriteProgress::Partial), bytes);
            }

            let (position, num) = match header {
                Some((_, _, position, num)) if header_size == 0 => (position, num + 1),
                _ => {
                    bytes.extend([group, variation, 0x28]);
                    let position = bytes.len();
                    bytes.extend([0, 0]);
                    (position, 1)
                }
            };
            for (dest, src) in bytes[position..position + 2]
                .iter_mut()
                .zip(num.to_le_bytes())
            {
                *dest = src;
            }
            header = Some((group, variation, position, num));

            bytes.extend(record.index.to_le_bytes());
            bytes.extend(data);
            record.state = State::Written;
            count += 1;
        }
        ((count, WriteProgress::Complete), bytes)
    }

    fn clear_written(&mut self) -> Vec<u64> {
        let cleared = self
            .records
            .iter()
            .filter(|r| r.state == State::Written)
            .map(|r| r.id)
            .collect();
        self.records.retain(|r| r.state != State::Written);
        if !self.is_any_full() {
            self.overflown = false;
        }
        cleared
    }

    fn discard(&mut self, classes: EventClasses) -> ClassCount {
        let mut count = ClassCount::default();
        self.records.retain(|r| {
            let discard = r.state == State::Unselected && matches(classes, r.class);
            if discard {
                let value = class_count_mut(&mut count, r.class);
                *value = value.saturating_add(1);
            }
            !discard
        });
        if !self.is_any_full() {
            self.overflown = false;
        }
        count
    }

    fn reset(&mut self) {
        for record in self.records.iter_mut() {
            record.state = State::Unselected;
        }
    }

    fn unwritten_classes(&self) -> EventClasses {
        let unwritten = |class| {
            self.records
                .iter()
                .any(|r| r.state != State::Written && r.class == class)
        };
        EventClasses::new(
            unwritten(EventClass::Class1),
            unwritten(EventClass::Class2),
            unwritten(EventClass::Class3),
        )
    }

    fn buffer_state(&self) -> BufferState {
        let mut state = BufferState {
            classes: ClassCount::default(),
            types: no_types(),
        };
        for record in self.records.iter() {
            let class = class_count_mut(&mut state.classes, record.class);
            *class = class.saturating_add(1);
            let kind = record.kind.count_mut(&mut state.types);
            *kind = kind.saturating_add(1);
        }
        state
    }

    fn check(&self, buffer: &EventBuffer) {
        assert_eq!(buffer.buffer_state(), self.buffer_state());
        assert_eq!(buffer.unwritten_classes(), self.unwritten_classes());
        assert_eq!(buffer.is_overflown(), self.overflown);
        for kind in KINDS {
            assert_eq!(kind.has_events(buffer), self.count(kind) > 0, "{kind:?}");
        }
    }

    fn apply(&mut self, buffer: &mut EventBuffer, op: Op) {
        match op {
            Op::Insert(kind, index, class) => {
                assert_eq!(
                    kind.insert(buffer, index, class),
                    self.insert(kind, index, class)
                );
            }
            Op::SelectClasses(classes) => {
                let selected = buffer.select_by_class(classes, None);
                assert_eq!(
                    selected,
                    self.select(None, false, |r| matches(classes, r.class))
                );
            }
            Op::SelectClass(class, limit) => {
                let selected = buffer.select_by_class(class.into(), limit);
                assert_eq!(selected, self.select(limit, false, |r| r.class == class));
            }
            Op::SelectType(kind, specific, limit) => {
                let selected = kind.select(buffer, specific, limit);
                assert_eq!(selected, self.select(limit, specific, |r| r.kind == kind));
            }
            Op::Write(size) => {
                assert_eq!(write_bytes(buffer, size), self.write(size));
            }
            Op::ClearWritten => {
                assert_eq!(clear(buffer), self.clear_written());
            }
            Op::Discard(classes) => {
                assert_eq!(
                    buffer.remove_unselected_by_class(classes),
                    self.discard(classes)
                );
            }
            Op::Reset => {
                buffer.reset();
                self.reset();
            }
        }
    }
}

#[derive(Clone, Debug)]
enum Op {
    Insert(Kind, u16, EventClass),
    // the unsolicited path
    SelectClasses(EventClasses),
    // a class READ header
    SelectClass(EventClass, Option<usize>),
    SelectType(Kind, bool, Option<usize>),
    Write(usize),
    ClearWritten,
    Discard(EventClasses),
    Reset,
}

fn kind() -> impl Strategy<Value = Kind> {
    (0usize..KINDS.len()).prop_map(|i| KINDS[i])
}

fn class() -> impl Strategy<Value = EventClass> {
    (0usize..CLASSES.len()).prop_map(|i| CLASSES[i])
}

fn classes() -> impl Strategy<Value = EventClasses> {
    (any::<bool>(), any::<bool>(), any::<bool>()).prop_map(|(a, b, c)| EventClasses::new(a, b, c))
}

fn limit() -> impl Strategy<Value = Option<usize>> {
    prop::option::of(0usize..4)
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        6 => (kind(), any::<u16>(), class()).prop_map(|(k, i, c)| Op::Insert(k, i, c)),
        1 => classes().prop_map(Op::SelectClasses),
        2 => (class(), limit()).prop_map(|(c, l)| Op::SelectClass(c, l)),
        2 => (kind(), any::<bool>(), limit()).prop_map(|(k, s, l)| Op::SelectType(k, s, l)),
        // small cursors write part of the selection
        3 => (0usize..48).prop_map(Op::Write),
        1 => Just(Op::Write(4096)),
        2 => Just(Op::ClearWritten),
        1 => classes().prop_map(Op::Discard),
        1 => Just(Op::Reset),
    ]
}

fn config() -> impl Strategy<Value = EventBufferConfig> {
    prop::collection::vec(0u16..4, KINDS.len())
        .prop_map(|m| EventBufferConfig::new(m[0], m[1], m[2], m[3], m[4], m[5], m[6], m[7]))
}

proptest! {
    #![proptest_config(crate::util::test_support::proptest_config(256))]

    #[test]
    fn behaves_like_a_list_with_per_type_limits(
        config in config(),
        ops in prop::collection::vec(op(), 0..96),
    ) {
        let mut buffer = EventBuffer::new(config);
        let mut model = Model::new(config);
        model.check(&buffer);
        for op in ops {
            model.apply(&mut buffer, op);
            model.check(&buffer);
        }

        // whatever is left is reselected and delivered after a reset, in insertion order
        for op in [
            Op::Reset,
            Op::SelectClasses(EventClasses::all()),
            Op::Write(4096),
            Op::ClearWritten,
        ] {
            model.apply(&mut buffer, op);
            model.check(&buffer);
        }
        assert!(model.records.is_empty());
    }
}

/// A type READ that selects one binary event in the default variation and the next one in g2v2
/// writes a new header of the same group for the second, each event with its index and the second
/// with its time, and the confirm clears both in order. The property reaches a change of variation
/// within a group in only some runs.
#[test]
fn a_type_read_changes_the_variation_within_a_group() {
    let config = EventBufferConfig::all_types(3);
    let mut buffer = EventBuffer::new(config);
    let mut model = Model::new(config);
    model.check(&buffer);
    for op in [
        Op::Insert(Kind::Binary, 1, EventClass::Class1),
        Op::Insert(Kind::Counter, 2, EventClass::Class2),
        Op::Insert(Kind::DoubleBitBinary, 3, EventClass::Class3),
        Op::Insert(Kind::Binary, 4, EventClass::Class2),
        Op::Insert(Kind::Analog, 5, EventClass::Class1),
        // the first binary event in the default variation, then the other one in g2v2
        Op::SelectType(Kind::Binary, false, Some(1)),
        Op::SelectType(Kind::Binary, true, None),
    ] {
        model.apply(&mut buffer, op);
        model.check(&buffer);
    }

    let written = write_bytes(&mut buffer, 64);
    assert_eq!(
        written,
        (
            (2, WriteProgress::Complete),
            vec![
                // g2v1, count 1, index 1
                2, 1, 0x28, 0x01, 0x00, 0x01, 0x00, 0x81,
                // g2v2, count 1, index 4, with the 48-bit time
                2, 2, 0x28, 0x01, 0x00, 0x04, 0x00, 0x81, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06,
            ]
        )
    );
    assert_eq!(model.write(64), written);
    model.check(&buffer);

    let cleared = clear(&mut buffer);
    assert_eq!(cleared, [0, 3]);
    assert_eq!(model.clear_written(), cleared);
    model.check(&buffer);
}

/// Every type full at once, which fills the list that holds the records, then one more event of
/// each type, which must evict the oldest event of that type to make room. The property fills
/// every type at once in only some runs.
#[test]
fn each_type_holds_exactly_its_configured_maximum() {
    let config = EventBufferConfig::new(1, 2, 3, 0, 1, 2, 3, 2);
    let mut buffer = EventBuffer::new(config);
    let mut model = Model::new(config);
    model.check(&buffer);

    let fill = KINDS.into_iter().flat_map(|kind| {
        std::iter::repeat_n(Op::Insert(kind, 0, EventClass::Class1), kind.max(&config))
    });
    let one_more = KINDS
        .into_iter()
        .map(|kind| Op::Insert(kind, 0, EventClass::Class2));
    let drain = [
        Op::Reset,
        Op::SelectClasses(EventClasses::all()),
        Op::Write(4096),
        Op::ClearWritten,
    ];
    for op in fill.chain(one_more).chain(drain) {
        model.apply(&mut buffer, op);
        model.check(&buffer);
    }
    assert!(model.records.is_empty());
}

#[test]
fn event_classes_combine_and_class_counts_are_empty_only_at_zero() {
    assert_eq!(
        EventClass::Class1 | EventClass::Class3,
        EventClasses::new(true, false, true)
    );
    assert_eq!(
        EventClasses::new(true, false, false) | EventClasses::new(false, true, true),
        EventClasses::all()
    );

    assert!(ClassCount::default().is_empty());
    for class in CLASSES {
        let mut count = ClassCount::default();
        *class_count_mut(&mut count, class) = 1;
        assert!(!count.is_empty());
    }
}

/// Class reads written into small fragments, one of them led by a common time object: the
/// property doesn't generate CTO variations
#[test]
fn can_select_events_by_class_and_write_some() {
    let mut buffer = EventBuffer::new(EventBufferConfig::all_types(3));
    let time = Time::synchronized(0);
    assert_eq!(
        buffer.insert(
            1,
            EventClass::Class1,
            &BinaryInput::new(true, Flags::ONLINE, time),
            EventBinaryInputVariation::Group2Var1,
        ),
        Ok(0)
    );
    assert_eq!(
        buffer.insert(
            2,
            EventClass::Class2,
            &Counter::new(23, Flags::ONLINE, time),
            EventCounterVariation::Group22Var2,
        ),
        Ok(1)
    );
    assert_eq!(
        buffer.insert(
            3,
            EventClass::Class3,
            &DoubleBitBinaryInput::new(DoubleBit::DeterminedOn, Flags::ONLINE, time),
            EventDoubleBitBinaryInputVariation::Group4Var3,
        ),
        Ok(2)
    );
    assert_eq!(
        buffer.insert(
            4,
            EventClass::Class2,
            &BinaryInput::new(true, Flags::ONLINE, Time::synchronized(1234)),
            EventBinaryInputVariation::Group2Var1,
        ),
        Ok(3)
    );
    assert_eq!(
        buffer.insert(
            5,
            EventClass::Class1,
            &AnalogInput::new(42.0, Flags::ONLINE, time),
            EventAnalogInputVariation::Group32Var1,
        ),
        Ok(4)
    );

    // the class 2 events stay unselected
    assert_eq!(
        buffer.select_by_class(EventClass::Class1 | EventClass::Class3, None),
        (3, Visits::new(5))
    );

    // the g2v1 event fits, but not the CTO, header and g4v3 event after it
    assert_eq!(write(&mut buffer, 24), (1, WriteProgress::Partial));
    assert_eq!(buffer.unwritten_classes(), EventClasses::all());
    assert_eq!(clear(&mut buffer), [0]);
    assert_eq!(buffer.unwritten_classes(), EventClasses::all());

    // the CTO and the g4v3 event fit, but not the g32v1 event after them
    assert_eq!(write(&mut buffer, 24), (1, WriteProgress::Partial));
    assert_eq!(
        buffer.unwritten_classes(),
        EventClass::Class1 | EventClass::Class2
    );
    assert_eq!(clear(&mut buffer), [2]);
    assert_eq!(
        buffer.unwritten_classes(),
        EventClass::Class1 | EventClass::Class2
    );
}
