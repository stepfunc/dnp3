// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Fragmenting a READ response doesn't change what the master sees
//!
//! For generated database content, READ headers and solicited buffer sizes, the database is
//! written one fragment at a time, as the session writes it, clearing the written events where the
//! master would confirm each fragment. Every fragment fits and parses, and keeps each relative-time
//! event with its CTO; the series completes within a bound; and what master extraction reports
//! across the fragments equals what it reports when the same selection is written into the
//! largest fragment. In that run, each selected item is reported once, in order.
//!
//! Only objects that fit in an empty fragment by themselves are generated, and only selections
//! that fit in the largest fragment, so that the reference is written whole. Besides general
//! content, the generator weights runs of relative-time events that share CTOs across fragment
//! boundaries, and analog deadbands (group 34) that span fragments.

use std::collections::BTreeSet;

use proptest::prelude::*;
use scursor::WriteCursor;

use crate::app::attr::{AttrProp, AttrSet, OwnedAttrValue, OwnedAttribute};
use crate::app::parse::options::ParseOptions;
use crate::app::parse::parser::HeaderCollection;
use crate::app::{FunctionCode, QualifierCode, ResponseHeader, Timestamp, Variation as V};
use crate::master::extract::{extract_measurements_inner, ProcessableResponse};
use crate::master::extract_recorder::{Recorder, Value};
use crate::master::HeaderInfo;
use crate::outstation::config::OutstationConfig;
use crate::outstation::database::*;
use crate::outstation::OutstationApplication;

/// The largest solicited fragment
const LARGEST: usize = 65535;

/// A choice from a list, by position, wrapping around its end
#[derive(Copy, Clone, Debug)]
struct Pick(u8);

impl Pick {
    fn get<T>(self, list: &[T]) -> &T {
        list.iter().cycle().nth(self.0.into()).unwrap()
    }
}

/// The private attribute sets that are generated
const SETS: std::ops::RangeInclusive<u8> = 1..=3;
/// The attribute variations that are generated
const ATTR_VARIATIONS: std::ops::RangeInclusive<u8> = 1..=4;
/// Indices of the generated points
const INDICES: std::ops::Range<u16> = 0..64;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Binary,
    DoubleBit,
    BinaryOutputStatus,
    Counter,
    FrozenCounter,
    Analog,
    AnalogOutputStatus,
    OctetString,
}

const KINDS: [Kind; 8] = [
    Kind::Binary,
    Kind::DoubleBit,
    Kind::BinaryOutputStatus,
    Kind::Counter,
    Kind::FrozenCounter,
    Kind::Analog,
    Kind::AnalogOutputStatus,
    Kind::OctetString,
];

impl Kind {
    /// the kinds a class 0 READ reports with the default `ClassZeroConfig`, in order
    fn in_class_zero(self) -> bool {
        self != Kind::OctetString
    }

    /// the variations a READ of the static points can ask for, the first being variation 0
    fn static_reads(self) -> &'static [V] {
        match self {
            Kind::Binary => &[V::Group1Var0, V::Group1Var1, V::Group1Var2],
            Kind::DoubleBit => &[V::Group3Var0, V::Group3Var1, V::Group3Var2],
            Kind::BinaryOutputStatus => &[V::Group10Var0, V::Group10Var1, V::Group10Var2],
            Kind::Counter => &[
                V::Group20Var0,
                V::Group20Var1,
                V::Group20Var2,
                V::Group20Var5,
                V::Group20Var6,
            ],
            Kind::FrozenCounter => &[
                V::Group21Var0,
                V::Group21Var1,
                V::Group21Var2,
                V::Group21Var5,
                V::Group21Var6,
                V::Group21Var9,
                V::Group21Var10,
            ],
            Kind::Analog => &[
                V::Group30Var0,
                V::Group30Var1,
                V::Group30Var2,
                V::Group30Var3,
                V::Group30Var4,
                V::Group30Var5,
                V::Group30Var6,
            ],
            Kind::AnalogOutputStatus => &[
                V::Group40Var0,
                V::Group40Var1,
                V::Group40Var2,
                V::Group40Var3,
                V::Group40Var4,
            ],
            Kind::OctetString => &[V::Group110(0)],
        }
    }

    /// the variations a READ of the events can ask for, the first being variation 0
    fn event_reads(self) -> &'static [V] {
        match self {
            Kind::Binary => &[V::Group2Var0, V::Group2Var1, V::Group2Var2, V::Group2Var3],
            Kind::DoubleBit => &[V::Group4Var0, V::Group4Var1, V::Group4Var2, V::Group4Var3],
            Kind::BinaryOutputStatus => &[V::Group11Var0, V::Group11Var1, V::Group11Var2],
            Kind::Counter => &[
                V::Group22Var0,
                V::Group22Var1,
                V::Group22Var2,
                V::Group22Var5,
                V::Group22Var6,
            ],
            Kind::FrozenCounter => &[
                V::Group23Var0,
                V::Group23Var1,
                V::Group23Var2,
                V::Group23Var5,
                V::Group23Var6,
            ],
            Kind::Analog => &[
                V::Group32Var0,
                V::Group32Var1,
                V::Group32Var2,
                V::Group32Var3,
                V::Group32Var4,
                V::Group32Var5,
                V::Group32Var6,
                V::Group32Var7,
                V::Group32Var8,
            ],
            Kind::AnalogOutputStatus => &[
                V::Group42Var0,
                V::Group42Var1,
                V::Group42Var2,
                V::Group42Var3,
                V::Group42Var4,
                V::Group42Var5,
                V::Group42Var6,
                V::Group42Var7,
                V::Group42Var8,
            ],
            Kind::OctetString => &[V::Group111(0)],
        }
    }
}

