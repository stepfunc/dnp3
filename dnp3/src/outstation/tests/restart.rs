use crate::outstation::tests::harness::*;
use crate::outstation::traits::RestartDelay;

const CLEAR_RESTART_IIN: &[u8] = &[0xC0, 0x02, 80, 1, 0x00, 07, 07, 0x00];
const RESPONSE_NO_RESTART_IIN: &[u8] = &[0xC0, 0x81, 0x00, 0x00];
const COLD_RESTART: &[u8] = &[0xC0, 13];
const WARM_RESTART: &[u8] = &[0xC0, 14];
const RESPONSE_NO_FUNCTION_SUPPORT: &[u8] = &[0xC0, 0x81, 0x80, 0x01];
const RESPONSE_TIME_DELAY_FINE: &[u8] =
    &[0xC0, 0x81, 0x80, 0x00, 0x34, 0x02, 0x07, 0x01, 0xFE, 0xCA];
const RESPONSE_TIME_DELAY_COARSE: &[u8] =
    &[0xC0, 0x81, 0x80, 0x00, 0x34, 0x01, 0x07, 0x01, 0xFE, 0xCA];

#[tokio::test]
async fn can_clear_the_restart_iin_bit() {
    let mut harness = new_harness(get_default_config());
    harness
        .test_request_response(CLEAR_RESTART_IIN, RESPONSE_NO_RESTART_IIN)
        .await;
    harness.check_events(&[Event::ClearRestartIIN])
}

#[tokio::test]
async fn handles_cold_restart_when_not_supported() {
    let mut harness = new_harness(get_default_config());
    harness
        .test_request_response(COLD_RESTART, RESPONSE_NO_FUNCTION_SUPPORT)
        .await;
    harness.check_events(&[Event::ColdRestart(None)])
}

#[tokio::test]
async fn handles_cold_restart_when_supported_via_time_delay_fine() {
    let mut harness = new_harness(get_default_config());
    harness.application_data.lock().unwrap().restart_delay =
        Some(RestartDelay::Milliseconds(0xCAFE));
    harness
        .test_request_response(COLD_RESTART, RESPONSE_TIME_DELAY_FINE)
        .await;
    harness.check_events(&[Event::ColdRestart(Some(RestartDelay::Milliseconds(0xCAFE)))])
}

#[tokio::test]
async fn handles_cold_restart_when_supported_via_time_delay_coarse() {
    let mut harness = new_harness(get_default_config());
    harness.application_data.lock().unwrap().restart_delay = Some(RestartDelay::Seconds(0xCAFE));
    harness
        .test_request_response(COLD_RESTART, RESPONSE_TIME_DELAY_COARSE)
        .await;
    harness.check_events(&[Event::ColdRestart(Some(RestartDelay::Seconds(0xCAFE)))])
}

#[tokio::test]
async fn handles_warm_restart_when_not_supported() {
    let mut harness = new_harness(get_default_config());
    harness
        .test_request_response(WARM_RESTART, RESPONSE_NO_FUNCTION_SUPPORT)
        .await;
    harness.check_events(&[Event::WarmRestart(None)])
}

#[tokio::test]
async fn handles_warm_restart_when_supported_via_time_delay_fine() {
    let mut harness = new_harness(get_default_config());
    harness.application_data.lock().unwrap().restart_delay =
        Some(RestartDelay::Milliseconds(0xCAFE));
    harness
        .test_request_response(WARM_RESTART, RESPONSE_TIME_DELAY_FINE)
        .await;
    harness.check_events(&[Event::WarmRestart(Some(RestartDelay::Milliseconds(0xCAFE)))])
}

#[tokio::test]
async fn handles_warm_restart_when_supported_via_time_delay_coarse() {
    let mut harness = new_harness(get_default_config());
    harness.application_data.lock().unwrap().restart_delay = Some(RestartDelay::Seconds(0xCAFE));
    harness
        .test_request_response(WARM_RESTART, RESPONSE_TIME_DELAY_COARSE)
        .await;
    harness.check_events(&[Event::WarmRestart(Some(RestartDelay::Seconds(0xCAFE)))])
}
