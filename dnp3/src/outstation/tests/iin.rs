use crate::app::measurement::{BinaryInput, Flags, Time};
use crate::app::Timestamp;
use crate::outstation::database::{Add, BinaryInputConfig, EventClass, Update, UpdateOptions};

use super::harness::*;

const READ_CLASS_123: &[u8] = &[0xC0, 0x01, 60, 02, 0x06, 60, 03, 0x06, 60, 04, 0x06];
const READ_CLASS_1: &[u8] = &[0xC0, 0x01, 60, 02, 0x06];
const RESPONSE_WITH_OVERFLOW: &[u8] = &[
    0xE0, 0x81, 0x80, 0x08, 0x02, 0x01, 0x28, 0x05, 0x00, 0x00, 0x00, 0x81, 0x00, 0x00, 0x01, 0x00,
    0x00, 0x81, 0x00, 0x00, 0x01, 0x00, 0x00, 0x81,
];
const CONFIRM_SEQ_0: &[u8] = &[0xC0, 0x00];
const EMPTY_RESPONSE: &[u8] = &[0xC0, 0x81, 0x80, 0x00];

#[tokio::test]
async fn incomplete_request() {
    let mut harness = new_harness(get_default_config());

    harness
        .send_and_process(
            &[0xC0], // Incomplete request
        )
        .await;

    harness.check_no_events();
}

#[tokio::test]
async fn function_code_does_not_exist() {
    let mut harness = new_harness(get_default_config());

    harness
        .test_request_response(
            &[0xC0, 0x70],             // Invalid function code 0x70
            &[0xC0, 0x81, 0x80, 0x01], // IIN2.0 NO_FUNC_CODE_SUPPORT set
        )
        .await;
}

#[tokio::test]
async fn function_code_not_supported() {
    let mut harness = new_harness(get_default_config());

    harness
        .test_request_response(
            &[0xC0, 0x13],             // Function code SAVE_CONFIG (0x13) is not supported
            &[0xC0, 0x81, 0x80, 0x01], // IIN2.0 NO_FUNC_CODE_SUPPORT set
        )
        .await;
}

#[tokio::test]
async fn object_unknown() {
    let mut harness = new_harness(get_default_config());

    harness
        .test_request_response(
            &[0xC0, 0x01, 200, 0x00],  // Read g200v0
            &[0xC0, 0x81, 0x80, 0x02], // IIN2.1 OBJECT_UNKNOWN set
        )
        .await;
}

#[tokio::test]
async fn buffer_overflow() {
    let mut harness = new_harness(get_default_config());

    // Generate a buffer overflow
    harness.handle.database.transaction(|database| {
        database.add(0, Some(EventClass::Class1), BinaryInputConfig::default());

        for i in 0..6 {
            database.update(
                0,
                &BinaryInput::new(
                    i % 2 != 0,
                    Flags::ONLINE,
                    Time::Synchronized(Timestamp::new(0)),
                ),
                UpdateOptions::default(),
            );
        }
    });
    harness
        .test_request_response(READ_CLASS_123, RESPONSE_WITH_OVERFLOW)
        .await;

    // Do NOT send confirm, should still set the overflow bit
    harness
        .test_request_response(READ_CLASS_1, RESPONSE_WITH_OVERFLOW)
        .await;

    // Send confirmation, check that overflow bit is NOT set
    harness.send_and_process(CONFIRM_SEQ_0).await;
    harness
        .test_request_response(READ_CLASS_123, EMPTY_RESPONSE)
        .await;
}
