use crate::app::measurement::{BinaryInput, Flags, Time};
use crate::app::Timestamp;
use crate::outstation::database::{Add, BinaryInputConfig, EventClass, Update, UpdateOptions};
use crate::outstation::traits::ApplicationIin;

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

#[tokio::test]
async fn application_iin_forces_class_event_bits_with_empty_buffer() {
    let mut harness = new_harness(get_default_config());

    harness.application_data.lock().unwrap().application_iin = ApplicationIin {
        class_1_events: true,
        class_2_events: true,
        class_3_events: true,
        ..ApplicationIin::default()
    };

    // Empty event buffer; integrity poll for class 1/2/3.
    // Expected IIN1 = 0x80 (RESTART) | 0x02 (CLASS_1) | 0x04 (CLASS_2) | 0x08 (CLASS_3) = 0x8E.
    harness
        .test_request_response(READ_CLASS_123, &[0xC0, 0x81, 0x8E, 0x00])
        .await;
}

#[tokio::test]
async fn application_iin_class_event_fields_default_false_is_a_noop() {
    let mut harness = new_harness(get_default_config());

    // Sanity-check: leaving the new fields at their default `false` produces unchanged
    // behavior for an empty-buffer class 1/2/3 poll. Guards against accidental forcing.
    harness
        .test_request_response(READ_CLASS_123, EMPTY_RESPONSE)
        .await;
}

#[tokio::test]
async fn application_iin_class_1_events_maps_to_iin1_bit_1() {
    let mut harness = new_harness(get_default_config());
    harness.application_data.lock().unwrap().application_iin = ApplicationIin {
        class_1_events: true,
        ..ApplicationIin::default()
    };
    // IIN1 = 0x80 (RESTART) | 0x02 (CLASS_1) = 0x82.
    harness
        .test_request_response(READ_CLASS_123, &[0xC0, 0x81, 0x82, 0x00])
        .await;
}

#[tokio::test]
async fn application_iin_class_2_events_maps_to_iin1_bit_2() {
    let mut harness = new_harness(get_default_config());
    harness.application_data.lock().unwrap().application_iin = ApplicationIin {
        class_2_events: true,
        ..ApplicationIin::default()
    };
    // IIN1 = 0x80 (RESTART) | 0x04 (CLASS_2) = 0x84.
    harness
        .test_request_response(READ_CLASS_123, &[0xC0, 0x81, 0x84, 0x00])
        .await;
}

#[tokio::test]
async fn application_iin_class_3_events_maps_to_iin1_bit_3() {
    let mut harness = new_harness(get_default_config());
    harness.application_data.lock().unwrap().application_iin = ApplicationIin {
        class_3_events: true,
        ..ApplicationIin::default()
    };
    // IIN1 = 0x80 (RESTART) | 0x08 (CLASS_3) = 0x88.
    harness
        .test_request_response(READ_CLASS_123, &[0xC0, 0x81, 0x88, 0x00])
        .await;
}

#[tokio::test]
async fn application_iin_class_event_override_or_s_with_buffer_derived_bits() {
    let mut harness = new_harness(get_default_config());

    // Override asserts class 2 only.
    harness.application_data.lock().unwrap().application_iin = ApplicationIin {
        class_2_events: true,
        ..ApplicationIin::default()
    };

    // Push a class 1 event into the in-memory buffer; do NOT drain it via this read.
    harness.handle.database.transaction(|database| {
        database.add(0, Some(EventClass::Class1), BinaryInputConfig::default());
        database.update(
            0,
            &BinaryInput::new(true, Flags::ONLINE, Time::Synchronized(Timestamp::new(0))),
            UpdateOptions::default(),
        );
    });

    // Read class 0 (static data only) so the class 1 event remains unwritten and the
    // buffer-derived CLASS_1 bit is set when get_response_iin runs.
    // Expected IIN1 = 0x80 (RESTART) | 0x02 (CLASS_1 from buffer) | 0x04 (CLASS_2 from override) = 0x86.
    let read_class_0: &[u8] = &[0xC0, 0x01, 0x3C, 0x01, 0x06];
    let expected: &[u8] = &[
        0xC0, 0x81, 0x86, 0x00, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01,
    ];
    harness.test_request_response(read_class_0, expected).await;
}
