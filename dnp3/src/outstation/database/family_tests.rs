//! Pins for each measurement family, driven only through READ request bytes and the public
//! database API: the bytes of every event and static variation, update, deadband and class 0
//! dispatch, packed-bit promotion, CTO boundaries and octet-string lengths
//!
//! The expected bytes are decimal. In object headers, qualifier 1 is a 16-bit start and stop,
//! 7 an 8-bit count and 40 (0x28) a 16-bit count with 16-bit indices

#![expect(
    clippy::arithmetic_side_effects,
    reason = "tests compute event times and expected sizes"
)]

use scursor::WriteCursor;

use crate::app::parse::options::ParseOptions;
use crate::app::parse::parser::HeaderCollection;
use crate::app::{FunctionCode, Iin2};
use crate::outstation::config::OutstationConfig;
use crate::outstation::database::*;
use crate::outstation::{BufferState, ClassCount, OutstationApplication, TypeCount};

/// the index of the point in most tests
const INDEX: u16 = 3;
/// 48-bit time, distinct in each byte
const TIME: u64 = 0x0000_0605_0403_0201;

const CLASS_0: [u8; 3] = [60, 1, 0x06];
const CLASS_1: [u8; 3] = [60, 2, 0x06];

const NONE: Iin2 = Iin2::new(0);

fn time() -> Time {
    Time::synchronized(TIME)
}

fn handle_with(class_zero: ClassZeroConfig, events: EventBufferConfig) -> DatabaseHandle {
    DatabaseHandle::new(
        OutstationConfig::DEFAULT_MAX_READ_REQUEST_HEADERS,
        class_zero,
        events,
    )
}

fn new_handle() -> DatabaseHandle {
    handle_with(ClassZeroConfig::default(), EventBufferConfig::all_types(10))
}

/// select the object headers of a READ, then write everything selected into one fragment
fn read(handle: &mut DatabaseHandle, objects: &[u8]) -> (Iin2, Vec<u8>) {
    let headers = HeaderCollection::parse(
        ParseOptions::parse_everything(),
        FunctionCode::Read,
        objects,
    )
    .unwrap();
    handle.reset();
    let iin2 = handle.select(&headers);
    let mut buffer = [0u8; 4096];
    let mut cursor = WriteCursor::new(&mut buffer);
    let info = handle.write_response_headers(&mut cursor);
    assert!(info.complete);
    (iin2, cursor.written().to_vec())
}

/// set up a database, then READ it
fn respond(setup: impl Fn(&mut Database), objects: &[u8]) -> (Iin2, Vec<u8>) {
    let mut handle = new_handle();
    handle.transaction(|db| setup(db));
    read(&mut handle, objects)
}

fn ok(bytes: &[u8]) -> (Iin2, Vec<u8>) {
    (NONE, bytes.to_vec())
}

/// the bytes of the event at INDEX in each variation: as the point's default in a class 1
/// READ, then as the variation a READ asks for when the default is `other`
fn check_event_variations<V: Copy>(
    setup: impl Fn(&mut Database, V),
    other: V,
    cases: &[(V, [u8; 2], &[u8])],
) {
    for (variation, [group, var], expected) in cases {
        assert_eq!(
            respond(|db| setup(db, *variation), &CLASS_1),
            ok(expected),
            "default g{group}v{var}"
        );
        assert_eq!(
            respond(|db| setup(db, other), &[*group, *var, 0x06]),
            ok(expected),
            "specific g{group}v{var}"
        );
    }
}

/// the bytes of the static value at INDEX in each variation: as the point's default in a
/// class 0 READ, then as the variation a ranged READ asks for when the default is `other`
fn check_static_variations<V: Copy>(
    setup: impl Fn(&mut Database, V),
    other: V,
    cases: &[(V, [u8; 2], &[u8])],
) {
    for (variation, [group, var], expected) in cases {
        assert_eq!(
            respond(|db| setup(db, *variation), &CLASS_0),
            ok(expected),
            "default g{group}v{var}"
        );
        let index = u8::try_from(INDEX).unwrap();
        assert_eq!(
            respond(|db| setup(db, other), &[*group, *var, 0x00, index, index]),
            ok(expected),
            "specific g{group}v{var}"
        );
    }
}

// ---------- event variations ----------

#[test]
#[rustfmt::skip]
fn binary_input_event_variations() {
    use EventBinaryInputVariation::*;
    let value = BinaryInput::new(true, Flags::ONLINE, time());
    check_event_variations(
        |db, e_var| {
            db.add(INDEX, Some(EventClass::Class1), BinaryInputConfig { e_var, ..Default::default() });
            db.update(INDEX, &value, UpdateOptions::default());
        },
        Group2Var2,
        &[
            (Group2Var1, [2, 1], &[2, 1, 40, 1, 0, 3, 0, 129]),
            (Group2Var2, [2, 2], &[2, 2, 40, 1, 0, 3, 0, 129, 1, 2, 3, 4, 5, 6]),
            (Group2Var3, [2, 3], &[51, 1, 7, 1, 1, 2, 3, 4, 5, 6, 2, 3, 40, 1, 0, 3, 0, 129, 0, 0]),
        ],
    );
}

#[test]
#[rustfmt::skip]
fn double_bit_binary_input_event_variations() {
    use EventDoubleBitBinaryInputVariation::*;
    let value = DoubleBitBinaryInput::new(DoubleBit::DeterminedOn, Flags::ONLINE, time());
    check_event_variations(
        |db, e_var| {
            db.add(INDEX, Some(EventClass::Class1), DoubleBitBinaryInputConfig { e_var, ..Default::default() });
            db.update(INDEX, &value, UpdateOptions::default());
        },
        Group4Var2,
        &[
            (Group4Var1, [4, 1], &[4, 1, 40, 1, 0, 3, 0, 129]),
            (Group4Var2, [4, 2], &[4, 2, 40, 1, 0, 3, 0, 129, 1, 2, 3, 4, 5, 6]),
            (Group4Var3, [4, 3], &[51, 1, 7, 1, 1, 2, 3, 4, 5, 6, 4, 3, 40, 1, 0, 3, 0, 129, 0, 0]),
        ],
    );
}

#[test]
#[rustfmt::skip]
fn binary_output_status_event_variations() {
    use EventBinaryOutputStatusVariation::*;
    let value = BinaryOutputStatus::new(true, Flags::ONLINE, time());
    check_event_variations(
        |db, e_var| {
            db.add(INDEX, Some(EventClass::Class1), BinaryOutputStatusConfig { e_var, ..Default::default() });
            db.update(INDEX, &value, UpdateOptions::default());
        },
        Group11Var1,
        &[
            (Group11Var1, [11, 1], &[11, 1, 40, 1, 0, 3, 0, 129]),
            (Group11Var2, [11, 2], &[11, 2, 40, 1, 0, 3, 0, 129, 1, 2, 3, 4, 5, 6]),
        ],
    );
}

#[test]
#[rustfmt::skip]
fn counter_event_variations() {
    use EventCounterVariation::*;
    let value = Counter::new(0x0D0C_0B0A, Flags::ONLINE, time());
    check_event_variations(
        |db, e_var| {
            db.add(INDEX, Some(EventClass::Class1), CounterConfig { e_var, ..Default::default() });
            db.update(INDEX, &value, UpdateOptions::default());
        },
        Group22Var1,
        &[
            (Group22Var1, [22, 1], &[22, 1, 40, 1, 0, 3, 0, 1, 10, 11, 12, 13]),
            (Group22Var2, [22, 2], &[22, 2, 40, 1, 0, 3, 0, 1, 10, 11]),
            (Group22Var5, [22, 5], &[22, 5, 40, 1, 0, 3, 0, 1, 10, 11, 12, 13, 1, 2, 3, 4, 5, 6]),
            (Group22Var6, [22, 6], &[22, 6, 40, 1, 0, 3, 0, 1, 10, 11, 1, 2, 3, 4, 5, 6]),
        ],
    );
}

#[test]
#[rustfmt::skip]
fn frozen_counter_event_variations() {
    use EventFrozenCounterVariation::*;
    let value = FrozenCounter::new(0x0D0C_0B0A, Flags::ONLINE, time());
    check_event_variations(
        |db, e_var| {
            db.add(INDEX, Some(EventClass::Class1), FrozenCounterConfig { e_var, ..Default::default() });
            db.update(INDEX, &value, UpdateOptions::default());
        },
        Group23Var1,
        &[
            (Group23Var1, [23, 1], &[23, 1, 40, 1, 0, 3, 0, 1, 10, 11, 12, 13]),
            (Group23Var2, [23, 2], &[23, 2, 40, 1, 0, 3, 0, 1, 10, 11]),
            (Group23Var5, [23, 5], &[23, 5, 40, 1, 0, 3, 0, 1, 10, 11, 12, 13, 1, 2, 3, 4, 5, 6]),
            (Group23Var6, [23, 6], &[23, 6, 40, 1, 0, 3, 0, 1, 10, 11, 1, 2, 3, 4, 5, 6]),
        ],
    );
}

