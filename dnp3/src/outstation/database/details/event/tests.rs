use proptest::prelude::*;
use scursor::WriteCursor;

use crate::app::measurement::*;
use crate::master::EventClasses;
use crate::outstation::database::config::*;
use crate::outstation::database::details::WriteProgress;
use crate::outstation::database::read::EventReadHeader;
use crate::outstation::database::{EventBufferConfig, EventClass};
use crate::outstation::{BufferState, ClassCount, OutstationApplication, TypeCount};

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

    /// a READ header for this type, with the default variation or a specific one
    fn header(self, specific: bool, limit: Option<usize>) -> EventReadHeader {
        match self {
            Kind::Binary => EventReadHeader::Binary(
                specific.then_some(EventBinaryInputVariation::Group2Var2),
                limit,
            ),
            Kind::DoubleBitBinary => EventReadHeader::DoubleBitBinary(
                specific.then_some(EventDoubleBitBinaryInputVariation::Group4Var2),
                limit,
            ),
            Kind::BinaryOutputStatus => EventReadHeader::BinaryOutputStatus(
                specific.then_some(EventBinaryOutputStatusVariation::Group11Var2),
                limit,
            ),
            Kind::Counter => EventReadHeader::Counter(
                specific.then_some(EventCounterVariation::Group22Var5),
                limit,
            ),
            Kind::FrozenCounter => EventReadHeader::FrozenCounter(
                specific.then_some(EventFrozenCounterVariation::Group23Var5),
                limit,
            ),
            Kind::Analog => EventReadHeader::Analog(
                specific.then_some(EventAnalogInputVariation::Group32Var3),
                limit,
            ),
            Kind::AnalogOutputStatus => EventReadHeader::AnalogOutputStatus(
                specific.then_some(EventAnalogOutputStatusVariation::Group42Var3),
                limit,
            ),
            Kind::OctetString => EventReadHeader::OctetString(limit),
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

fn class_counts(num_class_1: usize, num_class_2: usize, num_class_3: usize) -> ClassCount {
    ClassCount {
        num_class_1,
        num_class_2,
        num_class_3,
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

fn types(counts: &[(Kind, usize)]) -> TypeCount {
    let mut types = no_types();
    for (kind, count) in counts {
        *kind.count_mut(&mut types) = *count;
    }
    types
}

/// records the ids of cleared events in order
#[derive(Default)]
struct Cleared {
    ids: Vec<u64>,
}

impl OutstationApplication for Cleared {
    fn event_cleared(&mut self, id: u64) {
        self.ids.push(id);
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
    let mut app = Cleared::default();
    let count = buffer.clear_written(&mut app);
    assert_eq!(count, app.ids.len());
    app.ids
}

/// re-arm, select, write and clear everything, returning the ids of what was left in order
fn drain(buffer: &mut EventBuffer) -> Vec<u64> {
    buffer.reset();
    let selected = buffer.select_by_class(EventClasses::all(), None);
    assert_eq!(write(buffer, 4096), (selected, WriteProgress::Complete));
    clear(buffer)
}

fn binary(buffer: &mut EventBuffer, class: EventClass) -> Result<u64, InsertError> {
    Kind::Binary.insert(buffer, 0, class)
}

#[test]
fn overflow_discards_the_oldest_event_of_the_full_type_only() {
    let mut config = EventBufferConfig::no_events();
    config.max_binary = 2;
    config.max_counter = 1;
    let mut buffer = EventBuffer::new(config);

    assert_eq!(binary(&mut buffer, EventClass::Class1), Ok(0));
    assert_eq!(
        Kind::Counter.insert(&mut buffer, 0, EventClass::Class2),
        Ok(1)
    );
    assert_eq!(binary(&mut buffer, EventClass::Class3), Ok(2));
    assert!(!buffer.is_overflown());

    assert_eq!(
        binary(&mut buffer, EventClass::Class1),
        Err(InsertError::Overflow {
            created: 3,
            discarded: 0
        })
    );
    assert!(buffer.is_overflown());
    assert_eq!(
        buffer.buffer_state(),
        BufferState {
            classes: class_counts(1, 1, 1),
            types: types(&[(Kind::Binary, 2), (Kind::Counter, 1)]),
        }
    );

    // a type with no space doesn't consume an id
    assert_eq!(
        Kind::Analog.insert(&mut buffer, 0, EventClass::Class1),
        Err(InsertError::TypeMaxIsZero)
    );
    assert_eq!(
        binary(&mut buffer, EventClass::Class2),
        Err(InsertError::Overflow {
            created: 4,
            discarded: 2
        })
    );
    assert_eq!(
        buffer.buffer_state(),
        BufferState {
            classes: class_counts(1, 2, 0),
            types: types(&[(Kind::Binary, 2), (Kind::Counter, 1)]),
        }
    );

    // the survivors keep their insertion order
    assert_eq!(drain(&mut buffer), [1, 3, 4]);
    assert!(!buffer.is_overflown());
    assert_eq!(
        buffer.buffer_state(),
        BufferState {
            classes: class_counts(0, 0, 0),
            types: no_types(),
        }
    );
}

#[test]
fn overflow_persists_until_no_type_is_full() {
    let mut config = EventBufferConfig::no_events();
    config.max_binary = 2;
    config.max_counter = 1;
    let mut buffer = EventBuffer::new(config);

    binary(&mut buffer, EventClass::Class1).unwrap();
    binary(&mut buffer, EventClass::Class1).unwrap();
    binary(&mut buffer, EventClass::Class1).unwrap_err();
    Kind::Counter
        .insert(&mut buffer, 0, EventClass::Class2)
        .unwrap();
    assert!(buffer.is_overflown());

    // confirming the binary events leaves the counter type full
    assert_eq!(buffer.select_by_header(Kind::Binary.header(false, None)), 2);
    assert_eq!(write(&mut buffer, 4096), (2, WriteProgress::Complete));
    assert!(buffer.is_overflown());
    assert_eq!(clear(&mut buffer), [1, 2]);
    assert!(buffer.is_overflown());

    // an empty confirm changes nothing
    assert!(clear(&mut buffer).is_empty());
    assert!(buffer.is_overflown());

    // discarding the counter frees its type
    assert_eq!(
        buffer.remove_unselected_by_class(EventClass::Class2.into()),
        class_counts(0, 1, 0)
    );
    assert!(!buffer.is_overflown());

    // a type that isn't full doesn't overflow again
    binary(&mut buffer, EventClass::Class1).unwrap();
    assert!(!buffer.is_overflown());
}

#[test]
fn confirm_clears_exactly_the_written_events() {
    let mut buffer = EventBuffer::new(EventBufferConfig::all_types(5));

    binary(&mut buffer, EventClass::Class1).unwrap(); // 0
    Kind::Counter
        .insert(&mut buffer, 1, EventClass::Class2)
        .unwrap(); // 1
    binary(&mut buffer, EventClass::Class3).unwrap(); // 2
    Kind::Analog
        .insert(&mut buffer, 2, EventClass::Class1)
        .unwrap(); // 3
    binary(&mut buffer, EventClass::Class1).unwrap(); // 4

    assert_eq!(
        buffer.select_by_class(EventClass::Class1 | EventClass::Class3, None),
        4
    );

    // room for the first two binary events only, which share a header
    assert_eq!(write(&mut buffer, 14), (2, WriteProgress::Partial));
    assert_eq!(
        buffer.unwritten_classes(),
        EventClasses::new(true, true, false)
    );
    assert_eq!(clear(&mut buffer), [0, 2]);
    assert_eq!(
        buffer.buffer_state(),
        BufferState {
            classes: class_counts(2, 1, 0),
            types: types(&[(Kind::Binary, 1), (Kind::Counter, 1), (Kind::Analog, 1)]),
        }
    );

    // the still-selected events can't be selected again, and are written next
    assert_eq!(buffer.select_by_class(EventClasses::all(), None), 1);
    assert_eq!(write(&mut buffer, 4096), (3, WriteProgress::Complete));
    assert_eq!(buffer.unwritten_classes(), EventClasses::none());
    assert_eq!(clear(&mut buffer), [1, 3, 4]);
    assert_eq!(
        buffer.buffer_state(),
        BufferState {
            classes: class_counts(0, 0, 0),
            types: no_types(),
        }
    );
}

#[test]
fn written_counts_accumulate_across_fragments_until_the_confirm() {
    let mut buffer = EventBuffer::new(EventBufferConfig::all_types(5));

    for class in [EventClass::Class1, EventClass::Class2, EventClass::Class3] {
        binary(&mut buffer, class).unwrap();
    }
    Kind::Counter
        .insert(&mut buffer, 0, EventClass::Class1)
        .unwrap();

    assert_eq!(buffer.select_by_class(EventClasses::all(), None), 4);

    // one binary event per fragment, each written without a confirm in between
    assert_eq!(write(&mut buffer, 10), (1, WriteProgress::Partial));
    assert_eq!(buffer.unwritten_classes(), EventClasses::all());
    assert_eq!(write(&mut buffer, 10), (1, WriteProgress::Partial));
    assert_eq!(
        buffer.unwritten_classes(),
        EventClasses::new(true, false, true)
    );
    assert_eq!(write(&mut buffer, 10), (1, WriteProgress::Partial));
    assert_eq!(
        buffer.unwritten_classes(),
        EventClasses::new(true, false, false)
    );
    assert_eq!(write(&mut buffer, 0), (0, WriteProgress::Partial));
    assert_eq!(write(&mut buffer, 4096), (1, WriteProgress::Complete));
    assert_eq!(buffer.unwritten_classes(), EventClasses::none());
    assert_eq!(write(&mut buffer, 4096), (0, WriteProgress::Complete));

    assert_eq!(clear(&mut buffer), [0, 1, 2, 3]);
    assert_eq!(buffer.unwritten_classes(), EventClasses::none());
    assert_eq!(buffer.buffer_state().classes, class_counts(0, 0, 0));
}

#[test]
fn evicting_a_written_event_leaves_the_other_written_events_to_the_confirm() {
    let mut config = EventBufferConfig::no_events();
    config.max_binary = 2;
    let mut buffer = EventBuffer::new(config);

    binary(&mut buffer, EventClass::Class1).unwrap();
    binary(&mut buffer, EventClass::Class2).unwrap();
    assert_eq!(buffer.select_by_class(EventClasses::all(), None), 2);
    assert_eq!(write(&mut buffer, 4096), (2, WriteProgress::Complete));
    assert_eq!(buffer.unwritten_classes(), EventClasses::none());

    // the oldest written event is discarded before the confirm
    assert_eq!(
        binary(&mut buffer, EventClass::Class3),
        Err(InsertError::Overflow {
            created: 2,
            discarded: 0
        })
    );
    assert_eq!(buffer.unwritten_classes(), EventClass::Class3.into());
    assert_eq!(
        buffer.buffer_state(),
        BufferState {
            classes: class_counts(0, 1, 1),
            types: types(&[(Kind::Binary, 2)]),
        }
    );

    // the confirm clears the remaining written event, and the new one stays unwritten
    assert_eq!(clear(&mut buffer), [1]);
    assert!(!buffer.is_overflown());
    assert_eq!(buffer.unwritten_classes(), EventClass::Class3.into());
    assert_eq!(
        buffer.buffer_state(),
        BufferState {
            classes: class_counts(0, 0, 1),
            types: types(&[(Kind::Binary, 1)]),
        }
    );
    assert_eq!(drain(&mut buffer), [2]);
}

#[test]
fn evicting_every_written_event_leaves_nothing_to_confirm() {
    let mut config = EventBufferConfig::no_events();
    config.max_binary = 2;
    let mut buffer = EventBuffer::new(config);

    binary(&mut buffer, EventClass::Class1).unwrap();
    binary(&mut buffer, EventClass::Class1).unwrap();
    assert_eq!(buffer.select_by_class(EventClasses::all(), None), 2);
    assert_eq!(write(&mut buffer, 4096), (2, WriteProgress::Complete));

    binary(&mut buffer, EventClass::Class2).unwrap_err();
    binary(&mut buffer, EventClass::Class2).unwrap_err();
    assert_eq!(buffer.unwritten_classes(), EventClass::Class2.into());
    assert_eq!(buffer.buffer_state().classes, class_counts(0, 2, 0));

    assert!(clear(&mut buffer).is_empty());
    // both binary slots are still occupied
    assert!(buffer.is_overflown());
    assert_eq!(buffer.unwritten_classes(), EventClass::Class2.into());
    assert_eq!(drain(&mut buffer), [2, 3]);
}

#[test]
fn reset_rearms_selected_and_written_events_after_an_unconfirmed_series() {
    let mut buffer = EventBuffer::new(EventBufferConfig::all_types(5));

    binary(&mut buffer, EventClass::Class1).unwrap();
    Kind::Counter
        .insert(&mut buffer, 0, EventClass::Class2)
        .unwrap();
    binary(&mut buffer, EventClass::Class1).unwrap();

    // an unsolicited series writes some of the events and is never confirmed
    assert_eq!(buffer.select_by_class(EventClasses::all(), None), 3);
    assert_eq!(write(&mut buffer, 10), (1, WriteProgress::Partial));
    assert_eq!(
        buffer.unwritten_classes(),
        EventClasses::new(true, true, false)
    );

    buffer.reset();
    assert_eq!(
        buffer.unwritten_classes(),
        EventClasses::new(true, true, false)
    );
    // nothing is written any more, so a confirm clears nothing
    assert!(clear(&mut buffer).is_empty());
    assert_eq!(buffer.buffer_state().classes, class_counts(2, 1, 0));

    // a READ of class 1 reselects both class 1 events, including the written one
    assert_eq!(buffer.select_by_class(EventClass::Class1.into(), None), 2);
    assert_eq!(write(&mut buffer, 4096), (2, WriteProgress::Complete));
    assert_eq!(buffer.unwritten_classes(), EventClass::Class2.into());
    assert_eq!(clear(&mut buffer), [0, 2]);
    assert_eq!(drain(&mut buffer), [1]);
}

#[test]
fn reset_after_a_partial_write_and_new_events_keeps_insertion_order() {
    let mut buffer = EventBuffer::new(EventBufferConfig::all_types(5));

    binary(&mut buffer, EventClass::Class1).unwrap();
    binary(&mut buffer, EventClass::Class1).unwrap();
    assert_eq!(buffer.select_by_class(EventClasses::all(), None), 2);
    assert_eq!(write(&mut buffer, 10), (1, WriteProgress::Partial));

    binary(&mut buffer, EventClass::Class1).unwrap();
    buffer.reset();

    // the limit takes the oldest events
    assert_eq!(buffer.select_by_header(EventReadHeader::Class1(Some(2))), 2);
    assert_eq!(write(&mut buffer, 4096), (2, WriteProgress::Complete));
    assert_eq!(clear(&mut buffer), [0, 1]);
    assert_eq!(drain(&mut buffer), [2]);
}

#[test]
fn each_type_holds_exactly_its_configured_maximum() {
    let config = EventBufferConfig::new(1, 2, 3, 0, 1, 2, 3, 2);
    let mut buffer = EventBuffer::new(config);

    let mut expected = no_types();
    let mut next: u64 = 0;
    // fill every type, leaving the whole buffer full
    for kind in KINDS {
        for _ in 0..kind.max(&config) {
            assert_eq!(kind.insert(&mut buffer, 0, EventClass::Class1), Ok(next));
            next = next.wrapping_add(1);
        }
        *kind.count_mut(&mut expected) = kind.max(&config);
        assert!(!buffer.is_overflown());
    }
    assert_eq!(buffer.buffer_state().types, expected);
    assert!(!Kind::Counter.has_events(&buffer));

    // one more of each type replaces that type's oldest event
    let mut kept: Vec<u64> = Vec::new();
    let mut replacements: Vec<u64> = Vec::new();
    let mut oldest: u64 = 0;
    for kind in KINDS {
        let max = u64::try_from(kind.max(&config)).unwrap();
        let result = kind.insert(&mut buffer, 0, EventClass::Class2);
        if max == 0 {
            assert_eq!(result, Err(InsertError::TypeMaxIsZero));
            continue;
        }
        assert_eq!(
            result,
            Err(InsertError::Overflow {
                created: next,
                discarded: oldest,
            })
        );
        assert!(buffer.is_overflown());
        kept.extend((1..max).map(|offset| oldest.wrapping_add(offset)));
        replacements.push(next);
        next = next.wrapping_add(1);
        oldest = oldest.wrapping_add(max);
    }
    assert_eq!(buffer.buffer_state().types, expected);

    // the events still in place keep their order, followed by the replacements
    kept.extend(replacements);
    assert_eq!(drain(&mut buffer), kept);
    assert!(!buffer.is_overflown());
}

#[test]
fn type_reads_select_only_their_type_with_limits() {
    let mut buffer = EventBuffer::new(EventBufferConfig::all_types(5));

    binary(&mut buffer, EventClass::Class1).unwrap(); // 0
    Kind::Analog
        .insert(&mut buffer, 0, EventClass::Class1)
        .unwrap(); // 1
    binary(&mut buffer, EventClass::Class2).unwrap(); // 2
    binary(&mut buffer, EventClass::Class3).unwrap(); // 3

    assert_eq!(
        buffer.select_by_header(Kind::Binary.header(true, Some(2))),
        2
    );
    // the analog event and the third binary event are still unselected
    assert_eq!(
        buffer.select_by_header(Kind::Counter.header(false, None)),
        0
    );
    assert_eq!(write(&mut buffer, 4096), (2, WriteProgress::Complete));
    assert_eq!(
        buffer.unwritten_classes(),
        EventClasses::new(true, false, true)
    );
    assert_eq!(clear(&mut buffer), [0, 2]);

    assert_eq!(buffer.select_by_header(Kind::Binary.header(false, None)), 1);
    assert_eq!(buffer.select_by_header(Kind::Analog.header(false, None)), 1);
    assert_eq!(write(&mut buffer, 4096), (2, WriteProgress::Complete));
    assert_eq!(clear(&mut buffer), [1, 3]);
}

#[test]
fn discard_updates_class_and_type_counts() {
    let mut buffer = EventBuffer::new(EventBufferConfig::all_types(5));

    binary(&mut buffer, EventClass::Class1).unwrap(); // 0
    Kind::Counter
        .insert(&mut buffer, 0, EventClass::Class2)
        .unwrap(); // 1
    Kind::OctetString
        .insert(&mut buffer, 0, EventClass::Class2)
        .unwrap(); // 2
    binary(&mut buffer, EventClass::Class3).unwrap(); // 3

    // the first binary event is in flight
    assert_eq!(buffer.select_by_class(EventClass::Class1.into(), None), 1);

    assert_eq!(
        buffer.remove_unselected_by_class(EventClass::Class1 | EventClass::Class2),
        class_counts(0, 2, 0)
    );
    assert_eq!(
        buffer.buffer_state(),
        BufferState {
            classes: class_counts(1, 0, 1),
            types: types(&[(Kind::Binary, 2)]),
        }
    );
    assert!(!Kind::Counter.has_events(&buffer));
    assert!(!Kind::OctetString.has_events(&buffer));
    assert!(Kind::Binary.has_events(&buffer));
    assert_eq!(
        buffer.unwritten_classes(),
        EventClass::Class1 | EventClass::Class3
    );

    assert_eq!(drain(&mut buffer), [0, 3]);
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
            let discarded = self.records.remove(position);
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

    fn select<F>(&mut self, limit: Option<usize>, specific: bool, matches: F) -> usize
    where
        F: Fn(&Record) -> bool,
    {
        let mut count: usize = 0;
        for record in self
            .records
            .iter_mut()
            .filter(|r| r.state == State::Unselected && matches(r))
            .take(limit.unwrap_or(usize::MAX))
        {
            record.state = State::Selected;
            record.specific = specific;
            count = count.saturating_add(1);
        }
        count
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
            bytes[position..position + 2].copy_from_slice(&num.to_le_bytes());
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
                let header = match class {
                    EventClass::Class1 => EventReadHeader::Class1(limit),
                    EventClass::Class2 => EventReadHeader::Class2(limit),
                    EventClass::Class3 => EventReadHeader::Class3(limit),
                };
                let selected = buffer.select_by_header(header);
                assert_eq!(selected, self.select(limit, false, |r| r.class == class));
            }
            Op::SelectType(kind, specific, limit) => {
                let selected = buffer.select_by_header(kind.header(specific, limit));
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
