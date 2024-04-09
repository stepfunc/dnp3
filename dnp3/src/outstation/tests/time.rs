use crate::app::Timestamp;

use super::harness::*;

const RESPONSE_TIME_DELAY_FINE_CAFE: &[u8] =
    &[0xC0, 0x81, 0x80, 0x00, 0x34, 0x02, 0x07, 0x01, 0xFE, 0xCA];
const WRITE_ABSOLUTE_TIME: &[u8] = &[
    0xC1, 0x02, 50, 1, 0x07, 1, 0xC0, 0x24, 0x0E, 0xDA, 0x77, 0x01,
];

const RECORD_CURRENT_TIME: &[u8] = &[0xC0, 0x18];
const WRITE_LAST_RECORDED_TIME: &[u8] = &[
    0xC1, 0x02, 50, 3, 0x07, 1, 0xFE, 0xCA, 0x00, 0x00, 0x00, 0x00,
];

const EMPTY_RESPONSE_SEQ0: &[u8] = &[0xC0, 0x81, 0x80, 0x00];
const EMPTY_RESPONSE_SEQ1: &[u8] = &[0xC1, 0x81, 0x80, 0x00];

#[tokio::test]
async fn responds_to_delay_measure() {
    let mut harness = new_harness(get_default_config());

    harness.application_data.lock().unwrap().processing_delay = 0xCAFE;

    harness
        .test_request_response(super::data::DELAY_MEASURE, RESPONSE_TIME_DELAY_FINE_CAFE)
        .await;
}

#[tokio::test]
async fn non_lan_procedure() {
    let mut harness = new_harness(get_default_config());

    harness.application_data.lock().unwrap().processing_delay = 0xCAFE;

    harness
        .test_request_response(super::data::DELAY_MEASURE, RESPONSE_TIME_DELAY_FINE_CAFE)
        .await;
    harness
        .test_request_response(WRITE_ABSOLUTE_TIME, EMPTY_RESPONSE_SEQ1)
        .await;

    harness.check_events(&[Event::WriteAbsoluteTime(Timestamp::new(1614271096000))]);
}

#[tokio::test(start_paused = true)]
async fn lan_procedure() {
    let mut harness = new_harness(get_default_config());

    harness.application_data.lock().unwrap().processing_delay = 0xCAFE;

    harness
        .test_request_response(RECORD_CURRENT_TIME, EMPTY_RESPONSE_SEQ0)
        .await;
    harness
        .test_request_response(WRITE_LAST_RECORDED_TIME, EMPTY_RESPONSE_SEQ1)
        .await;

    harness.check_events(&[Event::WriteAbsoluteTime(Timestamp::new(0xCAFE))]);
}