/// Generated values, from which a measurement of any kind is made
#[derive(Clone, Debug)]
struct Sample {
    state: u8,
    flags: u8,
    count: u32,
    number: f64,
    time: Option<Time>,
    bytes: Vec<u8>,
}

/// A point, with the default variations from its configuration
#[derive(Clone, Debug)]
struct Point {
    kind: Kind,
    index: u16,
    class: Option<EventClass>,
    static_variation: Pick,
    event_variation: Pick,
    deadband: f64,
    value: Sample,
}

/// A forced update of a point, which creates an event if it has a class
#[derive(Clone, Debug)]
struct Change {
    point: Pick,
    value: Sample,
}

#[derive(Copy, Clone, Debug)]
enum Range {
    All,
    /// a start and stop, with 1 or 2-byte indices
    Some(u16, u16, bool),
}

/// A header of the READ
#[derive(Clone, Debug)]
enum Read {
    Class0,
    /// the events of a class 1 to 3, all or at most a count
    Class(u8, Option<u8>),
    /// the static points of a kind in a variation (0 included)
    Static(Kind, Pick, Range),
    /// the events of a kind in a variation (0 included), all or at most a count
    Event(Kind, Pick, Option<u8>),
    /// the deadbands of the analog inputs in a group 34 variation (0 included)
    DeadBand(Pick, Range),
    /// g0v254 or g0v255 for every set
    AllSets(u8),
    /// a variation of one set: 254, 255 or a specific one
    Set(u8, u8),
}

#[derive(Clone, Debug)]
struct Spec {
    /// the solicited fragment size
    size: usize,
    points: Vec<Point>,
    changes: Vec<Change>,
    attrs: Vec<OwnedAttribute>,
    reads: Vec<Read>,
}

// ---------- the database and the READ ----------

fn flags(sample: &Sample) -> Flags {
    Flags::new(sample.flags)
}

fn double_bit(state: u8) -> DoubleBit {
    match (state & 0x02 != 0, state & 0x01 != 0) {
        (false, false) => DoubleBit::Intermediate,
        (false, true) => DoubleBit::DeterminedOff,
        (true, false) => DoubleBit::DeterminedOn,
        (true, true) => DoubleBit::Indeterminate,
    }
}