#[test]
#[rustfmt::skip]
fn analog_input_event_variations() {
    use EventAnalogInputVariation::*;
    let value = AnalogInput::new(1000.25, Flags::ONLINE, time());
    check_event_variations(
        |db, e_var| {
            db.add(INDEX, Some(EventClass::Class1), AnalogInputConfig { e_var, ..Default::default() });
            db.update(INDEX, &value, UpdateOptions::default());
        },
        Group32Var1,
        &[
            (Group32Var1, [32, 1], &[32, 1, 40, 1, 0, 3, 0, 1, 232, 3, 0, 0]),
            (Group32Var2, [32, 2], &[32, 2, 40, 1, 0, 3, 0, 1, 232, 3]),
            (Group32Var3, [32, 3], &[32, 3, 40, 1, 0, 3, 0, 1, 232, 3, 0, 0, 1, 2, 3, 4, 5, 6]),
            (Group32Var4, [32, 4], &[32, 4, 40, 1, 0, 3, 0, 1, 232, 3, 1, 2, 3, 4, 5, 6]),
            (Group32Var5, [32, 5], &[32, 5, 40, 1, 0, 3, 0, 1, 0, 16, 122, 68]),
            (Group32Var6, [32, 6], &[32, 6, 40, 1, 0, 3, 0, 1, 0, 0, 0, 0, 0, 66, 143, 64]),
            (Group32Var7, [32, 7], &[32, 7, 40, 1, 0, 3, 0, 1, 0, 16, 122, 68, 1, 2, 3, 4, 5, 6]),
            (Group32Var8, [32, 8], &[32, 8, 40, 1, 0, 3, 0, 1, 0, 0, 0, 0, 0, 66, 143, 64, 1, 2, 3, 4, 5, 6]),
        ],
    );
}

#[test]
#[rustfmt::skip]
fn analog_output_status_event_variations() {
    use EventAnalogOutputStatusVariation::*;
    let value = AnalogOutputStatus::new(1000.25, Flags::ONLINE, time());
    check_event_variations(
        |db, e_var| {
            db.add(INDEX, Some(EventClass::Class1), AnalogOutputStatusConfig { e_var, ..Default::default() });
            db.update(INDEX, &value, UpdateOptions::default());
        },
        Group42Var1,
        &[
            (Group42Var1, [42, 1], &[42, 1, 40, 1, 0, 3, 0, 1, 232, 3, 0, 0]),
            (Group42Var2, [42, 2], &[42, 2, 40, 1, 0, 3, 0, 1, 232, 3]),
            (Group42Var3, [42, 3], &[42, 3, 40, 1, 0, 3, 0, 1, 232, 3, 0, 0, 1, 2, 3, 4, 5, 6]),
            (Group42Var4, [42, 4], &[42, 4, 40, 1, 0, 3, 0, 1, 232, 3, 1, 2, 3, 4, 5, 6]),
            (Group42Var5, [42, 5], &[42, 5, 40, 1, 0, 3, 0, 1, 0, 16, 122, 68]),
            (Group42Var6, [42, 6], &[42, 6, 40, 1, 0, 3, 0, 1, 0, 0, 0, 0, 0, 66, 143, 64]),
            (Group42Var7, [42, 7], &[42, 7, 40, 1, 0, 3, 0, 1, 0, 16, 122, 68, 1, 2, 3, 4, 5, 6]),
            (Group42Var8, [42, 8], &[42, 8, 40, 1, 0, 3, 0, 1, 0, 0, 0, 0, 0, 66, 143, 64, 1, 2, 3, 4, 5, 6]),
        ],
    );
}

// ---------- static variations ----------

#[test]
#[rustfmt::skip]
fn binary_input_static_variations() {
    use StaticBinaryInputVariation::*;
    let value = BinaryInput::new(true, Flags::ONLINE, time());
    check_static_variations(
        |db, s_var| {
            db.add(INDEX, None, BinaryInputConfig { s_var, ..Default::default() });
            db.update(INDEX, &value, UpdateOptions::no_event());
        },
        Group1Var2,
        &[
            (Group1Var1, [1, 1], &[1, 1, 1, 3, 0, 3, 0, 1]),
            (Group1Var2, [1, 2], &[1, 2, 1, 3, 0, 3, 0, 129]),
        ],
    );
}

#[test]
#[rustfmt::skip]
fn double_bit_binary_input_static_variations() {
    use StaticDoubleBitBinaryInputVariation::*;
    let value = DoubleBitBinaryInput::new(DoubleBit::DeterminedOn, Flags::ONLINE, time());
    check_static_variations(
        |db, s_var| {
            db.add(INDEX, None, DoubleBitBinaryInputConfig { s_var, ..Default::default() });
            db.update(INDEX, &value, UpdateOptions::no_event());
        },
        Group3Var2,
        &[
            (Group3Var1, [3, 1], &[3, 1, 1, 3, 0, 3, 0, 2]),
            (Group3Var2, [3, 2], &[3, 2, 1, 3, 0, 3, 0, 129]),
        ],
    );
}

#[test]
#[rustfmt::skip]
fn binary_output_status_static_variations() {
    use StaticBinaryOutputStatusVariation::*;
    let value = BinaryOutputStatus::new(true, Flags::ONLINE, time());
    check_static_variations(
        |db, s_var| {
            db.add(INDEX, None, BinaryOutputStatusConfig { s_var, ..Default::default() });
            db.update(INDEX, &value, UpdateOptions::no_event());
        },
        Group10Var2,
        &[
            (Group10Var1, [10, 1], &[10, 1, 1, 3, 0, 3, 0, 1]),
            (Group10Var2, [10, 2], &[10, 2, 1, 3, 0, 3, 0, 129]),
        ],
    );
}

#[test]
#[rustfmt::skip]
fn counter_static_variations() {
    use StaticCounterVariation::*;
    let value = Counter::new(0x0D0C_0B0A, Flags::ONLINE, time());
    check_static_variations(
        |db, s_var| {
            db.add(INDEX, None, CounterConfig { s_var, ..Default::default() });
            db.update(INDEX, &value, UpdateOptions::no_event());
        },
        Group20Var1,
        &[
            (Group20Var1, [20, 1], &[20, 1, 1, 3, 0, 3, 0, 1, 10, 11, 12, 13]),
            (Group20Var2, [20, 2], &[20, 2, 1, 3, 0, 3, 0, 1, 10, 11]),
            (Group20Var5, [20, 5], &[20, 5, 1, 3, 0, 3, 0, 10, 11, 12, 13]),
            (Group20Var6, [20, 6], &[20, 6, 1, 3, 0, 3, 0, 10, 11]),
        ],
    );
}

#[test]
#[rustfmt::skip]
fn frozen_counter_static_variations() {
    use StaticFrozenCounterVariation::*;
    let value = FrozenCounter::new(0x0D0C_0B0A, Flags::ONLINE, time());
    check_static_variations(
        |db, s_var| {
            db.add(INDEX, None, FrozenCounterConfig { s_var, ..Default::default() });
            db.update(INDEX, &value, UpdateOptions::no_event());
        },
        Group21Var1,
        &[
            (Group21Var1, [21, 1], &[21, 1, 1, 3, 0, 3, 0, 1, 10, 11, 12, 13]),
            (Group21Var2, [21, 2], &[21, 2, 1, 3, 0, 3, 0, 1, 10, 11]),
            (Group21Var5, [21, 5], &[21, 5, 1, 3, 0, 3, 0, 1, 10, 11, 12, 13, 1, 2, 3, 4, 5, 6]),
            (Group21Var6, [21, 6], &[21, 6, 1, 3, 0, 3, 0, 1, 10, 11, 1, 2, 3, 4, 5, 6]),
            (Group21Var9, [21, 9], &[21, 9, 1, 3, 0, 3, 0, 10, 11, 12, 13]),
            (Group21Var10, [21, 10], &[21, 10, 1, 3, 0, 3, 0, 10, 11]),
        ],
    );
}

#[test]
#[rustfmt::skip]
fn analog_input_static_variations() {
    use StaticAnalogInputVariation::*;
    let value = AnalogInput::new(1000.25, Flags::ONLINE, time());
    check_static_variations(
        |db, s_var| {
            db.add(INDEX, None, AnalogInputConfig { s_var, ..Default::default() });
            db.update(INDEX, &value, UpdateOptions::no_event());
        },
        Group30Var1,
        &[
            (Group30Var1, [30, 1], &[30, 1, 1, 3, 0, 3, 0, 1, 232, 3, 0, 0]),
            (Group30Var2, [30, 2], &[30, 2, 1, 3, 0, 3, 0, 1, 232, 3]),
            (Group30Var3, [30, 3], &[30, 3, 1, 3, 0, 3, 0, 232, 3, 0, 0]),
            (Group30Var4, [30, 4], &[30, 4, 1, 3, 0, 3, 0, 232, 3]),
            (Group30Var5, [30, 5], &[30, 5, 1, 3, 0, 3, 0, 1, 0, 16, 122, 68]),
            (Group30Var6, [30, 6], &[30, 6, 1, 3, 0, 3, 0, 1, 0, 0, 0, 0, 0, 66, 143, 64]),
        ],
    );
}

#[test]
#[rustfmt::skip]
fn analog_output_status_static_variations() {
    use StaticAnalogOutputStatusVariation::*;
    let value = AnalogOutputStatus::new(1000.25, Flags::ONLINE, time());
    check_static_variations(
        |db, s_var| {
            db.add(INDEX, None, AnalogOutputStatusConfig { s_var, ..Default::default() });
            db.update(INDEX, &value, UpdateOptions::no_event());
        },
        Group40Var1,
        &[
            (Group40Var1, [40, 1], &[40, 1, 1, 3, 0, 3, 0, 1, 232, 3, 0, 0]),
            (Group40Var2, [40, 2], &[40, 2, 1, 3, 0, 3, 0, 1, 232, 3]),
            (Group40Var3, [40, 3], &[40, 3, 1, 3, 0, 3, 0, 1, 0, 16, 122, 68]),
            (Group40Var4, [40, 4], &[40, 4, 1, 3, 0, 3, 0, 1, 0, 0, 0, 0, 0, 66, 143, 64]),
        ],
    );
}

