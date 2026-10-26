// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

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
use crate::outstation::BufferState;

/// g60v2 (class 1)
const CLASS_1: &[u8] = &[0x3C, 0x02, 0x06];
/// g60v1 (class 0)
const CLASS_0: &[u8] = &[0x3C, 0x01, 0x06];
/// g0v254 (all attributes) of every set
const ALL_ATTRIBUTES: &[u8] = &[0x00, 0xFE, 0x06];
/// g110v0 (all static octet strings)
const ALL_STATIC_OCTET_STRINGS: &[u8] = &[110, 0, 0x06];

/// binary flags: ONLINE and state true
const BINARY_TRUE: u8 = 0x81;
/// flags of a point that has never been updated: RESTART
const RESTART: u8 = 0x02;

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

/// a response with no IIN2 bits and the objects of each header
fn response(control: u8, iin1: u8, objects: &[&[u8]]) -> Vec<u8> {
    let mut response = response_header(control, iin1, 0).to_vec();
    for object in objects {
        response.extend_from_slice(object);
    }
    response
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

fn add_oversize_octet_string(db: &mut Database) {
    assert!(db.add(0, None, OctetStringConfig));
    db.update(
        0,
        &OctetString::new(&[7; 255]).unwrap(),
        UpdateOptions::no_event(),
    );
}

#[tokio::test]
async fn unreadable_octet_string_ends_read_without_confirm() {
    crate::util::test_support::watchdog(async {
        let mut harness = new_harness(config_with_buffer(<BufferSize>::MIN));
        harness
            .handle
            .database
            .transaction(add_oversize_octet_string);

        harness
            .test_request_response(
                &read(&[ALL_STATIC_OCTET_STRINGS]),
                &empty_response(0, iin2::PARAMETER_ERROR),
            )
            .await;
        harness.check_no_events();
    })
    .await;
}

#[tokio::test]
async fn unreadable_octet_string_ends_later_fragment_without_confirm() {
    crate::util::test_support::watchdog(async {
        let mut harness = new_harness(config_with_buffer(<BufferSize>::MIN));
        harness.handle.database.transaction(|db| {
            db.add(0, None, BinaryInputConfig::default());
            add_oversize_octet_string(db);
        });

        let binary = g1v2(0, 0);
        harness
            .send_and_process(&read(&[&binary, ALL_STATIC_OCTET_STRINGS]))
            .await;
        harness
            .expect_response(&response(
                FIR_CON,
                iin1::RESTART,
                &[&g1v2_values(0, &[RESTART])],
            ))
            .await;
        harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);

        confirm(&mut harness, 0, 0..0, empty_buffer_state()).await;
        harness
            .expect_response(&response_header(
                FIN_SEQ_1,
                iin1::RESTART,
                iin2::PARAMETER_ERROR,
            ))
            .await;
        harness.check_no_events();

        harness
            .test_request_response(
                &read(&[&binary]),
                &response(FIR_FIN, iin1::RESTART, &[&g1v2_values(0, &[RESTART])]),
            )
            .await;
    })
    .await;
}

