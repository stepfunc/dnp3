use crate::app::{measurement::*, Timestamp};
use crate::outstation::config::OutstationConfig;
use crate::outstation::database::*;
use crate::outstation::session::RunError;

use super::harness::*;

const fn uns_confirm(seq: u8) -> u8 {
    0b1101_0000 | seq
}

const NULL_UNSOL_SEQ_0: &[u8] = &[0xF0, 0x82, 0x80, 0x00];
const NULL_UNSOL_SEQ_1: &[u8] = &[0xF1, 0x82, 0x80, 0x00];
const UNS_CONFIRM_SEQ_0: &[u8] = &[uns_confirm(0), 0x00];
const UNS_CONFIRM_SEQ_1: &[u8] = &[uns_confirm(1), 0x00];
const UNSOL_G2V1_SEQ1: &[u8] = &[
    0xF1, 0x82, 0x80, 0x00, 0x02, 0x01, 0x28, 0x01, 0x00, 0x00, 0x00, 0x81,
];
const UNSOL_G2V1_SEQ2: &[u8] = &[
    0xF2, 0x82, 0x80, 0x00, 0x02, 0x01, 0x28, 0x01, 0x00, 0x00, 0x00, 0x81,
];
const ENABLE_UNSOLICITED_SEQ0: &[u8] = &[
    0xC0, 0x14, 0x3C, 0x02, 0x06, 0x3C, 0x03, 0x06, 0x3C, 0x04, 0x06,
];
const DISABLE_UNSOLICITED_SEQ0: &[u8] = &[
    0xC0, 0x15, 0x3C, 0x02, 0x06, 0x3C, 0x03, 0x06, 0x3C, 0x04, 0x06,
];

const READ_CLASS_0: &[u8] = &[0xC0, 0x01, 0x3C, 0x01, 0x06];
const EMPTY_RESPONSE_SEQ0: &[u8] = &[0xC0, 0x81, 0x80, 0x00];
const CLASS_0_RESPONSE_SEQ0: &[u8] = &[
    0xC0, 0x81, 0x80, 0x00, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01,
];
const CLASS_0_RESPONSE_SEQ0_WITH_PENDING_EVENTS: &[u8] = &[
    0xC0, 0x81, 0x82, 0x00, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01,
];

fn generate_binary_event(handle: &mut DatabaseHandle) {
    handle.transaction(|db| {
        db.add(0, Some(EventClass::Class1), BinaryInputConfig::default());
        db.update(
            0,
            &BinaryInput::new(true, Flags::ONLINE, Time::synchronized(0)),
            UpdateOptions::default(),
        )
    });
}

fn enable_unsolicited<T>(harness: &mut OutstationTestHarness<T>)
where
    T: std::future::Future<Output = RunError>,
{
    harness.test_request_response(ENABLE_UNSOLICITED_SEQ0, EMPTY_RESPONSE_SEQ0);
}

fn confirm_null_unsolicited<T>(harness: &mut OutstationTestHarness<T>)
where
    T: std::future::Future<Output = RunError>,
{
    harness.expect_response(NULL_UNSOL_SEQ_0);
    harness.send(UNS_CONFIRM_SEQ_0);
    harness.check_events(&[
        Event::EnterUnsolicitedConfirmWait(0),
        Event::UnsolicitedConfirmReceived(0),
    ]);
}

fn config_with_limited_retries(retries: usize) -> OutstationConfig {
    let mut config = get_default_unsolicited_config();
    config.max_unsolicited_retries = Some(retries);
    config
}

#[test]
fn null_unsolicited_always_retries() {
    let mut harness = new_harness(get_default_unsolicited_config());
    harness.expect_response(NULL_UNSOL_SEQ_0);
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(0)]);

    crate::tokio::time::advance(OutstationConfig::DEFAULT_CONFIRM_TIMEOUT);
    harness.poll_pending();
    harness.check_events(&[Event::UnsolicitedConfirmTimeout(0, false)]);

    harness.expect_response(NULL_UNSOL_SEQ_1);
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);
}

#[test]
fn unsolicited_can_time_out_and_retry() {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness);
    enable_unsolicited(&mut harness);
    generate_binary_event(&mut harness.handle.database);

    // first response
    harness.expect_response(UNSOL_G2V1_SEQ1);
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);

    // this would go on forever, but let's just test 3 iterations
    for _ in 0..3 {
        crate::tokio::time::advance(OutstationConfig::DEFAULT_CONFIRM_TIMEOUT);
        harness.expect_response(UNSOL_G2V1_SEQ1);
        harness.check_events(&[Event::UnsolicitedConfirmTimeout(1, true)]);
    }
}