fn update(db: &mut Database, kind: Kind, index: u16, x: &Sample, options: UpdateOptions) {
    let (state, flags, time) = (x.state & 0x01 != 0, flags(x), x.time);
    let updated = match kind {
        Kind::Binary => {
            let value = BinaryInput {
                value: state,
                flags,
                time,
            };
            db.update(index, &value, options)
        }
        Kind::DoubleBit => {
            let value = DoubleBitBinaryInput {
                value: double_bit(x.state),
                flags,
                time,
            };
            db.update(index, &value, options)
        }
        Kind::BinaryOutputStatus => {
            let value = BinaryOutputStatus {
                value: state,
                flags,
                time,
            };
            db.update(index, &value, options)
        }
        Kind::Counter => {
            let value = Counter {
                value: x.count,
                flags,
                time,
            };
            db.update(index, &value, options)
        }
        Kind::FrozenCounter => {
            let value = FrozenCounter {
                value: x.count,
                flags,
                time,
            };
            db.update(index, &value, options)
        }
        Kind::Analog => {
            let value = AnalogInput {
                value: x.number,
                flags,
                time,
            };
            db.update(index, &value, options)
        }
        Kind::AnalogOutputStatus => {
            let value = AnalogOutputStatus {
                value: x.number,
                flags,
                time,
            };
            db.update(index, &value, options)
        }
        Kind::OctetString => db.update(index, &OctetString::new(&x.bytes).unwrap(), options),
    };
    assert!(updated);
}

fn add(db: &mut Database, x: &Point) -> bool {
    use EventAnalogInputVariation as E30;
    use EventAnalogOutputStatusVariation as E40;
    use EventCounterVariation as E20;
    use EventFrozenCounterVariation as E21;
    use StaticAnalogInputVariation as S30;
    use StaticAnalogOutputStatusVariation as S40;
    use StaticCounterVariation as S20;
    use StaticFrozenCounterVariation as S21;

    let (index, class, s, e) = (x.index, x.class, &x.static_variation, &x.event_variation);
    match x.kind {
        Kind::Binary => {
            use EventBinaryInputVariation as E;
            use StaticBinaryInputVariation as S;
            let config = BinaryInputConfig {
                s_var: *s.get(&[S::Group1Var1, S::Group1Var2]),
                e_var: *e.get(&[E::Group2Var1, E::Group2Var2, E::Group2Var3]),
            };
            db.add(index, class, config)
        }
        Kind::DoubleBit => {
            use EventDoubleBitBinaryInputVariation as E;
            use StaticDoubleBitBinaryInputVariation as S;
            let config = DoubleBitBinaryInputConfig {
                s_var: *s.get(&[S::Group3Var1, S::Group3Var2]),
                e_var: *e.get(&[E::Group4Var1, E::Group4Var2, E::Group4Var3]),
            };
            db.add(index, class, config)
        }
        Kind::BinaryOutputStatus => {
            use EventBinaryOutputStatusVariation as E;
            use StaticBinaryOutputStatusVariation as S;
            let config = BinaryOutputStatusConfig {
                s_var: *s.get(&[S::Group10Var1, S::Group10Var2]),
                e_var: *e.get(&[E::Group11Var1, E::Group11Var2]),
            };
            db.add(index, class, config)
        }
        Kind::Counter => {
            let config = CounterConfig {
                s_var: *s.get(&[
                    S20::Group20Var1,
                    S20::Group20Var2,
                    S20::Group20Var5,
                    S20::Group20Var6,
                ]),
                e_var: *e.get(&[
                    E20::Group22Var1,
                    E20::Group22Var2,
                    E20::Group22Var5,
                    E20::Group22Var6,
                ]),
                ..Default::default()
            };
            db.add(index, class, config)
        }
        Kind::FrozenCounter => {
            let config = FrozenCounterConfig {
                s_var: *s.get(&[
                    S21::Group21Var1,
                    S21::Group21Var2,
                    S21::Group21Var5,
                    S21::Group21Var6,
                    S21::Group21Var9,
                    S21::Group21Var10,
                ]),
                e_var: *e.get(&[
                    E21::Group23Var1,
                    E21::Group23Var2,
                    E21::Group23Var5,
                    E21::Group23Var6,
                ]),
                ..Default::default()
            };
            db.add(index, class, config)
        }
        Kind::Analog => {
            let config = AnalogInputConfig {
                s_var: *s.get(&[
                    S30::Group30Var1,
                    S30::Group30Var2,
                    S30::Group30Var3,
                    S30::Group30Var4,
                    S30::Group30Var5,
                    S30::Group30Var6,
                ]),
                e_var: *e.get(&[
                    E30::Group32Var1,
                    E30::Group32Var2,
                    E30::Group32Var3,
                    E30::Group32Var4,
                    E30::Group32Var5,
                    E30::Group32Var6,
                    E30::Group32Var7,
                    E30::Group32Var8,
                ]),
                deadband: x.deadband,
            };
            db.add(index, class, config)
        }
        Kind::AnalogOutputStatus => {
            let config = AnalogOutputStatusConfig {
                s_var: *s.get(&[
                    S40::Group40Var1,
                    S40::Group40Var2,
                    S40::Group40Var3,
                    S40::Group40Var4,
                ]),
                e_var: *e.get(&[
                    E40::Group42Var1,
                    E40::Group42Var2,
                    E40::Group42Var3,
                    E40::Group42Var4,
                    E40::Group42Var5,
                    E40::Group42Var6,
                    E40::Group42Var7,
                    E40::Group42Var8,
                ]),
                ..Default::default()
            };
            db.add(index, class, config)
        }
        Kind::OctetString => db.add(index, class, OctetStringConfig),
    }
}

