#![expect(
    clippy::arithmetic_side_effects,
    reason = "tests compute the expected number of values in a range"
)]

use crate::app::measurement::*;
use crate::app::BufferSize;
use crate::outstation::database::*;
use crate::outstation::tests::harness::*;

const CONFIRM_SEQ_0: &[u8] = &[0xC0, 0x00];

const READ_G1V0_ALL: &[u8] = &[0xC0, 0x01, 0x01, 0x00, 0x06];
const READ_G2V0_ALL: &[u8] = &[0xC0, 0x01, 0x02, 0x00, 0x06];
const READ_G2V0_ALL_SEQ_1: &[u8] = &[0xC1, 0x01, 0x02, 0x00, 0x06];

const NULL_RESPONSE: &[u8] = &[0xC0, 0x81, 0x80, 0x00];
const OBJECT_UNKNOWN_RESPONSE: &[u8] = &[0xC0, 0x81, 0x80, 0x02];
const PARAMETER_ERROR_RESPONSE: &[u8] = &[0xC0, 0x81, 0x80, 0x04];

const BINARY_EVENT_RESPONSE: &[u8] = &[
    0xE0, 0x81, 0x80, 0x00, 0x02, 0x01, 0x28, 0x01, 0x00, 0x00, 0x00, 0x81,
];

fn add_binary_inputs(database: &mut Database, indices: &[u16]) {
    for index in indices {
        database.add(
            *index,
            Some(EventClass::Class1),
            BinaryInputConfig::default(),
        );
        database.update(
            *index,
            &BinaryInput::new(true, Flags::ONLINE, Time::synchronized(0)),
            UpdateOptions::no_event(),
        );
    }
}

// a binary input event whose point is then removed
fn create_event_and_remove_point(database: &mut Database) {
    database.add(0, Some(EventClass::Class1), BinaryInputConfig::default());
    database.update(
        0,
        &BinaryInput::new(true, Flags::ONLINE, Time::synchronized(0)),
        UpdateOptions::default(),
    );
    assert!(Remove::<BinaryInput>::remove(database, 0));
}

#[tokio::test]
async fn read_of_static_type_without_points_sets_object_unknown() {
    let mut harness = new_harness(get_default_config());

    // all objects, 8-bit start/stop and 16-bit start/stop
    harness
        .test_request_response(READ_G1V0_ALL, OBJECT_UNKNOWN_RESPONSE)
        .await;
    harness
        .test_request_response(
            &[0xC0, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00],
            OBJECT_UNKNOWN_RESPONSE,
        )
        .await;
    harness
        .test_request_response(
            &[0xC0, 0x01, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00],
            OBJECT_UNKNOWN_RESPONSE,
        )
        .await;
}

#[tokio::test]
async fn read_of_all_objects_with_sparse_indices_has_no_error() {
    let mut harness = new_harness(get_default_config());

    harness
        .handle
        .database
        .transaction(|db| add_binary_inputs(db, &[0, 2]));

    harness
        .test_request_response(
            &[0xC0, 0x01, 0x01, 0x02, 0x06],
            &[
                0xC0, 0x81, 0x80, 0x00, // g1v2 - s/s == 0
                0x01, 0x02, 0x01, 0x00, 0x00, 0x00, 0x00, 0x81, // g1v2 - s/s == 2
                0x01, 0x02, 0x01, 0x02, 0x00, 0x02, 0x00, 0x81,
            ],
        )
        .await;
}

#[tokio::test]
async fn range_read_past_the_last_point_returns_it_with_parameter_error() {
    let mut harness = new_harness(get_default_config());

    harness
        .handle
        .database
        .transaction(|db| add_binary_inputs(db, &[0, 1, 2, 3, 4]));

    // g1v2, 8-bit start/stop, 4 to 5
    harness
        .test_request_response(
            &[0xC0, 0x01, 0x01, 0x02, 0x00, 0x04, 0x05],
            &[
                0xC0, 0x81, 0x80, 0x04, // g1v2 - s/s == 4
                0x01, 0x02, 0x01, 0x04, 0x00, 0x04, 0x00, 0x81,
            ],
        )
        .await;
}