#[test]
fn unsolicited_can_timeout_and_not_retry() {
    let mut harness = new_harness(config_with_limited_retries(2));
    confirm_null_unsolicited(&mut harness);
    enable_unsolicited(&mut harness);
    generate_binary_event(&mut harness.handle.database);

    // response
    harness.expect_response(UNSOL_G2V1_SEQ1);
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);

    // first retry
    crate::tokio::time::advance(OutstationConfig::DEFAULT_CONFIRM_TIMEOUT);
    harness.expect_response(UNSOL_G2V1_SEQ1);
    harness.check_events(&[Event::UnsolicitedConfirmTimeout(1, true)]);

    // second retry
    crate::tokio::time::advance(OutstationConfig::DEFAULT_CONFIRM_TIMEOUT);
    harness.expect_response(UNSOL_G2V1_SEQ1);
    harness.check_events(&[Event::UnsolicitedConfirmTimeout(1, true)]);

    // timeout
    crate::tokio::time::advance(OutstationConfig::DEFAULT_CONFIRM_TIMEOUT);
    harness.flush_io();
    harness.check_events(&[Event::UnsolicitedConfirmTimeout(1, false)]);

    // new series
    crate::tokio::time::advance(OutstationConfig::DEFAULT_UNSOLICITED_RETRY_DELAY);
    harness.expect_response(UNSOL_G2V1_SEQ2);
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(2)]);
}

#[test]
fn unsolicited_can_timeout_series_wait_and_start_another_series() {
    let mut harness = new_harness(config_with_limited_retries(0));
    confirm_null_unsolicited(&mut harness);
    enable_unsolicited(&mut harness);
    generate_binary_event(&mut harness.handle.database);

    // first response series
    harness.expect_response(UNSOL_G2V1_SEQ1);
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);
    crate::tokio::time::advance(OutstationConfig::DEFAULT_CONFIRM_TIMEOUT);
    harness.flush_io();
    harness.check_events(&[Event::UnsolicitedConfirmTimeout(1, false)]);

    // we're now back in IDLE, and need to wait to attempt a new series
    crate::tokio::time::advance(OutstationConfig::DEFAULT_UNSOLICITED_RETRY_DELAY);
    harness.expect_response(UNSOL_G2V1_SEQ2);
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(2)]);
}

#[test]
fn data_unsolicited_can_be_confirmed() {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness);
    enable_unsolicited(&mut harness);

    generate_binary_event(&mut harness.handle.database);
    harness.expect_response(UNSOL_G2V1_SEQ1);
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);
    harness.send(UNS_CONFIRM_SEQ_1);
    harness.check_events(&[Event::UnsolicitedConfirmReceived(1)]);
}

#[test]
fn defers_read_during_unsol_confirm_wait() {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness);
    enable_unsolicited(&mut harness);

    generate_binary_event(&mut harness.handle.database);
    harness.expect_response(UNSOL_G2V1_SEQ1);
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);
    // send a read which will be deferred
    harness.send(READ_CLASS_0);
    harness.check_no_events();
    // now send the confirm
    harness.send(UNS_CONFIRM_SEQ_1);
    harness.check_events(&[Event::UnsolicitedConfirmReceived(1)]);
    harness.expect_response(CLASS_0_RESPONSE_SEQ0);
    harness.flush_io();
    harness.check_no_events();
}

#[test]
fn defers_read_during_unsol_confirm_wait_timeout() {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness);
    enable_unsolicited(&mut harness);

    generate_binary_event(&mut harness.handle.database);
    harness.expect_response(UNSOL_G2V1_SEQ1);
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);
    // send a read which will be deferred
    harness.send(READ_CLASS_0);
    harness.check_no_events();
    // expire the unsolicited response
    crate::tokio::time::advance(OutstationConfig::DEFAULT_CONFIRM_TIMEOUT);
    harness.poll_pending();
    harness.check_events(&[Event::UnsolicitedConfirmTimeout(1, false)]);
    harness.expect_response(CLASS_0_RESPONSE_SEQ0_WITH_PENDING_EVENTS);
    harness.flush_io();
    harness.check_no_events();
}