impl Spec {
    /// the points that are added: the first at each kind and index
    fn added(&self) -> Vec<&Point> {
        let mut added = BTreeSet::new();
        self.points
            .iter()
            .filter(|x| added.insert((x.kind, x.index)))
            .collect()
    }

    fn database(&self) -> DatabaseHandle {
        let handle = DatabaseHandle::new(
            OutstationConfig::DEFAULT_MAX_READ_REQUEST_HEADERS,
            ClassZeroConfig::default(),
            EventBufferConfig::all_types(64),
        );
        handle.transaction(|db| {
            for point in &self.points {
                if add(db, point) {
                    update(
                        db,
                        point.kind,
                        point.index,
                        &point.value,
                        UpdateOptions::no_event(),
                    );
                }
            }
            let added = self.added();
            for change in &self.changes {
                let point = change.point.get(&added);
                let options = UpdateOptions::new(true, EventMode::Force);
                update(db, point.kind, point.index, &change.value, options);
            }
            for attr in &self.attrs {
                // a variation already defined in the set is skipped
                let _: Result<(), AttrDefError> = db.define_attr(AttrProp::default(), attr.clone());
            }
        });
        handle
    }

    fn request(&self) -> Vec<u8> {
        fn header(bytes: &mut Vec<u8>, variation: V, qualifier: QualifierCode) {
            let (group, var) = variation.to_group_and_var();
            bytes.extend([group, var, qualifier.as_u8()]);
        }

        fn range(bytes: &mut Vec<u8>, variation: V, range: Range) {
            match range {
                Range::All => header(bytes, variation, QualifierCode::AllObjects),
                Range::Some(start, stop, true) => {
                    header(bytes, variation, QualifierCode::Range16);
                    bytes.extend(start.to_le_bytes());
                    bytes.extend(stop.to_le_bytes());
                }
                Range::Some(start, stop, false) => {
                    header(bytes, variation, QualifierCode::Range8);
                    bytes.push(u8::try_from(start).unwrap());
                    bytes.push(u8::try_from(stop).unwrap());
                }
            }
        }

        fn count(bytes: &mut Vec<u8>, variation: V, count: Option<u8>) {
            match count {
                None => header(bytes, variation, QualifierCode::AllObjects),
                Some(count) => {
                    header(bytes, variation, QualifierCode::Count8);
                    bytes.push(count);
                }
            }
        }

        let mut bytes = Vec::new();
        for read in &self.reads {
            match read {
                Read::Class0 => header(&mut bytes, V::Group60Var1, QualifierCode::AllObjects),
                Read::Class(class, limit) => {
                    let variation = match class {
                        1 => V::Group60Var2,
                        2 => V::Group60Var3,
                        _ => V::Group60Var4,
                    };
                    count(&mut bytes, variation, *limit);
                }
                Read::Static(kind, variation, r) => {
                    range(&mut bytes, *variation.get(kind.static_reads()), *r)
                }
                Read::Event(kind, variation, limit) => {
                    count(&mut bytes, *variation.get(kind.event_reads()), *limit)
                }
                Read::DeadBand(variation, r) => {
                    let variations = [
                        V::Group34Var0,
                        V::Group34Var1,
                        V::Group34Var2,
                        V::Group34Var3,
                    ];
                    range(&mut bytes, *variation.get(&variations), *r)
                }
                Read::AllSets(var) => {
                    header(&mut bytes, V::Group0(*var), QualifierCode::AllObjects)
                }
                Read::Set(set, var) => {
                    let set = u16::from(*set);
                    range(&mut bytes, V::Group0(*var), Range::Some(set, set, false))
                }
            }
        }
        bytes
    }
}

