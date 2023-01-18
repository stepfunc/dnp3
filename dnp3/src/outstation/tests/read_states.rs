use crate::app::measurement::*;
use crate::app::*;
use crate::outstation::database::*;
use crate::outstation::tests::harness::*;

const EMPTY_READ: &[u8] = &[0xC0, 0x01];
const READ_CLASS_0: &[u8] = &[0xC0, 0x01, 60, 01, 0x06];
const READ_CLASS_123: &[u8] = &[0xC0, 0x01, 60, 02, 0x06, 60, 03, 0x06, 60, 04, 0x06];
const CONFIRM_SEQ_0: &[u8] = &[0xC0, 0x00];
const UNS_CONFIRM_SEQ_0: &[u8] = &[0b11010000, 0x00];
const CONFIRM_SEQ_1: &[u8] = &[0xC1, 0x00];
const EMPTY_RESPONSE: &[u8] = &[0xC0, 0x81, 0x80, 0x00];
const EMPTY_RESPONSE_WITH_PENDING_EVENTS: &[u8] = &[0xC0, 0x81, 0x82, 0x00];
const BINARY_EVENT_RESPONSE: &[u8] = &[
    0xE0, 0x81, 0x80, 0x00, 0x02, 0x01, 0x28, 0x01, 0x00, 0x00, 0x00, 0x81,
];

fn create_binary_and_event(database: &mut Database) {
    database.add(0, Some(EventClass::Class1), BinaryInputConfig::default());
    database.update(
        0,
        &BinaryInput::new(true, Flags::ONLINE, Time::Synchronized(Timestamp::new(0))),
        UpdateOptions::default(),
    );
}

fn create_one_analog_with_value_42(database: &mut Database) {
    database.add(0, Some(EventClass::Class1), AnalogInputConfig::default());
    database.update(
        0,
        &AnalogInput::new(42.0, Flags::ONLINE, Time::Synchronized(Timestamp::new(0))),
        UpdateOptions::no_event(),
    );
}

fn create_five_binary_inputs_with_odd_indices_true(database: &mut Database) {
    for i in 0..5 {
        database.add(i, Some(EventClass::Class1), BinaryInputConfig::default());
        database.update(
            i,
            &BinaryInput::new(
                i % 2 == 1,
                Flags::ONLINE,
                Time::Synchronized(Timestamp::new(0)),
            ),
            UpdateOptions::no_event(),
        );
    }
}

#[tokio::test]
async fn empty_read_yields_empty_response() {
    let mut harness = new_harness(get_default_config());
    harness
        .test_request_response(EMPTY_READ, EMPTY_RESPONSE)
        .await;
}

#[tokio::test]
async fn can_read_one_byte_range_for_specific_variation() {
    let mut harness = new_harness(get_default_config());

    harness
        .handle
        .database
        .transaction(create_one_analog_with_value_42);

    const READ_G30_V1_0_TO_0: &[u8] = &[0xC0, 0x01, 0x1E, 0x01, 0x00, 0x00, 0x00];
    harness
        .test_request_response(
            READ_G30_V1_0_TO_0,
            &[
                0xC0, 0x81, 0x80, 0x00, 0x1E, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x2A, 0x00,
                0x00, 0x00,
            ],
        )
        .await;
    harness.check_no_events();
}

#[tokio::test]
async fn can_read_one_byte_range_for_default_variation() {
    let mut harness = new_harness(get_default_config());

    harness
        .handle
        .database
        .transaction(create_one_analog_with_value_42);

    const READ_G30_V0_0_TO_0: &[u8] = &[0xC0, 0x01, 0x1E, 0x00, 0x00, 0x00, 0x00];
    harness
        .test_request_response(
            READ_G30_V0_0_TO_0,
            &[
                0xC0, 0x81, 0x80, 0x00, 0x1E, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x2A, 0x00,
                0x00, 0x00,
            ],
        )
        .await;
    harness.check_no_events();
}

#[tokio::test]
async fn can_read_two_byte_range_for_specific_variation() {
    let mut harness = new_harness(get_default_config());

    harness
        .handle
        .database
        .transaction(create_five_binary_inputs_with_odd_indices_true);

    const READ_G1_V2_1_TO_3: &[u8] = &[0xC0, 0x01, 0x01, 0x02, 0x01, 0x01, 0x00, 0x03, 0x00];
    harness
        .test_request_response(
            READ_G1_V2_1_TO_3,
            &[
                0xC0, 0x81, 0x80, 0x00, 0x01, 0x02, 0x01, 0x01, 0x00, 0x03, 0x00, 0x81, 0x01, 0x81,
            ],
        )
        .await;
    harness.check_no_events();
}

#[tokio::test]
async fn can_read_two_byte_range_for_default_variation() {
    let mut harness = new_harness(get_default_config());

    harness
        .handle
        .database
        .transaction(create_five_binary_inputs_with_odd_indices_true);

    const READ_G1_V0_1_TO_3: &[u8] = &[0xC0, 0x01, 0x01, 0x00, 0x01, 0x01, 0x00, 0x03, 0x00];
    harness
        .test_request_response(
            READ_G1_V0_1_TO_3,
            // the response here is g1v1, packed format, the bit pattern is 0b101 == 0x05
            &[
                0xC0, 0x81, 0x80, 0x00, 0x01, 0x01, 0x01, 0x01, 0x00, 0x03, 0x00, 0x05,
            ],
        )
        .await;
    harness.check_no_events();
}

#[tokio::test]
async fn can_read_and_confirm_events() {
    let mut harness = new_harness(get_default_config());

    harness.handle.database.transaction(create_binary_and_event);

    harness
        .test_request_response(READ_CLASS_123, BINARY_EVENT_RESPONSE)
        .await;

    harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);
    harness.send_and_process(CONFIRM_SEQ_0).await;
    harness.check_events(&[Event::SolicitedConfirmReceived(0)]);
}

#[tokio::test]
async fn ignores_confirm_with_wrong_seq() {
    let mut harness = new_harness(get_default_config());

    harness.handle.database.transaction(create_binary_and_event);

    harness
        .test_request_response(READ_CLASS_123, BINARY_EVENT_RESPONSE)
        .await;
    harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);

    harness.send_and_process(CONFIRM_SEQ_1).await;
    harness.check_events(&[Event::WrongSolicitedConfirmSeq(0, 1)]);
}

#[tokio::test]
async fn ignores_unsolicited_confirm_with_correct_seq() {
    let mut harness = new_harness(get_default_config());

    harness.handle.database.transaction(create_binary_and_event);

    harness
        .test_request_response(READ_CLASS_123, BINARY_EVENT_RESPONSE)
        .await;
    harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);
    harness.send_and_process(UNS_CONFIRM_SEQ_0).await;
    harness.check_events(&[Event::UnexpectedConfirm(true, 0)]);
}

#[tokio::test]
async fn confirm_can_time_out() {
    let mut harness = new_harness(get_default_config());

    harness.handle.database.transaction(create_binary_and_event);

    harness
        .test_request_response(READ_CLASS_123, BINARY_EVENT_RESPONSE)
        .await;
    harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);

    tokio::time::pause(); // auto advance timers

    harness
        .wait_for_events(&[Event::SolicitedConfirmTimeout(0)])
        .await;
}
