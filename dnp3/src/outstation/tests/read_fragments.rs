#![expect(
    clippy::arithmetic_side_effects,
    reason = "tests compute expected counts and fragment sizes"
)]

use std::ops::Range;

use crate::app::attr::*;
use crate::app::measurement::*;
use crate::app::BufferSize;
use crate::outstation::config::OutstationConfig;
use crate::outstation::database::*;
use crate::outstation::tests::harness::*;
use crate::outstation::{BufferState, ClassCount, TypeCount};

/// g60v2 (class 1)
const CLASS_1: &[u8] = &[0x3C, 0x02, 0x06];
/// g60v1 (class 0)
const CLASS_0: &[u8] = &[0x3C, 0x01, 0x06];
/// g0v254 (all attributes) of every set
const ALL_ATTRIBUTES: &[u8] = &[0x00, 0xFE, 0x06];

/// binary flags: ONLINE and state true
const BINARY_TRUE: u8 = 0x81;
/// flags of a point that has never been updated: RESTART
const RESTART: u8 = 0x02;

/// IIN1 with only DEVICE_RESTART
const IIN1_RESTART: u8 = 0x80;
/// IIN1 with DEVICE_RESTART and CLASS_1_EVENTS
const IIN1_RESTART_CLASS_1: u8 = 0x82;

// control fields of each response in a series
const FIR_FIN: u8 = 0xC0;
const FIR_FIN_CON: u8 = 0xE0;
const FIR_CON: u8 = 0xA0;
const FIN_CON_SEQ_1: u8 = 0x61;
const FIN_SEQ_1: u8 = 0x41;
const CON_SEQ_1: u8 = 0x21;
const FIN_SEQ_2: u8 = 0x42;

/// a response header and a g2v1 header leave room for this many 3-byte events in a minimum
/// fragment
const EVENTS_IN_MIN_FRAGMENT: u16 = 80;

fn config_with_buffer(size: usize) -> OutstationConfig {
    let mut config = get_default_config();
    config.solicited_buffer_size = BufferSize::new(size).unwrap();
    config.event_buffer_config = EventBufferConfig::all_types(100);
    config
}

/// the fragment size that g2v1 events 0 to `count - 1` exactly fill
fn exact_fragment_for_events(count: u16) -> usize {
    4 + 5 + 3 * usize::from(count)
}

fn read(headers: &[&[u8]]) -> Vec<u8> {
    let mut request = vec![0xC0, 0x01];
    for header in headers {
        request.extend_from_slice(header);
    }
    request
}

/// g1v2, 16-bit start/stop
fn g1v2(start: u16, stop: u16) -> Vec<u8> {
    let mut header = vec![0x01, 0x02, 0x01];
    header.extend(start.to_le_bytes());
    header.extend(stop.to_le_bytes());
    header
}

fn response(control: u8, iin1: u8, objects: &[&[u8]]) -> Vec<u8> {
    let mut response = vec![control, 0x81, iin1, 0x00];
    for object in objects {
        response.extend_from_slice(object);
    }
    response
}

/// g2v1 events for the indices, each with flags ONLINE and state true
fn g2v1_events(indices: Range<u16>) -> Vec<u8> {
    let count = indices.end - indices.start;
    let mut bytes = vec![0x02, 0x01, 0x28];
    bytes.extend(count.to_le_bytes());
    for index in indices {
        bytes.extend(index.to_le_bytes());
        bytes.push(BINARY_TRUE);
    }
    bytes
}

/// g1v2 values with 16-bit start/stop, one flags byte per value
fn g1v2_values(start: u16, flags: &[u8]) -> Vec<u8> {
    let stop = start + u16::try_from(flags.len()).unwrap() - 1;
    let mut bytes = g1v2(start, stop);
    bytes.extend_from_slice(flags);
    bytes
}

/// g20v1 values of 0 with RESTART, 16-bit start/stop
fn g20v1_restart_values(start: u16, count: u16) -> Vec<u8> {
    let stop = start + count - 1;
    let mut bytes = vec![0x14, 0x01, 0x01];
    bytes.extend(start.to_le_bytes());
    bytes.extend(stop.to_le_bytes());
    for _ in 0..count {
        bytes.extend([RESTART, 0x00, 0x00, 0x00, 0x00]);
    }
    bytes
}

/// device serial number attribute in the default set
fn serial_number(value: &str) -> Vec<u8> {
    let mut bytes = vec![0x00, var::DEVICE_SERIAL_NUMBER, 0x00, 0x00, 0x00, 0x01];
    bytes.push(value.len().try_into().unwrap());
    bytes.extend_from_slice(value.as_bytes());
    bytes
}

