// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! How a request is answered in each state it can arrive in: idle, the solicited confirm wait and
//! the unsolicited confirm wait (IEEE 1815-2012 6.1, Table 6-1)

use std::time::Duration;

use tokio::time::Instant;

use crate::app::measurement::{BinaryInput, Flags, Time};
use crate::app::FunctionCode;
use crate::link::header::BroadcastConfirmMode;
use crate::outstation::config::OutstationConfig;
use crate::outstation::database::{Add, BinaryInputConfig, EventClass, Update, UpdateOptions};
use crate::outstation::traits::{BroadcastAction, OperateType};
use crate::outstation::BufferState;
use crate::transport::mock::writer::LINK_STATUS_REQUEST;
use crate::util::test_support::watchdog;

use super::harness::*;

const RESTART: u8 = 0x80;
const BROADCAST: u8 = 0x01;
const CLASS_1: u8 = 0x02;

const PARAMETER_ERROR: u8 = 0x04;
const NO_FUNC_CODE_SUPPORT: u8 = 0x01;

const MODES: [BroadcastConfirmMode; 3] = [
    BroadcastConfirmMode::NotRequired,
    BroadcastConfirmMode::Optional,
    BroadcastConfirmMode::Mandatory,
];

const fn unknown_function(seq: u8) -> [u8; 2] {
    [0xC0 | seq, 0x70]
}

/// a READ of g60v1 without its qualifier, answered with IIN2.2
const fn malformed(seq: u8) -> [u8; 3] {
    [0xC0 | seq, 0x01, 60]
}

/// a response header without objects
const fn response(con: bool, seq: u8, iin1: u8, iin2: u8) -> [u8; 4] {
    let con = if con { 0x20 } else { 0x00 };
    [0xC0 | con | seq, 0x81, iin1, iin2]
}

/// the echo of a g41v2 request at index 7 with `status`
fn control_response(con: bool, seq: u8, iin1: u8, status: u8) -> Vec<u8> {
    let mut bytes = response(con, seq, iin1, 0).to_vec();
    bytes.extend_from_slice(&[41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, status]);
    bytes
}

/// the binary event as g2v1 in a response
const G2V1_EVENT: &[u8] = &[0x02, 0x01, 0x28, 0x01, 0x00, 0x00, 0x00, 0x81];

/// READ class 1 with sequence 0 and one binary event, whose response waits for a confirm
async fn enter_solicited_confirm_wait(harness: &mut OutstationHarness) {
    harness.handle.database.transaction(add_binary_event);
    let mut expected = response(true, 0, RESTART, 0).to_vec();
    expected.extend_from_slice(G2V1_EVENT);
    harness
        .test_request_response(&read_class_1(0), &expected)
        .await;
    harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);
}

/// the null unsolicited response at startup, which is never confirmed in these tests
async fn enter_unsolicited_confirm_wait(harness: &mut OutstationHarness) {
    harness.expect_response(&null_unsolicited(0)).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(0)]);
}

/// a broadcast RECORD_CURRENT_TIME, which gets no response
async fn broadcast(harness: &mut OutstationHarness, mode: BroadcastConfirmMode, seq: u8) {
    harness.set_broadcast(Some(mode));
    harness.send_and_process(&record_current_time(seq)).await;
    harness.set_broadcast(None);
}

const BROADCAST_PROCESSED: Event =
    Event::BroadcastReceived(FunctionCode::RecordCurrentTime, BroadcastAction::Processed);

/// the confirm of a solicited response, after which the response's events are deleted
async fn confirm_solicited(harness: &mut OutstationHarness, seq: u8) {
    harness.send_and_process(&confirm(seq)).await;
    harness.check_events(&[Event::SolicitedConfirmReceived(seq), Event::BeginConfirm]);
    assert!(matches!(harness.next_event().await, Event::EndConfirm(_)));
}

/// a request that changes nothing, answered with the IIN the outstation has now
async fn probe(harness: &mut OutstationHarness, seq: u8, iin1: u8) {
    harness
        .test_request_response(&record_current_time(seq), &response(false, seq, iin1, 0))
        .await;
    harness.check_no_events();
}

// ---------- repeats and malformed requests ----------

/// D3: the malformed request replaces the last request, so the request before it is new again
#[tokio::test]
async fn malformed_request_in_idle_replaces_the_repeat_history() {
    let mut harness = new_harness(get_default_config());

    let direct_operate = g41v2_control(FunctionCode::DirectOperate, 3, 7);
    let direct_operate_response = control_response(false, 3, RESTART, 0);
    for _ in 0..2 {
        harness
            .test_request_response(&direct_operate, &direct_operate_response)
            .await;
        harness.check_events(&operated(OperateType::DirectOperate));

        let malformed_response = response(false, 4, RESTART, PARAMETER_ERROR);
        harness
            .test_request_response(&malformed(4), &malformed_response)
            .await;
        harness
            .test_request_response(&malformed(4), &malformed_response)
            .await;
        harness.check_no_events();
    }
}

/// a mandatory broadcast makes every response ask for a confirm, so the solicited confirm wait
/// follows non-read requests
#[tokio::test]
async fn malformed_request_in_the_solicited_confirm_wait_ends_it_and_replaces_the_repeat_history() {
    let mut harness = new_harness(get_default_config());
    broadcast(&mut harness, BroadcastConfirmMode::Mandatory, 0).await;
    harness.check_events(&[BROADCAST_PROCESSED]);

    let direct_operate = g41v2_control(FunctionCode::DirectOperate, 3, 7);
    let direct_operate_response = control_response(true, 3, RESTART | BROADCAST, 0);
    harness
        .test_request_response(&direct_operate, &direct_operate_response)
        .await;
    harness.check_events(&operated(OperateType::DirectOperate));
    harness.check_events(&[Event::EnterSolicitedConfirmWait(3)]);

    let malformed_response = response(true, 4, RESTART | BROADCAST, PARAMETER_ERROR);
    harness
        .test_request_response(&malformed(4), &malformed_response)
        .await;
    harness.check_events(&[
        Event::SolicitedConfirmWaitNewRequest,
        Event::EnterSolicitedConfirmWait(4),
    ]);

    // the repeat ends the wait and waits again for the same response
    harness
        .test_request_response(&malformed(4), &malformed_response)
        .await;
    harness.check_events(&[
        Event::SolicitedConfirmWaitNewRequest,
        Event::EnterSolicitedConfirmWait(4),
    ]);

    // the DIRECT_OPERATE isn't a repeat any more
    harness
        .test_request_response(&direct_operate, &direct_operate_response)
        .await;
    harness.check_events(&[Event::SolicitedConfirmWaitNewRequest]);
    harness.check_events(&operated(OperateType::DirectOperate));
    harness.check_events(&[Event::EnterSolicitedConfirmWait(3)]);

    confirm_solicited(&mut harness, 3).await;
    probe(&mut harness, 5, RESTART).await;
}

