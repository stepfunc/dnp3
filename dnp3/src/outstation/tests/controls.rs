#![expect(
    clippy::arithmetic_side_effects,
    reason = "tests count the controls each request operates"
)]

use tokio::time::Duration;

use crate::app::variations::Group41Var2;
use crate::app::{BufferSize, FunctionCode};
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

// an echo of this many g41v2 controls (8 + 4 * 61 bytes) exactly fills the response buffer
const MAX_CONTROLS_IN_RESPONSE: u8 = 61;
const RESPONSE_BUFFER_SIZE: usize = 252;

fn config_with_small_response_buffer() -> crate::outstation::OutstationConfig {
    let mut config = get_default_config();
    config.solicited_buffer_size = BufferSize::new(RESPONSE_BUFFER_SIZE).unwrap();
    config
}

// g41v2 with a 1-byte count and index, value 513 at indices 0 to count - 1, status SUCCESS
fn g41v2_request(seq: u8, function: FunctionCode, count: u8) -> Vec<u8> {
    let mut request = vec![0xC0 | seq, function.as_u8(), 41, 2, 0x17, count];
    for index in 0..count {
        request.extend_from_slice(&[index, 0x01, 0x02, 0x00]);
    }
    request
}

// response with restart IIN that echoes g41v2_request with every status set to `status`
fn g41v2_response(seq: u8, count: u8, status: u8) -> Vec<u8> {
    let mut response = vec![0xC0 | seq, 0x81, 0x80, 0x00, 41, 2, 0x17, count];
    for index in 0..count {
        response.extend_from_slice(&[index, 0x01, 0x02, status]);
    }
    response
}

// response with restart IIN and IIN2.2 PARAMETER_ERROR, and no objects
fn parameter_error_response(seq: u8) -> [u8; 4] {
    [0xC0 | seq, 0x81, 0x80, 0x04]
}

fn g41v2_select_events(count: u8) -> Vec<Event> {
    let mut events = vec![Event::BeginControls];
    for index in 0..count {
        events.push(Event::Select(Control::G41V2(
            Group41Var2::new(513),
            index.into(),
        )));
    }
    events.push(Event::EndControls);
    events
}

fn g41v2_operate_events(count: u8, operate_type: OperateType) -> Vec<Event> {
    let mut events = vec![Event::BeginControls];
    for index in 0..count {
        events.push(Event::Operate(
            Control::G41V2(Group41Var2::new(513), index.into()),
            operate_type,
        ));
    }
    events.push(Event::EndControls);
    events
}

#[tokio::test]
async fn direct_operate_whose_response_fits_exactly_operates() {
    let mut harness = new_harness(config_with_small_response_buffer());

    harness
        .test_request_response(
            &g41v2_request(0, FunctionCode::DirectOperate, MAX_CONTROLS_IN_RESPONSE),
            &g41v2_response(0, MAX_CONTROLS_IN_RESPONSE, 0),
        )
        .await;

    harness.check_events(&g41v2_operate_events(
        MAX_CONTROLS_IN_RESPONSE,
        OperateType::DirectOperate,
    ));
}

#[tokio::test]
async fn select_and_operate_whose_responses_fit_exactly_operate() {
    let mut harness = new_harness(config_with_small_response_buffer());

    harness
        .test_request_response(
            &g41v2_request(0, FunctionCode::Select, MAX_CONTROLS_IN_RESPONSE),
            &g41v2_response(0, MAX_CONTROLS_IN_RESPONSE, 0),
        )
        .await;
    harness.check_events(&g41v2_select_events(MAX_CONTROLS_IN_RESPONSE));

    harness
        .test_request_response(
            &g41v2_request(1, FunctionCode::Operate, MAX_CONTROLS_IN_RESPONSE),
            &g41v2_response(1, MAX_CONTROLS_IN_RESPONSE, 0),
        )
        .await;
    harness.check_events(&g41v2_operate_events(
        MAX_CONTROLS_IN_RESPONSE,
        OperateType::SelectBeforeOperate,
    ));
}

#[tokio::test]
async fn direct_operate_whose_headers_fit_alone_but_not_together_is_rejected_without_operating() {
    let mut harness = new_harness(config_with_small_response_buffer());

    // g41v2 with a 1-byte count and index, then g41v2 with a 2-byte count and index, 30
    // controls each: either header's echo fits on its own, but not both
    let mut request = vec![0xC0, FunctionCode::DirectOperate.as_u8(), 41, 2, 0x17, 30];
    for index in 0..30u8 {
        request.extend_from_slice(&[index, 0x01, 0x02, 0x00]);
    }
    request.extend_from_slice(&[41, 2, 0x28, 30, 0x00]);
    for index in 30..60u8 {
        request.extend_from_slice(&[index, 0x00, 0x01, 0x02, 0x00]);
    }

    harness
        .test_request_response(&request, &parameter_error_response(0))
        .await;

    harness.check_events(&[]);
}

#[tokio::test]
async fn direct_operate_whose_response_does_not_fit_is_rejected_without_operating() {
    let mut harness = new_harness(config_with_small_response_buffer());

    harness
        .test_request_response(
            &g41v2_request(0, FunctionCode::DirectOperate, MAX_CONTROLS_IN_RESPONSE + 1),
            &parameter_error_response(0),
        )
        .await;

    harness.check_events(&[]);
}

