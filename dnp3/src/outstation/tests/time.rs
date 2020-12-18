use super::harness::*;

const RESPONSE_TIME_DELAY_FINE_CAFE: &[u8] =
    &[0xC0, 0x81, 0x80, 0x00, 0x34, 0x02, 0x07, 0x01, 0xFE, 0xCA];

#[test]
fn responds_to_delay_measure() {
    let mut harness = new_harness(get_default_config());

    harness.application_data.lock().unwrap().processing_delay = 0xCAFE;

    harness.test_request_response(super::data::DELAY_MEASURE, RESPONSE_TIME_DELAY_FINE_CAFE);
}
