use crate::app::measurement::{BinaryInput, Flags, Time};
use crate::app::Timestamp;
use crate::outstation::database::{Add, BinaryInputConfig, EventClass, Update, UpdateOptions};

use super::harness::*;

const READ_CLASS_123: &[u8] = &[0xC0, 0x01, 60, 2, 0x06, 60, 3, 0x06, 60, 4, 0x06];
const READ_CLASS_1: &[u8] = &[0xC0, 0x01, 60, 2, 0x06];
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

const EMPTY_RESPONSE_OBJECT_UNKNOWN: &[u8] = &[0xC0, 0x81, 0x80, 0x02];
const EMPTY_RESPONSE_PARAM_ERROR: &[u8] = &[0xC0, 0x81, 0x80, 0x04];
const WRITE_ABSOLUTE_TIME: &[u8] = &[
    0xC0, 0x02, 50, 1, 0x07, 1, 0xC0, 0x24, 0x0E, 0xDA, 0x77, 0x01,
];

#[tokio::test]
async fn read_of_object_that_cannot_be_read_sets_object_unknown() {
    let mut harness = new_harness(get_default_config());

    harness
        .test_request_response(&[0xC0, 0x01, 13, 1, 0x06], EMPTY_RESPONSE_OBJECT_UNKNOWN)
        .await;
}

#[tokio::test]
async fn read_with_qualifier_not_supported_for_the_object_sets_parameter_error() {
    let mut harness = new_harness(get_default_config());

    harness
        .test_request_response(
            &[0xC0, 0x01, 2, 1, 0x17, 1, 0, 0x01],
            EMPTY_RESPONSE_PARAM_ERROR,
        )
        .await;
}

#[tokio::test]
async fn write_of_object_that_cannot_be_written_sets_object_unknown() {
    let mut harness = new_harness(get_default_config());

    harness
        .test_request_response(
            &[0xC0, 0x02, 1, 2, 0x00, 0, 0, 0x01],
            EMPTY_RESPONSE_OBJECT_UNKNOWN,
        )
        .await;
}

#[tokio::test]
async fn write_with_qualifier_not_supported_for_the_object_sets_parameter_error() {
    let mut harness = new_harness(get_default_config());

    harness
        .test_request_response(&[0xC0, 0x02, 0, 240, 0x06], EMPTY_RESPONSE_PARAM_ERROR)
        .await;
}

#[tokio::test]
async fn write_accepts_two_byte_qualifiers() {
    let mut harness = new_harness(get_default_config());

    // clear the restart IIN with a 2-byte start/stop
    harness
        .test_request_response(
            &[0xC0, 0x02, 80, 1, 0x01, 7, 0, 7, 0, 0x00],
            &[0xC0, 0x81, 0x00, 0x00],
        )
        .await;

    // write the time with a 2-byte count
    harness
        .test_request_response(
            &[
                0xC1, 0x02, 50, 1, 0x08, 1, 0, 0xC0, 0x24, 0x0E, 0xDA, 0x77, 0x01,
            ],
            &[0xC1, 0x81, 0x00, 0x00],
        )
        .await;

    harness.check_events(&[
        Event::ClearRestartIIN,
        Event::WriteAbsoluteTime(Timestamp::new(1614271096000)),
    ]);
}

#[tokio::test]
async fn write_keeps_the_error_of_an_earlier_header() {
    let mut harness = new_harness(get_default_config());

    // an unsupported g1v2 header followed by clearing the restart IIN
    harness
        .test_request_response(
            &[0xC0, 0x02, 1, 2, 0x00, 0, 0, 0x01, 80, 1, 0x00, 7, 7, 0x00],
            &[0xC0, 0x81, 0x00, 0x02],
        )
        .await;
}

#[tokio::test]
async fn requests_the_application_declines_set_object_unknown() {
    let mut harness = new_harness(get_default_config());
    harness.application_data.lock().unwrap().supported = false;

    harness
        .test_request_response(WRITE_ABSOLUTE_TIME, EMPTY_RESPONSE_OBJECT_UNKNOWN)
        .await;
    harness
        .test_request_response(
            &[0xC0, 0x02, 34, 1, 0x17, 1, 0, 0xCA, 0xFE],
            EMPTY_RESPONSE_OBJECT_UNKNOWN,
        )
        .await;
    harness
        .test_request_response(&[0xC0, 0x07, 20, 0, 0x06], EMPTY_RESPONSE_OBJECT_UNKNOWN)
        .await;

    harness.check_no_events();
}