// ---------- writing fragments ----------

struct Confirms;

impl OutstationApplication for Confirms {}

/// Write fragments of `size` bytes until the response is complete, returning what master
/// extraction reports across all of them
fn write_fragments(spec: &Spec, size: usize, max_fragments: usize) -> Vec<(HeaderInfo, Value)> {
    let mut handle = spec.database();
    let request = spec.request();
    let headers =
        HeaderCollection::parse(ParseOptions::default(), FunctionCode::Read, &request).unwrap();
    handle.select(&headers);

    let mut buffer = vec![0; size.checked_sub(ResponseHeader::LENGTH).unwrap()];
    let mut recorder = Recorder::default();
    for _ in 0..max_fragments {
        let mut cursor = WriteCursor::new(&mut buffer);
        let info = handle.write_response_headers(&mut cursor);

        // the objects parse, and every relative-time event follows a CTO in the same fragment
        let objects = ProcessableResponse::new(HeaderCollection::parse(
            ParseOptions::default(),
            FunctionCode::Response,
            cursor.written(),
        ))
        .unwrap();
        extract_measurements_inner(objects, &mut recorder);

        if info.complete {
            return recorder.values;
        }
        // the master confirms the fragment
        handle.transaction(|db| db.inner.clear_written_events(&mut Confirms));
    }
    panic!("the response is not complete after {max_fragments} fragments");
}

// ---------- the selection model ----------

/// What a value reported to the master identifies
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Item {
    Event(Kind, u16),
    Static(Kind, u16),
    DeadBand(u16),
    Attr(u8, u8),
    List(u8),
}

fn item((info, value): &(HeaderInfo, Value)) -> Item {
    let (kind, index) = match value {
        Value::Binary(_, i) => (Kind::Binary, *i),
        Value::DoubleBit(_, i) => (Kind::DoubleBit, *i),
        Value::BinaryOutputStatus(_, i) => (Kind::BinaryOutputStatus, *i),
        Value::Counter(_, i) => (Kind::Counter, *i),
        Value::FrozenCounter(_, i) => (Kind::FrozenCounter, *i),
        Value::Analog(_, i) => (Kind::Analog, *i),
        Value::AnalogOutputStatus(_, i) => (Kind::AnalogOutputStatus, *i),
        Value::OctetString(_, i) => (Kind::OctetString, *i),
        Value::AnalogDeadBand(_, i) => return Item::DeadBand(*i),
        Value::Attribute(x) => return Item::Attr(x.set.value(), x.variation),
        Value::AttributeList(set, _) => return Item::List(set.value()),
    };
    if info.is_event {
        Item::Event(kind, index)
    } else {
        Item::Static(kind, index)
    }
}

/// the indices of a kind's points that a range selects
fn select_range(indices: &BTreeSet<u16>, range: Range) -> Vec<u16> {
    match range {
        Range::All => indices.iter().copied().collect(),
        Range::Some(start, stop, _) => indices.range(start..=stop).copied().collect(),
    }
}

