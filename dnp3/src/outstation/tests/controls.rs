// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "tests count the controls each request operates"
)]

use tokio::time::Duration;

use crate::app::variations::{Group41Var2, Group41Var3, Group41Var4};
use crate::app::{BufferSize, FunctionCode};
use crate::link::header::BroadcastConfirmMode;
use crate::outstation::config::Feature;
use crate::outstation::tests::harness::*;
use crate::outstation::traits::{BroadcastAction, OperateType};

// select, seq == 0, g41v2 - count == 1, index == 7, value = 513, status == SUCCESS,
const SELECT_SEQ0_G41V2: &[u8] = &[0xC0, 0x03, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00];
// operate, seq == 1, g41v2 - count == 1, index == 7, value = 513, status == SUCCESS
const OPERATE_SEQ1_G41V2: &[u8] = &[0xC1, 0x04, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00];
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
// response, seq == 2, restart IIN + echo of request headers but with status == NO_SELECT
const RESPONSE_SEQ2_G41V2_NO_SELECT: &[u8] = &[
    0xC2, 0x81, 0x80, 0x00, 41, 2, 0x17, 0x1, 0x07, 0x01, 0x02, 0x02,
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

    harness.check_events(&operated(OperateType::DirectOperateNoAck))
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
async fn rejects_operate_with_non_consecutive_sequence() {
    let mut harness = new_harness(get_default_config());

    // ------------ select -------------
    harness
        .test_request_response(SELECT_SEQ0_G41V2, RESPONSE_SEQ0_G41V2_SUCCESS)
        .await;

    harness.check_events(&selected());

    // ------------ operate -------------

    harness
        .test_request_response(OPERATE_SEQ2_G41V2, RESPONSE_SEQ2_G41V2_NO_SELECT)
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

    harness.check_events(&selected());

    tokio::time::pause();
    tokio::time::advance(
        Duration::from(get_default_config().select_timeout) + Duration::from_millis(1),
    )
    .await;

    // ------------ operate -------------

    harness
        .test_request_response(OPERATE_SEQ1_G41V2, RESPONSE_SEQ1_G41V2_SELECT_TIMEOUT)
        .await;

    harness.check_no_events();
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
async fn control_request_with_a_non_control_object_gets_object_unknown() {
    let mut harness = new_harness(get_default_config());

    // g41v2 at index 7, then g2v1 at index 0 with flags ONLINE
    let mut request = DIRECT_OPERATE_SEQ0_G41V2.to_vec();
    request.extend_from_slice(&[2, 1, 0x17, 0x01, 0x00, 0x01]);

    // restart IIN and IIN2.1 OBJECT_UNKNOWN, and no objects
    harness
        .test_request_response(&request, &[0xC0, 0x81, 0x80, 0x02])
        .await;

    harness.check_no_events();
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

    harness.check_no_events();
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

    harness.check_no_events();
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

    harness.check_no_events();
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

    harness.check_no_events();
}

#[tokio::test]
async fn operate_after_a_different_select_whose_response_does_not_fit_is_rejected() {
    let mut harness = new_harness(config_with_small_response_buffer());

    harness
        .test_request_response(SELECT_SEQ0_G41V2, RESPONSE_SEQ0_G41V2_SUCCESS)
        .await;
    harness.check_events(&selected());

    harness
        .test_request_response(
            &g41v2_request(1, FunctionCode::Operate, MAX_CONTROLS_IN_RESPONSE + 1),
            &parameter_error_response(1),
        )
        .await;

    harness.check_no_events();
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

// Which requests end or keep a selection is in select_operate_model.rs, apart from a broadcast,
// which that model doesn't send

#[tokio::test]
async fn broadcast_direct_operate_no_ack_ends_the_selection() {
    let mut harness = new_harness(get_default_config());
    harness
        .test_request_response(SELECT_SEQ0_G41V2, RESPONSE_SEQ0_G41V2_SUCCESS)
        .await;
    harness.check_events(&selected());
    harness.set_broadcast(Some(BroadcastConfirmMode::Optional));
    harness
        .send_and_process(&g41v2_request(5, FunctionCode::DirectOperateNoResponse, 1))
        .await;
    harness.set_broadcast(None);
    harness.expect_no_response();
    harness.check_events(&g41v2_operate_events(1, OperateType::DirectOperateNoAck));
    harness.check_events(&[Event::BroadcastReceived(
        FunctionCode::DirectOperateNoResponse,
        BroadcastAction::Processed,
    )]);
    // restart IIN and IIN1.0 BROADCAST + echo with status == NO_SELECT
    harness
        .test_request_response(
            OPERATE_SEQ1_G41V2,
            &[
                0xC1, 0x81, 0x81, 0x00, 41, 2, 0x17, 0x1, 0x07, 0x01, 0x02, 0x02,
            ],
        )
        .await;
    harness.check_no_events();
}

// select, seq == 0, g12v1 count == 1, index == 7, status == SUCCESS. Bytes 8 to 15 (the CROB's
// count, on time and most of its off time) were solved by inverting xxh64 so that the whole request
// hashes to 0, which the outstation must treat like any other request hash
const SELECT_SEQ0_G12V1_HASH_ZERO: &[u8] = &[
    0xC0, 0x03, 12, 1, 0x17, 0x01, 0x07, 0x01, 0x15, 0x6E, 0x9A, 0xA5, 0x7B, 0xA4, 0xFE, 0xCB,
    0x00, 0x00,
];
const OPERATE_SEQ1_G12V1: &[u8] = &[
    0xC1, 0x04, 12, 1, 0x17, 0x01, 0x07, 0x01, 0x15, 0x6E, 0x9A, 0xA5, 0x7B, 0xA4, 0xFE, 0xCB,
    0x00, 0x00,
];

fn g12v1_index_7() -> Control {
    Control::G12V1(
        crate::app::variations::Group12Var1 {
            code: crate::app::control::ControlCode::from(0x01),
            count: 0x15,
            on_time: 0x7BA5_9A6E,
            off_time: 0x00CB_FEA4,
            status: crate::app::control::CommandStatus::Success,
        },
        7,
    )
}

fn g12v1_response(seq: u8) -> Vec<u8> {
    let mut response = vec![0xC0 | seq, 0x81, 0x80, 0x00];
    response.extend_from_slice(&SELECT_SEQ0_G12V1_HASH_ZERO[2..]);
    response
}

#[tokio::test]
async fn select_whose_request_hashes_to_zero_can_be_retried_and_operated() {
    assert_eq!(xxhash_rust::xxh64::xxh64(SELECT_SEQ0_G12V1_HASH_ZERO, 0), 0);

    let mut harness = new_harness(get_default_config());
    harness
        .test_request_response(SELECT_SEQ0_G12V1_HASH_ZERO, &g12v1_response(0))
        .await;
    harness.check_events(&[
        Event::BeginControls,
        Event::Select(g12v1_index_7()),
        Event::EndControls,
    ]);

    // the retry repeats the response and keeps the selection
    harness
        .test_request_response(SELECT_SEQ0_G12V1_HASH_ZERO, &g12v1_response(0))
        .await;
    harness.check_no_events();

    harness
        .test_request_response(OPERATE_SEQ1_G12V1, &g12v1_response(1))
        .await;
    harness.check_events(&[
        Event::BeginControls,
        Event::Operate(g12v1_index_7(), OperateType::SelectBeforeOperate),
        Event::EndControls,
    ]);
}

const NOT_SUPPORTED: u8 = 4;
const NO_SELECT: u8 = 2;

const NON_FINITE: [(f32, f64); 3] = [
    (f32::NAN, f64::NAN),
    (f32::INFINITY, f64::INFINITY),
    (f32::NEG_INFINITY, f64::NEG_INFINITY),
];

/// g41v2 513 at index 7, then g41v3 `f32` at index 8 and g41v4 `f64` at index 9, with
/// `statuses` in that order
fn analog_controls(values: (f32, f64), statuses: [u8; 3]) -> Vec<u8> {
    let (single, double) = values;
    let [g41v2, g41v3, g41v4] = statuses;
    let mut objects = vec![41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, g41v2];
    objects.extend([41, 3, 0x17, 0x01, 0x08]);
    objects.extend(single.to_le_bytes());
    objects.push(g41v3);
    objects.extend([41, 4, 0x17, 0x01, 0x09]);
    objects.extend(double.to_le_bytes());
    objects.push(g41v4);
    objects
}

fn analog_control_request(seq: u8, function: FunctionCode, values: (f32, f64)) -> Vec<u8> {
    [
        &[0xC0 | seq, function.as_u8()][..],
        &analog_controls(values, [0, 0, 0]),
    ]
    .concat()
}

fn analog_control_response(seq: u8, iin2: u8, values: (f32, f64), statuses: [u8; 3]) -> Vec<u8> {
    [
        &[0xC0 | seq, 0x81, 0x80, iin2][..],
        &analog_controls(values, statuses),
    ]
    .concat()
}

#[tokio::test]
async fn direct_operate_answers_non_finite_analog_values_with_not_supported() {
    for values in NON_FINITE {
        let mut harness = new_harness(get_default_config());

        // IIN2.2 because the first failure is NOT_SUPPORTED
        harness
            .test_request_response(
                &analog_control_request(0, FunctionCode::DirectOperate, values),
                &analog_control_response(0, 0x04, values, [0, NOT_SUPPORTED, NOT_SUPPORTED]),
            )
            .await;

        harness.check_events(&operated(OperateType::DirectOperate));
        harness.check_no_events();
    }
}

#[tokio::test]
async fn select_answers_non_finite_analog_values_with_not_supported() {
    for values in NON_FINITE {
        let mut harness = new_harness(get_default_config());

        harness
            .test_request_response(
                &analog_control_request(0, FunctionCode::Select, values),
                &analog_control_response(0, 0x04, values, [0, NOT_SUPPORTED, NOT_SUPPORTED]),
            )
            .await;

        harness.check_events(&selected());
        harness.check_no_events();

        // the failed SELECT selects nothing, so the OPERATE can't reach the handler either
        harness
            .test_request_response(
                &analog_control_request(1, FunctionCode::Operate, values),
                &analog_control_response(1, 0x00, values, [NO_SELECT; 3]),
            )
            .await;
        harness.check_no_events();
    }
}

#[tokio::test]
async fn direct_operate_no_ack_does_not_pass_non_finite_analog_values_to_the_handler() {
    for values in NON_FINITE {
        let mut harness = new_harness(get_default_config());

        harness
            .send_and_process(&analog_control_request(
                0,
                FunctionCode::DirectOperateNoResponse,
                values,
            ))
            .await;

        harness.check_events(&operated(OperateType::DirectOperateNoAck));
        harness.check_no_events();
    }
}

#[tokio::test]
async fn direct_operate_passes_the_largest_finite_analog_values_to_the_handler() {
    let values = (f32::MAX, f64::MAX);
    let mut harness = new_harness(get_default_config());

    harness
        .test_request_response(
            &analog_control_request(0, FunctionCode::DirectOperate, values),
            &analog_control_response(0, 0x00, values, [0, 0, 0]),
        )
        .await;

    harness.check_events(&[
        Event::BeginControls,
        Event::Operate(G41V2_INDEX_7, OperateType::DirectOperate),
        Event::Operate(
            Control::G41V3(Group41Var3::new(f32::MAX), 8),
            OperateType::DirectOperate,
        ),
        Event::Operate(
            Control::G41V4(Group41Var4::new(f64::MAX), 9),
            OperateType::DirectOperate,
        ),
        Event::EndControls,
    ]);
}

const TOO_MANY_OPS: u8 = 8;

#[tokio::test]
async fn control_limit_applies_before_the_analog_value_check() {
    // the limit, the statuses of g41v2, g41v3 and g41v4, and the IIN2 of the response
    let cases = [
        // the non-finite g41v3 is within the limit, the g41v4 isn't
        (2, [0, NOT_SUPPORTED, TOO_MANY_OPS], 0x04),
        // both are past the limit, so the first failure is TOO_MANY_OPS, without IIN2.2
        (1, [0, TOO_MANY_OPS, TOO_MANY_OPS], 0x00),
    ];

    for (limit, statuses, iin2) in cases {
        for values in NON_FINITE {
            for function in [FunctionCode::Select, FunctionCode::DirectOperate] {
                let mut config = get_default_config();
                config.max_controls_per_request = Some(limit);
                let mut harness = new_harness(config);

                harness
                    .test_request_response(
                        &analog_control_request(0, function, values),
                        &analog_control_response(0, iin2, values, statuses),
                    )
                    .await;

                let control = match function {
                    FunctionCode::Select => Event::Select(G41V2_INDEX_7),
                    _ => Event::Operate(G41V2_INDEX_7, OperateType::DirectOperate),
                };
                harness.check_events(&[Event::BeginControls, control, Event::EndControls]);
                harness.check_no_events();
            }
        }
    }
}