/// D2: the stored octets, not the IIN the outstation has now
#[tokio::test]
async fn repeated_non_read_in_the_unsolicited_confirm_wait_gets_the_stored_octets() {
    let mut harness = new_harness(get_default_unsolicited_config());
    enter_unsolicited_confirm_wait(&mut harness).await;

    let direct_operate = g41v2_control(FunctionCode::DirectOperate, 0, 7);
    let direct_operate_response = control_response(false, 0, RESTART, 0);
    harness
        .test_request_response(&direct_operate, &direct_operate_response)
        .await;
    harness.check_events(&operated(OperateType::DirectOperate));

    harness.handle.database.transaction(add_binary_event);

    harness
        .test_request_response(&direct_operate, &direct_operate_response)
        .await;
    harness.check_no_events();

    probe(&mut harness, 1, RESTART | CLASS_1).await;
}

#[tokio::test]
async fn unknown_function_in_the_unsolicited_confirm_wait_keeps_the_repeat_history() {
    let mut harness = new_harness(get_default_unsolicited_config());
    enter_unsolicited_confirm_wait(&mut harness).await;

    let direct_operate = g41v2_control(FunctionCode::DirectOperate, 0, 7);
    let direct_operate_response = control_response(false, 0, RESTART, 0);
    harness
        .test_request_response(&direct_operate, &direct_operate_response)
        .await;
    harness.check_events(&operated(OperateType::DirectOperate));

    harness
        .test_request_response(
            &unknown_function(1),
            &response(false, 1, RESTART, NO_FUNC_CODE_SUPPORT),
        )
        .await;

    harness
        .test_request_response(&direct_operate, &direct_operate_response)
        .await;
    harness.check_no_events();
}

/// The IIN2.0 reply carries the mandatory broadcast's CON, but the outstation doesn't wait for its
/// confirm, and the request isn't the last request
#[tokio::test]
async fn unknown_function_in_the_solicited_confirm_wait_ends_it_and_keeps_the_repeat_history() {
    let mut harness = new_harness(get_default_config());
    broadcast(&mut harness, BroadcastConfirmMode::Mandatory, 0).await;
    harness.check_events(&[BROADCAST_PROCESSED]);

    let direct_operate = g41v2_control(FunctionCode::DirectOperate, 0, 7);
    let direct_operate_response = control_response(true, 0, RESTART | BROADCAST, 0);
    harness
        .test_request_response(&direct_operate, &direct_operate_response)
        .await;
    harness.check_events(&operated(OperateType::DirectOperate));
    harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);

    harness
        .test_request_response(
            &unknown_function(1),
            &response(true, 1, RESTART | BROADCAST, NO_FUNC_CODE_SUPPORT),
        )
        .await;
    harness.check_events(&[Event::SolicitedConfirmWaitNewRequest]);

    // idle ignores the confirm of the IIN2.0 reply
    harness.send_and_process(&confirm(1)).await;
    harness.expect_no_response();
    harness.check_no_events();

    harness
        .test_request_response(&direct_operate, &direct_operate_response)
        .await;
    harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);

    confirm_solicited(&mut harness, 0).await;
    probe(&mut harness, 2, RESTART).await;
}

#[tokio::test]
async fn repeated_non_read_in_the_solicited_confirm_wait_ends_it_and_waits_again() {
    let mut harness = new_harness(get_default_config());
    broadcast(&mut harness, BroadcastConfirmMode::Mandatory, 0).await;
    harness.check_events(&[BROADCAST_PROCESSED]);

    let direct_operate = g41v2_control(FunctionCode::DirectOperate, 0, 7);
    let direct_operate_response = control_response(true, 0, RESTART | BROADCAST, 0);
    harness
        .test_request_response(&direct_operate, &direct_operate_response)
        .await;
    harness.check_events(&operated(OperateType::DirectOperate));
    harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);

    harness
        .test_request_response(&direct_operate, &direct_operate_response)
        .await;
    harness.check_events(&[
        Event::SolicitedConfirmWaitNewRequest,
        Event::EnterSolicitedConfirmWait(0),
    ]);

    confirm_solicited(&mut harness, 0).await;
    probe(&mut harness, 1, RESTART).await;
}

// ---------- broadcasts ----------

#[tokio::test]
async fn broadcast_in_the_solicited_confirm_wait_ends_it_and_sets_the_broadcast_iin() {
    for mode in MODES {
        let mut harness = new_harness(get_default_config());
        enter_solicited_confirm_wait(&mut harness).await;

        broadcast(&mut harness, mode, 1).await;
        harness.expect_no_response();
        harness.check_events(&[Event::SolicitedConfirmWaitNewRequest, BROADCAST_PROCESSED]);

        // the unconfirmed event is pending again
        let iin1 = RESTART | BROADCAST | CLASS_1;
        if mode == BroadcastConfirmMode::Mandatory {
            harness
                .test_request_response(&record_current_time(2), &response(true, 2, iin1, 0))
                .await;
            harness.check_events(&[Event::EnterSolicitedConfirmWait(2)]);
            harness.send_and_process(&confirm(2)).await;
            harness.check_events(&[
                Event::SolicitedConfirmReceived(2),
                Event::BeginConfirm,
                Event::EndConfirm(class_1_binaries(1)),
            ]);
        } else {
            harness
                .test_request_response(&record_current_time(2), &response(false, 2, iin1, 0))
                .await;
            harness.check_no_events();
        }

        probe(&mut harness, 3, RESTART | CLASS_1).await;
    }
}

/// Table 6-1 row 24: a broadcast discards the deferred READ
#[tokio::test]
async fn broadcast_in_the_unsolicited_confirm_wait_discards_the_deferred_read() {
    for mode in MODES {
        let mut harness = new_harness(get_default_unsolicited_config());
        enter_unsolicited_confirm_wait(&mut harness).await;

        harness.send_and_process(&read_class_0(1)).await;
        harness.expect_no_response();

        broadcast(&mut harness, mode, 2).await;
        harness.expect_no_response();
        harness.check_events(&[BROADCAST_PROCESSED]);

        harness.send_and_process(&unsolicited_confirm(0)).await;
        harness.check_events(&[Event::UnsolicitedConfirmReceived(0)]);
        harness.expect_no_response();

        let con = mode == BroadcastConfirmMode::Mandatory;
        harness
            .test_request_response(
                &record_current_time(3),
                &response(con, 3, RESTART | BROADCAST, 0),
            )
            .await;
        if con {
            harness.check_events(&[Event::EnterSolicitedConfirmWait(3)]);
        }
        harness.check_no_events();
    }
}

