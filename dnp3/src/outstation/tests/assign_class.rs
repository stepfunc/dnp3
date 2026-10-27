// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! ASSIGN_CLASS through a session: the null response and its IIN, the callbacks that report an
//! accepted request, and the class of the events points generate afterwards, which the class IIN
//! bits of later responses show (buffered events report their class there until written)

use crate::app::measurement::*;
use crate::outstation::database::*;
use crate::outstation::tests::harness::*;
use crate::outstation::{PointIndices, PointType};
use crate::util::test_support::watchdog;

/// IIN1 of a response with the restart bit and these class bits
const RESTART: u8 = 0x80;
const CLASS_1: u8 = 0x02;
const CLASS_2: u8 = 0x04;

/// g60v2 (class 1)
const CLASS_1_HEADER: &[u8] = &[60, 2, 6];
/// g60v3 (class 2)
const CLASS_2_HEADER: &[u8] = &[60, 3, 6];
/// g60v1: no events
const NO_EVENTS_HEADER: &[u8] = &[60, 1, 6];
/// g1v0, all objects
const ALL_BINARY: &[u8] = &[1, 0, 6];

/// g1v0, 8-bit start/stop
fn binary_range(start: u8, stop: u8) -> Vec<u8> {
    vec![1, 0, 0, start, stop]
}

/// an ASSIGN_CLASS request with sequence number `seq` and these object headers
fn assign_class(seq: u8, headers: &[&[u8]]) -> Vec<u8> {
    [&[0xC0 | seq, 0x16][..], &headers.concat()].concat()
}

/// a READ of class 3, which has no events here, to see the class IIN bits
fn read_class_3(seq: u8) -> Vec<u8> {
    vec![0xC0 | seq, 0x01, 60, 4, 6]
}

fn response(seq: u8, iin1: u8, iin2: u8) -> [u8; 4] {
    [0xC0 | seq, 0x81, iin1, iin2]
}

/// `count` binary inputs in class 1
fn binary_inputs(count: u16) -> impl FnMut(&mut Database) {
    move |db| {
        for index in 0..count {
            db.add(
                index,
                Some(EventClass::Class1),
                BinaryInputConfig::default(),
            );
        }
    }
}

/// change binary input `index` to `value`, which generates an event in its class
fn change(index: u16, value: bool) -> impl FnMut(&mut Database) {
    move |db| {
        let value = BinaryInput::new(value, Flags::ONLINE, Time::synchronized(0));
        db.update(index, &value, UpdateOptions::detect_event());
    }
}

#[tokio::test]
async fn an_accepted_request_is_reported_in_order() {
    watchdog(async {
        let mut harness = new_harness(get_default_config());
        harness.handle.database.transaction(binary_inputs(4));
        let request = assign_class(
            0,
            &[
                CLASS_2_HEADER,
                &binary_range(1, 2),
                NO_EVENTS_HEADER,
                ALL_BINARY,
            ],
        );
        harness
            .test_request_response(&request, &response(0, RESTART, 0))
            .await;
        harness.check_events(&[
            Event::BeginAssignClass,
            Event::AssignClass(
                PointType::BinaryInput,
                PointIndices::Range(1, 2),
                Some(EventClass::Class2),
            ),
            Event::AssignClass(PointType::BinaryInput, PointIndices::All, None),
            Event::EndAssignClass,
        ]);
    })
    .await;
}

#[tokio::test]
async fn buffered_events_keep_their_class_and_later_ones_take_the_new_one() {
    watchdog(async {
        let mut harness = new_harness(get_default_config());
        harness.handle.database.transaction(binary_inputs(1));
        harness.handle.database.transaction(change(0, true));

        let request = assign_class(0, &[CLASS_2_HEADER, ALL_BINARY]);
        harness
            .test_request_response(&request, &response(0, RESTART | CLASS_1, 0))
            .await;
        harness.drain_events();

        harness.handle.database.transaction(change(0, false));
        harness
            .test_request_response(
                &read_class_3(1),
                &response(1, RESTART | CLASS_1 | CLASS_2, 0),
            )
            .await;
    })
    .await;
}

#[tokio::test]
async fn points_assigned_to_no_class_generate_no_events() {
    watchdog(async {
        let mut harness = new_harness(get_default_config());
        harness.handle.database.transaction(binary_inputs(1));
        let request = assign_class(0, &[NO_EVENTS_HEADER, ALL_BINARY]);
        harness
            .test_request_response(&request, &response(0, RESTART, 0))
            .await;

        harness.handle.database.transaction(change(0, true));
        harness
            .test_request_response(&read_class_3(1), &response(1, RESTART, 0))
            .await;
    })
    .await;
}

#[tokio::test]
async fn a_rejected_request_changes_nothing_and_is_not_reported() {
    watchdog(async {
        let mut harness = new_harness(get_default_config());
        harness.handle.database.transaction(binary_inputs(4));
        // the first header is valid, but the range of the second isn't fully installed
        let request = assign_class(
            0,
            &[CLASS_2_HEADER, &binary_range(0, 1), &binary_range(2, 9)],
        );
        harness
            .test_request_response(&request, &response(0, RESTART, 0x04))
            .await;
        harness.check_no_events();

        // binary input 0 is still in class 1
        harness.handle.database.transaction(change(0, true));
        harness
            .test_request_response(&read_class_3(1), &response(1, RESTART | CLASS_1, 0))
            .await;
    })
    .await;
}