/// binary inputs 0 to `count - 1`, the first `events` of them updated to true with an event
fn add_binaries(count: u16, events: u16) -> impl FnMut(&mut Database) {
    move |db| {
        for index in 0..count {
            db.add(
                index,
                Some(EventClass::Class1),
                BinaryInputConfig::default(),
            );
        }
        for index in 0..events {
            db.update(
                index,
                &BinaryInput::new(true, Flags::ONLINE, Time::synchronized(0)),
                UpdateOptions::default(),
            );
        }
    }
}

fn define_serial_number(value: &str) -> impl FnMut(&mut Database) {
    let value = value.to_owned();
    move |db| {
        db.define_attr(
            AttrProp::default(),
            StringAttr::DeviceSerialNumber.with_value(value.as_str()),
        )
        .unwrap();
    }
}

fn class_1_binaries(count: usize) -> BufferState {
    BufferState {
        classes: ClassCount {
            num_class_1: count,
            num_class_2: 0,
            num_class_3: 0,
        },
        types: TypeCount {
            num_binary_input: count,
            num_double_bit_binary_input: 0,
            num_binary_output_status: 0,
            num_counter: 0,
            num_frozen_counter: 0,
            num_analog: 0,
            num_analog_output_status: 0,
            num_octet_string: 0,
        },
    }
}

fn class_1_counter() -> BufferState {
    BufferState {
        classes: ClassCount {
            num_class_1: 1,
            num_class_2: 0,
            num_class_3: 0,
        },
        types: TypeCount {
            num_binary_input: 0,
            num_double_bit_binary_input: 0,
            num_binary_output_status: 0,
            num_counter: 1,
            num_frozen_counter: 0,
            num_analog: 0,
            num_analog_output_status: 0,
            num_octet_string: 0,
        },
    }
}

/// confirm the response with `seq`, expecting it to clear the events with `cleared` ids and leave
/// the `remaining` events
async fn confirm(
    harness: &mut OutstationHarness,
    seq: u8,
    cleared: Range<u64>,
    remaining: BufferState,
) {
    harness.send_and_process(&[0xC0 | seq, 0x00]).await;
    harness.check_events(&[Event::SolicitedConfirmReceived(seq), Event::BeginConfirm]);
    for id in cleared {
        harness.check_events(&[Event::Cleared(id)]);
    }
    harness.check_events(&[Event::EndConfirm(remaining)]);
}

/// expect the first fragment of a series, then confirm it
async fn expect_first_and_confirm(
    harness: &mut OutstationHarness,
    first: &[u8],
    cleared: Range<u64>,
    remaining: BufferState,
) {
    harness.expect_response(first).await;
    harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);
    confirm(harness, 0, cleared, remaining).await;
}

#[tokio::test]
async fn events_that_exactly_fill_a_fragment_leave_static_data_for_the_next() {
    let count = EVENTS_IN_MIN_FRAGMENT;
    assert_eq!(exact_fragment_for_events(count), <BufferSize>::MIN);
    let mut harness = new_harness(config_with_buffer(<BufferSize>::MIN));
    harness
        .handle
        .database
        .transaction(add_binaries(count, count));

    harness
        .send_and_process(&read(&[CLASS_1, &g1v2(0, 0)]))
        .await;
    let first = response(FIR_CON, IIN1_RESTART, &[&g2v1_events(0..count)]);
    assert_eq!(first.len(), <BufferSize>::MIN);
    expect_first_and_confirm(&mut harness, &first, 0..count.into(), class_1_binaries(0)).await;

    // no events, so no confirm
    harness
        .expect_response(&response(
            FIN_SEQ_1,
            IIN1_RESTART,
            &[&g1v2_values(0, &[BINARY_TRUE])],
        ))
        .await;
    harness.check_no_events();
}

// a response header, a g2v1 header and this many binary events leave 11 bytes in a fragment of
// `COUNTER_ONE_BYTE_SHORT`, one byte short of a g22v1 header and counter event but enough for the
// static value or attribute that follows it
const BINARIES_BEFORE_COUNTER: u16 = 77;
const COUNTER_ONE_BYTE_SHORT: usize = 4 + 5 + 3 * 77 + 12 - 1;

/// binary inputs 0 to `BINARIES_BEFORE_COUNTER - 1` and then counter 0, each with an event
fn add_binary_events_then_a_counter_event(db: &mut Database) {
    add_binaries(BINARIES_BEFORE_COUNTER, BINARIES_BEFORE_COUNTER)(db);
    db.add(0, Some(EventClass::Class1), CounterConfig::default());
    db.update(
        0,
        &Counter::new(1, Flags::ONLINE, Time::synchronized(0)),
        UpdateOptions::default(),
    );
}

