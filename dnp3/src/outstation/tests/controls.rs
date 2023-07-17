use tokio::time::Duration;

use crate::app::variations::Group41Var2;
use crate::app::FunctionCode;
use crate::link::header::BroadcastConfirmMode;
use crate::outstation::config::Feature;
use crate::outstation::tests::harness::*;
use crate::outstation::traits::{BroadcastAction, OperateType};

const G41V2_INDEX_7: Control = Control::G41V2(Group41Var2::new(513), 7);
// select, seq == 0, g41v2 - count == 1, index == 7, value = 513, status == SUCCESS,
const SELECT_SEQ0_G41V2: &[u8] = &[0xC0, 0x03, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00];
// operate, seq == 1, g41v2 - count == 1, index == 7, value = 513, status == SUCCESS
const OPERATE_SEQ1_G41V2: &[u8] = &[0xC1, 0x04, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00];
// operate, seq == 1, g41v2 - count == 1, index == 8, value = 513, status == SUCCESS
const OPERATE_SEQ1_G41V2_INDEX_8: &[u8] = &[0xC1, 0x04, 41, 2, 0x17, 0x01, 0x08, 0x01, 0x02, 0x00];
// operate, seq == 2, g41v2 - count == 1, index == 7, value = 513, status == SUCCESS
const OPERATE_SEQ2_G41V2: &[u8] = &[0xC2, 0x04, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00];
// direct operate, seq == 0, g41v2 - count == 1, index == 7, value = 513, status == SUCCESS
const DIRECT_OPERATE_SEQ0_G41V2: &[u8] = &[0xC0, 0x05, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00];
// direct operate no ack, seq == 0, g41v2 - count == 1, index == 7, value = 513, status == SUCCESS
const DIRECT_OPERATE_NO_ACK_SEQ0_G41V2: &[u8] =
    &[0xC0, 0x06, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00];
// response, seq == 0, restart IIN + echo of request headers
const RESPONSE_SEQ0_G41V2_SUCCESS: &[u8] = &[
    0xC0, 0x81, 0x80, 0x00, 41, 2, 0x17, 0x1, 0x07, 0x01, 0x02, 0x00,
];
// response, seq == 1, restart IIN + echo of request headers
const RESPONSE_SEQ1_G41V2_SUCCESS: &[u8] = &[
    0xC1, 0x81, 0x80, 0x00, 41, 2, 0x17, 0x1, 0x07, 0x01, 0x02, 0x00,
];
// response, seq == 2, restart IIN + echo of request headers but with status == NO_SELECT
const RESPONSE_SEQ2_G41V2_NO_SELECT: &[u8] = &[
    0xC2, 0x81, 0x80, 0x00, 41, 2, 0x17, 0x1, 0x07, 0x01, 0x02, 0x02,
];
// response, seq == 1, restart IIN + echo of request headers, index == 8, status == 2 (NO_SELECT)
const RESPONSE_SEQ1_G41V2_INDEX8_NO_SELECT: &[u8] = &[
    0xC1, 0x81, 0x80, 0x00, 41, 2, 0x17, 0x1, 0x08, 0x01, 0x02, 0x02,
];
// response, seq == 1, restart IIN + echo of request headers but with STATUS == 1 (TIMEOUT)
const RESPONSE_SEQ1_G41V2_SELECT_TIMEOUT: &[u8] = &[
    0xC1, 0x81, 0x80, 0x00, 41, 2, 0x17, 0x1, 0x07, 0x01, 0x02, 0x01,
];

#[tokio::test]
async fn performs_direct_operate() {
    let mut harness = new_harness(get_default_config());

    harness
        .test_request_response(DIRECT_OPERATE_SEQ0_G41V2, RESPONSE_SEQ0_G41V2_SUCCESS)
        .await;

    harness.check_events(&[
        Event::BeginControls,
        Event::Operate(
            Control::G41V2(Group41Var2::new(513), 7),
            OperateType::DirectOperate,
        ),
        Event::EndControls,
    ]);
}

#[tokio::test]
async fn performs_direct_operate_no_ack() {
    let mut harness = new_harness(get_default_config());

    harness
        .send_and_process(DIRECT_OPERATE_NO_ACK_SEQ0_G41V2)
        .await;

    harness.check_events(&[
        Event::BeginControls,
        Event::Operate(G41V2_INDEX_7, OperateType::DirectOperateNoAck),
        Event::EndControls,
    ])
}

#[tokio::test]
async fn performs_direct_operate_no_ack_via_broadcast() {
    let mut harness =
        new_harness_for_broadcast(get_default_config(), BroadcastConfirmMode::Mandatory);

    harness
        .send_and_process(DIRECT_OPERATE_NO_ACK_SEQ0_G41V2)
        .await;

    harness.check_events(&[
        Event::BeginControls,
        Event::Operate(G41V2_INDEX_7, OperateType::DirectOperateNoAck),
        Event::EndControls,
        Event::BroadcastReceived(
            FunctionCode::DirectOperateNoResponse,
            BroadcastAction::Processed,
        ),
    ]);
}

