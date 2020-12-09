const DELAY_MEASURE: &[u8] = &[0xC0, 23];
const RESPONSE_TIME_DELAY_FINE: &[u8] =
    &[0xC0, 0x81, 0x80, 0x00, 0x34, 0x02, 0x07, 0x01, 0xFE, 0xCA];

#[test]
fn rejects_operate_with_non_consecutive_sequence() {
    let mut harness = super::harness::new_harness();

    harness.application_data.lock().unwrap().processing_delay = 0xCAFE;

    harness.test_request_response(DELAY_MEASURE, RESPONSE_TIME_DELAY_FINE);
}
