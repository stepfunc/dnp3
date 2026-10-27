// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Links the outstation's event buffer, and its selection and writing of READ responses and
//! unsolicited responses, with a database, requests and output buffers the compiler can't know.
//!
//! The database is built from the input outside the checked functions: points of every type,
//! static and event variation and class, and updates with any value, flags, time and event
//! mode. Any sequence of the operations below follows, so each one sees the state the earlier
//! ones left: events inserted until their type overflows, events still selected for a
//! continuation fragment, and confirmed or discarded events removed between fragments. Output
//! buffers are input-sized from 0 bytes to 16 MiB. Diagnostics go to `Quiet`, which hands their
//! arguments to `black_box`, so the check covers what the diagnostics reached by these wrappers
//! report but not `tracing`. The check proves only that no panic is reachable: the existing
//! tests pin the bytes and the selection.
//!
//! Checked, and what each covers:
//!
//! - [`select`]: `ReadHeaders` over any parsed request (the mapping of each header to what it
//!   selects, and the error and summary for the ones READ doesn't support), the reset, and event
//!   selection by class and by type with any count, with its IIN2 bits. It dispatches the event
//!   headers as `Database::select_read` does
//! - [`respond`]: the event writers for solicited responses, every variation, with and without
//!   CTO, and the progress that decides whether a fragment continues
//! - [`unsolicited`]: `select_unsolicited` and `write_events_only`
//! - [`insert`]: `EventBuffer::insert` for every fixed-size type and `insert_event` for octet
//!   strings, whose bytes are allocated outside the check: the per-type maximum, eviction of the
//!   oldest event of a full type, and the event list's slots
//! - [`confirm`]: `clear_written`, which removes the written events after a confirm and passes
//!   each id on, and recomputes the overflow
//! - [`discard`]: `remove_unselected_by_class`
//! - [`static_points`]: `write_points` for every static type and `write_dead_bands`, with any
//!   points, default and requested variations, and range: the choice and promotion of each
//!   point's variation, every static encoder, packing of bits and double bits, and the range to
//!   resume at when a point doesn't fit. A continuation is another call with the rest of the
//!   range, which the input ranges already include
//! - [`attributes`]: `write_attr_list` (g0v255) for any set and list, and `write_attribute` for
//!   any attribute, each written whole or rolled back
//! - [`defer`]: `DeferredRead::set`, which stores what a READ selects while the outstation waits
//!   for a confirm, and logs the headers that don't fit
//!
//! Not checked: anything that walks or searches a `BTreeMap`, which holds the static points and
//! the attributes. Its navigation steps through checked unwraps that rely on the tree's
//! structure (alloc's btree `navigate.rs` and `search.rs`), which the optimizer removes or not
//! depending on inlining, so a proof can't rest on it; see #846. That leaves out static
//! selection (class 0 and static headers, including snapshot copying, class 0 configuration and
//! the push of each selected range), `StaticDatabase::write`'s walk and its loop over the
//! selected ranges, attribute selection and the attribute writer's loop over the selection.
//! Also not checked: `select_read`, `write_response_headers` and `DeferredRead::select` as
//! wholes, since they reach those maps, and `Database::update`, which finds the point in its map
//! before inserting the event; allocating an octet string event's bytes; the database mutex and
//! its notification; the session's assembly of READ responses and its logging; clocks; the
//! application, information and control callbacks; the transport and I/O; and the `Tracing`
//! log.

use std::hint::black_box;

use scursor::WriteCursor;

use crate::app::attr::{AttrItem, AttrProp, AttrSet, BadAttribute, OwnedAttribute};
use crate::app::measurement::*;
use crate::app::parse::options::ParseOptions;
use crate::app::parse::parser::{HeaderCollection, ParsedFragment};
use crate::app::{Iin2, QualifierCode, Sequence, Variation};
use crate::link::EndpointAddress;
use crate::master::EventClasses;
use crate::outstation::database::details::attrs::{write_attr_list, write_attribute};
use crate::outstation::database::details::event::buffer::Event;
use crate::outstation::database::details::family::Family;
use crate::outstation::database::details::range::static_db::{
    write_dead_bands, write_points, IndexRange, VariationRange,
};
use crate::outstation::database::details::WriteProgress;
use crate::outstation::database::read::{ReadHeader, ReadHeaders};
use crate::outstation::database::*;
use crate::outstation::read_log::ReadLog;
use crate::outstation::session::deferred::DeferredRead;
use crate::phys::PhysAddr;
use crate::transport::{FragmentAddr, FragmentInfo};
use crate::util::skipped::Count;