#[tokio::test]
async fn unknown_function_reply_in_idle_reports_a_mandatory_broadcast_without_waiting_for_its_confirm(
) {
    let mut harness = new_harness(get_default_config());
    broadcast(&mut harness, BroadcastConfirmMode::Mandatory, 0).await;
    harness.check_events(&[BROADCAST_PROCESSED]);

    harness
        .test_request_response(
            &unknown_function(1),
            &response(true, 1, RESTART | BROADCAST, NO_FUNC_CODE_SUPPORT),
        )
        .await;
    harness.check_no_events();

    // idle ignores the confirm, so the broadcast is still reported
    harness.send_and_process(&confirm(1)).await;
    harness.expect_no_response();
    harness.check_no_events();

    harness
        .test_request_response(
            &record_current_time(2),
            &response(true, 2, RESTART | BROADCAST, 0),
        )
        .await;
    harness.check_events(&[Event::EnterSolicitedConfirmWait(2)]);
    confirm_solicited(&mut harness, 2).await;
    probe(&mut harness, 3, RESTART).await;
}

#[tokio::test]
async fn malformed_request_in_idle_reports_a_mandatory_broadcast_and_waits_for_its_confirm() {
    let mut harness = new_harness(get_default_config());
    broadcast(&mut harness, BroadcastConfirmMode::Mandatory, 0).await;
    harness.check_events(&[BROADCAST_PROCESSED]);

    harness
        .test_request_response(
            &malformed(1),
            &response(true, 1, RESTART | BROADCAST, PARAMETER_ERROR),
        )
        .await;
    harness.check_events(&[Event::EnterSolicitedConfirmWait(1)]);
    confirm_solicited(&mut harness, 1).await;
    probe(&mut harness, 2, RESTART).await;
}

/// the confirm of the response that reported the broadcast clears it
#[tokio::test]
async fn malformed_request_in_the_unsolicited_confirm_wait_reports_a_mandatory_broadcast() {
    let mut harness = new_harness(get_default_unsolicited_config());
    enter_unsolicited_confirm_wait(&mut harness).await;
    broadcast(&mut harness, BroadcastConfirmMode::Mandatory, 0).await;
    harness.check_events(&[BROADCAST_PROCESSED]);

    harness
        .test_request_response(
            &malformed(1),
            &response(true, 1, RESTART | BROADCAST, PARAMETER_ERROR),
        )
        .await;
    harness.check_no_events();

    harness.send_and_process(&confirm(1)).await;
    harness.expect_no_response();
    harness.check_no_events();

    probe(&mut harness, 2, RESTART).await;
}

/// a broadcast that doesn't require a confirm ends with the first response that reports it, even
/// an unsolicited one
#[tokio::test]
async fn unsolicited_response_consumes_a_broadcast_that_does_not_require_a_confirm() {
    for mode in [
        BroadcastConfirmMode::NotRequired,
        BroadcastConfirmMode::Optional,
    ] {
        let mut harness = new_harness(get_default_unsolicited_config());
        confirm_null_unsolicited(&mut harness).await;
        enable_unsolicited(&mut harness).await;

        broadcast(&mut harness, mode, 1).await;
        harness.check_events(&[BROADCAST_PROCESSED]);

        harness.handle.database.transaction(add_binary_event);
        let unsolicited = unsolicited(1, RESTART | BROADCAST, G2V1_EVENT);
        harness.expect_response(&unsolicited).await;
        harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);

        probe(&mut harness, 2, RESTART).await;
    }
}

#[tokio::test(start_paused = true)]
async fn unconfirmed_unsolicited_response_leaves_a_mandatory_broadcast_to_report() {
    let mut config = get_default_unsolicited_config();
    config.max_unsolicited_retries = Some(0);
    let mut harness = new_harness(config);
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;

    broadcast(&mut harness, BroadcastConfirmMode::Mandatory, 1).await;
    harness.check_events(&[BROADCAST_PROCESSED]);

    harness.handle.database.transaction(add_binary_event);
    let unsolicited = unsolicited(1, RESTART | BROADCAST, G2V1_EVENT);
    harness.expect_response(&unsolicited).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);
    harness
        .wait_for_events(&[Event::UnsolicitedConfirmTimeout(1, false)])
        .await;

    harness
        .test_request_response(
            &record_current_time(2),
            &response(true, 2, RESTART | BROADCAST | CLASS_1, 0),
        )
        .await;
    harness.check_events(&[Event::EnterSolicitedConfirmWait(2)]);
    harness.send_and_process(&confirm(2)).await;
    harness.check_events(&[
        Event::SolicitedConfirmReceived(2),
        Event::BeginConfirm,
        Event::EndConfirm(class_1_binaries(1)),
    ]);
    probe(&mut harness, 3, RESTART | CLASS_1).await;
}

/// a deferred READ is answered as from idle, and its response reports a mandatory broadcast
#[tokio::test]
async fn deferred_read_reports_a_mandatory_broadcast_and_waits_for_its_confirm() {
    let mut harness = new_harness(get_default_unsolicited_config());
    enter_unsolicited_confirm_wait(&mut harness).await;

    harness.send_and_process(&read_class_0(1)).await;
    harness.expect_no_response();

    // the READ was deferred before the broadcast
    broadcast(&mut harness, BroadcastConfirmMode::Mandatory, 2).await;
    harness.check_events(&[BROADCAST_PROCESSED]);
    harness.send_and_process(&read_class_0(3)).await;
    harness.expect_no_response();

    harness.send_and_process(&unsolicited_confirm(0)).await;
    harness
        .expect_response(&response(true, 3, RESTART | BROADCAST, 0))
        .await;
    harness.check_events(&[
        Event::UnsolicitedConfirmReceived(0),
        Event::EnterSolicitedConfirmWait(3),
    ]);
    confirm_solicited(&mut harness, 3).await;
    probe(&mut harness, 4, RESTART).await;
}

// ---------- confirms ----------

