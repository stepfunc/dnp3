use crate::app::measurement::*;
use crate::app::variations::Group41Var2;
use crate::app::{BufferSize, FunctionCode, Timestamp};
use crate::link::header::BroadcastConfirmMode;
use crate::outstation::database::*;
use crate::outstation::tests::harness::*;
use crate::outstation::traits::{BroadcastAction, OperateType};

// g30v1, 16-bit start/stop, 0 to 149, then class 1 events
const READ_G30V1_0_TO_149_AND_CLASS_1: &[u8] = &[
    0xC0, 0x01, 0x1E, 0x01, 0x01, 0x00, 0x00, 0x95, 0x00, 0x3C, 0x02, 0x06,
];
const CONFIRM_SEQ_0: &[u8] = &[0xC0, 0x00];
const CONFIRM_SEQ_1: &[u8] = &[0xC1, 0x00];
const DELAY_MEASURE_SEQ_0: &[u8] = &[0xC0, 0x17];
const RECORD_CURRENT_TIME_SEQ_1: &[u8] = &[0xC1, 0x18];
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
const G41V2_INDEX_7: Control = Control::G41V2(Group41Var2::new(513), 7);

async fn direct_operate(harness: &mut OutstationHarness) {
    harness
        .test_request_response(DIRECT_OPERATE_SEQ_0, DIRECT_OPERATE_RESPONSE_SEQ_0)
        .await;
    harness.check_events(&[
        Event::BeginControls,
        Event::Operate(G41V2_INDEX_7, OperateType::DirectOperate),
        Event::EndControls,
    ]);
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
        db.add(0, Some(EventClass::Class1), BinaryInputConfig::default());
        db.update(
            0,
            &BinaryInput::new(true, Flags::ONLINE, Time::Synchronized(Timestamp::new(0))),
            UpdateOptions::default(),
        );
    });

    harness
        .send_and_process(READ_G30V1_0_TO_149_AND_CLASS_1)
        .await;
    let first = harness.expect_write().await;
    harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);

    harness.send_and_process(CONFIRM_SEQ_0).await;
    let second = harness.expect_write().await;
    assert_eq!(&second[0..2], &[0x21, 0x81]); // CON, seq 1
    assert_ne!(first.len(), second.len());

    harness
        .send_and_process(READ_G30V1_0_TO_149_AND_CLASS_1)
        .await;
    harness.expect_response(&second).await;

    // still waiting for the confirm of the second fragment
    harness.send_and_process(CONFIRM_SEQ_1).await;
    let third = harness.expect_write().await;
    assert_eq!(third[0] & 0x0F, 2);
}

#[tokio::test]
async fn repeated_request_after_an_error_reply_gets_the_original_response() {
    let mut harness = new_harness(get_default_config());

    harness.send_and_process(DELAY_MEASURE_SEQ_0).await;
    let response = harness.expect_write().await;
    assert!(response.len() > 4);

    // an unknown function code
    harness
        .test_request_response(&[0xC1, 0x70], &[0xC1, 0x81, 0x80, 0x01])
        .await;

    harness.send_and_process(DELAY_MEASURE_SEQ_0).await;
    harness.expect_response(&response).await;
}

#[tokio::test]
async fn repeated_request_with_an_empty_response_gets_no_objects() {
    let mut harness = new_harness(get_default_config());

    harness.send_and_process(DELAY_MEASURE_SEQ_0).await;
    assert!(harness.expect_write().await.len() > 4);

    harness
        .test_request_response(RECORD_CURRENT_TIME_SEQ_1, EMPTY_RESPONSE_SEQ_1)
        .await;
    harness
        .test_request_response(RECORD_CURRENT_TIME_SEQ_1, EMPTY_RESPONSE_SEQ_1)
        .await;
}