use super::Source;

/// Hands each diagnostic's arguments to `black_box` instead of logging them
#[derive(Copy, Clone)]
struct Quiet;

impl ReadLog for Quiet {
    fn unsupported_headers(self, header: (Variation, QualifierCode), count: Count) {
        black_box((header, count));
    }

    fn attr_selection_full(self, max: usize) {
        black_box(max);
    }

    fn attr_variation_with_all_objects(self, variation: u8) {
        black_box(variation);
    }

    fn attr_range_unsupported(self, range: IndexRange) {
        black_box(range);
    }

    fn attr_not_encodable(self, err: BadAttribute) {
        black_box(err);
    }

    fn deferred_capacity(self, max: usize, header: (Variation, QualifierCode), count: Count) {
        black_box((max, header, count));
    }
}

#[no_panic::no_panic]
fn select(db: &mut Database, headers: &HeaderCollection) -> Iin2 {
    db.inner.reset();
    let mut headers = ReadHeaders::new(headers);
    let mut iin2 = Iin2::default();
    for (header, _read_with) in headers.by_ref() {
        if let ReadHeader::Event(target, limit) = header {
            iin2 |= db.inner.select_events(target, limit);
        }
    }
    iin2 | headers.finish(Quiet)
}

#[no_panic::no_panic]
fn respond(db: &mut Database, buffer: &mut [u8]) {
    let mut cursor = WriteCursor::new(buffer);
    let (num_events, progress) = db.inner.write_events(&mut cursor);
    black_box((num_events, progress == WriteProgress::Complete));
    black_box(cursor.written());
}

#[no_panic::no_panic]
fn attributes(set: AttrSet, items: &[AttrItem], attr: &OwnedAttribute, buffer: &mut [u8]) {
    let mut cursor = WriteCursor::new(buffer);
    let list = write_attr_list(set, &mut cursor, items.iter().copied());
    let attr = write_attribute(&mut cursor, attr, Quiet);
    black_box((
        list == WriteProgress::Complete,
        attr == WriteProgress::Complete,
    ));
    black_box(cursor.written());
}

/// The points of one type, each with its index, selected value and default variation, and the
/// variation requested for all of them, if any
enum Points {
    Binary(
        Vec<(u16, BinaryInput, StaticBinaryInputVariation)>,
        Option<StaticBinaryInputVariation>,
    ),
    DoubleBitBinary(
        Vec<(
            u16,
            DoubleBitBinaryInput,
            StaticDoubleBitBinaryInputVariation,
        )>,
        Option<StaticDoubleBitBinaryInputVariation>,
    ),
    BinaryOutputStatus(
        Vec<(u16, BinaryOutputStatus, StaticBinaryOutputStatusVariation)>,
        Option<StaticBinaryOutputStatusVariation>,
    ),
    Counter(
        Vec<(u16, Counter, StaticCounterVariation)>,
        Option<StaticCounterVariation>,
    ),
    FrozenCounter(
        Vec<(u16, FrozenCounter, StaticFrozenCounterVariation)>,
        Option<StaticFrozenCounterVariation>,
    ),
    Analog(
        Vec<(u16, AnalogInput, StaticAnalogInputVariation)>,
        Option<StaticAnalogInputVariation>,
    ),
    AnalogOutputStatus(
        Vec<(u16, AnalogOutputStatus, StaticAnalogOutputStatusVariation)>,
        Option<StaticAnalogOutputStatusVariation>,
    ),
    OctetString(Vec<(u16, OctetString, StaticOctetStringVariation)>),
    DeadBand(Vec<(u16, f64)>, Option<AnalogInputDeadBandVariation>),
}