#[tokio::test]
async fn confirms_in_idle_are_ignored_and_keep_the_repeat_history() {
    let mut harness = new_harness(get_default_config());

    let direct_operate = g41v2_control(FunctionCode::DirectOperate, 0, 7);
    let direct_operate_response = control_response(false, 0, RESTART, 0);
    harness
        .test_request_response(&direct_operate, &direct_operate_response)
        .await;
    harness.check_events(&operated(OperateType::DirectOperate));

    for request in [confirm(0), unsolicited_confirm(0), confirm(1)] {
        harness.send_and_process(&request).await;
        harness.expect_no_response();
        harness.check_no_events();
    }

    harness
        .test_request_response(&direct_operate, &direct_operate_response)
        .await;
    harness.check_no_events();
}

/// The IIN2.0 reply reports a mandatory broadcast during the unsolicited confirm wait, but it
/// isn't the response to a valid request, so its confirm doesn't clear the broadcast
#[tokio::test]
async fn solicited_confirm_of_an_unknown_function_reply_in_the_unsolicited_confirm_wait_does_not_clear_a_reported_broadcast(
) {
    let mut harness = new_harness(get_default_unsolicited_config());
    enter_unsolicited_confirm_wait(&mut harness).await;
    broadcast(&mut harness, BroadcastConfirmMode::Mandatory, 0).await;
    harness.check_events(&[BROADCAST_PROCESSED]);

    harness
        .test_request_response(
            &unknown_function(1),
            &response(true, 1, RESTART | BROADCAST, NO_FUNC_CODE_SUPPORT),
        )
        .await;

    harness.send_and_process(&confirm(1)).await;
    harness.expect_no_response();
    harness.check_no_events();

    // still reported, and the confirm of the response that reports it now clears it
    harness
        .test_request_response(
            &record_current_time(2),
            &response(true, 2, RESTART | BROADCAST, 0),
        )
        .await;
    harness.send_and_process(&confirm(2)).await;
    harness.expect_no_response();
    harness.check_no_events();

    probe(&mut harness, 3, RESTART).await;
}

/// A solicited confirm in the unsolicited confirm wait, after the response's confirm timeout,
/// doesn't clear the mandatory broadcast it reported (Table 6-1 row 30)
#[tokio::test(start_paused = true)]
async fn late_solicited_confirm_in_the_unsolicited_confirm_wait_does_not_clear_a_reported_broadcast(
) {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;

    harness.handle.database.transaction(add_binary_event);
    let unsolicited = unsolicited(1, RESTART, G2V1_EVENT);
    harness.expect_response(&unsolicited).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);

    broadcast(&mut harness, BroadcastConfirmMode::Mandatory, 1).await;
    harness.check_events(&[BROADCAST_PROCESSED]);
    harness
        .test_request_response(
            &record_current_time(2),
            &response(true, 2, RESTART | BROADCAST, 0),
        )
        .await;

    // the unsolicited response is retried, still in the same wait
    harness.expect_response(&unsolicited).await;
    harness.check_events(&[Event::UnsolicitedConfirmTimeout(1, true)]);
    tokio::time::advance(Duration::from_secs(1)).await;

    harness.send_and_process(&confirm(2)).await;
    harness.expect_no_response();
    harness.check_no_events();

    harness.send_and_process(&unsolicited_confirm(1)).await;
    harness.check_events(&[
        Event::UnsolicitedConfirmReceived(1),
        Event::BeginConfirm,
        Event::Cleared(0),
        Event::EndConfirm(empty_buffer_state()),
    ]);

    harness
        .test_request_response(
            &record_current_time(3),
            &response(true, 3, RESTART | BROADCAST, 0),
        )
        .await;
    harness.check_events(&[Event::EnterSolicitedConfirmWait(3)]);
    confirm_solicited(&mut harness, 3).await;
    probe(&mut harness, 4, RESTART).await;
}

/// A solicited response asks for a confirm and its wait times out; a solicited confirm in the
/// unsolicited confirm wait that follows doesn't clear the mandatory broadcast
#[tokio::test(start_paused = true)]
async fn solicited_confirm_in_the_unsolicited_confirm_wait_does_not_clear_a_broadcast_reported_before_it(
) {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;

    broadcast(&mut harness, BroadcastConfirmMode::Mandatory, 1).await;
    harness.check_events(&[BROADCAST_PROCESSED]);
    harness
        .test_request_response(
            &record_current_time(2),
            &response(true, 2, RESTART | BROADCAST, 0),
        )
        .await;
    harness.check_events(&[Event::EnterSolicitedConfirmWait(2)]);
    harness
        .wait_for_events(&[Event::SolicitedConfirmTimeout(2)])
        .await;

    harness.handle.database.transaction(add_binary_event);
    let unsolicited = unsolicited(1, RESTART | BROADCAST, G2V1_EVENT);
    harness.expect_response(&unsolicited).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);

    harness.send_and_process(&confirm(2)).await;
    harness.expect_no_response();
    harness.check_no_events();

    // still reported, and the confirm of the response that reports it now clears it
    harness
        .test_request_response(
            &record_current_time(3),
            &response(true, 3, RESTART | BROADCAST, 0),
        )
        .await;
    harness.send_and_process(&confirm(3)).await;
    harness.expect_no_response();
    harness.check_no_events();

    probe(&mut harness, 4, RESTART).await;
}

// ---------- SELECT and OPERATE ----------

/// a SELECT whose response asks for a confirm, because of a mandatory broadcast
async fn select_in_the_solicited_confirm_wait(harness: &mut OutstationHarness) {
    broadcast(harness, BroadcastConfirmMode::Mandatory, 0).await;
    harness.check_events(&[BROADCAST_PROCESSED]);
    harness
        .test_request_response(
            &g41v2_control(FunctionCode::Select, 0, 7),
            &control_response(true, 0, RESTART | BROADCAST, 0),
        )
        .await;
    harness.check_events(&selected());
    harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);
}

#[tokio::test]
async fn operate_in_the_solicited_confirm_wait_uses_the_selection() {
    let mut harness = new_harness(get_default_config());
    select_in_the_solicited_confirm_wait(&mut harness).await;

    harness
        .test_request_response(
            &g41v2_control(FunctionCode::Operate, 1, 7),
            &control_response(true, 1, RESTART | BROADCAST, 0),
        )
        .await;
    harness.check_events(&[Event::SolicitedConfirmWaitNewRequest]);
    harness.check_events(&operated(OperateType::SelectBeforeOperate));
    harness.check_events(&[Event::EnterSolicitedConfirmWait(1)]);
    confirm_solicited(&mut harness, 1).await;
}