fn class_1_counter() -> BufferState {
    let mut state = empty_buffer_state();
    state.classes.num_class_1 = 1;
    state.types.num_counter = 1;
    state
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
    let first = response(FIR_CON, iin1::RESTART, &[&g2v1_events(0..count)]);
    assert_eq!(first.len(), <BufferSize>::MIN);
    expect_first_and_confirm(&mut harness, &first, 0..count.into(), class_1_binaries(0)).await;

    // no events, so no confirm
    harness
        .expect_response(&response(
            FIN_SEQ_1,
            iin1::RESTART,
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
    let first = response(
        FIR_CON,
        iin1::RESTART | iin1::CLASS_1,
        &[&g2v1_events(0..binaries)],
    );
    expect_first_and_confirm(&mut harness, &first, 0..binaries.into(), class_1_counter()).await;

    harness
        .expect_response(&response(
            FIN_CON_SEQ_1,
            iin1::RESTART,
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
        iin1::RESTART,
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
        iin1::RESTART,
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
            iin1::RESTART,
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
    let first = response(FIR_CON, iin1::RESTART, &[&g2v1_events(0..count)]);
    assert_eq!(first.len(), <BufferSize>::MIN);
    expect_first_and_confirm(&mut harness, &first, 0..count.into(), class_1_binaries(0)).await;

    // no events, so no confirm
    harness
        .expect_response(&response(
            FIN_SEQ_1,
            iin1::RESTART,
            &[&encode_string(var::DEVICE_SERIAL_NUMBER, "S")],
        ))
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
    let first = response(
        FIR_CON,
        iin1::RESTART | iin1::CLASS_1,
        &[&g2v1_events(0..binaries)],
    );
    expect_first_and_confirm(&mut harness, &first, 0..binaries.into(), class_1_counter()).await;

    harness
        .expect_response(&response(
            FIN_CON_SEQ_1,
            iin1::RESTART,
            &[
                COUNTER_0_EVENT,
                &encode_string(var::DEVICE_SERIAL_NUMBER, "S"),
            ],
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
        iin1::RESTART,
        &[
            &g1v2_values(0, &vec![RESTART; count.into()]),
            &encode_string(var::DEVICE_SERIAL_NUMBER, "S"),
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
        iin1::RESTART,
        &[&g1v2_values(0, &vec![RESTART; count.into()])],
    );
    expect_first_and_confirm(&mut harness, &first, 0..0, class_1_binaries(0)).await;

    harness
        .expect_response(&response(
            FIN_SEQ_1,
            iin1::RESTART,
            &[&encode_string(var::DEVICE_SERIAL_NUMBER, "SN")],
        ))
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
        iin1::RESTART,
        &[&g20v1_restart_values(0, COUNTERS_IN_MIN_FRAGMENT)],
    );
    expect_first_and_confirm(&mut harness, &first, 0..0, class_1_binaries(0)).await;

    harness
        .expect_response(&response(
            CON_SEQ_1,
            iin1::RESTART,
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
            iin1::RESTART,
            &[
                &g20v1_restart_values(2 * COUNTERS_IN_MIN_FRAGMENT, 6),
                &encode_string(var::DEVICE_SERIAL_NUMBER, "S"),
            ],
        ))
        .await;
    harness.check_no_events();
}

/// a g51 CTO with the 48-bit time
fn cto(variation: u8, time: u64) -> Vec<u8> {
    let mut bytes = vec![0x33, variation, 0x07, 0x01];
    bytes.extend_from_slice(&time.to_le_bytes()[..6]);
    bytes
}

/// a variation 3 header of events with relative times, as (index, flags, time after the CTO)
fn relative_time_events(group: u8, events: &[(u16, u8, u16)]) -> Vec<u8> {
    let mut bytes = vec![group, 0x03, 0x28];
    bytes.extend(u16::try_from(events.len()).unwrap().to_le_bytes());
    for &(index, flags, time) in events {
        bytes.extend(index.to_le_bytes());
        bytes.push(flags);
        bytes.extend(time.to_le_bytes());
    }
    bytes
}

/// a variation 2 header of events with absolute times, as (index, flags, time)
fn absolute_time_events(group: u8, events: &[(u16, u8, u64)]) -> Vec<u8> {
    let mut bytes = vec![group, 0x02, 0x28];
    bytes.extend(u16::try_from(events.len()).unwrap().to_le_bytes());
    for &(index, flags, time) in events {
        bytes.extend(index.to_le_bytes());
        bytes.push(flags);
        bytes.extend_from_slice(&time.to_le_bytes()[..6]);
    }
    bytes
}

/// a response header, a g51v2 CTO and a g2v3 header leave room for this many 5-byte events in a
/// minimum fragment
const RELATIVE_EVENTS_IN_MIN_FRAGMENT: u16 = 46;
const UNSYNCHRONIZED_START: u64 = 1000;

#[tokio::test]
async fn unsynchronized_g2v2_events_continue_after_a_new_cto_in_the_next_fragment() {
    let count = RELATIVE_EVENTS_IN_MIN_FRAGMENT;
    let mut harness = new_harness(config_with_buffer(<BufferSize>::MIN));
    harness.handle.database.transaction(|db| {
        for index in 0..count + 2 {
            db.add(
                index,
                Some(EventClass::Class1),
                BinaryInputConfig::new(
                    StaticBinaryInputVariation::Group1Var2,
                    EventBinaryInputVariation::Group2Var2,
                ),
            );
            db.update(
                index,
                &BinaryInput::new(
                    true,
                    Flags::ONLINE,
                    Time::unsynchronized(UNSYNCHRONIZED_START + u64::from(index)),
                ),
                UpdateOptions::default(),
            );
        }
    });

    harness.send_and_process(&read(&[CLASS_1])).await;
    let events: Vec<_> = (0..count).map(|i| (i, BINARY_TRUE, i)).collect();
    let first = response(
        FIR_CON,
        iin1::RESTART | iin1::CLASS_1,
        &[
            &cto(2, UNSYNCHRONIZED_START),
            &relative_time_events(2, &events),
        ],
    );
    assert_eq!(first.len(), <BufferSize>::MIN);
    expect_first_and_confirm(&mut harness, &first, 0..count.into(), class_1_binaries(2)).await;

    harness
        .expect_response(&response(
            FIN_CON_SEQ_1,
            iin1::RESTART,
            &[
                &cto(2, UNSYNCHRONIZED_START + u64::from(count)),
                &relative_time_events(2, &[(count, BINARY_TRUE, 0), (count + 1, BINARY_TRUE, 1)]),
            ],
        ))
        .await;
    let next = u64::from(count);
    confirm(&mut harness, 1, next..next + 2, class_1_binaries(0)).await;
    harness.check_no_events();
}

/// double-bit flags: ONLINE and state determined on
const DOUBLE_BIT_ON: u8 = 0x81;
/// synchronized g4v2 events in the first fragment
const ABSOLUTE_EVENTS_BEFORE_CTO: u16 = 25;
/// a response header and a g4v2 header with `ABSOLUTE_EVENTS_BEFORE_CTO` events leave 19 bytes,
/// enough for a g51v2 CTO and a g4v3 header but one byte short of the first g4v3 event as well
const CTO_EVENT_ONE_BYTE_SHORT: usize = 4 + 5 + 9 * 25 + 10 + 5 + 5 - 1;
const SYNCHRONIZED_START: u64 = 2000;

fn class_1_double_bits(count: usize) -> BufferState {
    let mut state = empty_buffer_state();
    state.classes.num_class_1 = count;
    state.types.num_double_bit_binary_input = count;
    state
}

#[tokio::test]
async fn unsynchronized_g4v2_event_whose_cto_does_not_fit_starts_the_next_fragment() {
    let absolute = ABSOLUTE_EVENTS_BEFORE_CTO;
    let times: Vec<Time> = (0..absolute)
        .map(|i| Time::synchronized(SYNCHRONIZED_START + u64::from(i)))
        .chain([
            Time::unsynchronized(UNSYNCHRONIZED_START),
            Time::unsynchronized(UNSYNCHRONIZED_START + 1),
            Time::synchronized(SYNCHRONIZED_START + 100),
        ])
        .collect();
    let mut harness = new_harness(config_with_buffer(CTO_EVENT_ONE_BYTE_SHORT));
    harness.handle.database.transaction(|db| {
        for (index, time) in (0..).zip(times.iter().copied()) {
            db.add(
                index,
                Some(EventClass::Class1),
                DoubleBitBinaryInputConfig::new(
                    StaticDoubleBitBinaryInputVariation::Group3Var2,
                    EventDoubleBitBinaryInputVariation::Group4Var2,
                ),
            );
            db.update(
                index,
                &DoubleBitBinaryInput::new(DoubleBit::DeterminedOn, Flags::ONLINE, time),
                UpdateOptions::default(),
            );
        }
    });

    harness.send_and_process(&read(&[CLASS_1])).await;
    let events: Vec<_> = (0..absolute)
        .map(|i| (i, DOUBLE_BIT_ON, SYNCHRONIZED_START + u64::from(i)))
        .collect();
    let first = response(
        FIR_CON,
        iin1::RESTART | iin1::CLASS_1,
        &[&absolute_time_events(4, &events)],
    );
    assert_eq!(first.len(), CTO_EVENT_ONE_BYTE_SHORT - 19);
    expect_first_and_confirm(
        &mut harness,
        &first,
        0..absolute.into(),
        class_1_double_bits(3),
    )
    .await;

    harness
        .expect_response(&response(
            FIN_CON_SEQ_1,
            iin1::RESTART,
            &[
                &cto(2, UNSYNCHRONIZED_START),
                &relative_time_events(
                    4,
                    &[
                        (absolute, DOUBLE_BIT_ON, 0),
                        (absolute + 1, DOUBLE_BIT_ON, 1),
                    ],
                ),
                &absolute_time_events(
                    4,
                    &[(absolute + 2, DOUBLE_BIT_ON, SYNCHRONIZED_START + 100)],
                ),
            ],
        ))
        .await;
    let next = u64::from(absolute);
    confirm(&mut harness, 1, next..next + 3, class_1_double_bits(0)).await;
    harness.check_no_events();
}

/// confirm the unsolicited response with `seq`, expecting it to clear the events with `cleared`
/// ids and leave `remaining` class 1 binary events
async fn confirm_unsolicited(
    harness: &mut OutstationHarness,
    seq: u8,
    cleared: Range<u64>,
    remaining: BufferState,
) {
    harness.send_and_process(&unsolicited_confirm(seq)).await;
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
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;
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

    let expected = unsolicited(1, iin1::RESTART, &g2v1_events(0..count));
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
            iin1::RESTART | iin1::CLASS_1,
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
            iin1::RESTART,
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
