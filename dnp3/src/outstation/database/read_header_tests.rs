//! Pins for READ headers, driven by the raw object header bytes of a READ: whether each
//! (variation, qualifier) pair parses, and if it does, what the database selects for it, with
//! which IIN2, and the response objects
//!
//! The database holds two points of every supported type, at indices 0 and 1, each with one
//! event: binary inputs, double-bit binary inputs and binary output status in class 1, counters
//! and frozen counters in class 2, analog inputs, analog output status and octet strings in
//! class 3. It also holds two attributes, the device serial number in set 0 and an octet string
//! in set 1.
//!
//! The expected bytes are decimal. In response object headers, qualifier 1 is a 16-bit start and
//! stop, 7 an 8-bit count, 23 (0x17) an 8-bit count with 8-bit indices and 40 (0x28) a 16-bit
//! count with 16-bit indices

use scursor::{ReadCursor, WriteCursor};

use crate::app::attr::*;
use crate::app::parse::options::ParseOptions;
use crate::app::parse::parser::HeaderCollection;
use crate::app::{FunctionCode, Iin2, ObjectParseError, QualifierCode, Variation};
use crate::outstation::config::OutstationConfig;
use crate::outstation::database::*;

use ObjectParseError as E;

/// 48-bit time, distinct in each byte
const TIME: u64 = 0x0000_0605_0403_0201;

const NONE: Iin2 = Iin2::new(0);
const OU: Iin2 = Iin2::OBJECT_UNKNOWN;
const PE: Iin2 = Iin2::PARAMETER_ERROR;

/// What a READ of some object headers does
#[derive(Debug, PartialEq)]
enum Outcome {
    /// the parser rejects the request
    Malformed(ObjectParseError),
    /// the database selects, with this IIN2 and these response objects
    Read(Iin2, Vec<u8>),
}