fn write<T: Family>(
    cursor: &mut WriteCursor,
    range: IndexRange,
    variation: Option<T::StaticVariation>,
    points: &[(u16, T, T::StaticVariation)],
) -> Result<(), VariationRange> {
    let points = points
        .iter()
        .map(|(index, value, default)| (*index, value, *default));
    write_points(cursor, range, variation, points)
}

#[no_panic::no_panic]
fn static_points(points: &Points, range: IndexRange, buffer: &mut [u8]) {
    let mut cursor = WriteCursor::new(buffer);
    let cursor = &mut cursor;
    let result = match points {
        Points::Binary(points, variation) => write(cursor, range, *variation, points),
        Points::DoubleBitBinary(points, variation) => write(cursor, range, *variation, points),
        Points::BinaryOutputStatus(points, variation) => write(cursor, range, *variation, points),
        Points::Counter(points, variation) => write(cursor, range, *variation, points),
        Points::FrozenCounter(points, variation) => write(cursor, range, *variation, points),
        Points::Analog(points, variation) => write(cursor, range, *variation, points),
        Points::AnalogOutputStatus(points, variation) => write(cursor, range, *variation, points),
        Points::OctetString(points) => write(cursor, range, None, points),
        Points::DeadBand(dead_bands, variation) => {
            let dead_bands = dead_bands.iter().map(|(index, value)| (*index, value));
            write_dead_bands(cursor, range, *variation, dead_bands)
        }
    };
    black_box(result.err());
    black_box(cursor.written());
}

#[no_panic::no_panic]
fn unsolicited(db: &mut Database, classes: EventClasses, buffer: &mut [u8]) {
    let mut cursor = WriteCursor::new(buffer);
    if db.inner.select_unsolicited(classes) != 0 {
        black_box(db.inner.write_events_only(&mut cursor));
    }
    black_box(cursor.written());
}

/// A value of one type and the default event variation to record it in
enum NewEvent {
    Binary(BinaryInput, EventBinaryInputVariation),
    DoubleBitBinary(DoubleBitBinaryInput, EventDoubleBitBinaryInputVariation),
    BinaryOutputStatus(BinaryOutputStatus, EventBinaryOutputStatusVariation),
    Counter(Counter, EventCounterVariation),
    FrozenCounter(FrozenCounter, EventFrozenCounterVariation),
    Analog(AnalogInput, EventAnalogInputVariation),
    AnalogOutputStatus(AnalogOutputStatus, EventAnalogOutputStatusVariation),
    /// created outside the check, since its bytes are allocated
    OctetString(Event),
}

#[no_panic::no_panic]
fn insert(db: &mut Database, index: u16, class: EventClass, event: NewEvent) {
    let buffer = db.inner.event_buffer();
    let result = match event {
        NewEvent::Binary(value, variation) => buffer.insert(index, class, &value, variation),
        NewEvent::DoubleBitBinary(value, variation) => {
            buffer.insert(index, class, &value, variation)
        }
        NewEvent::BinaryOutputStatus(value, variation) => {
            buffer.insert(index, class, &value, variation)
        }
        NewEvent::Counter(value, variation) => buffer.insert(index, class, &value, variation),
        NewEvent::FrozenCounter(value, variation) => buffer.insert(index, class, &value, variation),
        NewEvent::Analog(value, variation) => buffer.insert(index, class, &value, variation),
        NewEvent::AnalogOutputStatus(value, variation) => {
            buffer.insert(index, class, &value, variation)
        }
        NewEvent::OctetString(event) => buffer.insert_event(index, class, event),
    };
    black_box(result.is_ok());
    black_box(result.err());
}

#[no_panic::no_panic]
fn confirm(db: &mut Database) {
    let count = db.inner.event_buffer().clear_written(|id| {
        black_box(id);
    });
    black_box(count);
}

#[no_panic::no_panic]
fn discard(db: &mut Database, classes: EventClasses) {
    black_box(db.inner.event_buffer().remove_unselected_by_class(classes));
}

#[no_panic::no_panic]
fn defer(
    deferred: &mut DeferredRead,
    hash: u64,
    seq: Sequence,
    info: FragmentInfo,
    headers: HeaderCollection,
) {
    deferred.set(hash, seq, info, headers, Quiet);
    black_box(deferred.is_set());
}