#[test]
#[rustfmt::skip]
fn analog_input_dead_band_variations() {
    let setup = |db: &mut Database| {
        db.add(INDEX, None, AnalogInputConfig { deadband: 1000.25, ..Default::default() });
    };
    // the default variation is the float
    assert_eq!(respond(setup, &[34, 0, 0x06]), ok(&[34, 3, 1, 3, 0, 3, 0, 0, 16, 122, 68]));
    // the integers round
    assert_eq!(respond(setup, &[34, 1, 0x06]), ok(&[34, 1, 1, 3, 0, 3, 0, 232, 3]));
    assert_eq!(respond(setup, &[34, 2, 0x06]), ok(&[34, 2, 1, 3, 0, 3, 0, 232, 3, 0, 0]));
    assert_eq!(respond(setup, &[34, 3, 0x00, 3, 3]), ok(&[34, 3, 1, 3, 0, 3, 0, 0, 16, 122, 68]));
}

// ---------- dispatch by type ----------

/// one measurement family, as the tests below drive it
struct Family {
    static_group: u8,
    event_group: u8,
    add: fn(&mut Database, Option<EventClass>) -> bool,
    /// update the point at INDEX to a value that differs from the last by `step`
    update: fn(&mut Database, u8, UpdateOptions) -> UpdateInfo,
    remove: fn(&mut Database) -> bool,
    exists: fn(&Database) -> bool,
}

fn octet_string(len: usize, fill: u8) -> OctetString {
    OctetString::new(&vec![fill; len]).unwrap()
}

fn double_bit(step: u8) -> DoubleBit {
    if step % 2 == 1 {
        DoubleBit::DeterminedOn
    } else {
        DoubleBit::DeterminedOff
    }
}

const FAMILIES: [Family; 8] = [
    Family {
        static_group: 1,
        event_group: 2,
        add: |db, class| db.add(INDEX, class, BinaryInputConfig::default()),
        update: |db, step, options| {
            let value = BinaryInput::new(step % 2 == 1, Flags::ONLINE, time());
            db.update2(INDEX, &value, options)
        },
        remove: |db| Remove::<BinaryInput>::remove(db, INDEX),
        exists: |db| Get::<BinaryInput>::get(db, INDEX).is_some(),
    },
    Family {
        static_group: 3,
        event_group: 4,
        add: |db, class| db.add(INDEX, class, DoubleBitBinaryInputConfig::default()),
        update: |db, step, options| {
            let value = DoubleBitBinaryInput::new(double_bit(step), Flags::ONLINE, time());
            db.update2(INDEX, &value, options)
        },
        remove: |db| Remove::<DoubleBitBinaryInput>::remove(db, INDEX),
        exists: |db| Get::<DoubleBitBinaryInput>::get(db, INDEX).is_some(),
    },
    Family {
        static_group: 10,
        event_group: 11,
        add: |db, class| db.add(INDEX, class, BinaryOutputStatusConfig::default()),
        update: |db, step, options| {
            let value = BinaryOutputStatus::new(step % 2 == 1, Flags::ONLINE, time());
            db.update2(INDEX, &value, options)
        },
        remove: |db| Remove::<BinaryOutputStatus>::remove(db, INDEX),
        exists: |db| Get::<BinaryOutputStatus>::get(db, INDEX).is_some(),
    },
    Family {
        static_group: 20,
        event_group: 22,
        add: |db, class| db.add(INDEX, class, CounterConfig::default()),
        update: |db, step, options| {
            let value = Counter::new(step.into(), Flags::ONLINE, time());
            db.update2(INDEX, &value, options)
        },
        remove: |db| Remove::<Counter>::remove(db, INDEX),
        exists: |db| Get::<Counter>::get(db, INDEX).is_some(),
    },
    Family {
        static_group: 21,
        event_group: 23,
        add: |db, class| db.add(INDEX, class, FrozenCounterConfig::default()),
        update: |db, step, options| {
            let value = FrozenCounter::new(step.into(), Flags::ONLINE, time());
            db.update2(INDEX, &value, options)
        },
        remove: |db| Remove::<FrozenCounter>::remove(db, INDEX),
        exists: |db| Get::<FrozenCounter>::get(db, INDEX).is_some(),
    },
    Family {
        static_group: 30,
        event_group: 32,
        add: |db, class| db.add(INDEX, class, AnalogInputConfig::default()),
        update: |db, step, options| {
            let value = AnalogInput::new(step.into(), Flags::ONLINE, time());
            db.update2(INDEX, &value, options)
        },
        remove: |db| Remove::<AnalogInput>::remove(db, INDEX),
        exists: |db| Get::<AnalogInput>::get(db, INDEX).is_some(),
    },
    Family {
        static_group: 40,
        event_group: 42,
        add: |db, class| db.add(INDEX, class, AnalogOutputStatusConfig::default()),
        update: |db, step, options| {
            let value = AnalogOutputStatus::new(step.into(), Flags::ONLINE, time());
            db.update2(INDEX, &value, options)
        },
        remove: |db| Remove::<AnalogOutputStatus>::remove(db, INDEX),
        exists: |db| Get::<AnalogOutputStatus>::get(db, INDEX).is_some(),
    },
    Family {
        static_group: 110,
        event_group: 111,
        add: |db, class| db.add(INDEX, class, OctetStringConfig),
        update: |db, step, options| db.update2(INDEX, &octet_string(1, step), options),
        remove: |db| Remove::<OctetString>::remove(db, INDEX),
        exists: |db| Get::<OctetString>::get(db, INDEX).is_some(),
    },
];

fn created_if(condition: bool) -> UpdateInfo {
    if condition {
        UpdateInfo::Created(0)
    } else {
        UpdateInfo::NoPoint
    }
}

fn unknown_unless(condition: bool) -> Iin2 {
    if condition {
        NONE
    } else {
        Iin2::OBJECT_UNKNOWN
    }
}

#[test]
fn each_type_has_its_own_points() {
    for (added, family) in FAMILIES.iter().enumerate() {
        let mut handle = new_handle();
        handle.transaction(|db| {
            assert!((family.add)(db, Some(EventClass::Class1)));
            // a second point at the same index is refused
            assert!(!(family.add)(db, Some(EventClass::Class1)));
            for (other, f) in FAMILIES.iter().enumerate() {
                assert_eq!((f.exists)(db), other == added);
                assert_eq!(
                    (f.update)(db, 1, UpdateOptions::default()),
                    created_if(other == added)
                );
            }
        });

        // READs of the other types have no points or events to report
        for (other, f) in FAMILIES.iter().enumerate() {
            let expected = unknown_unless(other == added);
            assert_eq!(read(&mut handle, &[f.static_group, 0, 0x06]).0, expected);
            assert_eq!(read(&mut handle, &[f.event_group, 0, 0x06]).0, expected);
        }

        handle.transaction(|db| {
            for (other, f) in FAMILIES.iter().enumerate() {
                assert_eq!((f.remove)(db), other == added);
                assert!(!(f.exists)(db));
            }
        });

        // the event outlives its point
        assert_eq!(
            read(&mut handle, &[family.static_group, 0, 0x06]).0,
            Iin2::OBJECT_UNKNOWN
        );
        let (iin2, objects) = read(&mut handle, &[family.event_group, 0, 0x06]);
        assert_eq!(iin2, NONE);
        assert_eq!(objects.first(), Some(&family.event_group));
    }
}

#[test]
fn event_mode_and_class_apply_to_every_type() {
    let forced = UpdateOptions::new(true, EventMode::Force);
    let suppressed = UpdateOptions::new(true, EventMode::Suppress);
    for family in FAMILIES.iter() {
        let handle = new_handle();
        handle.transaction(|db| {
            assert!((family.add)(db, Some(EventClass::Class1)));
            assert_eq!(
                (family.update)(db, 1, UpdateOptions::default()),
                UpdateInfo::Created(0)
            );
            // an unchanged value only produces an event when forced
            assert_eq!(
                (family.update)(db, 1, UpdateOptions::default()),
                UpdateInfo::NoEvent
            );
            assert_eq!((family.update)(db, 1, forced), UpdateInfo::Created(1));
            assert_eq!((family.update)(db, 2, suppressed), UpdateInfo::NoEvent);
            // detection compares to the last event, not the static value
            assert_eq!(
                (family.update)(db, 2, UpdateOptions::default()),
                UpdateInfo::Created(2)
            );
            assert_eq!(
                (family.update)(db, 2, UpdateOptions::no_event()),
                UpdateInfo::NoEvent
            );
        });

        // without a class, the point never produces events
        let handle = new_handle();
        handle.transaction(|db| {
            assert!((family.add)(db, None));
            assert_eq!(
                (family.update)(db, 1, UpdateOptions::default()),
                UpdateInfo::NoEvent
            );
            assert_eq!((family.update)(db, 1, forced), UpdateInfo::NoEvent);
        });
    }
}