/// An expected outcome in the tables below
#[derive(Copy, Clone, Debug)]
enum Expect {
    /// the parser rejects the variation with the qualifier of the request
    BadQ,
    /// the parser rejects the request with this error
    Bad(ObjectParseError),
    /// the database selects, with this IIN2 and these response objects
    R(Iin2, &'static [u8]),
}

use Expect::*;

impl Expect {
    fn outcome(self, variation: Variation, qualifier: QualifierCode) -> Outcome {
        match self {
            BadQ => Outcome::Malformed(E::InvalidQualifierForVariation(variation, qualifier)),
            Bad(err) => Outcome::Malformed(err),
            R(iin2, objects) => Outcome::Read(iin2, objects.to_vec()),
        }
    }
}

fn time() -> Time {
    Time::synchronized(TIME)
}

fn setup(db: &mut Database) {
    let options = UpdateOptions::default();
    let flags = Flags::ONLINE;
    for index in [0, 1] {
        let class_1 = Some(EventClass::Class1);
        let class_2 = Some(EventClass::Class2);
        let class_3 = Some(EventClass::Class3);
        db.add(index, class_1, BinaryInputConfig::default());
        db.update(index, &BinaryInput::new(true, flags, time()), options);
        db.add(index, class_1, DoubleBitBinaryInputConfig::default());
        let value = DoubleBitBinaryInput::new(DoubleBit::DeterminedOn, flags, time());
        db.update(index, &value, options);
        db.add(index, class_1, BinaryOutputStatusConfig::default());
        db.update(
            index,
            &BinaryOutputStatus::new(true, flags, time()),
            options,
        );
        db.add(index, class_2, CounterConfig::default());
        db.update(index, &Counter::new(5, flags, time()), options);
        db.add(index, class_2, FrozenCounterConfig::default());
        db.update(index, &FrozenCounter::new(6, flags, time()), options);
        db.add(index, class_3, AnalogInputConfig::default());
        db.update(index, &AnalogInput::new(7.0, flags, time()), options);
        db.add(index, class_3, AnalogOutputStatusConfig::default());
        db.update(index, &AnalogOutputStatus::new(8.0, flags, time()), options);
        db.add(index, class_3, OctetStringConfig);
        db.update(index, &OctetString::new(&[9]).unwrap(), options);
    }
    db.define_attr(
        AttrProp::default(),
        StringAttr::DeviceSerialNumber.with_value("S"),
    )
    .unwrap();
    db.define_attr(
        AttrProp::default(),
        OwnedAttribute::new(AttrSet::new(1), 1, OwnedAttrValue::OctetString(vec![0xAA])),
    )
    .unwrap();
}

/// select the object headers of a READ from the database, then write everything selected into
/// one fragment
fn read(objects: &[u8]) -> Outcome {
    let headers = match HeaderCollection::parse(
        ParseOptions::parse_everything(),
        FunctionCode::Read,
        objects,
    ) {
        Ok(headers) => headers,
        Err(err) => return Outcome::Malformed(err),
    };
    let mut handle = DatabaseHandle::new(
        OutstationConfig::DEFAULT_MAX_READ_REQUEST_HEADERS,
        ClassZeroConfig::default(),
        EventBufferConfig::all_types(10),
    );
    handle.transaction(setup);
    handle.reset();
    let iin2 = handle.select(&headers);
    let mut buffer = [0u8; 4096];
    let mut cursor = WriteCursor::new(&mut buffer);
    let info = handle.write_response_headers(&mut cursor);
    assert!(info.complete);
    Outcome::Read(iin2, cursor.written().to_vec())
}

fn variation(group: u8, var: u8) -> Variation {
    Variation::parse(&mut ReadCursor::new(&[group, var])).unwrap()
}

fn header(gv: [u8; 2], qualifier: QualifierCode, rest: &[u8]) -> Vec<u8> {
    let mut objects = vec![gv[0], gv[1], qualifier.as_u8()];
    objects.extend_from_slice(rest);
    objects
}

fn check(gv: [u8; 2], qualifier: QualifierCode, rest: &[u8], expect: Expect) {
    let [group, var] = gv;
    assert_eq!(
        read(&header(gv, qualifier, rest)),
        expect.outcome(variation(group, var), qualifier),
        "g{group}v{var} {qualifier:?} {rest:?}"
    );
}

/// The outcome of a READ of one variation with each qualifier
struct Row {
    gv: [u8; 2],
    /// all objects
    all: Expect,
    /// start and stop of 1, at both widths
    range: Expect,
    /// count of 1, at both widths
    count: Expect,
    /// count of 0, at both widths
    zero: Expect,
    /// count of 0 with prefixes, at both widths
    prefixed: Expect,
    /// one free-format object with no bytes
    free: Expect,
}

const fn row(
    gv: [u8; 2],
    all: Expect,
    range: Expect,
    count: Expect,
    zero: Expect,
    prefixed: Expect,
    free: Expect,
) -> Row {
    Row {
        gv,
        all,
        range,
        count,
        zero,
        prefixed,
        free,
    }
}

fn check_row(row: &Row) {
    let gv = row.gv;
    check(gv, QualifierCode::AllObjects, &[], row.all);
    check(gv, QualifierCode::Range8, &[1, 1], row.range);
    check(gv, QualifierCode::Range16, &[1, 0, 1, 0], row.range);
    check(gv, QualifierCode::Count8, &[1], row.count);
    check(gv, QualifierCode::Count16, &[1, 0], row.count);
    check(gv, QualifierCode::Count8, &[0], row.zero);
    check(gv, QualifierCode::Count16, &[0, 0], row.zero);
    check(gv, QualifierCode::CountAndPrefix8, &[0], row.prefixed);
    check(gv, QualifierCode::CountAndPrefix16, &[0, 0], row.prefixed);
    check(gv, QualifierCode::FreeFormat16, &[1, 0, 0], row.free);
}

#[rustfmt::skip]
const ROWS: &[Row] = &[
    row([0, 1], R(PE, &[]), R(NONE, &[0, 1, 0, 1, 1, 5, 1, 170]), BadQ, BadQ, Bad(E::BadAttribute(AttrParseError::CountNotOne(0))), BadQ),
    row([0, 7], R(PE, &[]), R(OU, &[]), BadQ, BadQ, Bad(E::BadAttribute(AttrParseError::CountNotOne(0))), BadQ),
    row([0, 200], R(PE, &[]), R(OU, &[]), BadQ, BadQ, Bad(E::BadAttribute(AttrParseError::CountNotOne(0))), BadQ),
    row([0, 240], R(PE, &[]), R(OU, &[]), BadQ, BadQ, Bad(E::BadAttribute(AttrParseError::CountNotOne(0))), BadQ),
    row([0, 248], R(PE, &[]), R(OU, &[]), BadQ, BadQ, Bad(E::BadAttribute(AttrParseError::CountNotOne(0))), BadQ),
    row([0, 252], R(PE, &[]), R(OU, &[]), BadQ, BadQ, Bad(E::BadAttribute(AttrParseError::CountNotOne(0))), BadQ),
    row([0, 254], R(NONE, &[0, 248, 0, 0, 0, 1, 1, 83, 0, 1, 0, 1, 1, 5, 1, 170]), R(NONE, &[0, 1, 0, 1, 1, 5, 1, 170]), BadQ, BadQ, BadQ, BadQ),
    row([0, 255], R(NONE, &[0, 255, 0, 0, 0, 254, 2, 248, 0, 0, 255, 0, 1, 1, 254, 2, 1, 0]), R(NONE, &[0, 255, 0, 1, 1, 254, 2, 1, 0]), BadQ, BadQ, Bad(E::BadAttribute(AttrParseError::CountNotOne(0))), BadQ),
    row([1, 0], R(NONE, &[1, 1, 1, 0, 0, 1, 0, 3]), R(NONE, &[1, 1, 1, 1, 0, 1, 0, 1]), BadQ, BadQ, BadQ, BadQ),
    row([1, 1], R(NONE, &[1, 1, 1, 0, 0, 1, 0, 3]), R(NONE, &[1, 1, 1, 1, 0, 1, 0, 1]), BadQ, BadQ, BadQ, BadQ),
    row([1, 2], R(NONE, &[1, 2, 1, 0, 0, 1, 0, 129, 129]), R(NONE, &[1, 2, 1, 1, 0, 1, 0, 129]), BadQ, BadQ, BadQ, BadQ),
    row([2, 0], R(NONE, &[2, 1, 40, 2, 0, 0, 0, 129, 1, 0, 129]), BadQ, R(NONE, &[2, 1, 40, 1, 0, 0, 0, 129]), R(NONE, &[]), BadQ, BadQ),
    row([2, 1], R(NONE, &[2, 1, 40, 2, 0, 0, 0, 129, 1, 0, 129]), BadQ, R(NONE, &[2, 1, 40, 1, 0, 0, 0, 129]), R(NONE, &[]), R(PE, &[]), BadQ),
    row([2, 2], R(NONE, &[2, 2, 40, 2, 0, 0, 0, 129, 1, 2, 3, 4, 5, 6, 1, 0, 129, 1, 2, 3, 4, 5, 6]), BadQ, R(NONE, &[2, 2, 40, 1, 0, 0, 0, 129, 1, 2, 3, 4, 5, 6]), R(NONE, &[]), R(PE, &[]), BadQ),
    row([2, 3], R(NONE, &[51, 1, 7, 1, 1, 2, 3, 4, 5, 6, 2, 3, 40, 2, 0, 0, 0, 129, 0, 0, 1, 0, 129, 0, 0]), BadQ, R(NONE, &[51, 1, 7, 1, 1, 2, 3, 4, 5, 6, 2, 3, 40, 1, 0, 0, 0, 129, 0, 0]), R(NONE, &[]), R(PE, &[]), BadQ),
    row([3, 0], R(NONE, &[3, 1, 1, 0, 0, 1, 0, 10]), R(NONE, &[3, 1, 1, 1, 0, 1, 0, 2]), BadQ, BadQ, BadQ, BadQ),
    row([3, 1], R(NONE, &[3, 1, 1, 0, 0, 1, 0, 10]), R(NONE, &[3, 1, 1, 1, 0, 1, 0, 2]), BadQ, BadQ, BadQ, BadQ),
    row([3, 2], R(NONE, &[3, 2, 1, 0, 0, 1, 0, 129, 129]), R(NONE, &[3, 2, 1, 1, 0, 1, 0, 129]), BadQ, BadQ, BadQ, BadQ),
    row([4, 0], R(NONE, &[4, 1, 40, 2, 0, 0, 0, 129, 1, 0, 129]), BadQ, R(NONE, &[4, 1, 40, 1, 0, 0, 0, 129]), R(NONE, &[]), BadQ, BadQ),
    row([4, 1], R(NONE, &[4, 1, 40, 2, 0, 0, 0, 129, 1, 0, 129]), BadQ, R(NONE, &[4, 1, 40, 1, 0, 0, 0, 129]), R(NONE, &[]), R(PE, &[]), BadQ),
    row([4, 2], R(NONE, &[4, 2, 40, 2, 0, 0, 0, 129, 1, 2, 3, 4, 5, 6, 1, 0, 129, 1, 2, 3, 4, 5, 6]), BadQ, R(NONE, &[4, 2, 40, 1, 0, 0, 0, 129, 1, 2, 3, 4, 5, 6]), R(NONE, &[]), R(PE, &[]), BadQ),
    row([4, 3], R(NONE, &[51, 1, 7, 1, 1, 2, 3, 4, 5, 6, 4, 3, 40, 2, 0, 0, 0, 129, 0, 0, 1, 0, 129, 0, 0]), BadQ, R(NONE, &[51, 1, 7, 1, 1, 2, 3, 4, 5, 6, 4, 3, 40, 1, 0, 0, 0, 129, 0, 0]), R(NONE, &[]), R(PE, &[]), BadQ),
    row([10, 0], R(NONE, &[10, 1, 1, 0, 0, 1, 0, 3]), R(NONE, &[10, 1, 1, 1, 0, 1, 0, 1]), BadQ, BadQ, BadQ, BadQ),
    row([10, 1], R(NONE, &[10, 1, 1, 0, 0, 1, 0, 3]), R(NONE, &[10, 1, 1, 1, 0, 1, 0, 1]), BadQ, BadQ, BadQ, BadQ),
    row([10, 2], R(NONE, &[10, 2, 1, 0, 0, 1, 0, 129, 129]), R(NONE, &[10, 2, 1, 1, 0, 1, 0, 129]), BadQ, BadQ, BadQ, BadQ),
    row([11, 0], R(NONE, &[11, 2, 40, 2, 0, 0, 0, 129, 1, 2, 3, 4, 5, 6, 1, 0, 129, 1, 2, 3, 4, 5, 6]), BadQ, R(NONE, &[11, 2, 40, 1, 0, 0, 0, 129, 1, 2, 3, 4, 5, 6]), R(NONE, &[]), BadQ, BadQ),
    row([11, 1], R(NONE, &[11, 1, 40, 2, 0, 0, 0, 129, 1, 0, 129]), BadQ, R(NONE, &[11, 1, 40, 1, 0, 0, 0, 129]), R(NONE, &[]), R(PE, &[]), BadQ),
    row([11, 2], R(NONE, &[11, 2, 40, 2, 0, 0, 0, 129, 1, 2, 3, 4, 5, 6, 1, 0, 129, 1, 2, 3, 4, 5, 6]), BadQ, R(NONE, &[11, 2, 40, 1, 0, 0, 0, 129, 1, 2, 3, 4, 5, 6]), R(NONE, &[]), R(PE, &[]), BadQ),
    row([12, 1], BadQ, BadQ, BadQ, BadQ, R(OU, &[]), BadQ),
    row([13, 1], R(OU, &[]), BadQ, R(OU, &[]), R(OU, &[]), R(OU, &[]), BadQ),
    row([13, 2], R(OU, &[]), BadQ, R(OU, &[]), R(OU, &[]), R(OU, &[]), BadQ),
    row([20, 0], R(NONE, &[20, 1, 1, 0, 0, 1, 0, 1, 5, 0, 0, 0, 1, 5, 0, 0, 0]), R(NONE, &[20, 1, 1, 1, 0, 1, 0, 1, 5, 0, 0, 0]), BadQ, BadQ, BadQ, BadQ),
    row([20, 1], R(NONE, &[20, 1, 1, 0, 0, 1, 0, 1, 5, 0, 0, 0, 1, 5, 0, 0, 0]), R(NONE, &[20, 1, 1, 1, 0, 1, 0, 1, 5, 0, 0, 0]), BadQ, BadQ, BadQ, BadQ),
    row([20, 2], R(NONE, &[20, 2, 1, 0, 0, 1, 0, 1, 5, 0, 1, 5, 0]), R(NONE, &[20, 2, 1, 1, 0, 1, 0, 1, 5, 0]), BadQ, BadQ, BadQ, BadQ),
    row([20, 5], R(NONE, &[20, 5, 1, 0, 0, 1, 0, 5, 0, 0, 0, 5, 0, 0, 0]), R(NONE, &[20, 5, 1, 1, 0, 1, 0, 5, 0, 0, 0]), BadQ, BadQ, BadQ, BadQ),
    row([20, 6], R(NONE, &[20, 6, 1, 0, 0, 1, 0, 5, 0, 5, 0]), R(NONE, &[20, 6, 1, 1, 0, 1, 0, 5, 0]), BadQ, BadQ, BadQ, BadQ),
    row([21, 0], R(NONE, &[21, 1, 1, 0, 0, 1, 0, 1, 6, 0, 0, 0, 1, 6, 0, 0, 0]), R(NONE, &[21, 1, 1, 1, 0, 1, 0, 1, 6, 0, 0, 0]), BadQ, BadQ, BadQ, BadQ),
    row([21, 1], R(NONE, &[21, 1, 1, 0, 0, 1, 0, 1, 6, 0, 0, 0, 1, 6, 0, 0, 0]), R(NONE, &[21, 1, 1, 1, 0, 1, 0, 1, 6, 0, 0, 0]), BadQ, BadQ, BadQ, BadQ),
    row([21, 2], R(NONE, &[21, 2, 1, 0, 0, 1, 0, 1, 6, 0, 1, 6, 0]), R(NONE, &[21, 2, 1, 1, 0, 1, 0, 1, 6, 0]), BadQ, BadQ, BadQ, BadQ),
    row([21, 5], R(NONE, &[21, 5, 1, 0, 0, 1, 0, 1, 6, 0, 0, 0, 1, 2, 3, 4, 5, 6, 1, 6, 0, 0, 0, 1, 2, 3, 4, 5, 6]), R(NONE, &[21, 5, 1, 1, 0, 1, 0, 1, 6, 0, 0, 0, 1, 2, 3, 4, 5, 6]), BadQ, BadQ, BadQ, BadQ),
    row([21, 6], R(NONE, &[21, 6, 1, 0, 0, 1, 0, 1, 6, 0, 1, 2, 3, 4, 5, 6, 1, 6, 0, 1, 2, 3, 4, 5, 6]), R(NONE, &[21, 6, 1, 1, 0, 1, 0, 1, 6, 0, 1, 2, 3, 4, 5, 6]), BadQ, BadQ, BadQ, BadQ),
    row([21, 9], R(NONE, &[21, 9, 1, 0, 0, 1, 0, 6, 0, 0, 0, 6, 0, 0, 0]), R(NONE, &[21, 9, 1, 1, 0, 1, 0, 6, 0, 0, 0]), BadQ, BadQ, BadQ, BadQ),
    row([21, 10], R(NONE, &[21, 10, 1, 0, 0, 1, 0, 6, 0, 6, 0]), R(NONE, &[21, 10, 1, 1, 0, 1, 0, 6, 0]), BadQ, BadQ, BadQ, BadQ),
    row([22, 0], R(NONE, &[22, 1, 40, 2, 0, 0, 0, 1, 5, 0, 0, 0, 1, 0, 1, 5, 0, 0, 0]), BadQ, R(NONE, &[22, 1, 40, 1, 0, 0, 0, 1, 5, 0, 0, 0]), R(NONE, &[]), BadQ, BadQ),
    row([22, 1], R(NONE, &[22, 1, 40, 2, 0, 0, 0, 1, 5, 0, 0, 0, 1, 0, 1, 5, 0, 0, 0]), BadQ, R(NONE, &[22, 1, 40, 1, 0, 0, 0, 1, 5, 0, 0, 0]), R(NONE, &[]), R(PE, &[]), BadQ),
    row([22, 2], R(NONE, &[22, 2, 40, 2, 0, 0, 0, 1, 5, 0, 1, 0, 1, 5, 0]), BadQ, R(NONE, &[22, 2, 40, 1, 0, 0, 0, 1, 5, 0]), R(NONE, &[]), R(PE, &[]), BadQ),
    row([22, 5], R(NONE, &[22, 5, 40, 2, 0, 0, 0, 1, 5, 0, 0, 0, 1, 2, 3, 4, 5, 6, 1, 0, 1, 5, 0, 0, 0, 1, 2, 3, 4, 5, 6]), BadQ, R(NONE, &[22, 5, 40, 1, 0, 0, 0, 1, 5, 0, 0, 0, 1, 2, 3, 4, 5, 6]), R(NONE, &[]), R(PE, &[]), BadQ),
    row([22, 6], R(NONE, &[22, 6, 40, 2, 0, 0, 0, 1, 5, 0, 1, 2, 3, 4, 5, 6, 1, 0, 1, 5, 0, 1, 2, 3, 4, 5, 6]), BadQ, R(NONE, &[22, 6, 40, 1, 0, 0, 0, 1, 5, 0, 1, 2, 3, 4, 5, 6]), R(NONE, &[]), R(PE, &[]), BadQ),
    row([23, 0], R(NONE, &[23, 1, 40, 2, 0, 0, 0, 1, 6, 0, 0, 0, 1, 0, 1, 6, 0, 0, 0]), BadQ, R(NONE, &[23, 1, 40, 1, 0, 0, 0, 1, 6, 0, 0, 0]), R(NONE, &[]), BadQ, BadQ),
    row([23, 1], R(NONE, &[23, 1, 40, 2, 0, 0, 0, 1, 6, 0, 0, 0, 1, 0, 1, 6, 0, 0, 0]), BadQ, R(NONE, &[23, 1, 40, 1, 0, 0, 0, 1, 6, 0, 0, 0]), R(NONE, &[]), R(PE, &[]), BadQ),
    row([23, 2], R(NONE, &[23, 2, 40, 2, 0, 0, 0, 1, 6, 0, 1, 0, 1, 6, 0]), BadQ, R(NONE, &[23, 2, 40, 1, 0, 0, 0, 1, 6, 0]), R(NONE, &[]), R(PE, &[]), BadQ),
    row([23, 5], R(NONE, &[23, 5, 40, 2, 0, 0, 0, 1, 6, 0, 0, 0, 1, 2, 3, 4, 5, 6, 1, 0, 1, 6, 0, 0, 0, 1, 2, 3, 4, 5, 6]), BadQ, R(NONE, &[23, 5, 40, 1, 0, 0, 0, 1, 6, 0, 0, 0, 1, 2, 3, 4, 5, 6]), R(NONE, &[]), R(PE, &[]), BadQ),
    row([23, 6], R(NONE, &[23, 6, 40, 2, 0, 0, 0, 1, 6, 0, 1, 2, 3, 4, 5, 6, 1, 0, 1, 6, 0, 1, 2, 3, 4, 5, 6]), BadQ, R(NONE, &[23, 6, 40, 1, 0, 0, 0, 1, 6, 0, 1, 2, 3, 4, 5, 6]), R(NONE, &[]), R(PE, &[]), BadQ),
    row([30, 0], R(NONE, &[30, 1, 1, 0, 0, 1, 0, 1, 7, 0, 0, 0, 1, 7, 0, 0, 0]), R(NONE, &[30, 1, 1, 1, 0, 1, 0, 1, 7, 0, 0, 0]), BadQ, BadQ, BadQ, BadQ),
    row([30, 1], R(NONE, &[30, 1, 1, 0, 0, 1, 0, 1, 7, 0, 0, 0, 1, 7, 0, 0, 0]), R(NONE, &[30, 1, 1, 1, 0, 1, 0, 1, 7, 0, 0, 0]), BadQ, BadQ, BadQ, BadQ),
    row([30, 2], R(NONE, &[30, 2, 1, 0, 0, 1, 0, 1, 7, 0, 1, 7, 0]), R(NONE, &[30, 2, 1, 1, 0, 1, 0, 1, 7, 0]), BadQ, BadQ, BadQ, BadQ),
    row([30, 3], R(NONE, &[30, 3, 1, 0, 0, 1, 0, 7, 0, 0, 0, 7, 0, 0, 0]), R(NONE, &[30, 3, 1, 1, 0, 1, 0, 7, 0, 0, 0]), BadQ, BadQ, BadQ, BadQ),
    row([30, 4], R(NONE, &[30, 4, 1, 0, 0, 1, 0, 7, 0, 7, 0]), R(NONE, &[30, 4, 1, 1, 0, 1, 0, 7, 0]), BadQ, BadQ, BadQ, BadQ),
    row([30, 5], R(NONE, &[30, 5, 1, 0, 0, 1, 0, 1, 0, 0, 224, 64, 1, 0, 0, 224, 64]), R(NONE, &[30, 5, 1, 1, 0, 1, 0, 1, 0, 0, 224, 64]), BadQ, BadQ, BadQ, BadQ),
    row([30, 6], R(NONE, &[30, 6, 1, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 28, 64, 1, 0, 0, 0, 0, 0, 0, 28, 64]), R(NONE, &[30, 6, 1, 1, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 28, 64]), BadQ, BadQ, BadQ, BadQ),
    row([31, 0], R(OU, &[]), R(OU, &[]), BadQ, BadQ, BadQ, BadQ),
    row([31, 1], R(OU, &[]), R(OU, &[]), BadQ, BadQ, BadQ, BadQ),
    row([31, 2], R(OU, &[]), R(OU, &[]), BadQ, BadQ, BadQ, BadQ),
    row([31, 3], R(OU, &[]), R(OU, &[]), BadQ, BadQ, BadQ, BadQ),
    row([31, 4], R(OU, &[]), R(OU, &[]), BadQ, BadQ, BadQ, BadQ),
    row([31, 5], R(OU, &[]), R(OU, &[]), BadQ, BadQ, BadQ, BadQ),
    row([31, 6], R(OU, &[]), R(OU, &[]), BadQ, BadQ, BadQ, BadQ),
    row([31, 7], R(OU, &[]), R(OU, &[]), BadQ, BadQ, BadQ, BadQ),
    row([31, 8], R(OU, &[]), R(OU, &[]), BadQ, BadQ, BadQ, BadQ),
    row([32, 0], R(NONE, &[32, 1, 40, 2, 0, 0, 0, 1, 7, 0, 0, 0, 1, 0, 1, 7, 0, 0, 0]), BadQ, R(NONE, &[32, 1, 40, 1, 0, 0, 0, 1, 7, 0, 0, 0]), R(NONE, &[]), BadQ, BadQ),
    row([32, 1], R(NONE, &[32, 1, 40, 2, 0, 0, 0, 1, 7, 0, 0, 0, 1, 0, 1, 7, 0, 0, 0]), BadQ, R(NONE, &[32, 1, 40, 1, 0, 0, 0, 1, 7, 0, 0, 0]), R(NONE, &[]), R(PE, &[]), BadQ),
    row([32, 2], R(NONE, &[32, 2, 40, 2, 0, 0, 0, 1, 7, 0, 1, 0, 1, 7, 0]), BadQ, R(NONE, &[32, 2, 40, 1, 0, 0, 0, 1, 7, 0]), R(NONE, &[]), R(PE, &[]), BadQ),
    row([32, 3], R(NONE, &[32, 3, 40, 2, 0, 0, 0, 1, 7, 0, 0, 0, 1, 2, 3, 4, 5, 6, 1, 0, 1, 7, 0, 0, 0, 1, 2, 3, 4, 5, 6]), BadQ, R(NONE, &[32, 3, 40, 1, 0, 0, 0, 1, 7, 0, 0, 0, 1, 2, 3, 4, 5, 6]), R(NONE, &[]), R(PE, &[]), BadQ),
    row([32, 4], R(NONE, &[32, 4, 40, 2, 0, 0, 0, 1, 7, 0, 1, 2, 3, 4, 5, 6, 1, 0, 1, 7, 0, 1, 2, 3, 4, 5, 6]), BadQ, R(NONE, &[32, 4, 40, 1, 0, 0, 0, 1, 7, 0, 1, 2, 3, 4, 5, 6]), R(NONE, &[]), R(PE, &[]), BadQ),
    row([32, 5], R(NONE, &[32, 5, 40, 2, 0, 0, 0, 1, 0, 0, 224, 64, 1, 0, 1, 0, 0, 224, 64]), BadQ, R(NONE, &[32, 5, 40, 1, 0, 0, 0, 1, 0, 0, 224, 64]), R(NONE, &[]), R(PE, &[]), BadQ),
    row([32, 6], R(NONE, &[32, 6, 40, 2, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 28, 64, 1, 0, 1, 0, 0, 0, 0, 0, 0, 28, 64]), BadQ, R(NONE, &[32, 6, 40, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 28, 64]), R(NONE, &[]), R(PE, &[]), BadQ),
    row([32, 7], R(NONE, &[32, 7, 40, 2, 0, 0, 0, 1, 0, 0, 224, 64, 1, 2, 3, 4, 5, 6, 1, 0, 1, 0, 0, 224, 64, 1, 2, 3, 4, 5, 6]), BadQ, R(NONE, &[32, 7, 40, 1, 0, 0, 0, 1, 0, 0, 224, 64, 1, 2, 3, 4, 5, 6]), R(NONE, &[]), R(PE, &[]), BadQ),
    row([32, 8], R(NONE, &[32, 8, 40, 2, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 28, 64, 1, 2, 3, 4, 5, 6, 1, 0, 1, 0, 0, 0, 0, 0, 0, 28, 64, 1, 2, 3, 4, 5, 6]), BadQ, R(NONE, &[32, 8, 40, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 28, 64, 1, 2, 3, 4, 5, 6]), R(NONE, &[]), R(PE, &[]), BadQ),
    row([33, 0], R(OU, &[]), BadQ, R(OU, &[]), R(OU, &[]), BadQ, BadQ),
    row([33, 1], R(OU, &[]), BadQ, R(OU, &[]), R(OU, &[]), R(PE, &[]), BadQ),
    row([33, 2], R(OU, &[]), BadQ, R(OU, &[]), R(OU, &[]), R(PE, &[]), BadQ),
    row([33, 3], R(OU, &[]), BadQ, R(OU, &[]), R(OU, &[]), R(PE, &[]), BadQ),
    row([33, 4], R(OU, &[]), BadQ, R(OU, &[]), R(OU, &[]), R(PE, &[]), BadQ),
    row([33, 5], R(OU, &[]), BadQ, R(OU, &[]), R(OU, &[]), R(PE, &[]), BadQ),
    row([33, 6], R(OU, &[]), BadQ, R(OU, &[]), R(OU, &[]), R(PE, &[]), BadQ),
    row([33, 7], R(OU, &[]), BadQ, R(OU, &[]), R(OU, &[]), R(PE, &[]), BadQ),
    row([33, 8], R(OU, &[]), BadQ, R(OU, &[]), R(OU, &[]), R(PE, &[]), BadQ),
    row([34, 0], R(NONE, &[34, 3, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0]), R(NONE, &[34, 3, 1, 1, 0, 1, 0, 0, 0, 0, 0]), BadQ, BadQ, BadQ, BadQ),
    row([34, 1], R(NONE, &[34, 1, 1, 0, 0, 1, 0, 0, 0, 0, 0]), R(NONE, &[34, 1, 1, 1, 0, 1, 0, 0, 0]), BadQ, BadQ, R(PE, &[]), BadQ),
    row([34, 2], R(NONE, &[34, 2, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0]), R(NONE, &[34, 2, 1, 1, 0, 1, 0, 0, 0, 0, 0]), BadQ, BadQ, R(PE, &[]), BadQ),
    row([34, 3], R(NONE, &[34, 3, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0]), R(NONE, &[34, 3, 1, 1, 0, 1, 0, 0, 0, 0, 0]), BadQ, BadQ, R(PE, &[]), BadQ),
    row([40, 0], R(NONE, &[40, 1, 1, 0, 0, 1, 0, 1, 8, 0, 0, 0, 1, 8, 0, 0, 0]), R(NONE, &[40, 1, 1, 1, 0, 1, 0, 1, 8, 0, 0, 0]), BadQ, BadQ, BadQ, BadQ),
    row([40, 1], R(NONE, &[40, 1, 1, 0, 0, 1, 0, 1, 8, 0, 0, 0, 1, 8, 0, 0, 0]), R(NONE, &[40, 1, 1, 1, 0, 1, 0, 1, 8, 0, 0, 0]), BadQ, BadQ, BadQ, BadQ),
    row([40, 2], R(NONE, &[40, 2, 1, 0, 0, 1, 0, 1, 8, 0, 1, 8, 0]), R(NONE, &[40, 2, 1, 1, 0, 1, 0, 1, 8, 0]), BadQ, BadQ, BadQ, BadQ),
    row([40, 3], R(NONE, &[40, 3, 1, 0, 0, 1, 0, 1, 0, 0, 0, 65, 1, 0, 0, 0, 65]), R(NONE, &[40, 3, 1, 1, 0, 1, 0, 1, 0, 0, 0, 65]), BadQ, BadQ, BadQ, BadQ),
    row([40, 4], R(NONE, &[40, 4, 1, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 32, 64, 1, 0, 0, 0, 0, 0, 0, 32, 64]), R(NONE, &[40, 4, 1, 1, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 32, 64]), BadQ, BadQ, BadQ, BadQ),
    row([41, 1], BadQ, BadQ, BadQ, BadQ, R(OU, &[]), BadQ),
    row([41, 2], BadQ, BadQ, BadQ, BadQ, R(OU, &[]), BadQ),
    row([41, 3], BadQ, BadQ, BadQ, BadQ, R(OU, &[]), BadQ),
    row([41, 4], BadQ, BadQ, BadQ, BadQ, R(OU, &[]), BadQ),
    row([42, 0], R(NONE, &[42, 1, 40, 2, 0, 0, 0, 1, 8, 0, 0, 0, 1, 0, 1, 8, 0, 0, 0]), BadQ, R(NONE, &[42, 1, 40, 1, 0, 0, 0, 1, 8, 0, 0, 0]), R(NONE, &[]), BadQ, BadQ),
    row([42, 1], R(NONE, &[42, 1, 40, 2, 0, 0, 0, 1, 8, 0, 0, 0, 1, 0, 1, 8, 0, 0, 0]), BadQ, R(NONE, &[42, 1, 40, 1, 0, 0, 0, 1, 8, 0, 0, 0]), R(NONE, &[]), R(PE, &[]), BadQ),
    row([42, 2], R(NONE, &[42, 2, 40, 2, 0, 0, 0, 1, 8, 0, 1, 0, 1, 8, 0]), BadQ, R(NONE, &[42, 2, 40, 1, 0, 0, 0, 1, 8, 0]), R(NONE, &[]), R(PE, &[]), BadQ),
    row([42, 3], R(NONE, &[42, 3, 40, 2, 0, 0, 0, 1, 8, 0, 0, 0, 1, 2, 3, 4, 5, 6, 1, 0, 1, 8, 0, 0, 0, 1, 2, 3, 4, 5, 6]), BadQ, R(NONE, &[42, 3, 40, 1, 0, 0, 0, 1, 8, 0, 0, 0, 1, 2, 3, 4, 5, 6]), R(NONE, &[]), R(PE, &[]), BadQ),
    row([42, 4], R(NONE, &[42, 4, 40, 2, 0, 0, 0, 1, 8, 0, 1, 2, 3, 4, 5, 6, 1, 0, 1, 8, 0, 1, 2, 3, 4, 5, 6]), BadQ, R(NONE, &[42, 4, 40, 1, 0, 0, 0, 1, 8, 0, 1, 2, 3, 4, 5, 6]), R(NONE, &[]), R(PE, &[]), BadQ),
    row([42, 5], R(NONE, &[42, 5, 40, 2, 0, 0, 0, 1, 0, 0, 0, 65, 1, 0, 1, 0, 0, 0, 65]), BadQ, R(NONE, &[42, 5, 40, 1, 0, 0, 0, 1, 0, 0, 0, 65]), R(NONE, &[]), R(PE, &[]), BadQ),
    row([42, 6], R(NONE, &[42, 6, 40, 2, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 32, 64, 1, 0, 1, 0, 0, 0, 0, 0, 0, 32, 64]), BadQ, R(NONE, &[42, 6, 40, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 32, 64]), R(NONE, &[]), R(PE, &[]), BadQ),
    row([42, 7], R(NONE, &[42, 7, 40, 2, 0, 0, 0, 1, 0, 0, 0, 65, 1, 2, 3, 4, 5, 6, 1, 0, 1, 0, 0, 0, 65, 1, 2, 3, 4, 5, 6]), BadQ, R(NONE, &[42, 7, 40, 1, 0, 0, 0, 1, 0, 0, 0, 65, 1, 2, 3, 4, 5, 6]), R(NONE, &[]), R(PE, &[]), BadQ),
    row([42, 8], R(NONE, &[42, 8, 40, 2, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 32, 64, 1, 2, 3, 4, 5, 6, 1, 0, 1, 0, 0, 0, 0, 0, 0, 32, 64, 1, 2, 3, 4, 5, 6]), BadQ, R(NONE, &[42, 8, 40, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 32, 64, 1, 2, 3, 4, 5, 6]), R(NONE, &[]), R(PE, &[]), BadQ),
    row([43, 1], R(OU, &[]), BadQ, R(OU, &[]), R(OU, &[]), R(OU, &[]), BadQ),
    row([43, 2], R(OU, &[]), BadQ, R(OU, &[]), R(OU, &[]), R(OU, &[]), BadQ),
    row([43, 3], R(OU, &[]), BadQ, R(OU, &[]), R(OU, &[]), R(OU, &[]), BadQ),
    row([43, 4], R(OU, &[]), BadQ, R(OU, &[]), R(OU, &[]), R(OU, &[]), BadQ),
    row([43, 5], R(OU, &[]), BadQ, R(OU, &[]), R(OU, &[]), R(OU, &[]), BadQ),
    row([43, 6], R(OU, &[]), BadQ, R(OU, &[]), R(OU, &[]), R(OU, &[]), BadQ),
    row([43, 7], R(OU, &[]), BadQ, R(OU, &[]), R(OU, &[]), R(OU, &[]), BadQ),
    row([43, 8], R(OU, &[]), BadQ, R(OU, &[]), R(OU, &[]), R(OU, &[]), BadQ),
    row([50, 1], BadQ, BadQ, R(OU, &[]), R(OU, &[]), BadQ, BadQ),
    row([50, 2], BadQ, BadQ, R(OU, &[]), R(OU, &[]), BadQ, BadQ),
    row([50, 3], BadQ, BadQ, R(OU, &[]), R(OU, &[]), BadQ, BadQ),
    row([50, 4], BadQ, BadQ, R(OU, &[]), R(OU, &[]), BadQ, BadQ),
    row([51, 1], BadQ, BadQ, R(OU, &[]), R(OU, &[]), BadQ, BadQ),
    row([51, 2], BadQ, BadQ, R(OU, &[]), R(OU, &[]), BadQ, BadQ),
    row([52, 1], BadQ, BadQ, R(OU, &[]), R(OU, &[]), BadQ, BadQ),
    row([52, 2], BadQ, BadQ, R(OU, &[]), R(OU, &[]), BadQ, BadQ),
    row([60, 1], R(NONE, &[1, 1, 1, 0, 0, 1, 0, 3, 3, 1, 1, 0, 0, 1, 0, 10, 10, 1, 1, 0, 0, 1, 0, 3, 20, 1, 1, 0, 0, 1, 0, 1, 5, 0, 0, 0, 1, 5, 0, 0, 0, 21, 1, 1, 0, 0, 1, 0, 1, 6, 0, 0, 0, 1, 6, 0, 0, 0, 30, 1, 1, 0, 0, 1, 0, 1, 7, 0, 0, 0, 1, 7, 0, 0, 0, 40, 1, 1, 0, 0, 1, 0, 1, 8, 0, 0, 0, 1, 8, 0, 0, 0]), BadQ, BadQ, BadQ, BadQ, BadQ),
    row([60, 2], R(NONE, &[2, 1, 40, 1, 0, 0, 0, 129, 4, 1, 40, 1, 0, 0, 0, 129, 11, 2, 40, 1, 0, 0, 0, 129, 1, 2, 3, 4, 5, 6, 2, 1, 40, 1, 0, 1, 0, 129, 4, 1, 40, 1, 0, 1, 0, 129, 11, 2, 40, 1, 0, 1, 0, 129, 1, 2, 3, 4, 5, 6]), BadQ, R(NONE, &[2, 1, 40, 1, 0, 0, 0, 129]), R(NONE, &[]), BadQ, BadQ),
    row([60, 3], R(NONE, &[22, 1, 40, 1, 0, 0, 0, 1, 5, 0, 0, 0, 23, 1, 40, 1, 0, 0, 0, 1, 6, 0, 0, 0, 22, 1, 40, 1, 0, 1, 0, 1, 5, 0, 0, 0, 23, 1, 40, 1, 0, 1, 0, 1, 6, 0, 0, 0]), BadQ, R(NONE, &[22, 1, 40, 1, 0, 0, 0, 1, 5, 0, 0, 0]), R(NONE, &[]), BadQ, BadQ),
    row([60, 4], R(NONE, &[32, 1, 40, 1, 0, 0, 0, 1, 7, 0, 0, 0, 42, 1, 40, 1, 0, 0, 0, 1, 8, 0, 0, 0, 111, 1, 40, 1, 0, 0, 0, 9, 32, 1, 40, 1, 0, 1, 0, 1, 7, 0, 0, 0, 42, 1, 40, 1, 0, 1, 0, 1, 8, 0, 0, 0, 111, 1, 40, 1, 0, 1, 0, 9]), BadQ, R(NONE, &[32, 1, 40, 1, 0, 0, 0, 1, 7, 0, 0, 0]), R(NONE, &[]), BadQ, BadQ),
    row([70, 2], BadQ, BadQ, BadQ, BadQ, BadQ, Bad(E::InsufficientBytes)),
    row([70, 3], BadQ, BadQ, BadQ, BadQ, BadQ, Bad(E::InsufficientBytes)),
    row([70, 4], BadQ, BadQ, BadQ, BadQ, BadQ, Bad(E::InsufficientBytes)),
    row([70, 5], BadQ, BadQ, BadQ, BadQ, BadQ, Bad(E::InsufficientBytes)),
    row([70, 6], BadQ, BadQ, BadQ, BadQ, BadQ, Bad(E::InsufficientBytes)),
    row([70, 7], BadQ, BadQ, BadQ, BadQ, BadQ, Bad(E::InsufficientBytes)),
    row([70, 8], BadQ, BadQ, BadQ, BadQ, BadQ, R(OU, &[])),
    row([80, 1], R(OU, &[]), R(OU, &[]), BadQ, BadQ, BadQ, BadQ),
    row([102, 0], R(OU, &[]), R(OU, &[]), BadQ, BadQ, BadQ, BadQ),
    row([102, 1], R(OU, &[]), R(OU, &[]), BadQ, BadQ, BadQ, BadQ),
    row([110, 0], R(NONE, &[110, 1, 1, 0, 0, 1, 0, 9, 9]), R(NONE, &[110, 1, 1, 1, 0, 1, 0, 9]), BadQ, BadQ, BadQ, BadQ),
    row([110, 1], BadQ, BadQ, BadQ, BadQ, BadQ, BadQ),
    row([110, 255], BadQ, BadQ, BadQ, BadQ, BadQ, BadQ),
    row([111, 0], R(NONE, &[111, 1, 40, 2, 0, 0, 0, 9, 1, 0, 9]), BadQ, R(NONE, &[111, 1, 40, 1, 0, 0, 0, 9]), R(NONE, &[]), R(PE, &[]), BadQ),
    row([111, 1], BadQ, BadQ, R(OU, &[]), R(OU, &[]), R(OU, &[]), BadQ),
    row([111, 255], BadQ, BadQ, R(OU, &[]), R(OU, &[]), R(OU, &[]), BadQ),
];

#[test]
fn every_variation_with_each_qualifier() {
    for row in ROWS {
        check_row(row);
    }
}

/// the groups whose variation is a parameter, which the table samples
const DYNAMIC_GROUPS: [u8; 3] = [0, 110, 111];

#[test]
fn table_covers_every_variation_outside_the_dynamic_groups() {
    for variation in Variation::all() {
        let (group, var) = variation.to_group_and_var();
        if DYNAMIC_GROUPS.contains(&group) {
            continue;
        }
        assert!(
            ROWS.iter().any(|row| row.gv == [group, var]),
            "g{group}v{var}"
        );
    }
}

#[test]
fn a_range_with_start_after_stop_is_rejected_for_every_variation() {
    for row in ROWS {
        let gv = row.gv;
        check(
            gv,
            QualifierCode::Range8,
            &[2, 1],
            Bad(E::InvalidRange(2, 1)),
        );
        check(
            gv,
            QualifierCode::Range16,
            &[2, 0, 1, 0],
            Bad(E::InvalidRange(2, 1)),
        );
    }
}

#[rustfmt::skip]
const EDGES: &[([u8; 2], QualifierCode, &[u8], Expect)] = &[
    ([0, 1], QualifierCode::Range8, &[5, 6], R(PE, &[])),
    ([0, 1], QualifierCode::Range8, &[1, 6], R(PE, &[])),
    ([0, 1], QualifierCode::Range8, &[0, 255], R(PE, &[])),
    ([0, 1], QualifierCode::Range16, &[0, 0, 255, 255], R(PE, &[])),
    ([0, 1], QualifierCode::Count8, &[255], BadQ),
    ([0, 1], QualifierCode::Count16, &[255, 255], BadQ),
    ([0, 254], QualifierCode::Range8, &[5, 6], R(PE, &[])),
    ([0, 254], QualifierCode::Range8, &[1, 6], R(PE, &[])),
    ([0, 254], QualifierCode::Range8, &[0, 255], R(PE, &[])),
    ([0, 254], QualifierCode::Range16, &[0, 0, 255, 255], R(PE, &[])),
    ([0, 254], QualifierCode::Count8, &[255], BadQ),
    ([0, 254], QualifierCode::Count16, &[255, 255], BadQ),
    ([1, 0], QualifierCode::Range8, &[5, 6], R(PE, &[])),
    ([1, 0], QualifierCode::Range8, &[1, 6], R(PE, &[1, 1, 1, 1, 0, 1, 0, 1])),
    ([1, 0], QualifierCode::Range8, &[0, 255], R(PE, &[1, 1, 1, 0, 0, 1, 0, 3])),
    ([1, 0], QualifierCode::Range16, &[0, 0, 255, 255], R(PE, &[1, 1, 1, 0, 0, 1, 0, 3])),
    ([1, 0], QualifierCode::Count8, &[255], BadQ),
    ([1, 0], QualifierCode::Count16, &[255, 255], BadQ),
    ([1, 2], QualifierCode::Range8, &[5, 6], R(PE, &[])),
    ([1, 2], QualifierCode::Range8, &[1, 6], R(PE, &[1, 2, 1, 1, 0, 1, 0, 129])),
    ([1, 2], QualifierCode::Range8, &[0, 255], R(PE, &[1, 2, 1, 0, 0, 1, 0, 129, 129])),
    ([1, 2], QualifierCode::Range16, &[0, 0, 255, 255], R(PE, &[1, 2, 1, 0, 0, 1, 0, 129, 129])),
    ([1, 2], QualifierCode::Count8, &[255], BadQ),
    ([1, 2], QualifierCode::Count16, &[255, 255], BadQ),
    ([2, 0], QualifierCode::Range8, &[5, 6], BadQ),
    ([2, 0], QualifierCode::Range8, &[1, 6], BadQ),
    ([2, 0], QualifierCode::Range8, &[0, 255], BadQ),
    ([2, 0], QualifierCode::Range16, &[0, 0, 255, 255], BadQ),
    ([2, 0], QualifierCode::Count8, &[255], R(NONE, &[2, 1, 40, 2, 0, 0, 0, 129, 1, 0, 129])),
    ([2, 0], QualifierCode::Count16, &[255, 255], R(NONE, &[2, 1, 40, 2, 0, 0, 0, 129, 1, 0, 129])),
    ([2, 1], QualifierCode::Range8, &[5, 6], BadQ),
    ([2, 1], QualifierCode::Range8, &[1, 6], BadQ),
    ([2, 1], QualifierCode::Range8, &[0, 255], BadQ),
    ([2, 1], QualifierCode::Range16, &[0, 0, 255, 255], BadQ),
    ([2, 1], QualifierCode::Count8, &[255], R(NONE, &[2, 1, 40, 2, 0, 0, 0, 129, 1, 0, 129])),
    ([2, 1], QualifierCode::Count16, &[255, 255], R(NONE, &[2, 1, 40, 2, 0, 0, 0, 129, 1, 0, 129])),
    ([3, 0], QualifierCode::Range8, &[5, 6], R(PE, &[])),
    ([3, 0], QualifierCode::Range8, &[1, 6], R(PE, &[3, 1, 1, 1, 0, 1, 0, 2])),
    ([3, 0], QualifierCode::Range8, &[0, 255], R(PE, &[3, 1, 1, 0, 0, 1, 0, 10])),
    ([3, 0], QualifierCode::Range16, &[0, 0, 255, 255], R(PE, &[3, 1, 1, 0, 0, 1, 0, 10])),
    ([3, 0], QualifierCode::Count8, &[255], BadQ),
    ([3, 0], QualifierCode::Count16, &[255, 255], BadQ),
    ([10, 2], QualifierCode::Range8, &[5, 6], R(PE, &[])),
    ([10, 2], QualifierCode::Range8, &[1, 6], R(PE, &[10, 2, 1, 1, 0, 1, 0, 129])),
    ([10, 2], QualifierCode::Range8, &[0, 255], R(PE, &[10, 2, 1, 0, 0, 1, 0, 129, 129])),
    ([10, 2], QualifierCode::Range16, &[0, 0, 255, 255], R(PE, &[10, 2, 1, 0, 0, 1, 0, 129, 129])),
    ([10, 2], QualifierCode::Count8, &[255], BadQ),
    ([10, 2], QualifierCode::Count16, &[255, 255], BadQ),
    ([13, 1], QualifierCode::Range8, &[5, 6], BadQ),
    ([13, 1], QualifierCode::Range8, &[1, 6], BadQ),
    ([13, 1], QualifierCode::Range8, &[0, 255], BadQ),
    ([13, 1], QualifierCode::Range16, &[0, 0, 255, 255], BadQ),
    ([13, 1], QualifierCode::Count8, &[255], R(OU, &[])),
    ([13, 1], QualifierCode::Count16, &[255, 255], R(OU, &[])),
    ([20, 1], QualifierCode::Range8, &[5, 6], R(PE, &[])),
    ([20, 1], QualifierCode::Range8, &[1, 6], R(PE, &[20, 1, 1, 1, 0, 1, 0, 1, 5, 0, 0, 0])),
    ([20, 1], QualifierCode::Range8, &[0, 255], R(PE, &[20, 1, 1, 0, 0, 1, 0, 1, 5, 0, 0, 0, 1, 5, 0, 0, 0])),
    ([20, 1], QualifierCode::Range16, &[0, 0, 255, 255], R(PE, &[20, 1, 1, 0, 0, 1, 0, 1, 5, 0, 0, 0, 1, 5, 0, 0, 0])),
    ([20, 1], QualifierCode::Count8, &[255], BadQ),
    ([20, 1], QualifierCode::Count16, &[255, 255], BadQ),
    ([21, 0], QualifierCode::Range8, &[5, 6], R(PE, &[])),
    ([21, 0], QualifierCode::Range8, &[1, 6], R(PE, &[21, 1, 1, 1, 0, 1, 0, 1, 6, 0, 0, 0])),
    ([21, 0], QualifierCode::Range8, &[0, 255], R(PE, &[21, 1, 1, 0, 0, 1, 0, 1, 6, 0, 0, 0, 1, 6, 0, 0, 0])),
    ([21, 0], QualifierCode::Range16, &[0, 0, 255, 255], R(PE, &[21, 1, 1, 0, 0, 1, 0, 1, 6, 0, 0, 0, 1, 6, 0, 0, 0])),
    ([21, 0], QualifierCode::Count8, &[255], BadQ),
    ([21, 0], QualifierCode::Count16, &[255, 255], BadQ),
    ([22, 0], QualifierCode::Range8, &[5, 6], BadQ),
    ([22, 0], QualifierCode::Range8, &[1, 6], BadQ),
    ([22, 0], QualifierCode::Range8, &[0, 255], BadQ),
    ([22, 0], QualifierCode::Range16, &[0, 0, 255, 255], BadQ),
    ([22, 0], QualifierCode::Count8, &[255], R(NONE, &[22, 1, 40, 2, 0, 0, 0, 1, 5, 0, 0, 0, 1, 0, 1, 5, 0, 0, 0])),
    ([22, 0], QualifierCode::Count16, &[255, 255], R(NONE, &[22, 1, 40, 2, 0, 0, 0, 1, 5, 0, 0, 0, 1, 0, 1, 5, 0, 0, 0])),
    ([30, 6], QualifierCode::Range8, &[5, 6], R(PE, &[])),
    ([30, 6], QualifierCode::Range8, &[1, 6], R(PE, &[30, 6, 1, 1, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 28, 64])),
    ([30, 6], QualifierCode::Range8, &[0, 255], R(PE, &[30, 6, 1, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 28, 64, 1, 0, 0, 0, 0, 0, 0, 28, 64])),
    ([30, 6], QualifierCode::Range16, &[0, 0, 255, 255], R(PE, &[30, 6, 1, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 28, 64, 1, 0, 0, 0, 0, 0, 0, 28, 64])),
    ([30, 6], QualifierCode::Count8, &[255], BadQ),
    ([30, 6], QualifierCode::Count16, &[255, 255], BadQ),
    ([31, 0], QualifierCode::Range8, &[5, 6], R(OU, &[])),
    ([31, 0], QualifierCode::Range8, &[1, 6], R(OU, &[])),
    ([31, 0], QualifierCode::Range8, &[0, 255], R(OU, &[])),
    ([31, 0], QualifierCode::Range16, &[0, 0, 255, 255], R(OU, &[])),
    ([31, 0], QualifierCode::Count8, &[255], BadQ),
    ([31, 0], QualifierCode::Count16, &[255, 255], BadQ),
    ([32, 0], QualifierCode::Range8, &[5, 6], BadQ),
    ([32, 0], QualifierCode::Range8, &[1, 6], BadQ),
    ([32, 0], QualifierCode::Range8, &[0, 255], BadQ),
    ([32, 0], QualifierCode::Range16, &[0, 0, 255, 255], BadQ),
    ([32, 0], QualifierCode::Count8, &[255], R(NONE, &[32, 1, 40, 2, 0, 0, 0, 1, 7, 0, 0, 0, 1, 0, 1, 7, 0, 0, 0])),
    ([32, 0], QualifierCode::Count16, &[255, 255], R(NONE, &[32, 1, 40, 2, 0, 0, 0, 1, 7, 0, 0, 0, 1, 0, 1, 7, 0, 0, 0])),
    ([33, 0], QualifierCode::Range8, &[5, 6], BadQ),
    ([33, 0], QualifierCode::Range8, &[1, 6], BadQ),
    ([33, 0], QualifierCode::Range8, &[0, 255], BadQ),
    ([33, 0], QualifierCode::Range16, &[0, 0, 255, 255], BadQ),
    ([33, 0], QualifierCode::Count8, &[255], R(OU, &[])),
    ([33, 0], QualifierCode::Count16, &[255, 255], R(OU, &[])),
    ([34, 0], QualifierCode::Range8, &[5, 6], R(PE, &[])),
    ([34, 0], QualifierCode::Range8, &[1, 6], R(PE, &[34, 3, 1, 1, 0, 1, 0, 0, 0, 0, 0])),
    ([34, 0], QualifierCode::Range8, &[0, 255], R(PE, &[34, 3, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0])),
    ([34, 0], QualifierCode::Range16, &[0, 0, 255, 255], R(PE, &[34, 3, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0])),
    ([34, 0], QualifierCode::Count8, &[255], BadQ),
    ([34, 0], QualifierCode::Count16, &[255, 255], BadQ),
    ([34, 1], QualifierCode::Range8, &[5, 6], R(PE, &[])),
    ([34, 1], QualifierCode::Range8, &[1, 6], R(PE, &[34, 1, 1, 1, 0, 1, 0, 0, 0])),
    ([34, 1], QualifierCode::Range8, &[0, 255], R(PE, &[34, 1, 1, 0, 0, 1, 0, 0, 0, 0, 0])),
    ([34, 1], QualifierCode::Range16, &[0, 0, 255, 255], R(PE, &[34, 1, 1, 0, 0, 1, 0, 0, 0, 0, 0])),
    ([34, 1], QualifierCode::Count8, &[255], BadQ),
    ([34, 1], QualifierCode::Count16, &[255, 255], BadQ),
    ([40, 0], QualifierCode::Range8, &[5, 6], R(PE, &[])),
    ([40, 0], QualifierCode::Range8, &[1, 6], R(PE, &[40, 1, 1, 1, 0, 1, 0, 1, 8, 0, 0, 0])),
    ([40, 0], QualifierCode::Range8, &[0, 255], R(PE, &[40, 1, 1, 0, 0, 1, 0, 1, 8, 0, 0, 0, 1, 8, 0, 0, 0])),
    ([40, 0], QualifierCode::Range16, &[0, 0, 255, 255], R(PE, &[40, 1, 1, 0, 0, 1, 0, 1, 8, 0, 0, 0, 1, 8, 0, 0, 0])),
    ([40, 0], QualifierCode::Count8, &[255], BadQ),
    ([40, 0], QualifierCode::Count16, &[255, 255], BadQ),
    ([42, 1], QualifierCode::Range8, &[5, 6], BadQ),
    ([42, 1], QualifierCode::Range8, &[1, 6], BadQ),
    ([42, 1], QualifierCode::Range8, &[0, 255], BadQ),
    ([42, 1], QualifierCode::Range16, &[0, 0, 255, 255], BadQ),
    ([42, 1], QualifierCode::Count8, &[255], R(NONE, &[42, 1, 40, 2, 0, 0, 0, 1, 8, 0, 0, 0, 1, 0, 1, 8, 0, 0, 0])),
    ([42, 1], QualifierCode::Count16, &[255, 255], R(NONE, &[42, 1, 40, 2, 0, 0, 0, 1, 8, 0, 0, 0, 1, 0, 1, 8, 0, 0, 0])),
    ([43, 1], QualifierCode::Range8, &[5, 6], BadQ),
    ([43, 1], QualifierCode::Range8, &[1, 6], BadQ),
    ([43, 1], QualifierCode::Range8, &[0, 255], BadQ),
    ([43, 1], QualifierCode::Range16, &[0, 0, 255, 255], BadQ),
    ([43, 1], QualifierCode::Count8, &[255], R(OU, &[])),
    ([43, 1], QualifierCode::Count16, &[255, 255], R(OU, &[])),
    ([50, 1], QualifierCode::Range8, &[5, 6], BadQ),
    ([50, 1], QualifierCode::Range8, &[1, 6], BadQ),
    ([50, 1], QualifierCode::Range8, &[0, 255], BadQ),
    ([50, 1], QualifierCode::Range16, &[0, 0, 255, 255], BadQ),
    ([50, 1], QualifierCode::Count8, &[255], R(OU, &[])),
    ([50, 1], QualifierCode::Count16, &[255, 255], R(OU, &[])),
    ([60, 1], QualifierCode::Range8, &[5, 6], BadQ),
    ([60, 1], QualifierCode::Range8, &[1, 6], BadQ),
    ([60, 1], QualifierCode::Range8, &[0, 255], BadQ),
    ([60, 1], QualifierCode::Range16, &[0, 0, 255, 255], BadQ),
    ([60, 1], QualifierCode::Count8, &[255], BadQ),
    ([60, 1], QualifierCode::Count16, &[255, 255], BadQ),
    ([60, 2], QualifierCode::Range8, &[5, 6], BadQ),
    ([60, 2], QualifierCode::Range8, &[1, 6], BadQ),
    ([60, 2], QualifierCode::Range8, &[0, 255], BadQ),
    ([60, 2], QualifierCode::Range16, &[0, 0, 255, 255], BadQ),
    ([60, 2], QualifierCode::Count8, &[255], R(NONE, &[2, 1, 40, 1, 0, 0, 0, 129, 4, 1, 40, 1, 0, 0, 0, 129, 11, 2, 40, 1, 0, 0, 0, 129, 1, 2, 3, 4, 5, 6, 2, 1, 40, 1, 0, 1, 0, 129, 4, 1, 40, 1, 0, 1, 0, 129, 11, 2, 40, 1, 0, 1, 0, 129, 1, 2, 3, 4, 5, 6])),
    ([60, 2], QualifierCode::Count16, &[255, 255], R(NONE, &[2, 1, 40, 1, 0, 0, 0, 129, 4, 1, 40, 1, 0, 0, 0, 129, 11, 2, 40, 1, 0, 0, 0, 129, 1, 2, 3, 4, 5, 6, 2, 1, 40, 1, 0, 1, 0, 129, 4, 1, 40, 1, 0, 1, 0, 129, 11, 2, 40, 1, 0, 1, 0, 129, 1, 2, 3, 4, 5, 6])),
    ([60, 3], QualifierCode::Range8, &[5, 6], BadQ),
    ([60, 3], QualifierCode::Range8, &[1, 6], BadQ),
    ([60, 3], QualifierCode::Range8, &[0, 255], BadQ),
    ([60, 3], QualifierCode::Range16, &[0, 0, 255, 255], BadQ),
    ([60, 3], QualifierCode::Count8, &[255], R(NONE, &[22, 1, 40, 1, 0, 0, 0, 1, 5, 0, 0, 0, 23, 1, 40, 1, 0, 0, 0, 1, 6, 0, 0, 0, 22, 1, 40, 1, 0, 1, 0, 1, 5, 0, 0, 0, 23, 1, 40, 1, 0, 1, 0, 1, 6, 0, 0, 0])),
    ([60, 3], QualifierCode::Count16, &[255, 255], R(NONE, &[22, 1, 40, 1, 0, 0, 0, 1, 5, 0, 0, 0, 23, 1, 40, 1, 0, 0, 0, 1, 6, 0, 0, 0, 22, 1, 40, 1, 0, 1, 0, 1, 5, 0, 0, 0, 23, 1, 40, 1, 0, 1, 0, 1, 6, 0, 0, 0])),
    ([60, 4], QualifierCode::Range8, &[5, 6], BadQ),
    ([60, 4], QualifierCode::Range8, &[1, 6], BadQ),
    ([60, 4], QualifierCode::Range8, &[0, 255], BadQ),
    ([60, 4], QualifierCode::Range16, &[0, 0, 255, 255], BadQ),
    ([60, 4], QualifierCode::Count8, &[255], R(NONE, &[32, 1, 40, 1, 0, 0, 0, 1, 7, 0, 0, 0, 42, 1, 40, 1, 0, 0, 0, 1, 8, 0, 0, 0, 111, 1, 40, 1, 0, 0, 0, 9, 32, 1, 40, 1, 0, 1, 0, 1, 7, 0, 0, 0, 42, 1, 40, 1, 0, 1, 0, 1, 8, 0, 0, 0, 111, 1, 40, 1, 0, 1, 0, 9])),
    ([60, 4], QualifierCode::Count16, &[255, 255], R(NONE, &[32, 1, 40, 1, 0, 0, 0, 1, 7, 0, 0, 0, 42, 1, 40, 1, 0, 0, 0, 1, 8, 0, 0, 0, 111, 1, 40, 1, 0, 0, 0, 9, 32, 1, 40, 1, 0, 1, 0, 1, 7, 0, 0, 0, 42, 1, 40, 1, 0, 1, 0, 1, 8, 0, 0, 0, 111, 1, 40, 1, 0, 1, 0, 9])),
    ([80, 1], QualifierCode::Range8, &[5, 6], R(OU, &[])),
    ([80, 1], QualifierCode::Range8, &[1, 6], R(OU, &[])),
    ([80, 1], QualifierCode::Range8, &[0, 255], R(OU, &[])),
    ([80, 1], QualifierCode::Range16, &[0, 0, 255, 255], R(OU, &[])),
    ([80, 1], QualifierCode::Count8, &[255], BadQ),
    ([80, 1], QualifierCode::Count16, &[255, 255], BadQ),
    ([102, 0], QualifierCode::Range8, &[5, 6], R(OU, &[])),
    ([102, 0], QualifierCode::Range8, &[1, 6], R(OU, &[])),
    ([102, 0], QualifierCode::Range8, &[0, 255], R(OU, &[])),
    ([102, 0], QualifierCode::Range16, &[0, 0, 255, 255], R(OU, &[])),
    ([102, 0], QualifierCode::Count8, &[255], BadQ),
    ([102, 0], QualifierCode::Count16, &[255, 255], BadQ),
    ([110, 0], QualifierCode::Range8, &[5, 6], R(PE, &[])),
    ([110, 0], QualifierCode::Range8, &[1, 6], R(PE, &[110, 1, 1, 1, 0, 1, 0, 9])),
    ([110, 0], QualifierCode::Range8, &[0, 255], R(PE, &[110, 1, 1, 0, 0, 1, 0, 9, 9])),
    ([110, 0], QualifierCode::Range16, &[0, 0, 255, 255], R(PE, &[110, 1, 1, 0, 0, 1, 0, 9, 9])),
    ([110, 0], QualifierCode::Count8, &[255], BadQ),
    ([110, 0], QualifierCode::Count16, &[255, 255], BadQ),
    ([110, 1], QualifierCode::Range8, &[5, 6], BadQ),
    ([110, 1], QualifierCode::Range8, &[1, 6], BadQ),
    ([110, 1], QualifierCode::Range8, &[0, 255], BadQ),
    ([110, 1], QualifierCode::Range16, &[0, 0, 255, 255], BadQ),
    ([110, 1], QualifierCode::Count8, &[255], BadQ),
    ([110, 1], QualifierCode::Count16, &[255, 255], BadQ),
    ([111, 0], QualifierCode::Range8, &[5, 6], BadQ),
    ([111, 0], QualifierCode::Range8, &[1, 6], BadQ),
    ([111, 0], QualifierCode::Range8, &[0, 255], BadQ),
    ([111, 0], QualifierCode::Range16, &[0, 0, 255, 255], BadQ),
    ([111, 0], QualifierCode::Count8, &[255], R(NONE, &[111, 1, 40, 2, 0, 0, 0, 9, 1, 0, 9])),
    ([111, 0], QualifierCode::Count16, &[255, 255], R(NONE, &[111, 1, 40, 2, 0, 0, 0, 9, 1, 0, 9])),
    ([111, 1], QualifierCode::Range8, &[5, 6], BadQ),
    ([111, 1], QualifierCode::Range8, &[1, 6], BadQ),
    ([111, 1], QualifierCode::Range8, &[0, 255], BadQ),
    ([111, 1], QualifierCode::Range16, &[0, 0, 255, 255], BadQ),
    ([111, 1], QualifierCode::Count8, &[255], R(OU, &[])),
    ([111, 1], QualifierCode::Count16, &[255, 255], R(OU, &[])),
];

#[test]
fn edge_ranges_and_counts_of_representative_variations() {
    for (gv, qualifier, rest, expect) in EDGES {
        check(*gv, *qualifier, rest, *expect);
    }
}