fn class(source: &mut Source) -> Option<EventClass> {
    source.pick(&[
        None,
        Some(EventClass::Class1),
        Some(EventClass::Class2),
        Some(EventClass::Class3),
    ])
}

fn event_classes(source: &mut Source) -> EventClasses {
    EventClasses::new(source.bool(), source.bool(), source.bool())
}

fn flags(source: &mut Source) -> Flags {
    Flags::new(source.u8())
}

fn time(source: &mut Source) -> Option<Time> {
    match source.u8() % 3 {
        0 => None,
        1 => Some(Time::synchronized(source.u64())),
        _ => Some(Time::unsynchronized(source.u64())),
    }
}

fn options(source: &mut Source) -> UpdateOptions {
    let mode = source.pick(&[EventMode::Detect, EventMode::Force, EventMode::Suppress]);
    UpdateOptions::new(source.bool(), mode)
}

fn binary_input(source: &mut Source) -> BinaryInput {
    BinaryInput {
        value: source.bool(),
        flags: flags(source),
        time: time(source),
    }
}

fn double_bit_binary_input(source: &mut Source) -> DoubleBitBinaryInput {
    DoubleBitBinaryInput {
        value: source.pick(&[
            DoubleBit::Intermediate,
            DoubleBit::DeterminedOff,
            DoubleBit::DeterminedOn,
            DoubleBit::Indeterminate,
        ]),
        flags: flags(source),
        time: time(source),
    }
}

fn binary_output_status(source: &mut Source) -> BinaryOutputStatus {
    BinaryOutputStatus {
        value: source.bool(),
        flags: flags(source),
        time: time(source),
    }
}

fn counter(source: &mut Source) -> Counter {
    Counter {
        value: source.u32(),
        flags: flags(source),
        time: time(source),
    }
}

fn frozen_counter(source: &mut Source) -> FrozenCounter {
    FrozenCounter {
        value: source.u32(),
        flags: flags(source),
        time: time(source),
    }
}

fn analog_input(source: &mut Source) -> AnalogInput {
    AnalogInput {
        value: f64::from_bits(source.u64()),
        flags: flags(source),
        time: time(source),
    }
}

fn analog_output_status(source: &mut Source) -> AnalogOutputStatus {
    AnalogOutputStatus {
        value: f64::from_bits(source.u64()),
        flags: flags(source),
        time: time(source),
    }
}

fn octet_string(source: &mut Source) -> Option<OctetString> {
    OctetString::new(&source.bytes()).ok()
}

fn add(db: &mut Database, source: &mut Source) {
    let index = source.u16();
    let class = class(source);
    let added = match source.u8() % 8 {
        0 => db.add(
            index,
            class,
            BinaryInputConfig::new(
                source.pick(StaticBinaryInputVariation::ALL),
                source.pick(EventBinaryInputVariation::ALL),
            ),
        ),
        1 => db.add(
            index,
            class,
            DoubleBitBinaryInputConfig::new(
                source.pick(StaticDoubleBitBinaryInputVariation::ALL),
                source.pick(EventDoubleBitBinaryInputVariation::ALL),
            ),
        ),
        2 => db.add(
            index,
            class,
            BinaryOutputStatusConfig::new(
                source.pick(StaticBinaryOutputStatusVariation::ALL),
                source.pick(EventBinaryOutputStatusVariation::ALL),
            ),
        ),
        3 => db.add(
            index,
            class,
            CounterConfig::new(
                source.pick(StaticCounterVariation::ALL),
                source.pick(EventCounterVariation::ALL),
                source.u32(),
            ),
        ),
        4 => db.add(
            index,
            class,
            FrozenCounterConfig::new(
                source.pick(StaticFrozenCounterVariation::ALL),
                source.pick(EventFrozenCounterVariation::ALL),
                source.u32(),
            ),
        ),
        5 => db.add(
            index,
            class,
            AnalogInputConfig::new(
                source.pick(StaticAnalogInputVariation::ALL),
                source.pick(EventAnalogInputVariation::ALL),
                f64::from_bits(source.u64()),
            ),
        ),
        6 => db.add(
            index,
            class,
            AnalogOutputStatusConfig::new(
                source.pick(StaticAnalogOutputStatusVariation::ALL),
                source.pick(EventAnalogOutputStatusVariation::ALL),
                f64::from_bits(source.u64()),
            ),
        ),
        _ => db.add(index, class, OctetStringConfig),
    };
    black_box(added);
}