#[tokio::test]
async fn select_whose_response_does_not_fit_is_rejected_without_selecting() {
    let mut harness = new_harness(config_with_small_response_buffer());

    harness
        .test_request_response(
            &g41v2_request(0, FunctionCode::Select, MAX_CONTROLS_IN_RESPONSE + 1),
            &parameter_error_response(0),
        )
        .await;

    harness.check_events(&[]);
}

#[tokio::test]
async fn operate_without_select_whose_response_does_not_fit_is_rejected() {
    let mut harness = new_harness(config_with_small_response_buffer());

    harness
        .test_request_response(
            &g41v2_request(0, FunctionCode::Operate, MAX_CONTROLS_IN_RESPONSE + 1),
            &parameter_error_response(0),
        )
        .await;

    harness.check_events(&[]);
}

#[tokio::test]
async fn operate_after_a_different_select_whose_response_does_not_fit_is_rejected() {
    let mut harness = new_harness(config_with_small_response_buffer());

    harness
        .test_request_response(SELECT_SEQ0_G41V2, RESPONSE_SEQ0_G41V2_SUCCESS)
        .await;
    harness.check_events(&[
        Event::BeginControls,
        Event::Select(G41V2_INDEX_7),
        Event::EndControls,
    ]);

    harness
        .test_request_response(
            &g41v2_request(1, FunctionCode::Operate, MAX_CONTROLS_IN_RESPONSE + 1),
            &parameter_error_response(1),
        )
        .await;

    harness.check_events(&[]);
}

#[tokio::test]
async fn direct_operate_no_ack_operates_without_a_response_even_if_it_would_not_fit() {
    let mut harness = new_harness(config_with_small_response_buffer());

    harness
        .send_and_process(&g41v2_request(
            0,
            FunctionCode::DirectOperateNoResponse,
            MAX_CONTROLS_IN_RESPONSE + 1,
        ))
        .await;

    harness.check_events(&g41v2_operate_events(
        MAX_CONTROLS_IN_RESPONSE + 1,
        OperateType::DirectOperateNoAck,
    ));
    harness.expect_no_response();
}

// 65,535 g41v2 controls (2-byte count and index), then 11 more (1-byte count and index): more
// controls than a u16 counts
fn request_with_65546_controls(seq: u8, function: FunctionCode) -> Vec<u8> {
    let mut request = vec![0xC0 | seq, function.as_u8(), 41, 2, 0x28];
    request.extend_from_slice(&u16::MAX.to_le_bytes());
    for index in 0..u16::MAX {
        request.extend_from_slice(&index.to_le_bytes());
        request.extend_from_slice(&[0x01, 0x02, 0x00]);
    }
    request.extend_from_slice(&[41, 2, 0x17, 11]);
    for index in 0..11u8 {
        request.extend_from_slice(&[index, 0x01, 0x02, 0x00]);
    }
    request
}

async fn count_operates(max_controls_per_request: Option<u16>) -> usize {
    let mut config = get_default_config();
    config.rx_buffer_size = BufferSize::new(400_000).unwrap();
    config.max_controls_per_request = max_controls_per_request;
    let mut harness = new_harness(config);

    harness
        .send_and_process(&request_with_65546_controls(
            0,
            FunctionCode::DirectOperateNoResponse,
        ))
        .await;

    assert_eq!(harness.next_event().await, Event::BeginControls);
    let mut count = 0;
    loop {
        match harness.next_event().await {
            Event::Operate(_, OperateType::DirectOperateNoAck) => count += 1,
            Event::EndControls => break,
            x => panic!("unexpected event: {x:?}"),
        }
    }
    harness.expect_no_response();
    count
}

#[tokio::test]
async fn control_limit_holds_past_65535_controls_in_a_request() {
    assert_eq!(count_operates(Some(10)).await, 10);
}

#[tokio::test]
async fn control_limit_of_u16_max_holds_past_65535_controls_in_a_request() {
    assert_eq!(count_operates(Some(u16::MAX)).await, usize::from(u16::MAX));
}

#[tokio::test]
async fn all_controls_operate_without_a_limit() {
    assert_eq!(count_operates(None).await, 65546);
}

#[tokio::test]
async fn control_limit_holds_past_65535_controls_in_a_select_or_direct_operate() {
    for function in [FunctionCode::Select, FunctionCode::DirectOperate] {
        let mut config = get_default_config();
        config.rx_buffer_size = BufferSize::new(400_000).unwrap();
        config.solicited_buffer_size = BufferSize::new(400_000).unwrap();
        config.max_controls_per_request = Some(10);
        let mut harness = new_harness(config);

        harness
            .send_and_process(&request_with_65546_controls(0, function))
            .await;
        harness.expect_write().await;

        assert_eq!(harness.next_event().await, Event::BeginControls);
        let mut count = 0;
        loop {
            match harness.next_event().await {
                Event::Select(_) | Event::Operate(_, OperateType::DirectOperate) => count += 1,
                Event::EndControls => break,
                x => panic!("unexpected event: {x:?}"),
            }
        }
        assert_eq!(count, 10, "{function:?}");
    }
}