#[tokio::test]
async fn request_errors_are_reported_with_iin2() {
    watchdog(async {
        let mut harness = new_harness(get_default_config());
        harness.handle.database.transaction(binary_inputs(1));
        let cases: [(&[&[u8]], u8); 4] = [
            // no object headers
            (&[], 0x04),
            // points before any class header: no class, which isn't supported
            (&[ALL_BINARY, CLASS_1_HEADER], 0x04),
            // a type without points
            (&[CLASS_1_HEADER, &[20, 0, 6]], 0x02),
            // an event group
            (&[CLASS_1_HEADER, &[2, 0, 6]], 0x02),
        ];
        for (seq, (headers, iin2)) in (0u8..).zip(cases) {
            harness
                .test_request_response(&assign_class(seq, headers), &response(seq, RESTART, iin2))
                .await;
        }
        harness.check_no_events();
    })
    .await;
}

#[tokio::test]
async fn a_request_that_fails_to_parse_after_valid_headers_changes_nothing() {
    watchdog(async {
        let mut harness = new_harness(get_default_config());
        harness.handle.database.transaction(binary_inputs(1));
        // group 200 doesn't exist
        let request = assign_class(0, &[CLASS_2_HEADER, ALL_BINARY, &[200, 0, 6]]);
        harness
            .test_request_response(&request, &response(0, RESTART, 0x02))
            .await;
        harness.check_no_events();

        harness.handle.database.transaction(change(0, true));
        harness
            .test_request_response(&read_class_3(1), &response(1, RESTART | CLASS_1, 0))
            .await;
    })
    .await;
}

#[tokio::test]
async fn an_application_without_support_answers_no_function_code_support() {
    watchdog(async {
        let mut harness = new_harness(get_default_config());
        harness.handle.database.transaction(binary_inputs(1));
        harness
            .application_data
            .lock()
            .unwrap()
            .support_assign_class = false;
        let request = assign_class(0, &[CLASS_2_HEADER, ALL_BINARY]);
        harness
            .test_request_response(&request, &response(0, RESTART, 0x01))
            .await;
        harness.check_no_events();

        // binary input 0 is still in class 1
        harness.handle.database.transaction(change(0, true));
        harness
            .test_request_response(&read_class_3(1), &response(1, RESTART | CLASS_1, 0))
            .await;
    })
    .await;
}

#[tokio::test]
async fn a_repeated_request_is_answered_again_without_being_applied_again() {
    watchdog(async {
        let mut harness = new_harness(get_default_config());
        harness.handle.database.transaction(binary_inputs(1));
        let request = assign_class(0, &[CLASS_2_HEADER, ALL_BINARY]);
        harness
            .test_request_response(&request, &response(0, RESTART, 0))
            .await;
        harness.drain_events();
        harness
            .test_request_response(&request, &response(0, RESTART, 0))
            .await;
        harness.check_no_events();
    })
    .await;
}

#[tokio::test]
async fn assignments_are_charged_against_the_work_budget() {
    watchdog(async {
        // 1000 points allow 8000 visits, and each all objects header costs 1001: 7 fit, 8 don't
        let mut harness = new_harness(get_default_config());
        harness.handle.database.transaction(binary_inputs(1000));

        let seven = assign_class(0, &[CLASS_2_HEADER, &ALL_BINARY.repeat(7)]);
        harness
            .test_request_response(&seven, &response(0, RESTART, 0))
            .await;
        assert_eq!(harness.drain_events().len(), 9);

        let eight = assign_class(1, &[CLASS_1_HEADER, &ALL_BINARY.repeat(8)]);
        harness
            .test_request_response(&eight, &response(1, RESTART, 0x04))
            .await;
        harness.check_no_events();
    })
    .await;
}

#[tokio::test]
async fn an_assignment_during_an_unsolicited_confirm_wait_leaves_that_response_alone() {
    watchdog(async {
        let mut harness = new_harness(get_default_unsolicited_config());
        confirm_null_unsolicited(&mut harness).await;
        enable_unsolicited(&mut harness).await;
        harness.handle.database.transaction(add_binary_event);
        // binary input 0's class 1 event, waiting for its confirm
        harness
            .expect_response(&[
                0xF1, 0x82, 0x80, 0x00, 0x02, 0x01, 0x28, 0x01, 0x00, 0x00, 0x00, 0x81,
            ])
            .await;

        // answered right away, and the unsolicited response is still confirmed by its sequence
        let request = assign_class(1, &[CLASS_2_HEADER, ALL_BINARY]);
        harness
            .test_request_response(&request, &response(1, RESTART, 0))
            .await;
        harness.send_and_process(&unsolicited_confirm(1)).await;
        harness.drain_events();

        // the next event, now in class 2, is reported unsolicited as the next sequence
        harness.handle.database.transaction(change(0, false));
        harness
            .expect_response(&[
                0xF2, 0x82, 0x80, 0x00, 0x02, 0x01, 0x28, 0x01, 0x00, 0x00, 0x00, 0x01,
            ])
            .await;
    })
    .await;
}