#[tokio::test]
async fn select_retry_in_the_solicited_confirm_wait_keeps_the_selection() {
    let mut harness = new_harness(get_default_config());
    select_in_the_solicited_confirm_wait(&mut harness).await;

    harness
        .test_request_response(
            &g41v2_control(FunctionCode::Select, 0, 7),
            &control_response(true, 0, RESTART | BROADCAST, 0),
        )
        .await;
    harness.check_events(&[
        Event::SolicitedConfirmWaitNewRequest,
        Event::EnterSolicitedConfirmWait(0),
    ]);

    harness
        .test_request_response(
            &g41v2_control(FunctionCode::Operate, 1, 7),
            &control_response(true, 1, RESTART | BROADCAST, 0),
        )
        .await;
    harness.check_events(&[Event::SolicitedConfirmWaitNewRequest]);
    harness.check_events(&operated(OperateType::SelectBeforeOperate));
    harness.check_events(&[Event::EnterSolicitedConfirmWait(1)]);
}

/// Table 4-9: a SELECT with the selection's sequence number but other objects is discarded, so it
/// doesn't end the confirm wait either
#[tokio::test]
async fn discarded_select_in_the_solicited_confirm_wait_continues_the_wait_and_keeps_the_selection()
{
    let mut harness = new_harness(get_default_config());
    select_in_the_solicited_confirm_wait(&mut harness).await;

    harness
        .send_and_process(&g41v2_control(FunctionCode::Select, 0, 8))
        .await;
    harness.expect_no_response();
    harness.check_no_events();

    confirm_solicited(&mut harness, 0).await;

    harness
        .test_request_response(
            &g41v2_control(FunctionCode::Operate, 1, 7),
            &control_response(false, 1, RESTART, 0),
        )
        .await;
    harness.check_events(&operated(OperateType::SelectBeforeOperate));
}

#[tokio::test]
async fn discarded_select_and_confirms_in_the_unsolicited_confirm_wait_keep_the_selection() {
    let mut harness = new_harness(get_default_unsolicited_config());
    enter_unsolicited_confirm_wait(&mut harness).await;

    harness
        .test_request_response(
            &g41v2_control(FunctionCode::Select, 0, 7),
            &control_response(false, 0, RESTART, 0),
        )
        .await;
    harness.check_events(&selected());

    for request in [
        g41v2_control(FunctionCode::Select, 0, 8),
        confirm(0).to_vec(),
        unsolicited_confirm(5).to_vec(),
    ] {
        harness.send_and_process(&request).await;
        harness.expect_no_response();
        harness.check_no_events();
    }

    harness
        .test_request_response(
            &g41v2_control(FunctionCode::Operate, 1, 7),
            &control_response(false, 1, RESTART, 0),
        )
        .await;
    harness.check_events(&operated(OperateType::SelectBeforeOperate));
}

// ---------- READs during the confirm waits ----------

/// Table 6-1 row 25: only the last READ is answered
#[tokio::test]
async fn read_in_the_unsolicited_confirm_wait_replaces_the_deferred_read() {
    let mut harness = new_harness(get_default_unsolicited_config());
    enter_unsolicited_confirm_wait(&mut harness).await;

    for seq in [1, 2] {
        harness.send_and_process(&read_class_0(seq)).await;
        harness.expect_no_response();
    }

    harness.send_and_process(&unsolicited_confirm(0)).await;
    harness.check_events(&[Event::UnsolicitedConfirmReceived(0)]);
    harness
        .expect_response(&response(false, 2, RESTART, 0))
        .await;
    probe(&mut harness, 3, RESTART).await;
}

/// Table 6-1 rows 26-29: a request that isn't a READ discards the deferred READ
#[tokio::test]
async fn requests_in_the_unsolicited_confirm_wait_discard_the_deferred_read() {
    let direct_operate = g41v2_control(FunctionCode::DirectOperate, 0, 7);
    let cases: [(&[u8], Vec<u8>); 4] = [
        (
            &delay_measure(2),
            vec![
                0xC2, 0x81, RESTART, 0x00, 0x34, 0x02, 0x07, 0x01, 0x00, 0x00,
            ],
        ),
        (
            &malformed(2),
            response(false, 2, RESTART, PARAMETER_ERROR).to_vec(),
        ),
        (
            &unknown_function(2),
            response(false, 2, RESTART, NO_FUNC_CODE_SUPPORT).to_vec(),
        ),
        // the repeat of the DIRECT_OPERATE
        (&direct_operate, control_response(false, 0, RESTART, 0)),
    ];

    for (request, expected) in cases {
        let mut harness = new_harness(get_default_unsolicited_config());
        enter_unsolicited_confirm_wait(&mut harness).await;

        harness
            .test_request_response(&direct_operate, &control_response(false, 0, RESTART, 0))
            .await;
        harness.check_events(&operated(OperateType::DirectOperate));

        harness.send_and_process(&read_class_0(1)).await;
        harness.expect_no_response();

        harness.test_request_response(request, &expected).await;
        harness.check_no_events();

        harness.send_and_process(&unsolicited_confirm(0)).await;
        harness.check_events(&[Event::UnsolicitedConfirmReceived(0)]);
        probe(&mut harness, 3, RESTART).await;
    }
}

#[tokio::test]
async fn disable_unsolicited_in_the_unsolicited_confirm_wait_is_the_last_request() {
    watchdog(async {
        let mut harness = new_harness(get_default_unsolicited_config());
        enter_unsolicited_confirm_wait(&mut harness).await;

        harness.send_and_process(&read_class_0(1)).await;
        harness.expect_no_response();

        let disable = [0xC2, 0x15, 60, 2, 0x06, 60, 3, 0x06, 60, 4, 0x06];
        harness
            .test_request_response(&disable, &response(false, 2, RESTART, 0))
            .await;

        // the wait goes on, and the repeat gets the same response
        harness
            .test_request_response(&disable, &response(false, 2, RESTART, 0))
            .await;
        harness.check_no_events();

        // and the deferred READ is gone
        harness.send_and_process(&unsolicited_confirm(0)).await;
        harness.check_events(&[Event::UnsolicitedConfirmReceived(0)]);
        probe(&mut harness, 3, RESTART).await;
    })
    .await;
}

// ---------- process_request_from_idle ----------