#[tokio::test]
async fn range_read_with_a_missing_index_returns_the_others_with_parameter_error() {
    let mut harness = new_harness(get_default_config());

    harness
        .handle
        .database
        .transaction(|db| add_binary_inputs(db, &[0, 2]));

    // g1v2, 8-bit start/stop, 0 to 2
    harness
        .test_request_response(
            &[0xC0, 0x01, 0x01, 0x02, 0x00, 0x00, 0x02],
            &[
                0xC0, 0x81, 0x80, 0x04, // g1v2 - s/s == 0
                0x01, 0x02, 0x01, 0x00, 0x00, 0x00, 0x00, 0x81, // g1v2 - s/s == 2
                0x01, 0x02, 0x01, 0x02, 0x00, 0x02, 0x00, 0x81,
            ],
        )
        .await;
}

#[tokio::test]
async fn range_read_with_no_installed_index_sets_parameter_error() {
    let mut harness = new_harness(get_default_config());

    harness
        .handle
        .database
        .transaction(|db| add_binary_inputs(db, &[0, 1, 2, 3, 4]));

    // g1v0, 8-bit start/stop, 10 to 12
    harness
        .test_request_response(
            &[0xC0, 0x01, 0x01, 0x00, 0x00, 0x0A, 0x0C],
            PARAMETER_ERROR_RESPONSE,
        )
        .await;
}

#[tokio::test]
async fn errors_from_several_headers_are_combined_with_the_data() {
    let mut harness = new_harness(get_default_config());

    harness.handle.database.transaction(|db| {
        db.add(0, Some(EventClass::Class1), AnalogInputConfig::default());
        db.update(
            0,
            &AnalogInput::new(42.0, Flags::ONLINE, Time::synchronized(0)),
            UpdateOptions::no_event(),
        );
    });

    // g30v1 all objects, g1v0 all objects (no binary inputs), g30v1 5 to 5 (not installed)
    harness
        .test_request_response(
            &[
                0xC0, 0x01, 0x1E, 0x01, 0x06, 0x01, 0x00, 0x06, 0x1E, 0x01, 0x00, 0x05, 0x05,
            ],
            &[
                0xC0, 0x81, 0x80, 0x06, // g30v1 - s/s == 0, value == 42
                0x1E, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x2A, 0x00, 0x00, 0x00,
            ],
        )
        .await;
}

#[tokio::test]
async fn read_of_frozen_analogs_sets_object_unknown() {
    let mut harness = new_harness(get_default_config());

    // g31v0 all objects
    harness
        .test_request_response(&[0xC0, 0x01, 0x1F, 0x00, 0x06], OBJECT_UNKNOWN_RESPONSE)
        .await;
    // g33v0 all objects
    harness
        .test_request_response(&[0xC0, 0x01, 0x21, 0x00, 0x06], OBJECT_UNKNOWN_RESPONSE)
        .await;
}

#[tokio::test]
async fn parameter_error_is_only_in_the_first_fragment_of_a_partial_range_read() {
    let mut config = get_default_config();
    config.solicited_buffer_size = BufferSize::min();
    let mut harness = new_harness(config);

    // 300 points, more g1v2 values than fit in one fragment
    harness.handle.database.transaction(|db| {
        for index in 0..300 {
            db.add(
                index,
                Some(EventClass::Class1),
                BinaryInputConfig::default(),
            );
        }
    });

    // g1v2, 16-bit start/stop, 0 to 300
    harness
        .send_and_process(&[0xC0, 0x01, 0x01, 0x02, 0x01, 0x00, 0x00, 0x2C, 0x01])
        .await;

    let first = harness.expect_write().await;
    assert_eq!(&first[0..4], &[0xA0, 0x81, 0x80, 0x04]); // FIR, CON, IIN2.2
    harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);

    harness.send_and_process(CONFIRM_SEQ_0).await;
    harness.check_events(&[Event::SolicitedConfirmReceived(0)]);

    let last = harness.expect_write().await;
    assert_eq!(&last[0..4], &[0x41, 0x81, 0x80, 0x00]); // FIN, no error

    // together, the fragments report exactly the installed points 0 to 299
    let (first_start, first_stop) = g1v2_restart_values(&first[4..]);
    let (last_start, last_stop) = g1v2_restart_values(&last[4..]);
    assert_eq!(first_start, 0);
    assert_eq!(last_start, first_stop + 1);
    assert_eq!(last_stop, 299);
}