#[test]
fn update_can_leave_the_static_value_unchanged() {
    let handle = new_handle();
    handle.transaction(|db| {
        db.add(INDEX, Some(EventClass::Class1), CounterConfig::default());
        let value = |v| Counter::new(v, Flags::ONLINE, time());
        db.update(INDEX, &value(1), UpdateOptions::no_event());
        assert_eq!(
            db.update2(
                INDEX,
                &value(2),
                UpdateOptions::new(false, EventMode::Detect)
            ),
            UpdateInfo::Created(0)
        );
        assert_eq!(Get::<Counter>::get(db, INDEX), Some(value(1)));
        assert_eq!(
            db.update2(INDEX, &value(2), UpdateOptions::default()),
            UpdateInfo::NoEvent
        );
        assert_eq!(Get::<Counter>::get(db, INDEX), Some(value(2)));
    });
}

#[test]
#[rustfmt::skip]
fn update_flags_dispatches_by_type() {
    let flags = Flags::ONLINE | Flags::LOCAL_FORCED;
    let later = Time::synchronized(TIME + 1);
    let handle = new_handle();
    handle.transaction(|db| {
        for family in FAMILIES.iter() {
            (family.add)(db, Some(EventClass::Class1));
            (family.update)(db, 1, UpdateOptions::no_event());
        }
        for (expected, flags_type) in (0..).zip([
            UpdateFlagsType::BinaryInput,
            UpdateFlagsType::DoubleBitBinaryInput,
            UpdateFlagsType::BinaryOutputStatus,
            UpdateFlagsType::Counter,
            UpdateFlagsType::FrozenCounter,
            UpdateFlagsType::AnalogInput,
            UpdateFlagsType::AnalogOutputStatus,
        ]) {
            assert_eq!(
                db.update_flags(INDEX, flags_type, flags, Some(later), UpdateOptions::default()),
                UpdateInfo::Created(expected)
            );
            assert_eq!(
                db.update_flags(0, flags_type, flags, Some(later), UpdateOptions::default()),
                UpdateInfo::NoPoint
            );
        }
        assert_eq!(Get::<BinaryInput>::get(db, INDEX), Some(BinaryInput::new(true, flags, later)));
        assert_eq!(Get::<DoubleBitBinaryInput>::get(db, INDEX), Some(DoubleBitBinaryInput::new(DoubleBit::DeterminedOn, flags, later)));
        assert_eq!(Get::<BinaryOutputStatus>::get(db, INDEX), Some(BinaryOutputStatus::new(true, flags, later)));
        assert_eq!(Get::<Counter>::get(db, INDEX), Some(Counter::new(1, flags, later)));
        assert_eq!(Get::<FrozenCounter>::get(db, INDEX), Some(FrozenCounter::new(1, flags, later)));
        assert_eq!(Get::<AnalogInput>::get(db, INDEX), Some(AnalogInput::new(1.0, flags, later)));
        assert_eq!(Get::<AnalogOutputStatus>::get(db, INDEX), Some(AnalogOutputStatus::new(1.0, flags, later)));
        assert_eq!(Get::<OctetString>::get(db, INDEX), Some(octet_string(1, 1)));
    });
}

// ---------- event detection ----------

#[test]
#[rustfmt::skip]
fn binary_types_detect_value_and_flag_changes_but_not_time() {
    let handle = new_handle();
    handle.transaction(|db| {
        db.add(INDEX, Some(EventClass::Class1), BinaryInputConfig::default());
        db.add(INDEX, Some(EventClass::Class1), DoubleBitBinaryInputConfig::default());
        db.add(INDEX, Some(EventClass::Class1), BinaryOutputStatusConfig::default());
        let options = UpdateOptions::default();
        let t1 = Time::synchronized(1);
        let t2 = Time::synchronized(2);

        assert_eq!(db.update2(INDEX, &BinaryInput::new(true, Flags::ONLINE, t1), options), UpdateInfo::Created(0));
        assert_eq!(db.update2(INDEX, &BinaryInput::new(true, Flags::ONLINE, t2), options), UpdateInfo::NoEvent);
        assert_eq!(db.update2(INDEX, &BinaryInput::new(false, Flags::ONLINE, t2), options), UpdateInfo::Created(1));
        assert_eq!(db.update2(INDEX, &BinaryInput::new(false, Flags::COMM_LOST, t2), options), UpdateInfo::Created(2));

        assert_eq!(db.update2(INDEX, &DoubleBitBinaryInput::new(DoubleBit::DeterminedOn, Flags::ONLINE, t1), options), UpdateInfo::Created(3));
        assert_eq!(db.update2(INDEX, &DoubleBitBinaryInput::new(DoubleBit::DeterminedOn, Flags::ONLINE, t2), options), UpdateInfo::NoEvent);
        assert_eq!(db.update2(INDEX, &DoubleBitBinaryInput::new(DoubleBit::Intermediate, Flags::ONLINE, t2), options), UpdateInfo::Created(4));
        assert_eq!(db.update2(INDEX, &DoubleBitBinaryInput::new(DoubleBit::Intermediate, Flags::COMM_LOST, t2), options), UpdateInfo::Created(5));

        assert_eq!(db.update2(INDEX, &BinaryOutputStatus::new(true, Flags::ONLINE, t1), options), UpdateInfo::Created(6));
        assert_eq!(db.update2(INDEX, &BinaryOutputStatus::new(true, Flags::ONLINE, t2), options), UpdateInfo::NoEvent);
        assert_eq!(db.update2(INDEX, &BinaryOutputStatus::new(false, Flags::ONLINE, t2), options), UpdateInfo::Created(7));
        assert_eq!(db.update2(INDEX, &BinaryOutputStatus::new(false, Flags::COMM_LOST, t2), options), UpdateInfo::Created(8));
    });
}

#[test]
#[rustfmt::skip]
fn counter_types_detect_changes_past_the_deadband() {
    let handle = new_handle();
    handle.transaction(|db| {
        db.add(INDEX, Some(EventClass::Class1), CounterConfig { deadband: 5, ..Default::default() });
        db.add(INDEX, Some(EventClass::Class1), FrozenCounterConfig { deadband: 5, ..Default::default() });
        let options = UpdateOptions::default();
        let counter = |v| Counter::new(v, Flags::ONLINE, time());
        let frozen = |v| FrozenCounter::new(v, Flags::ONLINE, time());

        // the RESTART flag clears
        assert_eq!(db.update2(INDEX, &counter(10), options), UpdateInfo::Created(0));
        assert_eq!(db.update2(INDEX, &counter(15), options), UpdateInfo::NoEvent);
        assert_eq!(db.update2(INDEX, &counter(5), options), UpdateInfo::NoEvent);
        assert_eq!(db.update2(INDEX, &counter(16), options), UpdateInfo::Created(1));
        assert_eq!(db.update2(INDEX, &counter(10), options), UpdateInfo::Created(2));
        assert_eq!(db.update2(INDEX, &Counter::new(10, Flags::COMM_LOST, time()), options), UpdateInfo::Created(3));

        assert_eq!(db.update2(INDEX, &frozen(10), options), UpdateInfo::Created(4));
        assert_eq!(db.update2(INDEX, &frozen(15), options), UpdateInfo::NoEvent);
        assert_eq!(db.update2(INDEX, &frozen(5), options), UpdateInfo::NoEvent);
        assert_eq!(db.update2(INDEX, &frozen(16), options), UpdateInfo::Created(5));
        assert_eq!(db.update2(INDEX, &frozen(10), options), UpdateInfo::Created(6));
        assert_eq!(db.update2(INDEX, &FrozenCounter::new(10, Flags::COMM_LOST, time()), options), UpdateInfo::Created(7));
    });
}

#[test]
#[rustfmt::skip]
fn analog_types_detect_changes_past_the_deadband() {
    let handle = new_handle();
    handle.transaction(|db| {
        db.add(INDEX, Some(EventClass::Class1), AnalogInputConfig { deadband: 1.5, ..Default::default() });
        db.add(INDEX, Some(EventClass::Class1), AnalogOutputStatusConfig { deadband: 1.5, ..Default::default() });
        let options = UpdateOptions::default();
        let analog = |v| AnalogInput::new(v, Flags::ONLINE, time());
        let output = |v| AnalogOutputStatus::new(v, Flags::ONLINE, time());

        assert_eq!(db.update2(INDEX, &analog(10.0), options), UpdateInfo::Created(0));
        assert_eq!(db.update2(INDEX, &analog(11.5), options), UpdateInfo::NoEvent);
        assert_eq!(db.update2(INDEX, &analog(8.5), options), UpdateInfo::NoEvent);
        assert_eq!(db.update2(INDEX, &analog(11.75), options), UpdateInfo::Created(1));
        assert_eq!(db.update2(INDEX, &AnalogInput::new(11.75, Flags::COMM_LOST, time()), options), UpdateInfo::Created(2));

        assert_eq!(db.update2(INDEX, &output(10.0), options), UpdateInfo::Created(3));
        assert_eq!(db.update2(INDEX, &output(11.5), options), UpdateInfo::NoEvent);
        assert_eq!(db.update2(INDEX, &output(8.5), options), UpdateInfo::NoEvent);
        assert_eq!(db.update2(INDEX, &output(11.75), options), UpdateInfo::Created(4));
        assert_eq!(db.update2(INDEX, &AnalogOutputStatus::new(11.75, Flags::COMM_LOST, time()), options), UpdateInfo::Created(5));
    });
}