#[tokio::test]
async fn broadcast_support_can_be_disabled() {
    let mut config = get_default_config();
    config.features.broadcast = Feature::Disabled;

    let mut harness = new_harness_for_broadcast(config, BroadcastConfirmMode::Mandatory);

    harness
        .send_and_process(DIRECT_OPERATE_NO_ACK_SEQ0_G41V2)
        .await;

    harness.check_events(&[Event::BroadcastReceived(
        FunctionCode::DirectOperateNoResponse,
        BroadcastAction::IgnoredByConfiguration,
    )]);
}

#[tokio::test]
async fn performs_select_before_operate() {
    let mut harness = new_harness(get_default_config());

    // ------------ select -------------

    harness
        .test_request_response(SELECT_SEQ0_G41V2, RESPONSE_SEQ0_G41V2_SUCCESS)
        .await;

    harness.check_events(&[
        Event::BeginControls,
        Event::Select(G41V2_INDEX_7),
        Event::EndControls,
    ]);

    // ------------ operate -------------

    harness
        .test_request_response(OPERATE_SEQ1_G41V2, RESPONSE_SEQ1_G41V2_SUCCESS)
        .await;

    harness.check_events(&[
        Event::BeginControls,
        Event::Operate(G41V2_INDEX_7, OperateType::SelectBeforeOperate),
        Event::EndControls,
    ]);
}

#[tokio::test]
async fn rejects_operate_with_non_consecutive_sequence() {
    let mut harness = new_harness(get_default_config());

    // ------------ select -------------
    harness
        .test_request_response(SELECT_SEQ0_G41V2, RESPONSE_SEQ0_G41V2_SUCCESS)
        .await;

    harness.check_events(&[
        Event::BeginControls,
        Event::Select(G41V2_INDEX_7),
        Event::EndControls,
    ]);

    // ------------ operate -------------

    harness
        .test_request_response(OPERATE_SEQ2_G41V2, RESPONSE_SEQ2_G41V2_NO_SELECT)
        .await;

    harness.check_no_events();
}

#[tokio::test]
async fn rejects_operate_with_non_matching_headers() {
    let mut harness = new_harness(get_default_config());

    // ------------ select -------------
    harness
        .test_request_response(SELECT_SEQ0_G41V2, RESPONSE_SEQ0_G41V2_SUCCESS)
        .await;

    harness.check_events(&[
        Event::BeginControls,
        Event::Select(G41V2_INDEX_7),
        Event::EndControls,
    ]);

    // ------------ operate -------------

    harness
        .test_request_response(
            OPERATE_SEQ1_G41V2_INDEX_8,
            RESPONSE_SEQ1_G41V2_INDEX8_NO_SELECT,
        )
        .await;

    harness.check_no_events();
}

#[tokio::test]
async fn select_can_time_out() {
    let mut harness = new_harness(get_default_config());

    // ------------ select -------------
    harness
        .test_request_response(SELECT_SEQ0_G41V2, RESPONSE_SEQ0_G41V2_SUCCESS)
        .await;

    harness.check_events(&[
        Event::BeginControls,
        Event::Select(G41V2_INDEX_7),
        Event::EndControls,
    ]);

    tokio::time::pause();
    tokio::time::advance(get_default_config().select_timeout.0 + Duration::from_millis(1)).await;

    // ------------ operate -------------

    harness
        .test_request_response(OPERATE_SEQ1_G41V2, RESPONSE_SEQ1_G41V2_SELECT_TIMEOUT)
        .await;

    harness.check_no_events();
}

#[tokio::test]
async fn accept_two_identical_selects_before_operate() {
    let mut harness = new_harness(get_default_config());

    // ------------ select -------------

    harness
        .test_request_response(SELECT_SEQ0_G41V2, RESPONSE_SEQ0_G41V2_SUCCESS)
        .await;

    harness.check_events(&[
        Event::BeginControls,
        Event::Select(G41V2_INDEX_7),
        Event::EndControls,
    ]);

    // --------- second select ----------
    harness
        .test_request_response(SELECT_SEQ0_G41V2, RESPONSE_SEQ0_G41V2_SUCCESS)
        .await;

    harness.check_no_events();

    // ------------ operate -------------

    harness
        .test_request_response(OPERATE_SEQ1_G41V2, RESPONSE_SEQ1_G41V2_SUCCESS)
        .await;

    harness.check_events(&[
        Event::BeginControls,
        Event::Operate(G41V2_INDEX_7, OperateType::SelectBeforeOperate),
        Event::EndControls,
    ]);
}
