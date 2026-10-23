// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::variations::Group41Var2;
use crate::app::{BufferSize, FunctionCode};
use crate::link::header::BroadcastConfirmMode;
use crate::outstation::database::*;
use crate::outstation::tests::harness::*;
use crate::outstation::traits::{BroadcastAction, OperateType};

// g30v1, 16-bit start/stop, 0 to 149, then class 1 events
const READ_G30V1_0_TO_149_AND_CLASS_1: &[u8] = &[
    0xC0, 0x01, 0x1E, 0x01, 0x01, 0x00, 0x00, 0x95, 0x00, 0x3C, 0x02, 0x06,
];
// restart IIN, no objects
const EMPTY_RESPONSE_SEQ_1: &[u8] = &[0xC1, 0x81, 0x80, 0x00];
// direct operate no ack, seq == 1, g41v2 - count == 1, index == 7, value = 513
const DIRECT_OPERATE_NO_ACK_SEQ_1: &[u8] = &[0xC1, 0x06, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00];
// direct operate, seq == 0, g41v2 - count == 1, index == 7, value = 513
const DIRECT_OPERATE_SEQ_0: &[u8] = &[0xC0, 0x05, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00];
// restart IIN + echo of the direct operate with status == SUCCESS
const DIRECT_OPERATE_RESPONSE_SEQ_0: &[u8] = &[
    0xC0, 0x81, 0x80, 0x00, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00,
];

async fn direct_operate(harness: &mut OutstationHarness) {
    harness
        .test_request_response(DIRECT_OPERATE_SEQ_0, DIRECT_OPERATE_RESPONSE_SEQ_0)
        .await;
    harness.check_events(&operated(OperateType::DirectOperate));
}

#[tokio::test]
async fn repeated_read_during_a_later_fragment_resends_that_fragment() {
    let mut config = get_default_config();
    config.solicited_buffer_size = BufferSize::min();
    let mut harness = new_harness(config);

    // g30v1 values for several fragments, and an event that changes the length of the first
    harness.handle.database.transaction(|db| {
        for index in 0..150 {
            db.add(index, None, AnalogInputConfig::default());
        }
        add_binary_event(db);
    });

    harness
        .send_and_process(READ_G30V1_0_TO_149_AND_CLASS_1)
        .await;
    let first = harness.expect_write().await;
    harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);

    harness.send_and_process(&confirm(0)).await;
    let second = harness.expect_write().await;
    assert_eq!(&second[0..2], &[0x21, 0x81]); // CON, seq 1
    assert_ne!(first.len(), second.len());

    harness
        .send_and_process(READ_G30V1_0_TO_149_AND_CLASS_1)
        .await;
    harness.expect_response(&second).await;

    // still waiting for the confirm of the second fragment
    harness.send_and_process(&confirm(1)).await;
    let third = harness.expect_write().await;
    assert_eq!(third[0] & 0x0F, 2);
}

#[tokio::test]
async fn repeated_request_after_an_error_reply_gets_the_original_response() {
    let mut harness = new_harness(get_default_config());

    harness.send_and_process(&delay_measure(0)).await;
    let response = harness.expect_write().await;
    assert!(response.len() > 4);

    // an unknown function code
    harness
        .test_request_response(&[0xC1, 0x70], &[0xC1, 0x81, 0x80, 0x01])
        .await;

    harness.send_and_process(&delay_measure(0)).await;
    harness.expect_response(&response).await;
}

#[tokio::test]
async fn repeated_operate_without_select_gets_the_same_status() {
    let mut harness = new_harness(get_default_config());

    // operate, seq == 0, g41v2 - count == 1, index == 7, value = 513
    let operate = [0xC0, 0x04, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00];
    // echo with status == NO_SELECT
    let response = [
        0xC0, 0x81, 0x80, 0x00, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x02,
    ];

    harness.test_request_response(&operate, &response).await;
    harness.test_request_response(&operate, &response).await;
    harness.check_no_events();
}

#[tokio::test]
async fn request_after_a_malformed_request_during_unsolicited_confirm_wait_is_processed_again() {
    // direct operate, seq == 3, g41v2 - count == 1, index == 7, value = 513
    let direct_operate = [0xC3, 0x05, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00];
    // restart IIN + echo of the request with status == SUCCESS
    let response = [
        0xC3, 0x81, 0x80, 0x00, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00,
    ];
    // read, seq == 4, g1v2 with a 1-byte start/stop qualifier but no start or stop
    let malformed = [0xC4, 0x01, 0x01, 0x02, 0x00];
    // restart IIN and IIN2.2 PARAMETER_ERROR
    let malformed_response = [0xC4, 0x81, 0x80, 0x04];
    let operated = [
        Event::BeginControls,
        Event::Operate(
            Control::G41V2(Group41Var2::new(513), 7),
            OperateType::DirectOperate,
        ),
        Event::EndControls,
    ];

    let mut harness = new_harness(get_default_unsolicited_config());

    // the null unsolicited response is never confirmed
    harness.expect_response(&null_unsolicited(0)).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(0)]);

    harness
        .test_request_response(&direct_operate, &response)
        .await;
    harness.check_events(&operated);

    // the malformed request becomes the last request, and its repeat is answered as before
    harness
        .test_request_response(&malformed, &malformed_response)
        .await;
    harness
        .test_request_response(&malformed, &malformed_response)
        .await;
    harness.check_no_events();

    // so the direct operate isn't a repeat
    harness
        .test_request_response(&direct_operate, &response)
        .await;
    harness.check_events(&operated);
}