#[test]
fn analog_deadband_written_to_the_database_applies_to_detection() {
    let mut handle = new_handle();
    handle.transaction(|db| {
        db.add(
            INDEX,
            Some(EventClass::Class1),
            AnalogInputConfig::default(),
        );
        let options = UpdateOptions::default();
        let analog = |v| AnalogInput::new(v, Flags::ONLINE, time());
        assert_eq!(
            db.update2(INDEX, &analog(10.0), options),
            UpdateInfo::Created(0)
        );
        assert_eq!(db.inner.set_analog_deadband(INDEX, 2.0), Some(2.0));
        assert_eq!(
            db.update2(INDEX, &analog(12.0), options),
            UpdateInfo::NoEvent
        );
        assert_eq!(
            db.update2(INDEX, &analog(12.5), options),
            UpdateInfo::Created(1)
        );
    });
    assert_eq!(
        read(&mut handle, &[34, 1, 0x06]),
        ok(&[34, 1, 1, 3, 0, 3, 0, 2, 0])
    );
}

#[test]
fn octet_strings_detect_any_change_in_their_bytes() {
    let handle = new_handle();
    handle.transaction(|db| {
        db.add(INDEX, Some(EventClass::Class1), OctetStringConfig);
        let options = UpdateOptions::default();
        // the initial value is a single zero
        assert_eq!(
            db.update2(INDEX, &octet_string(1, 0), options),
            UpdateInfo::NoEvent
        );
        assert_eq!(
            db.update2(INDEX, &octet_string(1, 1), options),
            UpdateInfo::Created(0)
        );
        assert_eq!(
            db.update2(INDEX, &octet_string(2, 1), options),
            UpdateInfo::Created(1)
        );
        assert_eq!(
            db.update2(INDEX, &octet_string(2, 1), options),
            UpdateInfo::NoEvent
        );
        assert_eq!(
            db.update2(INDEX, &octet_string(0, 0), options),
            UpdateInfo::Created(2)
        );
    });
}

// ---------- event buffer limits and counts ----------

#[derive(Default)]
struct Confirms {
    cleared: Vec<u64>,
}

impl OutstationApplication for Confirms {
    fn event_cleared(&mut self, id: u64) {
        self.cleared.push(id);
    }
}

fn confirm(handle: &mut DatabaseHandle) -> (Vec<u64>, BufferState) {
    let mut app = Confirms::default();
    let state = handle.transaction(|db| db.inner.clear_written_events(&mut app));
    (app.cleared, state)
}

#[test]
fn each_type_has_its_own_event_limit() {
    let mut handle = handle_with(
        ClassZeroConfig::default(),
        EventBufferConfig::new(1, 2, 3, 4, 5, 6, 7, 8),
    );
    handle.transaction(|db| {
        let mut id = 0;
        for (max, family) in (1..).zip(FAMILIES.iter()) {
            (family.add)(db, Some(EventClass::Class2));
            for step in 1..=max {
                assert_eq!(
                    (family.update)(db, step, UpdateOptions::default()),
                    UpdateInfo::Created(id)
                );
                id += 1;
            }
            // one more discards the oldest of the type
            assert_eq!(
                (family.update)(db, max + 1, UpdateOptions::default()),
                UpdateInfo::Overflow {
                    created: id,
                    discarded: id - u64::from(max),
                }
            );
            id += 1;
        }
    });

    // the survivors are the newest of each type
    read(&mut handle, &[60, 3, 0x06]);
    let (cleared, state) = confirm(&mut handle);
    assert_eq!(
        cleared,
        vec![
            1, 3, 4, 6, 7, 8, 10, 11, 12, 13, 15, 16, 17, 18, 19, 21, 22, 23, 24, 25, 26, 28, 29,
            30, 31, 32, 33, 34, 36, 37, 38, 39, 40, 41, 42, 43
        ]
    );
    assert_eq!(
        state,
        BufferState {
            classes: ClassCount::default(),
            types: TypeCount {
                num_binary_input: 0,
                num_double_bit_binary_input: 0,
                num_binary_output_status: 0,
                num_counter: 0,
                num_frozen_counter: 0,
                num_analog: 0,
                num_analog_output_status: 0,
                num_octet_string: 0,
            },
        }
    );
}

#[test]
fn buffer_state_counts_each_type() {
    let mut handle = new_handle();
    handle.transaction(|db| {
        for (count, family) in (1..).zip(FAMILIES.iter()) {
            (family.add)(db, Some(EventClass::Class3));
            for step in 1..=count {
                (family.update)(db, step, UpdateOptions::default());
            }
        }
    });
    // nothing was written, so every event remains
    assert_eq!(
        confirm(&mut handle),
        (
            vec![],
            BufferState {
                classes: ClassCount {
                    num_class_1: 0,
                    num_class_2: 0,
                    num_class_3: 36,
                },
                types: TypeCount {
                    num_binary_input: 1,
                    num_double_bit_binary_input: 2,
                    num_binary_output_status: 3,
                    num_counter: 4,
                    num_frozen_counter: 5,
                    num_analog: 6,
                    num_analog_output_status: 7,
                    num_octet_string: 8,
                },
            }
        )
    );
}

// ---------- class 0 ----------

/// one point of every type at INDEX, with each type's default variations
fn all_types(db: &mut Database) {
    for family in FAMILIES.iter() {
        (family.add)(db, None);
        (family.update)(db, 1, UpdateOptions::no_event());
    }
}

#[rustfmt::skip]
const CLASS_0_HEADERS: [&[u8]; 8] = [
    &[1, 1, 1, 3, 0, 3, 0, 1],
    &[3, 1, 1, 3, 0, 3, 0, 2],
    &[10, 1, 1, 3, 0, 3, 0, 1],
    &[20, 1, 1, 3, 0, 3, 0, 1, 1, 0, 0, 0],
    &[21, 1, 1, 3, 0, 3, 0, 1, 1, 0, 0, 0],
    &[30, 1, 1, 3, 0, 3, 0, 1, 1, 0, 0, 0],
    &[40, 1, 1, 3, 0, 3, 0, 1, 1, 0, 0, 0],
    &[110, 1, 1, 3, 0, 3, 0, 1],
];

#[test]
fn class_zero_reports_each_enabled_type_in_order() {
    let only = |n: usize| {
        let [a, b, c, d, e, f, g, h]: [bool; 8] = std::array::from_fn(|i| i == n);
        ClassZeroConfig::new(a, b, c, d, e, f, g, h)
    };
    for (n, expected) in CLASS_0_HEADERS.iter().enumerate() {
        let mut handle = handle_with(only(n), EventBufferConfig::all_types(10));
        handle.transaction(all_types);
        assert_eq!(read(&mut handle, &CLASS_0), ok(expected));
    }

    let all = ClassZeroConfig::new(true, true, true, true, true, true, true, true);
    let mut handle = handle_with(all, EventBufferConfig::all_types(10));
    handle.transaction(all_types);
    assert_eq!(read(&mut handle, &CLASS_0), ok(&CLASS_0_HEADERS.concat()));

    // octet strings are left out by default
    let mut handle = new_handle();
    handle.transaction(all_types);
    assert_eq!(
        read(&mut handle, &CLASS_0),
        ok(&CLASS_0_HEADERS[..7].concat())
    );

    // types without points add nothing, and no error
    let mut handle = handle_with(all, EventBufferConfig::all_types(10));
    assert_eq!(read(&mut handle, &CLASS_0), ok(&[]));
}

// ---------- packed bits ----------

#[test]
#[rustfmt::skip]
fn packed_binary_inputs_promote_points_with_flags_other_than_online() {
    let mut handle = new_handle();
    handle.transaction(|db| {
        for index in 0..10 {
            db.add(index, None, BinaryInputConfig::default());
            // the state is ignored
            let value = BinaryInput::new(index % 3 == 0, Flags::ONLINE, time());
            db.update(index, &value, UpdateOptions::no_event());
        }
    });
    assert_eq!(read(&mut handle, &[1, 0, 0x06]), ok(&[1, 1, 1, 0, 0, 9, 0, 73, 2]));
    handle.transaction(|db| {
        db.update(4, &BinaryInput::new(true, Flags::ONLINE | Flags::CHATTER_FILTER, time()), UpdateOptions::no_event());
    });
    assert_eq!(read(&mut handle, &[1, 0, 0x06]), ok(&[1, 1, 1, 0, 0, 3, 0, 9, 1, 2, 1, 4, 0, 4, 0, 161, 1, 1, 1, 5, 0, 9, 0, 18]));
    // a READ of g1v1 promotes too
    assert_eq!(read(&mut handle, &[1, 1, 0x00, 3, 5]), ok(&[1, 1, 1, 3, 0, 3, 0, 1, 1, 2, 1, 4, 0, 4, 0, 161, 1, 1, 1, 5, 0, 5, 0, 0]));
    // and g1v2 is never demoted
    assert_eq!(read(&mut handle, &[1, 2, 0x00, 3, 5]), ok(&[1, 2, 1, 3, 0, 5, 0, 129, 161, 1]));
}

