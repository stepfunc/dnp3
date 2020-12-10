use super::harness::*;

const NULL_UNSOL: &[u8] = &[0xF0, 0x82, 0x80, 0x00];

#[test]
fn sends_null_unsolicited_on_startup() {
    let mut harness = new_harness(get_default_unsolicited_config());
    harness.expect_response(NULL_UNSOL);
    harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);
}