/// Each item the READ selects, in the order the response reports them: events in the order they
/// were created, then static points by header, then attributes by header
fn selection(spec: &Spec) -> Vec<Item> {
    let added = spec.added();
    let indices = |kind: Kind| -> BTreeSet<u16> {
        added
            .iter()
            .filter(|x| x.kind == kind)
            .map(|x| x.index)
            .collect()
    };
    let mut events: Vec<(&Point, bool)> = spec
        .changes
        .iter()
        .map(|x| *x.point.get(&added))
        .filter(|x| x.class.is_some())
        .map(|x| (x, false))
        .collect();
    let mut sets: BTreeSet<(u8, u8)> = BTreeSet::new();
    for attr in &spec.attrs {
        sets.insert((attr.set.value(), attr.variation));
    }
    let variations = |set: u8| sets.iter().filter(move |x| x.0 == set).map(|x| x.1);

    let mut statics = Vec::new();
    let mut attrs = Vec::new();
    for read in &spec.reads {
        let mut select_events = |matches: &dyn Fn(&Point) -> bool, limit: Option<u8>| {
            let limit = limit.map_or(usize::MAX, usize::from);
            for (_, selected) in events
                .iter_mut()
                .filter(|(x, selected)| !selected && matches(x))
                .take(limit)
            {
                *selected = true;
            }
        };
        match read {
            Read::Class0 => {
                for kind in KINDS.into_iter().filter(|x| x.in_class_zero()) {
                    statics.extend(indices(kind).into_iter().map(|i| Item::Static(kind, i)));
                }
            }
            Read::Class(class, limit) => {
                let class = match class {
                    1 => EventClass::Class1,
                    2 => EventClass::Class2,
                    _ => EventClass::Class3,
                };
                select_events(&|x| x.class == Some(class), *limit);
            }
            Read::Static(kind, _, range) => {
                let selected = select_range(&indices(*kind), *range);
                statics.extend(selected.into_iter().map(|i| Item::Static(*kind, i)));
            }
            Read::Event(kind, _, limit) => select_events(&|x| x.kind == *kind, *limit),
            Read::DeadBand(_, range) => {
                let selected = select_range(&indices(Kind::Analog), *range);
                statics.extend(selected.into_iter().map(Item::DeadBand));
            }
            Read::AllSets(254) => {
                for set in SETS {
                    attrs.extend(variations(set).map(|var| Item::Attr(set, var)));
                }
            }
            Read::AllSets(_) => {
                for set in SETS.filter(|x| variations(*x).next().is_some()) {
                    attrs.push(Item::List(set));
                }
            }
            Read::Set(set, 254) => attrs.extend(variations(*set).map(|var| Item::Attr(*set, var))),
            Read::Set(set, 255) => {
                if variations(*set).next().is_some() {
                    attrs.push(Item::List(*set));
                }
            }
            Read::Set(set, var) => {
                if sets.contains(&(*set, *var)) {
                    attrs.push(Item::Attr(*set, *var));
                }
            }
        }
    }

    events
        .into_iter()
        .filter(|(_, selected)| *selected)
        .map(|(x, _)| Item::Event(x.kind, x.index))
        .chain(statics)
        .chain(attrs)
        .collect()
}

/// More bytes than the items can take to encode, each with a header of its own, and a CTO for
/// an event
fn encoded_bound(items: &[Item]) -> usize {
    // a CTO (10 bytes) and a header (5) for an event with an index (2) and the largest fixed-size
    // object (15) is the most for any item but an octet string or attribute
    const ITEM: usize = 32;
    // a header (7) and the longest string (255)
    const STRING: usize = 262;
    items
        .iter()
        .map(|x| match x {
            Item::Event(Kind::OctetString, _)
            | Item::Static(Kind::OctetString, _)
            | Item::Attr(..) => STRING,
            Item::Event(..) | Item::Static(..) | Item::DeadBand(_) | Item::List(_) => ITEM,
        })
        .fold(0, usize::saturating_add)
}

// ---------- generators ----------

fn pick() -> impl Strategy<Value = Pick> {
    any::<u8>().prop_map(Pick)
}

/// a sample with only a state, flags and time, where the other values do not matter
fn sample_at(time: Option<Time>, state: u8, flags: u8) -> Sample {
    Sample {
        state,
        flags,
        count: 0,
        number: 0.0,
        time,
        bytes: Vec::new(),
    }
}

fn sample(max_bytes: usize) -> impl Strategy<Value = Sample> {
    let timestamp = prop_oneof![
        Just(0),
        Just(Timestamp::MAX_VALUE),
        0..=Timestamp::MAX_VALUE
    ]
    .prop_map(Timestamp::new);
    let time = prop_oneof![
        Just(None),
        timestamp.clone().prop_map(|x| Some(Time::Synchronized(x))),
        timestamp.prop_map(|x| Some(Time::Unsynchronized(x))),
    ];
    let number = prop_oneof![
        prop::sample::select(vec![f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, 1e40]),
        any::<f64>(),
        any::<i32>().prop_map(f64::from),
    ];
    (
        any::<u8>(),
        prop_oneof![Just(0x01), any::<u8>()],
        any::<u32>(),
        number,
        time,
        prop_oneof![
            prop::collection::vec(any::<u8>(), max_bytes),
            prop::collection::vec(any::<u8>(), 1..=max_bytes),
        ],
    )
        .prop_map(|(state, flags, count, number, time, bytes)| Sample {
            state,
            flags,
            count,
            number,
            time,
            bytes,
        })
}

