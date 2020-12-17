use super::harness::*;
use crate::app::flags::Flags;
use crate::app::measurement::{Binary, Time};
use crate::outstation::config::OutstationConfig;
use crate::outstation::database::config::BinaryConfig;
use crate::outstation::database::{Add, DatabaseHandle, EventClass, Update, UpdateOptions};
use crate::util::task::RunError;

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
const ENABLE_UNSOLICITED_SEQ0: &[u8] = &[
    0xC0, 0x14, 0x3C, 0x02, 0x06, 0x3C, 0x03, 0x06, 0x3C, 0x04, 0x06,
];
const EMPTY_RESPONSE_SEQ0: &[u8] = &[0xC0, 0x81, 0x80, 0x00];

fn generate_binary_event(handle: &mut DatabaseHandle) {
    handle.transaction(|db| {
        db.add(0, Some(EventClass::Class1), BinaryConfig::default());
        db.update(
            0,
            &Binary::new(true, Flags::ONLINE, Time::synchronized(0)),
            UpdateOptions::default(),
        )
    });
}

fn enable_unsolicited<T>(harness: &mut OutstationTestHarness<T>)
where
    T: std::future::Future<Output = Result<(), RunError>>,
{
    harness.test_request_response(ENABLE_UNSOLICITED_SEQ0, EMPTY_RESPONSE_SEQ0);
}

fn confirm_null_unsolicited<T>(harness: &mut OutstationTestHarness<T>)
where
    T: std::future::Future<Output = Result<(), RunError>>,
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
fn null_unsolicited_can_time_out_and_retry() {
    let mut harness = new_harness(get_default_unsolicited_config());
    harness.expect_response(NULL_UNSOL_SEQ_0);
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(0)]);

    // this would go on forever, but let's just test 3 iterations
    for _ in 0..3 {
        crate::tokio::time::advance(OutstationConfig::DEFAULT_CONFIRM_TIMEOUT);
        harness.expect_response(NULL_UNSOL_SEQ_0);
        harness.check_events(&[Event::UnsolicitedConfirmTimeout(0, true)]);
    }
}

#[test]
fn null_unsolicited_can_timeout_and_not_retry() {
    let mut harness = new_harness(config_with_limited_retries(1));
    harness.expect_response(NULL_UNSOL_SEQ_0);
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(0)]);

    // a single retry
    crate::tokio::time::advance(OutstationConfig::DEFAULT_CONFIRM_TIMEOUT);
    harness.expect_response(NULL_UNSOL_SEQ_0);
    harness.check_events(&[Event::UnsolicitedConfirmTimeout(0, true)]);

    crate::tokio::time::advance(OutstationConfig::DEFAULT_CONFIRM_TIMEOUT);
    harness.poll_pending();
    harness.check_events(&[Event::UnsolicitedConfirmTimeout(0, false)]);
    harness.check_all_io_consumed();
}

#[test]
fn null_unsolicited_can_timeout_series_wait_and_start_another_series() {
    let mut harness = new_harness(config_with_limited_retries(0));

    // first response series
    harness.expect_response(NULL_UNSOL_SEQ_0);
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(0)]);
    crate::tokio::time::advance(OutstationConfig::DEFAULT_CONFIRM_TIMEOUT);
    harness.poll_pending();
    harness.check_events(&[Event::UnsolicitedConfirmTimeout(0, false)]);
    harness.check_all_io_consumed();

    // we're now back in IDLE, and need to wait to attempt a new series
    crate::tokio::time::advance(OutstationConfig::DEFAULT_UNSOLICITED_RETRY_DELAY);
    harness.expect_response(NULL_UNSOL_SEQ_1);
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);
}

#[test]
fn data_unsolicited_can_be_confirmed() {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness);
    enable_unsolicited(&mut harness);

    generate_binary_event(&mut harness.database);
    harness.expect_response(UNSOL_G2V1_SEQ1);
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);
    harness.send(UNS_CONFIRM_SEQ_1);
    harness.check_events(&[Event::UnsolicitedConfirmReceived(1)]);
}