/// g22v1 event for counter 0 with value 1 and flags ONLINE
const COUNTER_0_EVENT: &[u8] = &[
    0x16, 0x01, 0x28, 0x01, 0x00, 0x00, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00,
];

#[tokio::test]
async fn event_one_byte_past_a_fragment_moves_to_the_next_ahead_of_static_data() {
    let binaries = BINARIES_BEFORE_COUNTER;
    let mut harness = new_harness(config_with_buffer(COUNTER_ONE_BYTE_SHORT));
    harness
        .handle
        .database
        .transaction(add_binary_events_then_a_counter_event);

    harness
        .send_and_process(&read(&[CLASS_1, &g1v2(0, 0)]))
        .await;
    let first = response(FIR_CON, IIN1_RESTART_CLASS_1, &[&g2v1_events(0..binaries)]);
    expect_first_and_confirm(&mut harness, &first, 0..binaries.into(), class_1_counter()).await;

    harness
        .expect_response(&response(
            FIN_CON_SEQ_1,
            IIN1_RESTART,
            &[COUNTER_0_EVENT, &g1v2_values(0, &[BINARY_TRUE])],
        ))
        .await;
    let counter_id = u64::from(binaries);
    confirm(
        &mut harness,
        1,
        counter_id..counter_id + 1,
        class_1_binaries(0),
    )
    .await;
    harness.check_no_events();
}

/// flags of binaries 0 to `count - 1` where only binary 0 was updated
fn first_binary_true(count: u16) -> Vec<u8> {
    let mut flags = vec![RESTART; count.into()];
    flags[0] = BINARY_TRUE;
    flags
}

// a response header, one g2v1 event and a g1v2 header leave room for this many static values in
// a minimum fragment
const VALUES_AFTER_ONE_EVENT: u16 = 230;

#[tokio::test]
async fn an_event_and_static_data_that_exactly_fill_a_fragment_are_sent_in_one() {
    let count = VALUES_AFTER_ONE_EVENT;
    let mut harness = new_harness(config_with_buffer(<BufferSize>::MIN));
    harness.handle.database.transaction(add_binaries(count, 1));

    let expected = response(
        FIR_FIN_CON,
        IIN1_RESTART,
        &[
            &g2v1_events(0..1),
            &g1v2_values(0, &first_binary_true(count)),
        ],
    );
    assert_eq!(expected.len(), <BufferSize>::MIN);
    harness
        .test_request_response(&read(&[CLASS_1, &g1v2(0, count - 1)]), &expected)
        .await;
    harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);
    confirm(&mut harness, 0, 0..1, class_1_binaries(0)).await;
    harness.check_no_events();
}

#[tokio::test]
async fn static_value_one_byte_past_an_event_fragment_is_sent_in_the_next() {
    let count = VALUES_AFTER_ONE_EVENT + 1;
    let mut harness = new_harness(config_with_buffer(<BufferSize>::MIN));
    harness.handle.database.transaction(add_binaries(count, 1));

    harness
        .send_and_process(&read(&[CLASS_1, &g1v2(0, count - 1)]))
        .await;
    let flags = first_binary_true(count);
    let first = response(
        FIR_CON,
        IIN1_RESTART,
        &[
            &g2v1_events(0..1),
            &g1v2_values(0, &flags[..flags.len() - 1]),
        ],
    );
    assert_eq!(first.len(), <BufferSize>::MIN);
    expect_first_and_confirm(&mut harness, &first, 0..1, class_1_binaries(0)).await;

    // no events, so no confirm
    harness
        .expect_response(&response(
            FIN_SEQ_1,
            IIN1_RESTART,
            &[&g1v2_values(count - 1, &[RESTART])],
        ))
        .await;
    harness.check_no_events();
}

#[tokio::test]
async fn events_that_exactly_fill_a_fragment_leave_attributes_for_the_next() {
    let count = EVENTS_IN_MIN_FRAGMENT;
    let mut harness = new_harness(config_with_buffer(<BufferSize>::MIN));
    harness
        .handle
        .database
        .transaction(add_binaries(count, count));
    harness
        .handle
        .database
        .transaction(define_serial_number("S"));

    harness
        .send_and_process(&read(&[CLASS_1, ALL_ATTRIBUTES]))
        .await;
    let first = response(FIR_CON, IIN1_RESTART, &[&g2v1_events(0..count)]);
    assert_eq!(first.len(), <BufferSize>::MIN);
    expect_first_and_confirm(&mut harness, &first, 0..count.into(), class_1_binaries(0)).await;

    // no events, so no confirm
    harness
        .expect_response(&response(FIN_SEQ_1, IIN1_RESTART, &[&serial_number("S")]))
        .await;
    harness.check_no_events();
}