#[tokio::test]
async fn repeated_direct_operate_gets_the_original_response_after_the_iin_changes() {
    let mut harness = new_harness(get_default_config());

    direct_operate(&mut harness).await;

    // a class 1 event sets IIN1.1 in the responses that follow
    harness.handle.database.transaction(add_binary_event);

    harness
        .test_request_response(DIRECT_OPERATE_SEQ_0, DIRECT_OPERATE_RESPONSE_SEQ_0)
        .await;
    harness.check_no_events();

    // a new request reports the current IIN
    harness
        .test_request_response(&record_current_time(1), &[0xC1, 0x81, 0x82, 0x00])
        .await;
}

#[tokio::test]
async fn repeated_request_after_a_broadcast_leaves_the_broadcast_iin_for_the_next_response() {
    for mode in [
        BroadcastConfirmMode::Optional,
        BroadcastConfirmMode::NotRequired,
        BroadcastConfirmMode::Mandatory,
    ] {
        let mut harness = new_harness(get_default_config());

        direct_operate(&mut harness).await;

        harness.set_broadcast(Some(mode));
        harness.send_and_process(DIRECT_OPERATE_NO_ACK_SEQ_1).await;
        harness.expect_no_response();
        harness.check_events(&[
            Event::BeginControls,
            Event::Operate(G41V2_INDEX_7, OperateType::DirectOperateNoAck),
            Event::EndControls,
            Event::BroadcastReceived(
                FunctionCode::DirectOperateNoResponse,
                BroadcastAction::Processed,
            ),
        ]);
        harness.set_broadcast(None);

        harness
            .test_request_response(DIRECT_OPERATE_SEQ_0, DIRECT_OPERATE_RESPONSE_SEQ_0)
            .await;
        harness.check_no_events();

        // restart IIN and IIN1.0 BROADCAST, which asks for a confirm if the broadcast required one
        if mode == BroadcastConfirmMode::Mandatory {
            harness
                .test_request_response(&record_current_time(1), &[0xE1, 0x81, 0x81, 0x00])
                .await;
            harness.check_events(&[Event::EnterSolicitedConfirmWait(1)]);
        } else {
            harness
                .test_request_response(&record_current_time(1), &[0xC1, 0x81, 0x81, 0x00])
                .await;
            harness.check_no_events();
        }
    }
}

#[tokio::test(start_paused = true)]
async fn repeated_request_whose_response_asked_for_a_confirm_waits_for_it_again() {
    // CON, restart IIN and IIN1.0 BROADCAST + echo of the direct operate with status == SUCCESS
    let response = [
        0xE0, 0x81, 0x81, 0x00, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00,
    ];
    let mut harness = new_harness(get_default_config());

    // a broadcast that requires a confirm
    harness.set_broadcast(Some(BroadcastConfirmMode::Mandatory));
    harness.send_and_process(DIRECT_OPERATE_NO_ACK_SEQ_1).await;
    harness.expect_no_response();
    harness.check_events(&[
        Event::BeginControls,
        Event::Operate(G41V2_INDEX_7, OperateType::DirectOperateNoAck),
        Event::EndControls,
        Event::BroadcastReceived(
            FunctionCode::DirectOperateNoResponse,
            BroadcastAction::Processed,
        ),
    ]);
    harness.set_broadcast(None);

    harness
        .test_request_response(DIRECT_OPERATE_SEQ_0, &response)
        .await;
    harness.check_events(&[
        Event::BeginControls,
        Event::Operate(G41V2_INDEX_7, OperateType::DirectOperate),
        Event::EndControls,
        Event::EnterSolicitedConfirmWait(0),
    ]);
    harness
        .wait_for_events(&[Event::SolicitedConfirmTimeout(0)])
        .await;

    harness
        .test_request_response(DIRECT_OPERATE_SEQ_0, &response)
        .await;
    harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);

    harness.send_and_process(&confirm(0)).await;
    harness.check_events(&[Event::SolicitedConfirmReceived(0), Event::BeginConfirm]);
    assert!(matches!(harness.next_event().await, Event::EndConfirm(_)));

    // the confirm ended the broadcast's IIN1.0 and its confirm requirement
    harness
        .test_request_response(&record_current_time(1), EMPTY_RESPONSE_SEQ_1)
        .await;
    harness.check_no_events();
}