fn update(db: &mut Database, source: &mut Source) {
    let index = source.u16();
    let info = match source.u8() % 8 {
        0 => db.update(index, &binary_input(source), options(source)),
        1 => db.update(index, &double_bit_binary_input(source), options(source)),
        2 => db.update(index, &binary_output_status(source), options(source)),
        3 => db.update(index, &counter(source), options(source)),
        4 => db.update(index, &frozen_counter(source), options(source)),
        5 => db.update(index, &analog_input(source), options(source)),
        6 => db.update(index, &analog_output_status(source), options(source)),
        _ => match octet_string(source) {
            Some(value) => db.update(index, &value, options(source)),
            None => return,
        },
    };
    black_box(info);
}

/// Points with any indexes, in any order, with values from `value` and default variations from
/// `defaults`
fn entries<T: Family>(
    source: &mut Source,
    value: fn(&mut Source) -> T,
    defaults: &[T::StaticVariation],
) -> Vec<(u16, T, T::StaticVariation)> {
    source.many(|source| (source.u16(), value(source), source.pick(defaults)))
}

/// One of `all`, or none
fn requested<T: Copy>(source: &mut Source, all: &[T]) -> Option<T> {
    if source.bool() {
        Some(source.pick(all))
    } else {
        None
    }
}

fn points(source: &mut Source) -> Points {
    match source.u8() % 9 {
        0 => Points::Binary(
            entries(source, binary_input, StaticBinaryInputVariation::ALL),
            requested(source, StaticBinaryInputVariation::ALL),
        ),
        1 => Points::DoubleBitBinary(
            entries(
                source,
                double_bit_binary_input,
                StaticDoubleBitBinaryInputVariation::ALL,
            ),
            requested(source, StaticDoubleBitBinaryInputVariation::ALL),
        ),
        2 => Points::BinaryOutputStatus(
            entries(
                source,
                binary_output_status,
                StaticBinaryOutputStatusVariation::ALL,
            ),
            requested(source, StaticBinaryOutputStatusVariation::ALL),
        ),
        3 => Points::Counter(
            entries(source, counter, StaticCounterVariation::ALL),
            requested(source, StaticCounterVariation::ALL),
        ),
        4 => Points::FrozenCounter(
            entries(source, frozen_counter, StaticFrozenCounterVariation::ALL),
            requested(source, StaticFrozenCounterVariation::ALL),
        ),
        5 => Points::Analog(
            entries(source, analog_input, StaticAnalogInputVariation::ALL),
            requested(source, StaticAnalogInputVariation::ALL),
        ),
        6 => Points::AnalogOutputStatus(
            entries(
                source,
                analog_output_status,
                StaticAnalogOutputStatusVariation::ALL,
            ),
            requested(source, StaticAnalogOutputStatusVariation::ALL),
        ),
        7 => Points::OctetString(
            source
                .many(|source| {
                    let index = source.u16();
                    octet_string(source).map(|value| (index, value, StaticOctetStringVariation))
                })
                .into_iter()
                .flatten()
                .collect(),
        ),
        _ => Points::DeadBand(
            source.many(|source| (source.u16(), f64::from_bits(source.u64()))),
            requested(
                source,
                &[
                    AnalogInputDeadBandVariation::Group34Var1,
                    AnalogInputDeadBandVariation::Group34Var2,
                    AnalogInputDeadBandVariation::Group34Var3,
                ],
            ),
        ),
    }
}