#[tokio::test]
async fn event_one_byte_past_a_fragment_moves_to_the_next_ahead_of_attributes() {
    let binaries = BINARIES_BEFORE_COUNTER;
    let mut harness = new_harness(config_with_buffer(COUNTER_ONE_BYTE_SHORT));
    harness
        .handle
        .database
        .transaction(add_binary_events_then_a_counter_event);
    harness
        .handle
        .database
        .transaction(define_serial_number("S"));

    harness
        .send_and_process(&read(&[CLASS_1, ALL_ATTRIBUTES]))
        .await;
    let first = response(FIR_CON, IIN1_RESTART_CLASS_1, &[&g2v1_events(0..binaries)]);
    expect_first_and_confirm(&mut harness, &first, 0..binaries.into(), class_1_counter()).await;

    harness
        .expect_response(&response(
            FIN_CON_SEQ_1,
            IIN1_RESTART,
            &[COUNTER_0_EVENT, &serial_number("S")],
        ))
        .await;
    let counter_id = u64::from(binaries);
    confirm(
        &mut harness,
        1,
        counter_id..counter_id + 1,
        class_1_binaries(0),
    )
    .await;
    harness.check_no_events();
}

// a response header, a g1v2 header and a 1-character serial number attribute leave room for this
// many static values in a minimum fragment
const VALUES_BEFORE_ONE_ATTRIBUTE: u16 = 230;

#[tokio::test]
async fn static_data_and_an_attribute_that_exactly_fill_a_fragment_are_sent_in_one() {
    let count = VALUES_BEFORE_ONE_ATTRIBUTE;
    let mut harness = new_harness(config_with_buffer(<BufferSize>::MIN));
    harness.handle.database.transaction(add_binaries(count, 0));
    harness
        .handle
        .database
        .transaction(define_serial_number("S"));

    let expected = response(
        FIR_FIN,
        IIN1_RESTART,
        &[
            &g1v2_values(0, &vec![RESTART; count.into()]),
            &serial_number("S"),
        ],
    );
    assert_eq!(expected.len(), <BufferSize>::MIN);
    harness
        .test_request_response(&read(&[&g1v2(0, count - 1), ALL_ATTRIBUTES]), &expected)
        .await;
    harness.check_no_events();
}

#[tokio::test]
async fn attribute_one_byte_past_a_static_fragment_is_sent_in_the_next() {
    let count = VALUES_BEFORE_ONE_ATTRIBUTE;
    let mut harness = new_harness(config_with_buffer(<BufferSize>::MIN));
    harness.handle.database.transaction(add_binaries(count, 0));
    harness
        .handle
        .database
        .transaction(define_serial_number("SN"));

    harness
        .send_and_process(&read(&[&g1v2(0, count - 1), ALL_ATTRIBUTES]))
        .await;
    let first = response(
        FIR_CON,
        IIN1_RESTART,
        &[&g1v2_values(0, &vec![RESTART; count.into()])],
    );
    expect_first_and_confirm(&mut harness, &first, 0..0, class_1_binaries(0)).await;

    harness
        .expect_response(&response(FIN_SEQ_1, IIN1_RESTART, &[&serial_number("SN")]))
        .await;
    harness.check_no_events();
}

// a response header and a g20v1 header leave room for this many 5-byte values in a minimum
// fragment
const COUNTERS_IN_MIN_FRAGMENT: u16 = 47;

#[tokio::test]
async fn class_0_and_attributes_span_fragments_without_a_confirm_for_the_last() {
    let count = 2 * COUNTERS_IN_MIN_FRAGMENT + 6;
    let mut harness = new_harness(config_with_buffer(<BufferSize>::MIN));
    harness.handle.database.transaction(|db| {
        for index in 0..count {
            db.add(index, Some(EventClass::Class1), CounterConfig::default());
        }
    });
    harness
        .handle
        .database
        .transaction(define_serial_number("S"));

    harness
        .send_and_process(&read(&[CLASS_0, ALL_ATTRIBUTES]))
        .await;
    let first = response(
        FIR_CON,
        IIN1_RESTART,
        &[&g20v1_restart_values(0, COUNTERS_IN_MIN_FRAGMENT)],
    );
    expect_first_and_confirm(&mut harness, &first, 0..0, class_1_binaries(0)).await;

    harness
        .expect_response(&response(
            CON_SEQ_1,
            IIN1_RESTART,
            &[&g20v1_restart_values(
                COUNTERS_IN_MIN_FRAGMENT,
                COUNTERS_IN_MIN_FRAGMENT,
            )],
        ))
        .await;
    confirm(&mut harness, 1, 0..0, class_1_binaries(0)).await;

    // no events, so no confirm
    harness
        .expect_response(&response(
            FIN_SEQ_2,
            IIN1_RESTART,
            &[
                &g20v1_restart_values(2 * COUNTERS_IN_MIN_FRAGMENT, 6),
                &serial_number("S"),
            ],
        ))
        .await;
    harness.check_no_events();
}