fn kind() -> impl Strategy<Value = Kind> {
    prop::sample::select(KINDS.to_vec())
}

fn class() -> impl Strategy<Value = Option<EventClass>> {
    prop::sample::select(vec![
        None,
        Some(EventClass::Class1),
        Some(EventClass::Class2),
        Some(EventClass::Class3),
    ])
}

fn point(max_bytes: usize) -> impl Strategy<Value = Point> {
    let deadband = prop_oneof![
        prop::sample::select(vec![
            0.0,
            0.5,
            65535.5,
            4_294_967_295.5,
            1e40,
            f64::INFINITY
        ]),
        0.0..1e6,
    ];
    (
        kind(),
        INDICES,
        class(),
        pick(),
        pick(),
        deadband,
        sample(max_bytes),
    )
        .prop_map(
            |(kind, index, class, static_variation, event_variation, deadband, value)| Point {
                kind,
                index,
                class,
                static_variation,
                event_variation,
                deadband,
                value,
            },
        )
}

fn attr(max_bytes: usize) -> impl Strategy<Value = OwnedAttribute> {
    let value = prop_oneof![
        prop::string::string_regex(&format!("[ -~]{{1,{max_bytes}}}"))
            .unwrap()
            .prop_map(OwnedAttrValue::VisibleString),
        any::<u32>().prop_map(OwnedAttrValue::UnsignedInt),
        any::<i32>().prop_map(OwnedAttrValue::SignedInt),
        prop::collection::vec(any::<u8>(), 1..=max_bytes).prop_map(OwnedAttrValue::OctetString),
    ];
    (SETS, ATTR_VARIATIONS, value).prop_map(|(set, variation, value)| {
        OwnedAttribute::new(AttrSet::new(set), variation, value)
    })
}

/// all objects, or a range partly outside the indices of the points
fn range() -> impl Strategy<Value = Range> {
    prop_oneof![
        Just(Range::All),
        (0u16..72, 0u16..16, any::<bool>()).prop_map(|(start, length, wide)| {
            Range::Some(start, start.saturating_add(length), wide)
        }),
    ]
}

fn limit() -> impl Strategy<Value = Option<u8>> {
    prop::option::of(0u8..12)
}

fn read() -> impl Strategy<Value = Read> {
    prop_oneof![
        2 => Just(Read::Class0),
        3 => (1u8..=3, limit()).prop_map(|(class, limit)| Read::Class(class, limit)),
        3 => (kind(), pick(), range())
            .prop_map(|(kind, variation, range)| Read::Static(kind, variation, range)),
        2 => (kind(), pick(), limit())
            .prop_map(|(kind, variation, limit)| Read::Event(kind, variation, limit)),
        1 => (pick(), range()).prop_map(|(variation, range)| Read::DeadBand(variation, range)),
        1 => prop::sample::select(vec![254, 255]).prop_map(Read::AllSets),
        1 => (0u8..=4, prop_oneof![Just(254u8), Just(255u8), ATTR_VARIATIONS])
            .prop_map(|(set, var)| Read::Set(set, var)),
    ]
}

/// the solicited fragment size: near the smallest, or the default
fn size() -> impl Strategy<Value = usize> {
    prop_oneof![
        4 => 249usize..=300,
        1 => Just(249usize),
        1 => Just(2048usize),
    ]
}

fn general_spec() -> impl Strategy<Value = Spec> {
    size().prop_flat_map(|size| {
        // the most bytes of an octet string or attribute that fit in an empty fragment with
        // their header: 4 bytes of response header and 7 of object header
        let max_bytes = size.checked_sub(11).unwrap().min(255);
        (
            prop::collection::vec(point(max_bytes), 1..64),
            prop::collection::vec((pick(), sample(max_bytes)), 0..40),
            prop::collection::vec(attr(max_bytes), 0..6),
            prop::collection::vec(read(), 1..=6),
        )
            .prop_map(move |(points, changes, attrs, reads)| Spec {
                size,
                points,
                changes: changes
                    .into_iter()
                    .map(|(point, value)| Change { point, value })
                    .collect(),
                attrs,
                reads,
            })
    })
}