#[test]
#[rustfmt::skip]
fn packed_double_bit_inputs_promote_points_with_flags_other_than_online() {
    let mut handle = new_handle();
    let states = [DoubleBit::Intermediate, DoubleBit::DeterminedOff, DoubleBit::DeterminedOn, DoubleBit::Indeterminate];
    handle.transaction(|db| {
        for (index, state) in (0..6).zip(states.iter().cycle()) {
            db.add(index, None, DoubleBitBinaryInputConfig::default());
            // the state is ignored
            let value = DoubleBitBinaryInput::new(*state, Flags::ONLINE, time());
            db.update(index, &value, UpdateOptions::no_event());
        }
    });
    assert_eq!(read(&mut handle, &[3, 0, 0x06]), ok(&[3, 1, 1, 0, 0, 5, 0, 228, 4]));
    handle.transaction(|db| {
        db.update(4, &DoubleBitBinaryInput::new(DoubleBit::DeterminedOn, Flags::ONLINE | Flags::LOCAL_FORCED, time()), UpdateOptions::no_event());
    });
    assert_eq!(read(&mut handle, &[3, 0, 0x06]), ok(&[3, 1, 1, 0, 0, 3, 0, 228, 3, 2, 1, 4, 0, 4, 0, 145, 3, 1, 1, 5, 0, 5, 0, 1]));
    assert_eq!(read(&mut handle, &[3, 1, 0x00, 3, 5]), ok(&[3, 1, 1, 3, 0, 3, 0, 3, 3, 2, 1, 4, 0, 4, 0, 145, 3, 1, 1, 5, 0, 5, 0, 1]));
}

#[test]
#[rustfmt::skip]
fn packed_binary_output_status_promotes_points_with_flags_other_than_online() {
    let mut handle = new_handle();
    handle.transaction(|db| {
        for index in 0..10 {
            db.add(index, None, BinaryOutputStatusConfig::default());
            let value = BinaryOutputStatus::new(index % 3 == 0, Flags::ONLINE, time());
            db.update(index, &value, UpdateOptions::no_event());
        }
    });
    assert_eq!(read(&mut handle, &[10, 0, 0x06]), ok(&[10, 1, 1, 0, 0, 9, 0, 73, 2]));
    handle.transaction(|db| {
        db.update(8, &BinaryOutputStatus::new(false, Flags::COMM_LOST, time()), UpdateOptions::no_event());
    });
    assert_eq!(read(&mut handle, &[10, 0, 0x06]), ok(&[10, 1, 1, 0, 0, 7, 0, 73, 10, 2, 1, 8, 0, 8, 0, 4, 10, 1, 1, 9, 0, 9, 0, 1]));
    assert_eq!(read(&mut handle, &[10, 1, 0x00, 7, 9]), ok(&[10, 1, 1, 7, 0, 7, 0, 0, 10, 2, 1, 8, 0, 8, 0, 4, 10, 1, 1, 9, 0, 9, 0, 1]));
}

#[test]
fn packed_bits_start_a_new_header_at_a_gap_in_the_indices() {
    let mut handle = new_handle();
    handle.transaction(|db| {
        for index in [0, 1, 2, 5, 6] {
            db.add(index, None, BinaryInputConfig::default());
            db.add(index, None, DoubleBitBinaryInputConfig::default());
            let binary = BinaryInput::new(true, Flags::ONLINE, time());
            let double = DoubleBitBinaryInput::new(DoubleBit::DeterminedOn, Flags::ONLINE, time());
            db.update(index, &binary, UpdateOptions::no_event());
            db.update(index, &double, UpdateOptions::no_event());
        }
    });
    assert_eq!(
        read(&mut handle, &[1, 0, 0x06, 3, 0, 0x06]),
        ok(&[
            1, 1, 1, 0, 0, 2, 0, 7, 1, 1, 1, 5, 0, 6, 0, 3, 3, 1, 1, 0, 0, 2, 0, 42, 3, 1, 1, 5, 0,
            6, 0, 10
        ])
    );
}

// ---------- CTO ----------

/// class 1 g2v3 events at INDEX with each of the times, alternating the value
fn binary_cto_events(times: &[Option<Time>]) -> (Iin2, Vec<u8>) {
    let mut handle = handle_with(
        ClassZeroConfig::default(),
        EventBufferConfig::all_types(100),
    );
    handle.transaction(|db| {
        let config = BinaryInputConfig {
            e_var: EventBinaryInputVariation::Group2Var3,
            ..Default::default()
        };
        db.add(INDEX, Some(EventClass::Class1), config);
        for (i, time) in times.iter().enumerate() {
            let value = BinaryInput {
                value: i % 2 == 0,
                flags: Flags::ONLINE,
                time: *time,
            };
            db.update(INDEX, &value, UpdateOptions::default());
        }
    });
    read(&mut handle, &CLASS_1)
}

fn sync(ts: u64) -> Option<Time> {
    Some(Time::synchronized(ts))
}

fn unsync(ts: u64) -> Option<Time> {
    Some(Time::unsynchronized(ts))
}

#[test]
fn cto_holds_for_the_largest_difference_it_can_encode() {
    assert_eq!(
        binary_cto_events(&[sync(TIME), sync(TIME + 65535)]),
        ok(&[51, 1, 7, 1, 1, 2, 3, 4, 5, 6, 2, 3, 40, 2, 0, 3, 0, 129, 0, 0, 3, 0, 1, 255, 255]),
    );
}

#[test]
fn cto_changes_past_the_largest_difference() {
    assert_eq!(
        binary_cto_events(&[sync(TIME), sync(TIME + 65536)]),
        ok(&[
            51, 1, 7, 1, 1, 2, 3, 4, 5, 6, 2, 3, 40, 1, 0, 3, 0, 129, 0, 0, 51, 1, 7, 1, 1, 2, 4,
            4, 5, 6, 2, 3, 40, 1, 0, 3, 0, 1, 0, 0
        ]),
    );
}

#[test]
fn cto_changes_for_a_time_before_it() {
    assert_eq!(
        binary_cto_events(&[sync(TIME), sync(TIME - 1), sync(TIME)]),
        ok(&[
            51, 1, 7, 1, 1, 2, 3, 4, 5, 6, 2, 3, 40, 1, 0, 3, 0, 129, 0, 0, 51, 1, 7, 1, 0, 2, 3,
            4, 5, 6, 2, 3, 40, 2, 0, 3, 0, 1, 0, 0, 3, 0, 129, 1, 0
        ]),
    );
}

#[test]
fn cto_changes_with_synchronization() {
    assert_eq!(
        binary_cto_events(&[sync(TIME), unsync(TIME), unsync(TIME + 1), sync(TIME + 1)]),
        ok(&[
            51, 1, 7, 1, 1, 2, 3, 4, 5, 6, 2, 3, 40, 1, 0, 3, 0, 129, 0, 0, 51, 2, 7, 1, 1, 2, 3,
            4, 5, 6, 2, 3, 40, 2, 0, 3, 0, 1, 0, 0, 3, 0, 129, 1, 0, 51, 1, 7, 1, 2, 2, 3, 4, 5, 6,
            2, 3, 40, 1, 0, 3, 0, 1, 0, 0
        ]),
    );
}

#[test]
fn event_without_time_uses_an_unsynchronized_cto_of_zero() {
    assert_eq!(
        binary_cto_events(&[None, unsync(0), unsync(1)]),
        ok(&[
            51, 2, 7, 1, 0, 0, 0, 0, 0, 0, 2, 3, 40, 3, 0, 3, 0, 129, 0, 0, 3, 0, 1, 0, 0, 3, 0,
            129, 1, 0
        ])
    );
}

#[test]
#[rustfmt::skip]
fn cto_starts_again_after_another_header() {
    let mut handle = new_handle();
    handle.transaction(|db| {
        db.add(0, Some(EventClass::Class1), BinaryInputConfig { e_var: EventBinaryInputVariation::Group2Var3, ..Default::default() });
        db.add(1, Some(EventClass::Class1), DoubleBitBinaryInputConfig { e_var: EventDoubleBitBinaryInputVariation::Group4Var3, ..Default::default() });
        db.add(2, Some(EventClass::Class1), BinaryInputConfig { e_var: EventBinaryInputVariation::Group2Var1, ..Default::default() });
        let at = |offset| Time::synchronized(TIME + offset);
        db.update(0, &BinaryInput::new(true, Flags::ONLINE, at(0)), UpdateOptions::default());
        db.update(1, &DoubleBitBinaryInput::new(DoubleBit::DeterminedOn, Flags::ONLINE, at(1)), UpdateOptions::default());
        db.update(1, &DoubleBitBinaryInput::new(DoubleBit::DeterminedOff, Flags::ONLINE, at(2)), UpdateOptions::default());
        db.update(0, &BinaryInput::new(false, Flags::ONLINE, at(3)), UpdateOptions::default());
        db.update(2, &BinaryInput::new(true, Flags::ONLINE, at(4)), UpdateOptions::default());
        db.update(0, &BinaryInput::new(true, Flags::ONLINE, at(5)), UpdateOptions::default());
    });
    assert_eq!(read(&mut handle, &CLASS_1), ok(&[51, 1, 7, 1, 1, 2, 3, 4, 5, 6, 2, 3, 40, 1, 0, 0, 0, 129, 0, 0, 51, 1, 7, 1, 2, 2, 3, 4, 5, 6, 4, 3, 40, 2, 0, 1, 0, 129, 0, 0, 1, 0, 65, 1, 0, 51, 1, 7, 1, 4, 2, 3, 4, 5, 6, 2, 3, 40, 1, 0, 0, 0, 1, 0, 0, 2, 1, 40, 1, 0, 2, 0, 129, 51, 1, 7, 1, 6, 2, 3, 4, 5, 6, 2, 3, 40, 1, 0, 0, 0, 129, 0, 0]));
}

// ---------- octet strings ----------