// the start and stop of the only g1v2 header in the objects, whose values all have just RESTART set
fn g1v2_restart_values(objects: &[u8]) -> (u16, u16) {
    assert_eq!(&objects[0..3], &[0x01, 0x02, 0x01]);
    let start = u16::from_le_bytes([objects[3], objects[4]]);
    let stop = u16::from_le_bytes([objects[5], objects[6]]);
    let values = &objects[7..];
    assert_eq!(values.len(), usize::from(stop - start) + 1);
    assert!(values.iter().all(|x| *x == 0x02));
    (start, stop)
}

#[tokio::test]
async fn read_of_event_type_without_points_sets_object_unknown() {
    let mut harness = new_harness(get_default_config());

    harness
        .test_request_response(READ_G2V0_ALL, OBJECT_UNKNOWN_RESPONSE)
        .await;
}

#[tokio::test]
async fn read_of_other_event_types_without_points_sets_object_unknown() {
    let mut harness = new_harness(get_default_config());

    // g4v0, g11v0, g22v0, g23v0, g32v0, g42v0, g111v0
    for group in [4, 11, 22, 23, 32, 42] {
        harness
            .test_request_response(&[0xC0, 0x01, group, 0x00, 0x06], OBJECT_UNKNOWN_RESPONSE)
            .await;
    }
    harness
        .test_request_response(&[0xC0, 0x01, 111, 0x00, 0x06], OBJECT_UNKNOWN_RESPONSE)
        .await;
}

#[tokio::test]
async fn class_reads_without_points_have_no_error() {
    let mut harness = new_harness(get_default_config());

    // g60v1, g60v2, g60v3, g60v4 all objects
    harness
        .test_request_response(
            &[
                0xC0, 0x01, 0x3C, 0x01, 0x06, 0x3C, 0x02, 0x06, 0x3C, 0x03, 0x06, 0x3C, 0x04, 0x06,
            ],
            NULL_RESPONSE,
        )
        .await;
}

#[tokio::test]
async fn read_of_event_type_with_points_but_no_event_class_has_no_error() {
    let mut harness = new_harness(get_default_config());

    harness
        .handle
        .database
        .transaction(|db| db.add(0, None, BinaryInputConfig::default()));

    harness
        .test_request_response(READ_G2V0_ALL, NULL_RESPONSE)
        .await;
}

#[tokio::test]
async fn events_of_a_removed_point_are_reported_until_confirmed() {
    let mut harness = new_harness(get_default_config());

    harness
        .handle
        .database
        .transaction(create_event_and_remove_point);

    harness
        .test_request_response(READ_G2V0_ALL, BINARY_EVENT_RESPONSE)
        .await;
    harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);
    harness.send_and_process(CONFIRM_SEQ_0).await;
    harness.check_events(&[Event::SolicitedConfirmReceived(0)]);

    // no points and no events left
    harness
        .test_request_response(READ_G2V0_ALL_SEQ_1, &[0xC1, 0x81, 0x80, 0x02])
        .await;
}

#[tokio::test]
async fn event_type_read_after_a_class_read_of_the_same_events_has_no_error() {
    let mut harness = new_harness(get_default_config());

    harness
        .handle
        .database
        .transaction(create_event_and_remove_point);

    // g60v2 all objects, then g2v0 all objects
    harness
        .test_request_response(
            &[0xC0, 0x01, 0x3C, 0x02, 0x06, 0x02, 0x00, 0x06],
            BINARY_EVENT_RESPONSE,
        )
        .await;
}

#[tokio::test]
async fn repeated_event_type_read_has_no_error() {
    let mut harness = new_harness(get_default_config());

    harness
        .handle
        .database
        .transaction(create_event_and_remove_point);

    harness
        .test_request_response(
            &[0xC0, 0x01, 0x02, 0x00, 0x06, 0x02, 0x00, 0x06],
            BINARY_EVENT_RESPONSE,
        )
        .await;
}

#[tokio::test]
async fn event_type_read_of_zero_events_has_no_error() {
    let mut harness = new_harness(get_default_config());

    harness
        .handle
        .database
        .transaction(create_event_and_remove_point);

    // g2v0, 8-bit count of 0: selects nothing, the class 1 event is still pending
    harness
        .test_request_response(
            &[0xC0, 0x01, 0x02, 0x00, 0x07, 0x00],
            &[0xC0, 0x81, 0x82, 0x00],
        )
        .await;
}