/// requests and confirms in idle are reported, but not a request with an unknown function code
#[tokio::test]
async fn requests_and_confirms_in_idle_are_reported_to_the_information_callback() {
    // an empty response is none
    let cases: [(&[u8], Vec<u8>, Option<Event>); 6] = [
        (
            &read_class_0(0),
            response(false, 0, RESTART, 0).to_vec(),
            Some(from_idle(FunctionCode::Read, false, 0)),
        ),
        (
            &record_current_time(0),
            response(false, 0, RESTART, 0).to_vec(),
            Some(from_idle(FunctionCode::RecordCurrentTime, false, 0)),
        ),
        (
            &malformed(0),
            response(false, 0, RESTART, PARAMETER_ERROR).to_vec(),
            Some(from_idle(FunctionCode::Read, false, 0)),
        ),
        (
            &confirm(0),
            Vec::new(),
            Some(from_idle(FunctionCode::Confirm, false, 0)),
        ),
        (
            &unsolicited_confirm(0),
            Vec::new(),
            Some(from_idle(FunctionCode::Confirm, true, 0)),
        ),
        (
            &unknown_function(0),
            response(false, 0, RESTART, NO_FUNC_CODE_SUPPORT).to_vec(),
            None,
        ),
    ];

    for (request, expected, event) in cases {
        let mut harness = new_harness(get_default_config());
        harness.record_requests_from_idle();
        if expected.is_empty() {
            harness.send_and_process(request).await;
            harness.expect_no_response();
        } else {
            harness.test_request_response(request, &expected).await;
        }
        if let Some(event) = event {
            harness.check_events(&[event]);
        }
        harness.check_no_events();
    }
}

#[tokio::test]
async fn discarded_select_in_idle_is_not_reported_to_the_information_callback() {
    let mut harness = new_harness(get_default_config());
    harness.record_requests_from_idle();

    harness
        .test_request_response(
            &g41v2_control(FunctionCode::Select, 0, 7),
            &control_response(false, 0, RESTART, 0),
        )
        .await;
    harness.check_events(&[from_idle(FunctionCode::Select, false, 0)]);
    harness.check_events(&selected());

    harness
        .send_and_process(&g41v2_control(FunctionCode::Select, 0, 8))
        .await;
    harness.expect_no_response();
    harness.check_no_events();
}

/// the request that ends the solicited confirm wait is reported once the session is back in idle
#[tokio::test]
async fn request_retained_from_the_solicited_confirm_wait_is_reported_to_the_information_callback()
{
    let mut harness = new_harness(get_default_config());
    harness.record_requests_from_idle();

    harness.handle.database.transaction(add_binary_event);
    let mut expected = response(true, 0, RESTART, 0).to_vec();
    expected.extend_from_slice(G2V1_EVENT);
    harness
        .test_request_response(&read_class_1(0), &expected)
        .await;
    harness.check_events(&[
        from_idle(FunctionCode::Read, false, 0),
        Event::EnterSolicitedConfirmWait(0),
    ]);

    harness
        .test_request_response(
            &record_current_time(1),
            &response(false, 1, RESTART | CLASS_1, 0),
        )
        .await;
    harness.check_events(&[
        Event::SolicitedConfirmWaitNewRequest,
        from_idle(FunctionCode::RecordCurrentTime, false, 1),
    ]);
    harness.check_no_events();
}

/// neither the requests in the unsolicited confirm wait nor the deferred READ are reported
#[tokio::test]
async fn requests_in_the_unsolicited_confirm_wait_are_not_reported_to_the_information_callback() {
    let mut harness = new_harness(get_default_unsolicited_config());
    harness.record_requests_from_idle();
    enter_unsolicited_confirm_wait(&mut harness).await;

    harness
        .test_request_response(&record_current_time(1), &response(false, 1, RESTART, 0))
        .await;
    harness.send_and_process(&read_class_0(2)).await;
    harness.expect_no_response();
    harness.send_and_process(&confirm(1)).await;
    harness.check_no_events();

    harness.send_and_process(&unsolicited_confirm(0)).await;
    harness.check_events(&[Event::UnsolicitedConfirmReceived(0)]);
    harness
        .expect_response(&response(false, 2, RESTART, 0))
        .await;
    harness.check_no_events();

    harness
        .test_request_response(&record_current_time(3), &response(false, 3, RESTART, 0))
        .await;
    harness.check_events(&[from_idle(FunctionCode::RecordCurrentTime, false, 3)]);
}

// ---------- link activity ----------

const KEEP_ALIVE_TIMEOUT: Duration = Duration::from_secs(7);
const ACTIVITY_AT: Duration = Duration::from_secs(4);

fn keep_alive<F: FnOnce() -> OutstationConfig>(config: F) -> OutstationConfig {
    let mut config = config();
    config.keep_alive_timeout = Some(KEEP_ALIVE_TIMEOUT);
    config
}

#[tokio::test(start_paused = true)]
async fn requests_in_idle_postpone_the_link_status_request() {
    for (request, expected) in [
        (confirm(0), None),
        (record_current_time(0), Some(response(false, 0, RESTART, 0))),
    ] {
        let start = Instant::now();
        let mut harness = new_harness(keep_alive(get_default_config));
        tokio::time::advance(ACTIVITY_AT).await;
        harness.send_and_process(&request).await;
        if let Some(expected) = expected {
            harness.expect_response(&expected).await;
        }
        harness.expect_response(LINK_STATUS_REQUEST).await;
        assert_eq!(start.elapsed(), ACTIVITY_AT + KEEP_ALIVE_TIMEOUT);
    }
}

#[tokio::test(start_paused = true)]
async fn fragments_in_the_solicited_confirm_wait_postpone_the_link_status_request() {
    for (request, event) in [
        // not a valid request
        (vec![0xC1], None),
        (
            confirm(1).to_vec(),
            Some(Event::WrongSolicitedConfirmSeq(0, 1)),
        ),
        (
            unsolicited_confirm(0).to_vec(),
            Some(Event::UnexpectedConfirm(true, 0)),
        ),
    ] {
        let start = Instant::now();
        let mut harness = new_harness(keep_alive(get_default_config));
        enter_solicited_confirm_wait(&mut harness).await;
        tokio::time::advance(ACTIVITY_AT).await;
        harness.send_and_process(&request).await;
        if let Some(event) = event {
            harness.check_events(&[event]);
        }
        harness
            .wait_for_events(&[Event::SolicitedConfirmTimeout(0)])
            .await;
        harness.expect_response(LINK_STATUS_REQUEST).await;
        assert_eq!(start.elapsed(), ACTIVITY_AT + KEEP_ALIVE_TIMEOUT);
    }
}