#[test]
fn static_octet_strings_use_their_length_as_the_variation() {
    let mut handle = new_handle();
    handle.transaction(|db| {
        for (index, len) in [(0, 0), (1, 1), (2, 1), (3, 255), (4, 2)] {
            db.add(index, None, OctetStringConfig);
            db.update(index, &octet_string(len, 0xAA), UpdateOptions::no_event());
        }
    });
    let mut expected = vec![110, 0, 0x01, 0, 0, 0, 0];
    expected.extend([110, 1, 0x01, 1, 0, 2, 0, 0xAA, 0xAA]);
    expected.extend([110, 255, 0x01, 3, 0, 3, 0]);
    expected.extend([0xAA; 255]);
    expected.extend([110, 2, 0x01, 4, 0, 4, 0, 0xAA, 0xAA]);
    assert_eq!(read(&mut handle, &[110, 0, 0x06]), (NONE, expected));
    assert_eq!(
        read(&mut handle, &[110, 0, 0x00, 1, 2]),
        ok(&[110, 1, 0x01, 1, 0, 2, 0, 0xAA, 0xAA])
    );
}

#[test]
fn octet_string_events_use_their_length_as_the_variation() {
    let mut handle = new_handle();
    handle.transaction(|db| {
        for index in 0..4 {
            db.add(index, Some(EventClass::Class1), OctetStringConfig);
        }
        for (index, len) in [(0, 1), (1, 1), (2, 255), (3, 0), (0, 2)] {
            db.update(index, &octet_string(len, 0xBB), UpdateOptions::default());
        }
    });
    let mut expected = vec![111, 1, 0x28, 2, 0, 0, 0, 0xBB, 1, 0, 0xBB];
    expected.extend([111, 255, 0x28, 1, 0, 2, 0]);
    expected.extend([0xBB; 255]);
    expected.extend([111, 0, 0x28, 1, 0, 3, 0]);
    expected.extend([111, 2, 0x28, 1, 0, 0, 0, 0xBB, 0xBB]);
    assert_eq!(read(&mut handle, &CLASS_1), (NONE, expected));
    // a READ of g111 by count
    assert_eq!(
        read(&mut handle, &[111, 0, 0x07, 2]),
        ok(&[111, 1, 0x28, 2, 0, 0, 0, 0xBB, 1, 0, 0xBB])
    );
}

// ---------- event headers at their boundaries ----------

/// class 1 g4v3 events at INDEX with each of the times, alternating the value
fn double_bit_cto_events(times: &[Time]) -> (Iin2, Vec<u8>) {
    let mut handle = new_handle();
    handle.transaction(|db| {
        let config = DoubleBitBinaryInputConfig {
            e_var: EventDoubleBitBinaryInputVariation::Group4Var3,
            ..Default::default()
        };
        db.add(INDEX, Some(EventClass::Class1), config);
        for (step, time) in (1..).zip(times) {
            let value = DoubleBitBinaryInput::new(double_bit(step), Flags::ONLINE, *time);
            db.update(INDEX, &value, UpdateOptions::default());
        }
    });
    read(&mut handle, &CLASS_1)
}

#[test]
fn double_bit_cto_holds_for_the_largest_difference_it_can_encode() {
    let times = [Time::synchronized(TIME), Time::synchronized(TIME + 65535)];
    assert_eq!(
        double_bit_cto_events(&times),
        ok(&[51, 1, 7, 1, 1, 2, 3, 4, 5, 6, 4, 3, 40, 2, 0, 3, 0, 129, 0, 0, 3, 0, 65, 255, 255])
    );
}

#[test]
fn double_bit_cto_changes_past_the_largest_difference() {
    let times = [Time::synchronized(TIME), Time::synchronized(TIME + 65536)];
    assert_eq!(
        double_bit_cto_events(&times),
        ok(&[
            51, 1, 7, 1, 1, 2, 3, 4, 5, 6, 4, 3, 40, 1, 0, 3, 0, 129, 0, 0, 51, 1, 7, 1, 1, 2, 4,
            4, 5, 6, 4, 3, 40, 1, 0, 3, 0, 65, 0, 0
        ])
    );
}

#[test]
fn double_bit_cto_changes_for_a_time_before_it() {
    let times = [Time::synchronized(TIME), Time::synchronized(TIME - 1)];
    assert_eq!(
        double_bit_cto_events(&times),
        ok(&[
            51, 1, 7, 1, 1, 2, 3, 4, 5, 6, 4, 3, 40, 1, 0, 3, 0, 129, 0, 0, 51, 1, 7, 1, 0, 2, 3,
            4, 5, 6, 4, 3, 40, 1, 0, 3, 0, 65, 0, 0
        ])
    );
}

#[test]
fn double_bit_cto_changes_with_synchronization() {
    let times = [Time::synchronized(TIME), Time::unsynchronized(TIME)];
    assert_eq!(
        double_bit_cto_events(&times),
        ok(&[
            51, 1, 7, 1, 1, 2, 3, 4, 5, 6, 4, 3, 40, 1, 0, 3, 0, 129, 0, 0, 51, 2, 7, 1, 1, 2, 3,
            4, 5, 6, 4, 3, 40, 1, 0, 3, 0, 65, 0, 0
        ])
    );
}

/// events at their indices in the order given: g2v3 at 0, g4v3 at 1, g32v1 at 2, and g2v3 again
/// at 3, all at TIME plus their position
#[rustfmt::skip]
fn interleaved_events(order: &[u16]) -> DatabaseHandle {
    let handle = new_handle();
    handle.transaction(|db| {
        db.add(0, Some(EventClass::Class1), BinaryInputConfig { e_var: EventBinaryInputVariation::Group2Var3, ..Default::default() });
        db.add(1, Some(EventClass::Class1), DoubleBitBinaryInputConfig { e_var: EventDoubleBitBinaryInputVariation::Group4Var3, ..Default::default() });
        db.add(2, Some(EventClass::Class1), AnalogInputConfig::default());
        db.add(3, Some(EventClass::Class1), BinaryInputConfig { e_var: EventBinaryInputVariation::Group2Var3, ..Default::default() });
        for (step, index) in (1..).zip(order) {
            let time = Time::synchronized(TIME + u64::from(step));
            let options = UpdateOptions::new(true, EventMode::Force);
            match index {
                0 | 3 => db.update(*index, &BinaryInput::new(step % 2 == 1, Flags::ONLINE, time), options),
                1 => db.update(*index, &DoubleBitBinaryInput::new(double_bit(step), Flags::ONLINE, time), options),
                _ => db.update(*index, &AnalogInput::new(step.into(), Flags::ONLINE, time), options),
            };
        }
    });
    handle
}

#[test]
fn relative_time_headers_continue_only_with_their_own_variation() {
    // g2v3 events at two indices share a header, and g4v3 or g32v1 between them start new ones
    let mut handle = interleaved_events(&[0, 3, 1, 1, 0, 2, 3, 0]);
    assert_eq!(
        read(&mut handle, &CLASS_1),
        ok(&[
            51, 1, 7, 1, 2, 2, 3, 4, 5, 6, 2, 3, 40, 2, 0, 0, 0, 129, 0, 0, 3, 0, 1, 1, 0, 51, 1,
            7, 1, 4, 2, 3, 4, 5, 6, 4, 3, 40, 2, 0, 1, 0, 129, 0, 0, 1, 0, 65, 1, 0, 51, 1, 7, 1,
            6, 2, 3, 4, 5, 6, 2, 3, 40, 1, 0, 0, 0, 129, 0, 0, 32, 1, 40, 1, 0, 2, 0, 1, 6, 0, 0,
            0, 51, 1, 7, 1, 8, 2, 3, 4, 5, 6, 2, 3, 40, 2, 0, 3, 0, 129, 0, 0, 0, 0, 1, 1, 0
        ])
    );
}

/// select the object headers of a READ, then write fragments of `size` bytes until everything
/// selected is written or nothing more fits, returning each fragment and whether it completed
/// the response
fn read_fragments(
    handle: &mut DatabaseHandle,
    objects: &[u8],
    size: usize,
) -> Vec<(Vec<u8>, bool)> {
    let headers = HeaderCollection::parse(
        ParseOptions::parse_everything(),
        FunctionCode::Read,
        objects,
    )
    .unwrap();
    handle.reset();
    assert_eq!(handle.select(&headers), NONE);
    let mut fragments = Vec::new();
    let mut buffer = vec![0u8; size];
    loop {
        let mut cursor = WriteCursor::new(&mut buffer);
        let info = handle.write_response_headers(&mut cursor);
        let written = cursor.written().to_vec();
        let stuck = written.is_empty();
        fragments.push((written, info.complete));
        if info.complete || stuck {
            return fragments;
        }
    }
}

/// three g2v3 events at TIME, TIME + 1 and TIME + 2, which fill 30 bytes in one header
fn three_relative_time_events() -> DatabaseHandle {
    let handle = new_handle();
    handle.transaction(|db| {
        let config = BinaryInputConfig {
            e_var: EventBinaryInputVariation::Group2Var3,
            ..Default::default()
        };
        db.add(INDEX, Some(EventClass::Class1), config);
        for step in 0..3 {
            let value = BinaryInput::new(
                step % 2 == 0,
                Flags::ONLINE,
                Time::synchronized(TIME + step),
            );
            db.update(INDEX, &value, UpdateOptions::default());
        }
    });
    handle
}

#[test]
fn relative_time_header_at_exact_fit() {
    let mut handle = three_relative_time_events();
    assert_eq!(
        read_fragments(&mut handle, &CLASS_1, 30),
        vec![(
            vec![
                51, 1, 7, 1, 1, 2, 3, 4, 5, 6, 2, 3, 40, 3, 0, 3, 0, 129, 0, 0, 3, 0, 1, 1, 0, 3,
                0, 129, 2, 0
            ],
            true
        )]
    );
}

