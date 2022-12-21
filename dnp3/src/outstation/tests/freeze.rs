use crate::app::Timestamp;
use crate::outstation::tests::harness::*;
use crate::outstation::{FreezeIndices, FreezeInterval, FreezeType};

const EMPTY_RESPONSE: &[u8] = &[0xC0, 0x81, 0x80, 0x00];
const EMPTY_RESPONSE_PARAM_ERROR: &[u8] = &[0xC0, 0x81, 0x80, 0x04];
const EMPTY_RESPONSE_NO_FUNC_SUPPORTED: &[u8] = &[0xC0, 0x81, 0x80, 0x01];

#[tokio::test]
async fn immediate_freeze_all_counters() {
    let mut harness = new_harness(get_default_config());

    harness
        .test_request_response(&[0xC0, 0x07, 20, 0, 0x06], EMPTY_RESPONSE)
        .await;

    harness.check_events(&[Event::Freeze(
        FreezeIndices::All,
        FreezeType::ImmediateFreeze,
    )]);
}

#[tokio::test]
async fn immediate_freeze_range_of_counters() {
    let mut harness = new_harness(get_default_config());

    harness
        .test_request_response(&[0xC0, 0x07, 20, 0, 0x00, 0, 10], EMPTY_RESPONSE)
        .await;

    harness.check_events(&[Event::Freeze(
        FreezeIndices::Range(0, 10),
        FreezeType::ImmediateFreeze,
    )]);
}

#[tokio::test]
async fn immediate_freeze_no_response_all_counters() {
    let mut harness = new_harness(get_default_config());

    harness.send_and_process(&[0xC0, 0x08, 20, 0, 0x06]).await;

    harness.check_events(&[Event::Freeze(
        FreezeIndices::All,
        FreezeType::ImmediateFreeze,
    )]);
}

#[tokio::test]
async fn immediate_freeze_no_response_range_of_counters() {
    let mut harness = new_harness(get_default_config());

    harness
        .send_and_process(&[0xC0, 0x08, 20, 0, 0x00, 0, 10])
        .await;

    harness.check_events(&[Event::Freeze(
        FreezeIndices::Range(0, 10),
        FreezeType::ImmediateFreeze,
    )]);
}

#[tokio::test]
async fn freeze_and_clear_all_counters() {
    let mut harness = new_harness(get_default_config());

    harness
        .test_request_response(&[0xC0, 0x09, 20, 0, 0x06], EMPTY_RESPONSE)
        .await;

    harness.check_events(&[Event::Freeze(
        FreezeIndices::All,
        FreezeType::FreezeAndClear,
    )]);
}

#[tokio::test]
async fn freeze_and_clear_range_of_counters() {
    let mut harness = new_harness(get_default_config());

    harness
        .test_request_response(&[0xC0, 0x09, 20, 0, 0x00, 0, 10], EMPTY_RESPONSE)
        .await;

    harness.check_events(&[Event::Freeze(
        FreezeIndices::Range(0, 10),
        FreezeType::FreezeAndClear,
    )]);
}

#[tokio::test]
async fn freeze_and_clear_no_response_all_counters() {
    let mut harness = new_harness(get_default_config());

    harness.send_and_process(&[0xC0, 0x0A, 20, 0, 0x06]).await;

    harness.check_events(&[Event::Freeze(
        FreezeIndices::All,
        FreezeType::FreezeAndClear,
    )]);
}

#[tokio::test]
async fn freeze_and_clear_no_response_range_of_counters() {
    let mut harness = new_harness(get_default_config());

    harness
        .send_and_process(&[0xC0, 0x0A, 20, 0, 0x00, 0, 10])
        .await;

    harness.check_events(&[Event::Freeze(
        FreezeIndices::Range(0, 10),
        FreezeType::FreezeAndClear,
    )]);
}

#[tokio::test]
async fn freeze_invalid_object() {
    let mut harness = new_harness(get_default_config());

    harness
        .test_request_response(
            &[0xC0, 0x07, 22, 0, 0x06, 20, 0, 0x06],
            EMPTY_RESPONSE_NO_FUNC_SUPPORTED,
        )
        .await;

    harness.check_events(&[Event::Freeze(
        FreezeIndices::All,
        FreezeType::ImmediateFreeze,
    )]);
}

#[tokio::test]
async fn freeze_at_time_without_g50v1() {
    let mut harness = new_harness(get_default_config());

    harness
        .test_request_response(&[0xC0, 0x0B, 20, 0, 0x06], EMPTY_RESPONSE_PARAM_ERROR)
        .await;

    harness.check_no_events();
}

#[tokio::test]
async fn freeze_at_time() {
    let mut harness = new_harness(get_default_config());

    harness
        .test_request_response(
            &[
                0xC0, 0x0B, 0x32, 0x02, 0x07, 0x01, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0xAA, 0xBB,
                0xCC, 0xDD, 20, 0, 0x06,
            ],
            EMPTY_RESPONSE,
        )
        .await;

    harness.check_events(&[Event::Freeze(
        FreezeIndices::All,
        FreezeType::FreezeAtTime(FreezeInterval::PeriodicallyFreeze(
            Timestamp::new(0xFF),
            0xDDCCBBAA,
        )),
    )]);
}

#[tokio::test]
async fn freeze_at_time_no_ack() {
    let mut harness = new_harness(get_default_config());

    harness
        .send_and_process(&[
            0xC0, 0x0C, 0x32, 0x02, 0x07, 0x01, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0xAA, 0xBB,
            0xCC, 0xDD, 20, 0, 0x06,
        ])
        .await;

    harness.check_events(&[Event::Freeze(
        FreezeIndices::All,
        FreezeType::FreezeAtTime(FreezeInterval::PeriodicallyFreeze(
            Timestamp::new(0xFF),
            0xDDCCBBAA,
        )),
    )]);

    assert_eq!(harness.io.pop_event(), None);
}
