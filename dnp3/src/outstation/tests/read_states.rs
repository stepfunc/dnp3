use tokio::time::Duration;

use crate::app::measurement::*;
use crate::app::*;
use crate::outstation::database::*;
use crate::outstation::tests::harness::*;

const EMPTY_READ: &[u8] = &[0xC0, 0x01];
const READ_CLASS_123: &[u8] = &[0xC0, 0x01, 60, 02, 0x06, 60, 03, 0x06, 60, 04, 0x06];
const CONFIRM_SEQ_0: &[u8] = &[0xC0, 0x00];
const UNS_CONFIRM_SEQ_0: &[u8] = &[0b11010000, 0x00];
const CONFIRM_SEQ_1: &[u8] = &[0xC1, 0x00];
const EMPTY_RESPONSE: &[u8] = &[0xC0, 0x81, 0x80, 0x00];
const BINARY_EVENT_RESPONSE: &[u8] = &[
    0xE0, 0x81, 0x80, 0x00, 0x02, 0x01, 0x28, 0x01, 0x00, 0x00, 0x00, 0x81,
];

fn create_binary_and_event(database: &mut Database) {
    database.add(0, Some(EventClass::Class1), BinaryConfig::default());
    database.update(
        0,
        &Binary::new(true, Flags::ONLINE, Time::Synchronized(Timestamp::new(0))),
        UpdateOptions::default(),
    );
}

#[test]
fn empty_read_yields_empty_response() {
    let mut harness = new_harness(get_default_config());
    harness.test_request_response(EMPTY_READ, EMPTY_RESPONSE);
    harness.check_no_events();
}

#[test]
fn can_read_and_confirm_events() {
    let mut harness = new_harness(get_default_config());

    harness.handle.database.transaction(create_binary_and_event);

    harness.test_request_response(READ_CLASS_123, BINARY_EVENT_RESPONSE);
    harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);
    harness.send(CONFIRM_SEQ_0);
    harness.check_events(&[Event::SolicitedConfirmReceived(0)]);
}

#[test]
fn ignores_confirm_with_wrong_seq() {
    let mut harness = new_harness(get_default_config());

    harness.handle.database.transaction(create_binary_and_event);

    harness.test_request_response(READ_CLASS_123, BINARY_EVENT_RESPONSE);
    harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);
    harness.send(CONFIRM_SEQ_1);
    harness.check_events(&[Event::WrongSolicitedConfirmSeq(0, 1)]);
}

#[test]
fn ignores_unsolicited_confirm_with_correct_seq() {
    let mut harness = new_harness(get_default_config());

    harness.handle.database.transaction(create_binary_and_event);

    harness.test_request_response(READ_CLASS_123, BINARY_EVENT_RESPONSE);
    harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);
    harness.send(UNS_CONFIRM_SEQ_0);
    harness.check_events(&[Event::UnexpectedConfirm(true, 0)]);
}

#[test]
fn confirm_can_time_out() {
    let mut harness = new_harness(get_default_config());

    harness.handle.database.transaction(create_binary_and_event);

    harness.test_request_response(READ_CLASS_123, BINARY_EVENT_RESPONSE);
    harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);
    crate::tokio::time::advance(get_default_config().confirm_timeout + Duration::from_millis(1));
    harness.poll_pending();
    harness.check_events(&[Event::SolicitedConfirmTimeout(0)])
}

#[test]
fn sol_confirm_wait_goes_back_to_idle_with_new_request() {
    let mut harness = new_harness(get_default_config());

    harness.handle.database.transaction(create_binary_and_event);
    harness.test_request_response(READ_CLASS_123, BINARY_EVENT_RESPONSE);
    harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);
    // start a new request
    harness.test_request_response(EMPTY_READ, EMPTY_RESPONSE);
    harness.check_events(&[Event::SolicitedConfirmWaitNewRequest]);
}

#[test]
fn sol_confirm_wait_goes_back_to_idle_with_new_invalid_request() {
    let mut harness = new_harness(get_default_config());

    harness.handle.database.transaction(create_binary_and_event);
    harness.test_request_response(READ_CLASS_123, BINARY_EVENT_RESPONSE);
    harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);
    harness.test_request_response(
        &[0xC0, 0x70],             // Invalid function code
        &[0xC0, 0x81, 0x80, 0x01], // NO_FUNC_CODE_SUPPORT
    );
    harness.check_events(&[Event::SolicitedConfirmWaitNewRequest]);
}