fn new_event(source: &mut Source) -> Option<NewEvent> {
    let event = match source.u8() % 8 {
        0 => NewEvent::Binary(
            binary_input(source),
            source.pick(EventBinaryInputVariation::ALL),
        ),
        1 => NewEvent::DoubleBitBinary(
            double_bit_binary_input(source),
            source.pick(EventDoubleBitBinaryInputVariation::ALL),
        ),
        2 => NewEvent::BinaryOutputStatus(
            binary_output_status(source),
            source.pick(EventBinaryOutputStatusVariation::ALL),
        ),
        3 => NewEvent::Counter(counter(source), source.pick(EventCounterVariation::ALL)),
        4 => NewEvent::FrozenCounter(
            frozen_counter(source),
            source.pick(EventFrozenCounterVariation::ALL),
        ),
        5 => NewEvent::Analog(
            analog_input(source),
            source.pick(EventAnalogInputVariation::ALL),
        ),
        6 => NewEvent::AnalogOutputStatus(
            analog_output_status(source),
            source.pick(EventAnalogOutputStatusVariation::ALL),
        ),
        _ => NewEvent::OctetString(octet_string(source)?.create_event(EventOctetStringVariation)),
    };
    Some(event)
}

fn event_class(source: &mut Source) -> EventClass {
    source.pick(&[EventClass::Class1, EventClass::Class2, EventClass::Class3])
}

fn fragment_info(source: &mut Source) -> FragmentInfo {
    let addr = FragmentAddr {
        link: EndpointAddress::raw(source.u16()),
        phys: PhysAddr::None,
    };
    FragmentInfo::new(addr, None)
}

fn event_buffer_config(source: &mut Source) -> EventBufferConfig {
    EventBufferConfig::new(
        source.u16(),
        source.u16(),
        source.u16(),
        source.u16(),
        source.u16(),
        source.u16(),
        source.u16(),
        source.u16(),
    )
}

fn class_zero_config(source: &mut Source) -> ClassZeroConfig {
    ClassZeroConfig {
        binary: source.bool(),
        double_bit_binary: source.bool(),
        binary_output_status: source.bool(),
        counter: source.bool(),
        frozen_counter: source.bool(),
        analog: source.bool(),
        analog_output_status: source.bool(),
        octet_string: source.bool(),
    }
}

fn parse_options(source: &mut Source) -> ParseOptions {
    ParseOptions {
        parse_zero_length_strings: source.bool(),
    }
}

fn responses(source: &mut Source) {
    let mut db = Database::new(
        source.u16(),
        class_zero_config(source),
        event_buffer_config(source),
    );
    let mut deferred = DeferredRead::new(source.u16());
    let ops = source.len();
    for _ in 0..ops {
        match source.u8() % 12 {
            0 => add(&mut db, source),
            1 => update(&mut db, source),
            2 => {
                let set = AttrSet::new(source.u8());
                let items = source.many(|source| AttrItem {
                    variation: source.u8(),
                    properties: if source.bool() {
                        AttrProp::writable()
                    } else {
                        AttrProp::default()
                    },
                });
                let attr = super::encode::attribute(source);
                attributes(set, &items, &attr, &mut source.buffer());
            }
            3 => {
                let request = source.bytes();
                if let Ok(fragment) = ParsedFragment::parse(parse_options(source), &request) {
                    if let Ok(headers) = fragment.objects {
                        black_box(select(&mut db, &headers));
                    }
                }
            }
            4 => respond(&mut db, &mut source.buffer()),
            5 => {
                let points = points(source);
                if let Some(range) = IndexRange::new(source.u16(), source.u16()) {
                    static_points(&points, range, &mut source.buffer());
                }
            }
            6 => unsolicited(&mut db, event_classes(source), &mut source.buffer()),
            7 => {
                let request = source.bytes();
                if let Ok(fragment) = ParsedFragment::parse(parse_options(source), &request) {
                    if let Ok(headers) = fragment.objects {
                        let (hash, seq) = (source.u64(), Sequence::new(source.u8()));
                        defer(&mut deferred, hash, seq, fragment_info(source), headers);
                    }
                }
            }
            8 => deferred.clear(),
            9 => {
                let (index, class) = (source.u16(), event_class(source));
                if let Some(event) = new_event(source) {
                    insert(&mut db, index, class, event);
                }
            }
            // a confirm of the events written so far, as between continuation fragments
            10 => confirm(&mut db),
            _ => discard(&mut db, event_classes(source)),
        }
    }
}

#[test]
fn responses_with_unknown_input() {
    let data = black_box(super::input());
    responses(&mut Source(&data));
}