/// The null unsolicited response is sent again every 5 seconds, and the link status request
/// is only checked between the waits
#[tokio::test(start_paused = true)]
async fn requests_in_the_unsolicited_confirm_wait_postpone_the_link_status_request() {
    let requests: [&[u8]; 3] = [
        &unsolicited_confirm(1),
        &confirm(0),
        // too short for a request header, so it gets no response
        &[0xC0],
    ];
    for request in requests {
        let mut harness = new_harness(keep_alive(get_default_unsolicited_config));
        harness.expect_response(&null_unsolicited(0)).await;

        tokio::time::advance(ACTIVITY_AT).await;
        harness.send_and_process(request).await;

        // due at 11 seconds, so after the wait that ends at 15, not the one that ends at 10
        harness.expect_response(&null_unsolicited(1)).await;
        harness.expect_response(&null_unsolicited(2)).await;
        harness.expect_response(LINK_STATUS_REQUEST).await;
    }
}

// ---------- requests that end the solicited confirm wait while an unsolicited response is due ----------

const CLASS_2: u8 = 0x04;

/// unsolicited responses without link status requests, which would break the silences
fn unsolicited_config() -> OutstationConfig {
    let mut config = get_default_unsolicited_config();
    config.keep_alive_timeout = None;
    config
}

/// ENABLE_UNSOLICITED for class 2 only, so class 1 events are only reported to READs
const ENABLE_UNSOLICITED_CLASS_2: [u8; 5] = [0xC0, 0x14, 60, 3, 0x06];

const fn disable_unsolicited_class_2(seq: u8) -> [u8; 5] {
    [0xC0 | seq, 0x15, 60, 3, 0x06]
}

/// READ class 3, which has no events in these tests
const fn read_class_3(seq: u8) -> [u8; 5] {
    [0xC0 | seq, 0x01, 60, 4, 0x06]
}

/// a class 2 event at binary input `index`
fn add_class_2_event(harness: &mut OutstationHarness, index: u16) {
    harness.handle.database.transaction(|db| {
        db.add(
            index,
            Some(EventClass::Class2),
            BinaryInputConfig::default(),
        );
        db.update(
            index,
            &BinaryInput::new(true, Flags::ONLINE, Time::synchronized(0)),
            UpdateOptions::default(),
        );
    });
}

/// a buffer with one class 2 binary input event
fn class_2_binary() -> BufferState {
    let mut state = empty_buffer_state();
    state.classes.num_class_2 = 1;
    state.types.num_binary_input = 1;
    state
}

/// the response to READ class 1 with the binary event at index 0, which asks for a confirm
fn class_1_event_response(seq: u8, iin1: u8) -> Vec<u8> {
    let mut bytes = response(true, seq, iin1, 0).to_vec();
    bytes.extend_from_slice(G2V1_EVENT);
    bytes
}

/// the unsolicited response with sequence 1 and the class 2 event at index 1
fn class_2_unsolicited(iin1: u8) -> Vec<u8> {
    unsolicited(1, iin1, &g2v1_events(1..2))
}

/// Unsolicited responses enabled for class 2 only, and a READ of a class 1 event whose response
/// waits for its confirm (sequence 1). A class 2 event then makes an unsolicited response due.
/// Requests from idle are recorded from here on
async fn solicited_confirm_wait_with_unsolicited_due(harness: &mut OutstationHarness) {
    confirm_null_unsolicited(harness).await;
    harness
        .test_request_response(&ENABLE_UNSOLICITED_CLASS_2, &response(false, 0, RESTART, 0))
        .await;
    harness.handle.database.transaction(add_binary_event);
    harness
        .test_request_response(&read_class_1(1), &class_1_event_response(1, RESTART))
        .await;
    harness.check_events(&[Event::EnterSolicitedConfirmWait(1)]);

    add_class_2_event(harness, 1);
    harness.record_requests_from_idle();
}

/// the master confirms the unsolicited response with the class 2 event, while the class 1 event
/// is still pending
async fn confirm_class_2_unsolicited(harness: &mut OutstationHarness) {
    harness.send_and_process(&unsolicited_confirm(1)).await;
    harness.check_events(&[
        Event::UnsolicitedConfirmReceived(1),
        Event::BeginConfirm,
        Event::Cleared(1),
        Event::EndConfirm(class_1_binaries(1)),
    ]);
}

/// no response for longer than the confirm timeout and the unsolicited retry delay
async fn expect_silence(harness: &mut OutstationHarness) {
    tokio::time::sleep(Duration::from_secs(60)).await;
    harness.expect_no_response();
    harness.check_no_events();
}

/// The sequence numbers of a new request in the solicited confirm wait for the response to the
/// READ with sequence 1: the next one (Table 6-1 row 15), and the same one with other contents
/// (row 18). The other tests use the next one
const NEW_REQUEST_SEQS: [u8; 2] = [2, 1];

