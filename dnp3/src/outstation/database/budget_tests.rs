// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Pins for the READ budget: the records a READ's selection may visit, 8 per installed static
//! point or attribute set and 16 per event slot, but never less than 4096 for either
//!
//! A READ is answered as today while its visits fit, and rejected with IIN2.2 and no objects
//! after the header that exceeds them. An answered READ of static headers reports the same
//! objects as reading each header on its own, in order, which is the oracle here.

use scursor::WriteCursor;

use crate::app::parse::options::ParseOptions;
use crate::app::parse::parser::HeaderCollection;
use crate::app::{FunctionCode, Iin2};
use crate::outstation::config::OutstationConfig;
use crate::outstation::database::*;

const NONE: Iin2 = Iin2::new(0);
const OU: Iin2 = Iin2::OBJECT_UNKNOWN;
const PE: Iin2 = Iin2::PARAMETER_ERROR;

/// g60v1 (class 0)
const CLASS_0: &[u8] = &[60, 1, 6];
/// g60v2 (class 1)
const CLASS_1: &[u8] = &[60, 2, 6];
/// g1v0, all objects
const ALL_BINARY: &[u8] = &[1, 0, 6];
/// g1v0, 8-bit start/stop, 0 to 0
const BINARY_0: &[u8] = &[1, 0, 0, 0, 0];
/// g13v1, all objects: a binary output command event, which READ doesn't support
const COMMAND_EVENTS: &[u8] = &[13, 1, 6];

fn handle(events: EventBufferConfig) -> DatabaseHandle {
    DatabaseHandle::new(
        OutstationConfig::DEFAULT_MAX_READ_REQUEST_HEADERS,
        ClassZeroConfig::default(),
        events,
    )
}

/// a database of binary inputs at `indices`, with no events
fn binary_inputs(indices: impl Iterator<Item = u16>) -> DatabaseHandle {
    let indices: Vec<u16> = indices.collect();
    let database = handle(EventBufferConfig::no_events());
    database.transaction(|db| {
        for index in &indices {
            db.add(
                *index,
                Some(EventClass::Class1),
                BinaryInputConfig::default(),
            );
        }
    });
    database
}

/// a database of one binary input with `count` class 1 events, in a buffer of `count` slots
fn binary_events(count: u16) -> DatabaseHandle {
    let database = handle(EventBufferConfig::new(count, 0, 0, 0, 0, 0, 0, 0));
    database.transaction(|db| {
        db.add(0, Some(EventClass::Class1), BinaryInputConfig::default());
        for value in 0..count {
            let time = Time::synchronized(u64::from(value));
            let value = BinaryInput::new(value % 2 == 0, Flags::ONLINE, time);
            db.update(0, &value, UpdateOptions::detect_event());
        }
    });
    database
}

/// the headers repeated `count` times
fn repeat(header: &[u8], count: usize) -> Vec<u8> {
    header.repeat(count)
}

/// g1v0, 16-bit start/stop
fn binary_range(start: u16, stop: u16) -> Vec<u8> {
    let mut header = vec![1, 0, 1];
    header.extend(start.to_le_bytes());
    header.extend(stop.to_le_bytes());
    header
}

/// select a READ of the object headers, then write everything selected into one fragment
fn read(handle: &mut DatabaseHandle, objects: &[u8]) -> (Iin2, Vec<u8>) {
    let headers = HeaderCollection::parse(
        ParseOptions::parse_everything(),
        FunctionCode::Read,
        objects,
    )
    .unwrap();
    let iin2 = handle.select(&headers);
    let mut buffer = vec![0u8; 1 << 20];
    let mut cursor = WriteCursor::new(&mut buffer);
    let info = handle.write_response_headers(&mut cursor);
    assert!(info.complete);
    (iin2, cursor.written().to_vec())
}

/// the objects of reading each of `headers` on its own, in order
fn each_alone(handle: &mut DatabaseHandle, headers: &[&[u8]]) -> Vec<u8> {
    headers
        .iter()
        .flat_map(|header| read(handle, header).1)
        .collect()
}

#[test]
fn class_0_twice_is_answered() {
    let mut db = binary_inputs(0..1000);
    let expected = each_alone(&mut db, &[CLASS_0, CLASS_0]);
    assert_eq!(read(&mut db, &repeat(CLASS_0, 2)), (NONE, expected));
}