/// Binary and double-bit events with relative time, close enough in time that runs of them share
/// a CTO, and enough of them to straddle fragment boundaries
fn relative_time_spec() -> impl Strategy<Value = Spec> {
    let point = (any::<bool>(), INDICES).prop_map(|(double_bit, index)| Point {
        kind: if double_bit {
            Kind::DoubleBit
        } else {
            Kind::Binary
        },
        index,
        class: Some(EventClass::Class1),
        static_variation: Pick(0),
        // g2v3 or g4v3
        event_variation: Pick(2),
        deadband: 0.0,
        value: sample_at(None, 0, 0x01),
    });
    // mostly steps a CTO reaches, sometimes past it, and sometimes a change of synchronization
    let step = (
        prop_oneof![8 => 0u64..2000, 1 => 60_000u64..70_000],
        prop_oneof![8 => Just(false), 1 => Just(true)],
        any::<u8>(),
        any::<u8>(),
    );
    let read = prop_oneof![
        Just(Read::Class(1, None)),
        (
            prop::sample::select(vec![Kind::Binary, Kind::DoubleBit]),
            // variation 0 or 3
            prop::sample::select(vec![Pick(0), Pick(3)]),
            prop::option::of(20u8..64),
        )
            .prop_map(|(kind, variation, limit)| Read::Event(kind, variation, limit)),
    ];
    (
        249usize..=280,
        prop::collection::vec(point, 1..32),
        0..=0xFFFF_FFFE_0000u64,
        any::<bool>(),
        prop::collection::vec((pick(), step), 40..64),
        prop::collection::vec(read, 1..=3),
    )
        .prop_map(|(size, points, start, synchronized, steps, reads)| {
            let (mut timestamp, mut synchronized) = (start, synchronized);
            let changes = steps
                .into_iter()
                .map(|(point, (step, flip, state, flags))| {
                    timestamp = timestamp.saturating_add(step).min(Timestamp::MAX_VALUE);
                    synchronized ^= flip;
                    let time = Timestamp::new(timestamp);
                    let time = if synchronized {
                        Time::Synchronized(time)
                    } else {
                        Time::Unsynchronized(time)
                    };
                    let value = sample_at(Some(time), state, flags);
                    Change { point, value }
                })
                .collect();
            Spec {
                size,
                points,
                changes,
                attrs: Vec::new(),
                reads,
            }
        })
}

/// Analog inputs at consecutive indices whose deadbands, read in group 34, span fragments
fn dead_band_spec() -> impl Strategy<Value = Spec> {
    let read = prop_oneof![
        4 => pick().prop_map(|variation| Read::DeadBand(variation, Range::All)),
        1 => (pick(), range()).prop_map(|(variation, range)| Read::DeadBand(variation, range)),
        1 => (pick(), range()).prop_map(|(variation, range)| Read::Static(Kind::Analog, variation, range)),
    ];
    (
        249usize..=280,
        0u16..8,
        prop::collection::vec(
            prop_oneof![Just(0.0), 0.0..1e6, Just(f64::INFINITY)],
            40..=56,
        ),
        prop::collection::vec(read, 2..=4),
    )
        .prop_map(|(size, start, deadbands, reads)| {
            let points = (start..)
                .zip(deadbands)
                .map(|(index, deadband)| Point {
                    kind: Kind::Analog,
                    index,
                    class: None,
                    static_variation: Pick(0),
                    event_variation: Pick(0),
                    deadband,
                    value: sample_at(None, 0, 0x01),
                })
                .collect();
            Spec {
                size,
                points,
                changes: Vec::new(),
                attrs: Vec::new(),
                reads,
            }
        })
}

fn spec() -> impl Strategy<Value = Spec> {
    prop_oneof![
        3 => general_spec(),
        1 => relative_time_spec(),
        1 => dead_band_spec(),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn fragmenting_does_not_change_what_the_master_sees(spec in spec()) {
        let items = selection(&spec);
        // each fragment makes progress, so the series completes within this many
        let max_fragments = items.len().saturating_add(spec.reads.len()).saturating_add(2);

        // the selection fits in the largest fragment, so the reference is written whole
        prop_assume!(encoded_bound(&items) <= LARGEST - ResponseHeader::LENGTH);
        let whole = write_fragments(&spec, LARGEST, 1);
        prop_assert_eq!(whole.iter().map(item).collect::<Vec<_>>(), items);

        let fragmented = write_fragments(&spec, spec.size, max_fragments);
        prop_assert_eq!(fragmented, whole);
    }
}