#[test]
fn relative_time_header_one_byte_short_moves_the_last_event_to_a_new_cto() {
    let mut handle = three_relative_time_events();
    assert_eq!(
        read_fragments(&mut handle, &CLASS_1, 29),
        vec![
            (
                vec![51, 1, 7, 1, 1, 2, 3, 4, 5, 6, 2, 3, 40, 2, 0, 3, 0, 129, 0, 0, 3, 0, 1, 1, 0],
                false
            ),
            (
                vec![51, 1, 7, 1, 3, 2, 3, 4, 5, 6, 2, 3, 40, 1, 0, 3, 0, 129, 0, 0],
                true
            )
        ]
    );
}

#[test]
fn relative_time_header_then_another_type_at_exact_fit_and_one_byte_short() {
    // 20 bytes of g2v3 with its CTO, then 12 of g32v1
    let mut handle = interleaved_events(&[0, 2]);
    assert_eq!(
        read_fragments(&mut handle, &CLASS_1, 32),
        vec![(
            vec![
                51, 1, 7, 1, 2, 2, 3, 4, 5, 6, 2, 3, 40, 1, 0, 0, 0, 129, 0, 0, 32, 1, 40, 1, 0, 2,
                0, 1, 2, 0, 0, 0
            ],
            true
        )]
    );
    assert_eq!(
        read_fragments(&mut handle, &CLASS_1, 31),
        vec![
            (
                vec![51, 1, 7, 1, 2, 2, 3, 4, 5, 6, 2, 3, 40, 1, 0, 0, 0, 129, 0, 0],
                false
            ),
            (vec![32, 1, 40, 1, 0, 2, 0, 1, 2, 0, 0, 0], true)
        ]
    );
    // one byte short of the CTO, g2v3 header and event: none of them is written
    assert_eq!(
        read_fragments(&mut handle, &CLASS_1, 19),
        vec![(vec![], false)]
    );
    // the events stay selected, and the next fragment with room delivers them
    let mut buffer = [0u8; 64];
    let mut cursor = WriteCursor::new(&mut buffer);
    let info = handle.write_response_headers(&mut cursor);
    assert_eq!(
        (cursor.written(), info.complete),
        (
            &[
                51, 1, 7, 1, 2, 2, 3, 4, 5, 6, 2, 3, 40, 1, 0, 0, 0, 129, 0, 0, 32, 1, 40, 1, 0, 2,
                0, 1, 2, 0, 0, 0
            ][..],
            true
        )
    );
}

#[test]
fn double_bit_relative_time_header_at_exact_fit_and_one_byte_short() {
    // 25 bytes: the CTO, the header and two events
    let mut handle = interleaved_events(&[1, 1]);
    assert_eq!(
        read_fragments(&mut handle, &CLASS_1, 25),
        vec![(
            vec![51, 1, 7, 1, 2, 2, 3, 4, 5, 6, 4, 3, 40, 2, 0, 1, 0, 129, 0, 0, 1, 0, 65, 1, 0],
            true
        )]
    );
    assert_eq!(
        read_fragments(&mut handle, &CLASS_1, 24),
        vec![
            (
                vec![51, 1, 7, 1, 2, 2, 3, 4, 5, 6, 4, 3, 40, 1, 0, 1, 0, 129, 0, 0],
                false
            ),
            (
                vec![51, 1, 7, 1, 3, 2, 3, 4, 5, 6, 4, 3, 40, 1, 0, 1, 0, 65, 0, 0],
                true
            )
        ]
    );
}

#[test]
fn octet_string_event_headers_at_exact_fit_and_one_byte_short() {
    let mut handle = new_handle();
    handle.transaction(|db| {
        for index in 0..3 {
            db.add(index, Some(EventClass::Class1), OctetStringConfig);
        }
        // a header of two 3-byte strings (15 bytes), then one of a 2-byte string (9 bytes)
        for (index, len) in [(0, 3), (1, 3), (2, 2)] {
            db.update(index, &octet_string(len, 0xCC), UpdateOptions::default());
        }
    });
    assert_eq!(
        read_fragments(&mut handle, &CLASS_1, 24),
        vec![(
            vec![
                111, 3, 40, 2, 0, 0, 0, 204, 204, 204, 1, 0, 204, 204, 204, 111, 2, 40, 1, 0, 2, 0,
                204, 204
            ],
            true
        )]
    );
    assert_eq!(
        read_fragments(&mut handle, &CLASS_1, 23),
        vec![
            (
                vec![111, 3, 40, 2, 0, 0, 0, 204, 204, 204, 1, 0, 204, 204, 204],
                false
            ),
            (vec![111, 2, 40, 1, 0, 2, 0, 204, 204], true)
        ]
    );
    assert_eq!(
        read_fragments(&mut handle, &CLASS_1, 15),
        vec![
            (
                vec![111, 3, 40, 2, 0, 0, 0, 204, 204, 204, 1, 0, 204, 204, 204],
                false
            ),
            (vec![111, 2, 40, 1, 0, 2, 0, 204, 204], true)
        ]
    );
    assert_eq!(
        read_fragments(&mut handle, &CLASS_1, 14),
        vec![
            (vec![111, 3, 40, 1, 0, 0, 0, 204, 204, 204], false),
            (vec![111, 3, 40, 1, 0, 1, 0, 204, 204, 204], false),
            (vec![111, 2, 40, 1, 0, 2, 0, 204, 204], true)
        ]
    );
}

#[test]
fn static_octet_string_headers_at_exact_fit_and_one_byte_short() {
    let mut handle = new_handle();
    handle.transaction(|db| {
        // a range of two 3-byte strings (13 bytes), then one of a 2-byte string (9 bytes)
        for (index, len) in [(0, 3), (1, 3), (2, 2)] {
            db.add(index, None, OctetStringConfig);
            db.update(index, &octet_string(len, 0xDD), UpdateOptions::no_event());
        }
    });
    assert_eq!(
        read_fragments(&mut handle, &[110, 0, 0x06], 22),
        vec![(
            vec![
                110, 3, 1, 0, 0, 1, 0, 221, 221, 221, 221, 221, 221, 110, 2, 1, 2, 0, 2, 0, 221,
                221
            ],
            true
        )]
    );
    assert_eq!(
        read_fragments(&mut handle, &[110, 0, 0x06], 21),
        vec![
            (
                vec![110, 3, 1, 0, 0, 1, 0, 221, 221, 221, 221, 221, 221],
                false
            ),
            (vec![110, 2, 1, 2, 0, 2, 0, 221, 221], true)
        ]
    );
    assert_eq!(
        read_fragments(&mut handle, &[110, 0, 0x06], 13),
        vec![
            (
                vec![110, 3, 1, 0, 0, 1, 0, 221, 221, 221, 221, 221, 221],
                false
            ),
            (vec![110, 2, 1, 2, 0, 2, 0, 221, 221], true)
        ]
    );
    assert_eq!(
        read_fragments(&mut handle, &[110, 0, 0x06], 12),
        vec![
            (vec![110, 3, 1, 0, 0, 0, 0, 221, 221, 221], false),
            (vec![110, 3, 1, 1, 0, 1, 0, 221, 221, 221], false),
            (vec![110, 2, 1, 2, 0, 2, 0, 221, 221], true)
        ]
    );
}

#[test]
fn the_most_events_of_a_type_fit_in_one_header() {
    let max = u16::MAX;
    let mut handle = handle_with(
        ClassZeroConfig::default(),
        EventBufferConfig::new(max, 0, 0, 0, 0, 0, 0, 0),
    );
    handle.transaction(|db| {
        db.add(
            INDEX,
            Some(EventClass::Class1),
            BinaryInputConfig::default(),
        );
        for step in 0..max {
            let value = BinaryInput::new(step % 2 == 0, Flags::ONLINE, time());
            db.update(INDEX, &value, UpdateOptions::default());
        }
    });
    let size = 5 + 3 * usize::from(max);
    let fragments = read_fragments(&mut handle, &CLASS_1, size);
    assert_eq!(fragments.len(), 1);
    let (objects, complete) = &fragments[0];
    assert!(complete);
    assert_eq!(objects.len(), size);
    assert_eq!(objects[..5], [2, 1, 0x28, 0xFF, 0xFF]);
    assert_eq!(objects[size - 3..], [3, 0, 0x81]);
}

#[test]
fn a_full_header_count_starts_a_new_header() {
    use crate::outstation::database::details::event::writer::EventWriter;

    let count = usize::from(u16::MAX) + 2;
    let mut buffer = vec![0u8; 2 * 5 + 3 * count];
    let mut cursor = WriteCursor::new(&mut buffer);
    let mut writer = EventWriter::new();
    for index in 0..count {
        let value = BinaryInput::new(index % 2 == 0, Flags::ONLINE, time());
        let index = u16::try_from(index % 1000).unwrap();
        writer
            .write(
                &mut cursor,
                &value,
                index,
                EventBinaryInputVariation::Group2Var1,
            )
            .unwrap();
    }
    let written = cursor.written();
    assert_eq!(written.len(), 2 * 5 + 3 * count);
    assert_eq!(written[..5], [2, 1, 0x28, 0xFF, 0xFF]);
    let second = 5 + 3 * usize::from(u16::MAX);
    // the second header holds the last two events, indices 535 and 536
    assert_eq!(
        written[second..],
        [2, 1, 0x28, 0x02, 0x00, 0x17, 0x02, 0x01, 0x18, 0x02, 0x81]
    );
}