#[test]
fn handles_non_read_during_unsolicited_confirm_wait() {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness);
    enable_unsolicited(&mut harness);

    generate_binary_event(&mut harness.handle.database);
    harness.expect_response(UNSOL_G2V1_SEQ1);
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);
    // send a delay measurement request while still in unsolicited confirm wait
    harness.test_request_response(
        super::data::DELAY_MEASURE,
        super::data::RESPONSE_TIME_DELAY_FINE_ZERO,
    );
    harness.check_no_events();

    // now send the confirm
    harness.send(UNS_CONFIRM_SEQ_1);
    harness.check_events(&[Event::UnsolicitedConfirmReceived(1)]);
}

#[test]
fn handles_invalid_request_during_unsolicited_confirm_wait() {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness);
    enable_unsolicited(&mut harness);

    generate_binary_event(&mut harness.handle.database);
    harness.expect_response(UNSOL_G2V1_SEQ1);
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);
    // send a delay measurement request while still in unsolicited confirm wait
    harness.test_request_response(
        &[0xC0, 0x70],             // Invalid request
        &[0xC0, 0x81, 0x80, 0x01], // NO_FUNC_CODE_SUPPORT
    );
    harness.check_no_events();

    // now send the confirm
    harness.send(UNS_CONFIRM_SEQ_1);
    harness.check_events(&[Event::UnsolicitedConfirmReceived(1)]);
}

#[test]
fn handles_disable_unsolicited_during_unsolicited_confirm_wait() {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness);
    enable_unsolicited(&mut harness);

    generate_binary_event(&mut harness.handle.database);
    harness.expect_response(UNSOL_G2V1_SEQ1);
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);

    // send a disable unsolicited request
    harness.test_request_response(DISABLE_UNSOLICITED_SEQ0, EMPTY_RESPONSE_SEQ0);
    harness.check_no_events();

    // check that no other unsolicited responses are sent
    crate::tokio::time::advance(OutstationConfig::DEFAULT_UNSOLICITED_RETRY_DELAY);
    harness.poll_pending();
    harness.check_no_events();
}

#[test]
fn buffer_overflow_issue() {
    let mut config = get_default_unsolicited_config();
    config.event_buffer_config = EventBufferConfig::all_types(1);
    let mut harness = new_harness_with_custom_event_buffers(config);
    confirm_null_unsolicited(&mut harness);
    enable_unsolicited(&mut harness);

    fn generate_overflow(database: &mut DatabaseHandle) {
        database.transaction(|database| {
            database.update(
                0,
                &BinaryInput::new(true, Flags::ONLINE, Time::Synchronized(Timestamp::new(0))),
                UpdateOptions::default(),
            );
            database.update(
                0,
                &BinaryInput::new(false, Flags::ONLINE, Time::Synchronized(Timestamp::new(0))),
                UpdateOptions::default(),
            );
        });
    }

    // Add the point
    harness.handle.database.transaction(|database| {
        database.add(0, Some(EventClass::Class1), BinaryInputConfig::default());
    });

    generate_overflow(&mut harness.handle.database);

    // Unsolicited response with event data and EVENT_BUFFER_OVERFLOW
    harness.expect_response(&[
        0xF1, 0x82, 0x80, 0x08, // DEVICE_RESTART and EVENT_BUFFER_OVERFLOW asserted
        0x02, 0x01, 0x28, 0x01, 0x00, // 1 event g2v1 only
        0x00, 0x00, 0x01,
    ]);

    // THIS USED TO GENERATE A SUBTRACT OVERFLOW IN THE EVENT BUFFER (the panic occured in the next line)
    generate_overflow(&mut harness.handle.database);

    harness.send(&[0xD1, 0x00]);

    // New unsolicited response with a single event
    harness.expect_response(&[
        0xF2, 0x82, 0x80, 0x08, // DEVICE_RESTART and EVENT_BUFFER_OVERFLOW asserted
        0x02, 0x01, 0x28, 0x01, 0x00, // 1 event g2v1 only
        0x00, 0x00, 0x01,
    ]);
    harness.send(&[0xD2, 0x00]);

    // Integrity poll response should not contain EVENT_BUFFER_OVERFLOW flag anymore
    harness.test_request_response(
        &[
            0xC0, 0x01, 60, 2, 0x06, 60, 3, 0x06, 60, 4, 0x06, 60, 1, 0x06,
        ],
        &[
            0xC0, 0x81, 0x80, 0x00, // Only DEVICE_RESTART
            0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, // g1v1 [0, 0]
            0x00, // Current value
        ],
    );
}