const ENABLE_UNSOLICITED_SEQ0: &[u8] = &[
    0xC0, 0x14, 0x3C, 0x02, 0x06, 0x3C, 0x03, 0x06, 0x3C, 0x04, 0x06,
];

/// an unsolicited response with FIR, FIN and CON
fn unsolicited(seq: u8, iin1: u8, objects: &[u8]) -> Vec<u8> {
    let mut response = vec![0xF0 | seq, 0x82, iin1, 0x00];
    response.extend_from_slice(objects);
    response
}

/// confirm the unsolicited response with `seq`, expecting it to clear the events with `cleared`
/// ids and leave `remaining` class 1 binary events
async fn confirm_unsolicited(
    harness: &mut OutstationHarness,
    seq: u8,
    cleared: Range<u64>,
    remaining: BufferState,
) {
    harness.send_and_process(&[0xD0 | seq, 0x00]).await;
    harness.check_events(&[Event::UnsolicitedConfirmReceived(seq), Event::BeginConfirm]);
    for id in cleared {
        harness.check_events(&[Event::Cleared(id)]);
    }
    harness.check_events(&[Event::EndConfirm(remaining)]);
}

/// an outstation with unsolicited responses enabled for every class and fragments of `size`
async fn enabled_unsolicited_harness(size: usize) -> OutstationHarness {
    let mut config = get_default_unsolicited_config();
    config.unsolicited_buffer_size = BufferSize::new(size).unwrap();
    config.event_buffer_config = EventBufferConfig::all_types(100);
    let mut harness = new_harness(config);

    harness
        .expect_response(&unsolicited(0, IIN1_RESTART, &[]))
        .await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(0)]);
    harness.send_and_process(&[0xD0, 0x00]).await;
    harness.check_events(&[Event::UnsolicitedConfirmReceived(0)]);
    harness
        .test_request_response(
            ENABLE_UNSOLICITED_SEQ0,
            &response(FIR_FIN, IIN1_RESTART, &[]),
        )
        .await;
    harness
}

#[tokio::test]
async fn unsolicited_events_that_exactly_fill_a_fragment_are_sent_in_one() {
    let count = EVENTS_IN_MIN_FRAGMENT;
    let mut harness = enabled_unsolicited_harness(exact_fragment_for_events(count)).await;
    harness
        .handle
        .database
        .transaction(add_binaries(count, count));

    let expected = unsolicited(1, IIN1_RESTART, &g2v1_events(0..count));
    assert_eq!(expected.len(), <BufferSize>::MIN);
    harness.expect_response(&expected).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);
    confirm_unsolicited(&mut harness, 1, 0..count.into(), class_1_binaries(0)).await;
    harness.check_no_events();
    harness.expect_no_response();
}

#[tokio::test]
async fn unsolicited_event_one_byte_past_a_fragment_is_sent_in_the_next_series() {
    let count = EVENTS_IN_MIN_FRAGMENT + 1;
    let mut harness = enabled_unsolicited_harness(exact_fragment_for_events(count) - 1).await;
    harness
        .handle
        .database
        .transaction(add_binaries(count, count));

    harness
        .expect_response(&unsolicited(
            1,
            IIN1_RESTART_CLASS_1,
            &g2v1_events(0..count - 1),
        ))
        .await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);
    confirm_unsolicited(
        &mut harness,
        1,
        0..u64::from(count - 1),
        class_1_binaries(1),
    )
    .await;

    harness
        .expect_response(&unsolicited(
            2,
            IIN1_RESTART,
            &g2v1_events(count - 1..count),
        ))
        .await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(2)]);
    confirm_unsolicited(
        &mut harness,
        2,
        u64::from(count - 1)..count.into(),
        class_1_binaries(0),
    )
    .await;
    harness.check_no_events();
}