#[test]
fn static_headers_are_answered_up_to_8_visits_per_point() {
    // each header visits the 1000 points once, for a snapshot
    let mut db = binary_inputs(0..1000);
    let expected = each_alone(&mut db, &[ALL_BINARY; 8]);
    assert_eq!(read(&mut db, &repeat(ALL_BINARY, 8)), (NONE, expected));
    assert_eq!(read(&mut db, &repeat(ALL_BINARY, 9)), (PE, vec![]));
}

#[test]
fn a_range_counts_its_points_and_then_snapshots_them() {
    // each header visits the 1000 points twice
    let mut db = binary_inputs(0..1000);
    let range = binary_range(0, 999);
    let expected = each_alone(&mut db, &[range.as_slice(); 4]);
    assert_eq!(read(&mut db, &repeat(&range, 4)), (NONE, expected));
    assert_eq!(read(&mut db, &repeat(&range, 5)), (PE, vec![]));
}

#[test]
fn class_0_counts_against_the_static_budget() {
    let mut db = binary_inputs(0..1000);
    let expected = each_alone(&mut db, &[CLASS_0; 8]);
    assert_eq!(read(&mut db, &repeat(CLASS_0, 8)), (NONE, expected));
    assert_eq!(read(&mut db, &repeat(CLASS_0, 9)), (PE, vec![]));
}

#[test]
fn small_databases_answer_redundant_reads() {
    // 20 headers visit 200 points, under the minimum of 4096
    let mut db = binary_inputs(0..10);
    let expected = each_alone(&mut db, &[CLASS_0; 20]);
    assert_eq!(read(&mut db, &repeat(CLASS_0, 20)), (NONE, expected));
}

#[test]
fn a_scan_of_sparse_ranges_is_answered() {
    // 64 points in each of 32 blocks of 2048 indices: each range visits its 64 points twice
    let mut db = binary_inputs((0..32).flat_map(|block| (0..64).map(move |i| block * 2048 + i)));
    let ranges: Vec<Vec<u8>> = (0..32)
        .map(|block| binary_range(block * 2048, block * 2048 + 2047))
        .collect();
    let headers: Vec<&[u8]> = ranges.iter().map(Vec::as_slice).collect();
    let expected = each_alone(&mut db, &headers);
    // the indices missing from each range set IIN2.2, as they do today
    assert_eq!(read(&mut db, &ranges.concat()), (PE, expected));
}

#[test]
fn event_headers_are_answered_up_to_16_visits_per_slot() {
    // each class 1 header visits the 1000 events, whether or not an earlier header selected them
    let expected = read(&mut binary_events(1000), CLASS_1).1;
    assert_eq!(
        read(&mut binary_events(1000), &repeat(CLASS_1, 16)),
        (NONE, expected)
    );

    let mut db = binary_events(1000);
    assert_eq!(read(&mut db, &repeat(CLASS_1, 17)), (PE, vec![]));
    // the events the rejected READ selected are reported by the next one
    assert_eq!(
        read(&mut db, CLASS_1),
        read(&mut binary_events(1000), CLASS_1)
    );
}

#[test]
fn static_headers_past_the_selection_limit_cost_nothing() {
    // the 64 selections fill the queue, and the full headers after them are skipped with IIN2.2
    // without visiting the 1000 points
    let mut db = binary_inputs(0..1000);
    let max = usize::from(OutstationConfig::DEFAULT_MAX_READ_REQUEST_HEADERS);
    let expected = each_alone(&mut db, &vec![BINARY_0; max]);
    let request = [repeat(BINARY_0, max), repeat(ALL_BINARY, 20)].concat();
    assert_eq!(read(&mut db, &request), (PE, expected));
}

#[test]
fn a_rejected_read_still_reports_headers_read_does_not_support() {
    // the header after the one that exceeds the budget is still parsed, and reported
    let mut db = binary_inputs(0..1000);
    let request = [repeat(ALL_BINARY, 9), COMMAND_EVENTS.to_vec()].concat();
    assert_eq!(read(&mut db, &request), (PE | OU, vec![]));
}
