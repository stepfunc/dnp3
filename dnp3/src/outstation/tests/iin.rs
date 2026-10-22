use crate::app::measurement::{BinaryInput, Flags, Time};
use crate::app::Timestamp;
use crate::outstation::database::{Add, BinaryInputConfig, EventClass, Update, UpdateOptions};
use crate::outstation::{BufferState, ClassCount, TypeCount};

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
async fn object_headers_in_a_request_that_takes_none_set_parameter_error() {
    let mut harness = new_harness(get_default_config());

    // COLD_RESTART and SAVE_CONFIG take no objects
    harness
        .test_request_response(&[0xC0, 0x0D, 60, 1, 0x06], &[0xC0, 0x81, 0x80, 0x05])
        .await;
    harness.check_events(&[Event::ColdRestart(None)]);
    harness
        .test_request_response(&[0xC1, 0x13, 60, 1, 0x06], &[0xC1, 0x81, 0x80, 0x05])
        .await;

    // ASSIGN_CLASS takes objects
    harness
        .test_request_response(&[0xC2, 0x16, 60, 1, 0x06], &[0xC2, 0x81, 0x80, 0x01])
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

fn update_binary(database: &mut crate::outstation::database::Database, index: u16, value: bool) {
    database.update(
        index,
        &BinaryInput::new(value, Flags::ONLINE, Time::Synchronized(Timestamp::new(0))),
        UpdateOptions::default(),
    );
}

fn binary_states(binary: usize, classes: [usize; 3]) -> BufferState {
    BufferState {
        classes: ClassCount {
            num_class_1: classes[0],
            num_class_2: classes[1],
            num_class_3: classes[2],
        },
        types: TypeCount {
            num_binary_input: binary,
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

#[tokio::test]
async fn class_bits_and_buffer_state_follow_the_events_across_reads_and_confirms() {
    let mut harness = new_harness(get_default_config());

    // one event in each class, from binary inputs 0, 1 and 2
    harness.handle.database.transaction(|database| {
        database.add(0, Some(EventClass::Class1), BinaryInputConfig::default());
        database.add(1, Some(EventClass::Class2), BinaryInputConfig::default());
        database.add(2, Some(EventClass::Class3), BinaryInputConfig::default());
        for index in 0..3 {
            update_binary(database, index, true);
        }
    });

    // IIN1.1 is clear once the class 1 event is written, while IIN1.2 and IIN1.3 remain
    harness
        .test_request_response(
            READ_CLASS_1,
            &[
                0xE0, 0x81, 0x8C, 0x00, 0x02, 0x01, 0x28, 0x01, 0x00, 0x00, 0x00, 0x81,
            ],
        )
        .await;
    harness.send_and_process(CONFIRM_SEQ_0).await;
    harness.check_events(&[
        Event::EnterSolicitedConfirmWait(0),
        Event::SolicitedConfirmReceived(0),
        Event::BeginConfirm,
        Event::Cleared(0),
        Event::EndConfirm(binary_states(2, [0, 1, 1])),
    ]);

    harness
        .test_request_response(
            &[0xC1, 0x01, 60, 3, 0x06],
            &[
                0xE1, 0x81, 0x88, 0x00, 0x02, 0x01, 0x28, 0x01, 0x00, 0x01, 0x00, 0x81,
            ],
        )
        .await;
    harness.send_and_process(&[0xC1, 0x00]).await;
    harness.check_events(&[
        Event::EnterSolicitedConfirmWait(1),
        Event::SolicitedConfirmReceived(1),
        Event::BeginConfirm,
        Event::Cleared(1),
        Event::EndConfirm(binary_states(1, [0, 0, 1])),
    ]);

    // a new class 1 event, then a class 3 read that is never confirmed
    harness
        .handle
        .database
        .transaction(|database| update_binary(database, 0, false));
    harness
        .test_request_response(
            &[0xC2, 0x01, 60, 4, 0x06],
            &[
                0xE2, 0x81, 0x82, 0x00, 0x02, 0x01, 0x28, 0x01, 0x00, 0x02, 0x00, 0x81,
            ],
        )
        .await;
    harness.check_events(&[Event::EnterSolicitedConfirmWait(2)]);

    // the next request ends the confirm wait, and the class 3 event is pending again
    harness
        .test_request_response(&[0xC3, 0x01], &[0xC3, 0x81, 0x8A, 0x00])
        .await;
    harness.check_events(&[Event::SolicitedConfirmWaitNewRequest]);
    harness.check_no_events();

    // both events are reported in the order they were created
    harness
        .test_request_response(
            &[0xC4, 0x01, 60, 2, 0x06, 60, 3, 0x06, 60, 4, 0x06],
            &[
                0xE4, 0x81, 0x80, 0x00, 0x02, 0x01, 0x28, 0x02, 0x00, 0x02, 0x00, 0x81, 0x00, 0x00,
                0x01,
            ],
        )
        .await;
    harness.send_and_process(&[0xC4, 0x00]).await;
    harness.check_events(&[
        Event::EnterSolicitedConfirmWait(4),
        Event::SolicitedConfirmReceived(4),
        Event::BeginConfirm,
        Event::Cleared(2),
        Event::Cleared(3),
        Event::EndConfirm(binary_states(0, [0, 0, 0])),
    ]);
    harness.check_no_events();

    harness
        .test_request_response(&[0xC5, 0x01], &[0xC5, 0x81, 0x80, 0x00])
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
async fn write_of_counted_event_objects_is_malformed() {
    let mut harness = new_harness(get_default_config());

    // g32v2 with a count of 2 and its 6 bytes of object data, which also read as a g80v1 header
    // that clears the restart IIN
    harness
        .test_request_response(
            &[0xC0, 0x02, 32, 2, 0x07, 2, 80, 1, 0x00, 7, 7, 0x00],
            EMPTY_RESPONSE_PARAM_ERROR,
        )
        .await;
    harness.check_no_events();
}

#[tokio::test]
async fn write_of_counted_event_objects_without_data_is_malformed() {
    let mut harness = new_harness(get_default_config());

    // g32v1 with a 1-byte and a 2-byte count of 1, and no object data
    harness
        .test_request_response(&[0xC0, 0x02, 32, 1, 0x07, 1], EMPTY_RESPONSE_PARAM_ERROR)
        .await;
    harness
        .test_request_response(&[0xC1, 0x02, 32, 1, 0x08, 1, 0], &[0xC1, 0x81, 0x80, 0x04])
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

#[tokio::test]
async fn fragments_that_are_not_valid_requests_get_no_response() {
    let mut harness = new_harness(get_default_config());

    for request in [
        &[0x40, 0x01, 60, 1, 0x06][..], // FIR = 0
        &[0x80, 0x01, 60, 1, 0x06][..], // FIN = 0
        &[0xD0, 0x01, 60, 1, 0x06][..], // UNS = 1 on a READ
        &[0xC0, 0x81, 0x00, 0x00][..],  // a response function code
    ] {
        harness.send_and_process(request).await;
        harness.expect_no_response();
    }

    // a valid request is still answered
    harness
        .test_request_response(&[0xC0, 0x01, 60, 1, 0x06], EMPTY_RESPONSE)
        .await;
}

#[tokio::test]
async fn fragment_that_is_not_a_valid_request_does_not_end_a_confirm_wait() {
    let mut harness = new_harness(get_default_config());
    harness.handle.database.transaction(|database| {
        database.add(0, Some(EventClass::Class1), BinaryInputConfig::default());
        database.update(
            0,
            &BinaryInput::new(true, Flags::ONLINE, Time::Synchronized(Timestamp::new(0))),
            UpdateOptions::default(),
        );
    });

    harness.send_and_process(READ_CLASS_1).await;
    harness.expect_write().await;
    harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);

    // FIR = 0
    harness.send_and_process(&[0x40, 0x01, 60, 2, 0x06]).await;
    harness.expect_no_response();
    harness.check_no_events();

    harness.send_and_process(CONFIRM_SEQ_0).await;
    harness.check_events(&[Event::SolicitedConfirmReceived(0)]);
}

#[tokio::test]
async fn read_with_two_byte_count_and_prefix_sets_parameter_error() {
    let mut harness = new_harness(get_default_config());

    harness
        .test_request_response(
            &[0xC0, 0x01, 2, 1, 0x28, 1, 0, 0, 0, 0x01],
            EMPTY_RESPONSE_PARAM_ERROR,
        )
        .await;
}

#[tokio::test]
async fn read_with_free_format_qualifier_sets_object_unknown() {
    let mut harness = new_harness(get_default_config());

    // g70v5, free-format, 8 bytes: file handle and block number with no data
    harness
        .test_request_response(
            &[0xC0, 0x01, 70, 5, 0x5B, 1, 8, 0, 1, 2, 3, 4, 5, 6, 7, 8],
            EMPTY_RESPONSE_OBJECT_UNKNOWN,
        )
        .await;
}

#[tokio::test]
async fn write_of_g34_with_a_two_byte_range_sets_parameter_error() {
    let mut harness = new_harness(get_default_config());

    // g34v1 is writable, but not with a 2-byte start/stop
    harness
        .test_request_response(
            &[
                0xC0, 0x02, 0x22, 0x01, 0x01, 0x03, 0x00, 0x03, 0x00, 0xCA, 0xFE,
            ],
            EMPTY_RESPONSE_PARAM_ERROR,
        )
        .await;
}