#[tokio::test(start_paused = true)]
async fn read_in_the_solicited_confirm_wait_is_answered_before_a_due_unsolicited_response() {
    watchdog(async {
        for seq in NEW_REQUEST_SEQS {
            let mut harness = new_harness(unsolicited_config());
            solicited_confirm_wait_with_unsolicited_due(&mut harness).await;

            harness
                .test_request_response(
                    &read_class_3(seq),
                    &response(false, seq, RESTART | CLASS_1 | CLASS_2, 0),
                )
                .await;
            harness
                .expect_response(&class_2_unsolicited(RESTART | CLASS_1))
                .await;
            harness.check_events(&[
                Event::SolicitedConfirmWaitNewRequest,
                from_idle(FunctionCode::Read, false, seq),
                Event::EnterUnsolicitedConfirmWait(1),
            ]);

            confirm_class_2_unsolicited(&mut harness).await;
            expect_silence(&mut harness).await;
        }
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn non_read_in_the_solicited_confirm_wait_is_answered_before_a_due_unsolicited_response() {
    watchdog(async {
        for seq in NEW_REQUEST_SEQS {
            let mut harness = new_harness(unsolicited_config());
            solicited_confirm_wait_with_unsolicited_due(&mut harness).await;

            harness
                .test_request_response(
                    &record_current_time(seq),
                    &response(false, seq, RESTART | CLASS_1 | CLASS_2, 0),
                )
                .await;
            harness
                .expect_response(&class_2_unsolicited(RESTART | CLASS_1))
                .await;
            harness.check_events(&[
                Event::SolicitedConfirmWaitNewRequest,
                from_idle(FunctionCode::RecordCurrentTime, false, seq),
                Event::EnterUnsolicitedConfirmWait(1),
            ]);

            confirm_class_2_unsolicited(&mut harness).await;
            expect_silence(&mut harness).await;
        }
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn disable_unsolicited_in_the_solicited_confirm_wait_is_processed_before_a_due_unsolicited_response(
) {
    watchdog(async {
        let mut harness = new_harness(unsolicited_config());
        solicited_confirm_wait_with_unsolicited_due(&mut harness).await;

        harness
            .test_request_response(
                &disable_unsolicited_class_2(2),
                &response(false, 2, RESTART | CLASS_1 | CLASS_2, 0),
            )
            .await;
        harness.check_events(&[
            Event::SolicitedConfirmWaitNewRequest,
            from_idle(FunctionCode::DisableUnsolicited, false, 2),
        ]);

        // the class 2 event is never reported unsolicited
        expect_silence(&mut harness).await;
        harness
            .test_request_response(
                &record_current_time(3),
                &response(false, 3, RESTART | CLASS_1 | CLASS_2, 0),
            )
            .await;
        harness.check_events(&[from_idle(FunctionCode::RecordCurrentTime, false, 3)]);
    })
    .await;
}

/// Two READs in a row, each answered with the class 1 event and waiting for its confirm. The
/// unsolicited response follows once the last wait ends
#[tokio::test(start_paused = true)]
async fn reads_in_the_solicited_confirm_wait_are_answered_before_a_due_unsolicited_response() {
    watchdog(async {
        let mut harness = new_harness(unsolicited_config());
        solicited_confirm_wait_with_unsolicited_due(&mut harness).await;

        for seq in [2, 3] {
            harness
                .test_request_response(
                    &read_class_1(seq),
                    &class_1_event_response(seq, RESTART | CLASS_2),
                )
                .await;
            harness.check_events(&[
                Event::SolicitedConfirmWaitNewRequest,
                from_idle(FunctionCode::Read, false, seq),
                Event::EnterSolicitedConfirmWait(seq),
            ]);
        }

        harness.send_and_process(&confirm(3)).await;
        harness.expect_response(&class_2_unsolicited(RESTART)).await;
        harness.check_events(&[
            Event::SolicitedConfirmReceived(3),
            Event::BeginConfirm,
            Event::Cleared(0),
            Event::EndConfirm(class_2_binary()),
            Event::EnterUnsolicitedConfirmWait(1),
        ]);

        harness.send_and_process(&unsolicited_confirm(1)).await;
        harness.check_events(&[
            Event::UnsolicitedConfirmReceived(1),
            Event::BeginConfirm,
            Event::Cleared(1),
            Event::EndConfirm(empty_buffer_state()),
        ]);
        expect_silence(&mut harness).await;
    })
    .await;
}

/// A request with an unknown function code, which isn't reported as a request from idle, and a
/// malformed READ
#[tokio::test(start_paused = true)]
async fn invalid_requests_in_the_solicited_confirm_wait_are_answered_before_a_due_unsolicited_response(
) {
    watchdog(async {
        let cases: [(&[u8], u8, Option<Event>); 2] = [
            (&unknown_function(2), NO_FUNC_CODE_SUPPORT, None),
            (
                &malformed(2),
                PARAMETER_ERROR,
                Some(from_idle(FunctionCode::Read, false, 2)),
            ),
        ];
        for (request, iin2, from_idle) in cases {
            let mut harness = new_harness(unsolicited_config());
            solicited_confirm_wait_with_unsolicited_due(&mut harness).await;

            harness
                .test_request_response(
                    request,
                    &response(false, 2, RESTART | CLASS_1 | CLASS_2, iin2),
                )
                .await;
            harness
                .expect_response(&class_2_unsolicited(RESTART | CLASS_1))
                .await;
            harness.check_events(&[Event::SolicitedConfirmWaitNewRequest]);
            if let Some(event) = from_idle {
                harness.check_events(&[event]);
            }
            harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);

            confirm_class_2_unsolicited(&mut harness).await;
            expect_silence(&mut harness).await;
        }
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn broadcast_disable_unsolicited_in_the_solicited_confirm_wait_is_processed_before_a_due_unsolicited_response(
) {
    watchdog(async {
        let mut harness = new_harness(unsolicited_config());
        solicited_confirm_wait_with_unsolicited_due(&mut harness).await;

        harness.set_broadcast(Some(BroadcastConfirmMode::NotRequired));
        harness
            .send_and_process(&disable_unsolicited_class_2(2))
            .await;
        harness.set_broadcast(None);
        harness.check_events(&[
            Event::SolicitedConfirmWaitNewRequest,
            from_idle(FunctionCode::DisableUnsolicited, false, 2),
            Event::BroadcastReceived(FunctionCode::DisableUnsolicited, BroadcastAction::Processed),
        ]);

        // the class 2 event is never reported unsolicited
        expect_silence(&mut harness).await;
        harness
            .test_request_response(
                &record_current_time(3),
                &response(false, 3, RESTART | BROADCAST | CLASS_1 | CLASS_2, 0),
            )
            .await;
        harness.check_events(&[from_idle(FunctionCode::RecordCurrentTime, false, 3)]);
    })
    .await;
}

/// The solicited confirm wait of a deferred READ ends, and the session returns to idle, where it
/// answers the new request before the due unsolicited response
#[tokio::test(start_paused = true)]
async fn request_in_the_solicited_confirm_wait_of_a_deferred_read_is_answered_before_a_due_unsolicited_response(
) {
    watchdog(async {
        let mut harness = new_harness(unsolicited_config());
        confirm_null_unsolicited(&mut harness).await;
        harness
            .test_request_response(&ENABLE_UNSOLICITED_CLASS_2, &response(false, 0, RESTART, 0))
            .await;
        harness.handle.database.transaction(add_binary_event);
        add_class_2_event(&mut harness, 1);
        harness
            .expect_response(&class_2_unsolicited(RESTART | CLASS_1))
            .await;
        harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);

        harness.send_and_process(&read_class_1(1)).await;
        harness.expect_no_response();
        confirm_class_2_unsolicited(&mut harness).await;
        harness
            .expect_response(&class_1_event_response(1, RESTART))
            .await;
        harness.check_events(&[Event::EnterSolicitedConfirmWait(1)]);

        add_class_2_event(&mut harness, 2);
        harness.record_requests_from_idle();

        harness
            .test_request_response(
                &record_current_time(2),
                &response(false, 2, RESTART | CLASS_1 | CLASS_2, 0),
            )
            .await;
        harness
            .expect_response(&unsolicited(2, RESTART | CLASS_1, &g2v1_events(2..3)))
            .await;
        harness.check_events(&[
            Event::SolicitedConfirmWaitNewRequest,
            from_idle(FunctionCode::RecordCurrentTime, false, 2),
            Event::EnterUnsolicitedConfirmWait(2),
        ]);
    })
    .await;
}