#[tokio::test]
async fn repeated_no_ack_request_gets_no_response() {
    let mut harness = new_harness(get_default_config());

    harness.send_and_process(DELAY_MEASURE_SEQ_0).await;
    assert!(harness.expect_write().await.len() > 4);

    harness.send_and_process(DIRECT_OPERATE_NO_ACK_SEQ_1).await;
    harness.check_events(&[
        Event::BeginControls,
        Event::Operate(
            Control::G41V2(Group41Var2::new(513), 7),
            OperateType::DirectOperateNoAck,
        ),
        Event::EndControls,
    ]);
    harness.expect_no_response();

    harness.send_and_process(DIRECT_OPERATE_NO_ACK_SEQ_1).await;
    harness.check_no_events();
    harness.expect_no_response();
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
async fn repeated_select_whose_response_does_not_fit_gets_the_same_error() {
    let mut config = get_default_config();
    config.solicited_buffer_size = BufferSize::min();
    let mut harness = new_harness(config);

    harness.send_and_process(DELAY_MEASURE_SEQ_0).await;
    assert!(harness.expect_write().await.len() > 4);

    // g41v2 with a 1-byte count and index, more controls than the echo has room for
    let count = 70;
    let mut select = vec![0xC1, FunctionCode::Select.as_u8(), 41, 2, 0x17, count];
    for index in 0..count {
        select.extend_from_slice(&[index, 0x01, 0x02, 0x00]);
    }
    // restart IIN and IIN2.2 PARAMETER_ERROR
    let response = [0xC1, 0x81, 0x80, 0x04];

    harness.test_request_response(&select, &response).await;
    harness.test_request_response(&select, &response).await;
    harness.check_no_events();
}

#[tokio::test]
async fn repeated_request_after_a_malformed_request_during_unsolicited_confirm_wait_gets_the_original_response(
) {
    let mut harness = new_harness(get_default_unsolicited_config());

    // the null unsolicited response is never confirmed
    harness.expect_response(&[0xF0, 0x82, 0x80, 0x00]).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(0)]);

    harness.send_and_process(DELAY_MEASURE_SEQ_0).await;
    let response = harness.expect_write().await;
    assert!(response.len() > 4);

    // g1v2 with a 1-byte start/stop qualifier but no start or stop
    harness
        .test_request_response(&[0xC1, 0x01, 0x01, 0x02, 0x00], &[0xC1, 0x81, 0x80, 0x04])
        .await;

    harness.send_and_process(DELAY_MEASURE_SEQ_0).await;
    harness.expect_response(&response).await;
    harness.check_no_events();
}

#[tokio::test]
async fn repeated_direct_operate_gets_the_original_response_after_the_iin_changes() {
    let mut harness = new_harness(get_default_config());

    direct_operate(&mut harness).await;

    // a class 1 event sets IIN1.1 in the responses that follow
    harness.handle.database.transaction(|db| {
        db.add(0, Some(EventClass::Class1), BinaryInputConfig::default());
        db.update(
            0,
            &BinaryInput::new(true, Flags::ONLINE, Time::Synchronized(Timestamp::new(0))),
            UpdateOptions::default(),
        );
    });

    harness
        .test_request_response(DIRECT_OPERATE_SEQ_0, DIRECT_OPERATE_RESPONSE_SEQ_0)
        .await;
    harness.check_no_events();

    // a new request reports the current IIN
    harness
        .test_request_response(RECORD_CURRENT_TIME_SEQ_1, &[0xC1, 0x81, 0x82, 0x00])
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
                .test_request_response(RECORD_CURRENT_TIME_SEQ_1, &[0xE1, 0x81, 0x81, 0x00])
                .await;
            harness.check_events(&[Event::EnterSolicitedConfirmWait(1)]);
        } else {
            harness
                .test_request_response(RECORD_CURRENT_TIME_SEQ_1, &[0xC1, 0x81, 0x81, 0x00])
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

    harness.send_and_process(CONFIRM_SEQ_0).await;
    harness.check_events(&[Event::SolicitedConfirmReceived(0), Event::BeginConfirm]);
    assert!(matches!(harness.next_event().await, Event::EndConfirm(_)));

    // the confirm ended the broadcast's IIN1.0 and its confirm requirement
    harness
        .test_request_response(RECORD_CURRENT_TIME_SEQ_1, EMPTY_RESPONSE_SEQ_1)
        .await;
    harness.check_no_events();
}
