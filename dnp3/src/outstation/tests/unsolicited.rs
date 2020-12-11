use super::harness::*;
use crate::outstation::config::OutstationConfig;

const NULL_UNSOL: &[u8] = &[0xF0, 0x82, 0x80, 0x00];

fn config_with_limited_retries(retries: usize) -> OutstationConfig {
    let mut config = get_default_unsolicited_config();
    config.max_unsolicited_retries = Some(retries);
    config
}

#[test]
fn null_unsolicited_can_time_out_and_retry() {
    let mut harness = new_harness(get_default_unsolicited_config());
    harness.expect_response(NULL_UNSOL);
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(0)]);

    for _ in 0..3 {
        crate::tokio::time::advance(OutstationConfig::DEFAULT_CONFIRM_TIMEOUT);
        harness.expect_response(NULL_UNSOL);
        harness.check_events(&[Event::UnsolicitedConfirmTimeout(0, true)]);
    }
}

#[test]
fn null_unsolicited_can_timeout_and_not_retry() {
    let mut harness = new_harness(config_with_limited_retries(1));
    harness.expect_response(NULL_UNSOL);
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(0)]);

    // a single retry
    crate::tokio::time::advance(OutstationConfig::DEFAULT_CONFIRM_TIMEOUT);
    harness.expect_response(NULL_UNSOL);
    harness.check_events(&[Event::UnsolicitedConfirmTimeout(0, true)]);

    crate::tokio::time::advance(OutstationConfig::DEFAULT_CONFIRM_TIMEOUT);
    harness.poll_pending();
    harness.check_events(&[Event::UnsolicitedConfirmTimeout(0, false)]);
    harness.check_all_io_consumed();
}
