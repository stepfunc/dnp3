// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Each timer runs for its own configured duration: the confirm timeout, the select timeout, the
//! unsolicited retry delay, and no keep-alive at all when it is disabled

use std::time::Duration;

use tokio::time::Instant;

use crate::app::{FunctionCode, Timeout};
use crate::outstation::config::OutstationConfig;
use crate::outstation::traits::OperateType;
use crate::util::test_support::watchdog;

use super::harness::*;

const CONFIRM_TIMEOUT: Duration = Duration::from_secs(2);
const SELECT_TIMEOUT: Duration = Duration::from_secs(7);
const RETRY_DELAY: Duration = Duration::from_secs(11);

/// the configuration with a different duration for each timer
fn with_distinct_timeouts(mut config: OutstationConfig) -> OutstationConfig {
    config.confirm_timeout = Timeout::from_secs(CONFIRM_TIMEOUT.as_secs()).unwrap();
    config.select_timeout = Timeout::from_secs(SELECT_TIMEOUT.as_secs()).unwrap();
    config.unsolicited_retry_delay = RETRY_DELAY;
    config
}

/// the class 1 event of binary input 0 in a solicited response
const G2V1_RESPONSE_SEQ0: &[u8] = &[
    0xE0, 0x81, 0x80, 0x00, 0x02, 0x01, 0x28, 0x01, 0x00, 0x00, 0x00, 0x81,
];

#[tokio::test(start_paused = true)]
async fn solicited_confirm_wait_lasts_the_confirm_timeout() {
    watchdog(async {
        let mut harness = new_harness(with_distinct_timeouts(get_default_config()));
        harness.handle.database.transaction(add_binary_event);

        harness
            .test_request_response(&read_class_1(0), G2V1_RESPONSE_SEQ0)
            .await;
        let sent = Instant::now();
        harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);

        harness
            .wait_for_events(&[Event::SolicitedConfirmTimeout(0)])
            .await;
        assert_eq!(sent.elapsed(), CONFIRM_TIMEOUT);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn unsolicited_retries_follow_the_confirm_timeout_and_the_next_series_the_retry_delay() {
    watchdog(async {
        let mut config = with_distinct_timeouts(get_default_unsolicited_config());
        config.max_unsolicited_retries = Some(1);
        let mut harness = new_harness(config);
        confirm_null_unsolicited(&mut harness).await;
        enable_unsolicited(&mut harness).await;

        harness.handle.database.transaction(add_binary_event);
        let response = unsolicited(1, 0x80, &g2v1_events(0..1));
        harness.expect_response(&response).await;
        let sent = Instant::now();
        harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);

        // one retry after the confirm timeout
        harness
            .wait_for_events(&[Event::UnsolicitedConfirmTimeout(1, true)])
            .await;
        assert_eq!(sent.elapsed(), CONFIRM_TIMEOUT);
        harness.expect_response(&response).await;
        assert_eq!(sent.elapsed(), CONFIRM_TIMEOUT);

        // the series ends one confirm timeout later
        harness
            .wait_for_events(&[Event::UnsolicitedConfirmTimeout(1, false)])
            .await;
        assert_eq!(sent.elapsed(), 2 * CONFIRM_TIMEOUT);

        // and the next series starts after the retry delay
        harness
            .expect_response(&unsolicited(2, 0x80, &g2v1_events(0..1)))
            .await;
        assert_eq!(sent.elapsed(), 2 * CONFIRM_TIMEOUT + RETRY_DELAY);
        harness.check_events(&[Event::EnterUnsolicitedConfirmWait(2)]);
    })
    .await;
}

/// the response to a g41v2 control of index 7 with `status`
fn g41v2_response(seq: u8, status: u8) -> Vec<u8> {
    vec![
        0xC0 | seq,
        0x81,
        0x80,
        0x00,
        41,
        2,
        0x17,
        0x01,
        7,
        0x01,
        0x02,
        status,
    ]
}

const SUCCESS: u8 = 0;
const TIMEOUT: u8 = 1;

#[tokio::test(start_paused = true)]
async fn operate_is_accepted_at_the_select_timeout_and_rejected_one_millisecond_later() {
    watchdog(async {
        let mut harness = new_harness(with_distinct_timeouts(get_default_config()));

        harness
            .test_request_response(
                &g41v2_control(FunctionCode::Select, 0, 7),
                &g41v2_response(0, SUCCESS),
            )
            .await;
        harness.check_events(&selected());
        tokio::time::advance(SELECT_TIMEOUT).await;
        harness
            .test_request_response(
                &g41v2_control(FunctionCode::Operate, 1, 7),
                &g41v2_response(1, SUCCESS),
            )
            .await;
        harness.check_events(&operated(OperateType::SelectBeforeOperate));

        harness
            .test_request_response(
                &g41v2_control(FunctionCode::Select, 2, 7),
                &g41v2_response(2, SUCCESS),
            )
            .await;
        harness.check_events(&selected());
        tokio::time::advance(SELECT_TIMEOUT + Duration::from_millis(1)).await;
        harness
            .test_request_response(
                &g41v2_control(FunctionCode::Operate, 3, 7),
                &g41v2_response(3, TIMEOUT),
            )
            .await;
        harness.check_no_events();
    })
    .await;
}

/// long enough for any timer the outstation could have set to expire many times over
const LONG_IDLE: Duration = Duration::from_secs(4 * 60 * 60);

fn without_keep_alive(mut config: OutstationConfig) -> OutstationConfig {
    config.keep_alive_timeout = None;
    config
}

#[tokio::test(start_paused = true)]
async fn outstation_without_keep_alive_sends_nothing_while_idle_and_still_answers() {
    watchdog(async {
        let mut harness = new_harness(without_keep_alive(get_default_config()));

        // the clock only gets here once nothing is left to run before it
        tokio::time::sleep(LONG_IDLE).await;
        harness.expect_no_response();
        harness.check_no_events();

        harness
            .test_request_response(&read_class_0(0), &[0xC0, 0x81, 0x80, 0x00])
            .await;
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn outstation_without_keep_alive_sends_an_unsolicited_response_after_a_long_idle() {
    watchdog(async {
        let mut harness = new_harness(without_keep_alive(get_default_unsolicited_config()));
        confirm_null_unsolicited(&mut harness).await;
        enable_unsolicited(&mut harness).await;

        tokio::time::sleep(LONG_IDLE).await;
        harness.expect_no_response();
        harness.check_no_events();

        harness.handle.database.transaction(add_binary_event);
        harness
            .expect_response(&unsolicited(1, 0x80, &g2v1_events(0..1)))
            .await;
        harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);
    })
    .await;
}
