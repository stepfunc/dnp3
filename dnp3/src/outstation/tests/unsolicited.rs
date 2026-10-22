use crate::app::gen::prefixed::PrefixedVariation;
use crate::app::measurement::*;
use crate::app::parse::options::ParseOptions;
use crate::app::parse::parser::{ObjectHeader, ParsedFragment};
use crate::app::{BufferSize, FunctionCode, Timestamp};
use crate::link::header::BroadcastConfirmMode;
use crate::outstation::config::OutstationConfig;
use crate::outstation::database::*;
use crate::outstation::traits::{BroadcastAction, OperateType};
use crate::util::test_support::watchdog;

use std::time::Duration;
use tokio::time::Instant;

use super::harness::*;

const UNSOL_G2V1_SEQ1: &[u8] = &[
    0xF1, 0x82, 0x80, 0x00, 0x02, 0x01, 0x28, 0x01, 0x00, 0x00, 0x00, 0x81,
];
const UNSOL_G2V1_SEQ2: &[u8] = &[
    0xF2, 0x82, 0x80, 0x00, 0x02, 0x01, 0x28, 0x01, 0x00, 0x00, 0x00, 0x81,
];
const DISABLE_UNSOLICITED_SEQ0: &[u8] = &[
    0xC0, 0x15, 0x3C, 0x02, 0x06, 0x3C, 0x03, 0x06, 0x3C, 0x04, 0x06,
];

const EMPTY_RESPONSE_SEQ0: &[u8] = &[0xC0, 0x81, 0x80, 0x00];
const CLASS_0_RESPONSE_SEQ0: &[u8] = &[
    0xC0, 0x81, 0x80, 0x00, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01,
];
const CLASS_0_RESPONSE_SEQ0_WITH_PENDING_EVENTS: &[u8] = &[
    0xC0, 0x81, 0x82, 0x00, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01,
];

fn config_with_limited_retries(retries: usize) -> OutstationConfig {
    let mut config = get_default_unsolicited_config();
    config.max_unsolicited_retries = Some(retries);
    config
}

#[tokio::test]
async fn null_unsolicited_always_retries() {
    let mut harness = new_harness(get_default_unsolicited_config());
    harness.expect_response(&null_unsolicited(0)).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(0)]);

    tokio::time::pause(); //auto advance timer

    harness
        .wait_for_events(&[Event::UnsolicitedConfirmTimeout(0, false)])
        .await;
    harness.expect_response(&null_unsolicited(1)).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);
}

#[tokio::test]
async fn unsolicited_can_time_out_and_retry() {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;
    harness.handle.database.transaction(add_binary_event);

    // first response
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);

    // this would go on forever, but let's just test 3 iterations
    for _ in 0..3 {
        tokio::time::pause();
        harness.expect_response(UNSOL_G2V1_SEQ1).await;
        harness.check_events(&[Event::UnsolicitedConfirmTimeout(1, true)]);
        tokio::time::resume();
    }
}

#[tokio::test]
async fn unsolicited_can_timeout_and_not_retry() {
    let mut harness = new_harness(config_with_limited_retries(2));
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;
    harness.handle.database.transaction(add_binary_event);

    // response
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);

    // first retry
    tokio::time::pause();
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::UnsolicitedConfirmTimeout(1, true)]);
    tokio::time::resume();

    // second retry
    tokio::time::pause();
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::UnsolicitedConfirmTimeout(1, true)]);
    tokio::time::resume();

    // timeout
    tokio::time::pause();
    harness
        .wait_for_events(&[Event::UnsolicitedConfirmTimeout(1, false)])
        .await;
    tokio::time::resume();

    // new series
    tokio::time::pause();
    harness.expect_response(UNSOL_G2V1_SEQ2).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(2)]);
}

#[tokio::test]
async fn unsolicited_can_timeout_series_wait_and_start_another_series() {
    let mut harness = new_harness(config_with_limited_retries(0));
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;
    harness.handle.database.transaction(add_binary_event);

    // first response series
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);

    tokio::time::pause();
    harness
        .wait_for_events(&[Event::UnsolicitedConfirmTimeout(1, false)])
        .await;

    harness.expect_response(UNSOL_G2V1_SEQ2).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(2)]);
}

#[tokio::test]
async fn data_unsolicited_can_be_confirmed() {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;

    harness.handle.database.transaction(add_binary_event);
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);
    harness.send_and_process(&unsolicited_confirm(1)).await;
    harness.check_events(&[Event::UnsolicitedConfirmReceived(1)]);
}

#[tokio::test]
async fn defers_read_during_unsol_confirm_wait() {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;

    harness.handle.database.transaction(add_binary_event);
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);
    // send a read which will be deferred
    harness.send_and_process(&read_class_0(0)).await;
    harness.check_no_events();
    // now send the confirm
    harness.send_and_process(&unsolicited_confirm(1)).await;
    harness.check_events(&[Event::UnsolicitedConfirmReceived(1)]);
    harness.expect_response(CLASS_0_RESPONSE_SEQ0).await;
    harness.check_events(&[
        Event::BeginConfirm,
        Event::Cleared(0),
        Event::EndConfirm(empty_buffer_state()),
    ]);
    harness.check_no_events();
}

#[tokio::test]
async fn defers_read_during_unsol_confirm_wait_timeout() {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;

    harness.handle.database.transaction(add_binary_event);
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);
    // send a read which will be deferred
    harness.send_and_process(&read_class_0(0)).await;
    harness.check_no_events();

    // expire the unsolicited response
    tokio::time::pause();
    harness
        .wait_for_events(&[Event::UnsolicitedConfirmTimeout(1, false)])
        .await;
    harness
        .expect_response(CLASS_0_RESPONSE_SEQ0_WITH_PENDING_EVENTS)
        .await;
    harness.check_no_events();
}

#[tokio::test]
async fn deferred_read_of_type_without_points_sets_object_unknown() {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;

    harness.handle.database.transaction(add_binary_event);
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);
    // send a read of g3v0, which has no points, that will be deferred
    harness
        .send_and_process(&[0xC0, 0x01, 0x03, 0x00, 0x06])
        .await;
    harness.check_no_events();

    // expire the unsolicited response
    tokio::time::pause();
    harness
        .wait_for_events(&[Event::UnsolicitedConfirmTimeout(1, false)])
        .await;
    // the class 1 event is still pending, and IIN2.1 (OBJECT_UNKNOWN)
    harness.expect_response(&[0xC0, 0x81, 0x82, 0x02]).await;
    harness.check_no_events();
}

// IIN1.1 for the pending class 1 event
const CLASS_0_RESPONSE_SEQ1_WITH_PENDING_EVENTS: &[u8] = &[
    0xC1, 0x81, 0x82, 0x00, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01,
];
const G2V1_RESPONSE_SEQ1: &[u8] = &[
    0xE1, 0x81, 0x80, 0x00, 0x02, 0x01, 0x28, 0x01, 0x00, 0x00, 0x00, 0x81,
];
const EMPTY_RESPONSE_SEQ2: &[u8] = &[0xC2, 0x81, 0x80, 0x00];

/// a single unsolicited response that isn't retried, with a retry delay that outlasts the test
fn config_with_one_unsolicited_attempt() -> OutstationConfig {
    let mut config = config_with_limited_retries(0);
    config.unsolicited_retry_delay = std::time::Duration::from_secs(3600);
    config
}

/// send the class 1 event in an unsolicited response that is never confirmed
async fn time_out_unsolicited_event(harness: &mut OutstationHarness) {
    confirm_null_unsolicited(harness).await;
    enable_unsolicited(harness).await;

    harness.handle.database.transaction(add_binary_event);
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);

    tokio::time::pause();
    harness
        .wait_for_events(&[Event::UnsolicitedConfirmTimeout(1, false)])
        .await;
    tokio::time::resume();
}

#[tokio::test]
async fn read_during_unsolicited_retry_delay_reports_unconfirmed_events() {
    let mut harness = new_harness(config_with_one_unsolicited_attempt());
    time_out_unsolicited_event(&mut harness).await;

    // the event is still pending
    harness
        .test_request_response(
            &delay_measure(0),
            &[0xC0, 0x81, 0x82, 0x00, 0x34, 0x02, 0x07, 0x01, 0x00, 0x00],
        )
        .await;
    harness
        .test_request_response(&read_class_0(1), CLASS_0_RESPONSE_SEQ1_WITH_PENDING_EVENTS)
        .await;
    // and a class 1 READ reports it again
    harness
        .test_request_response(&read_class_1(1), G2V1_RESPONSE_SEQ1)
        .await;
    harness.check_events(&[Event::EnterSolicitedConfirmWait(1)]);
}

#[tokio::test]
async fn read_after_disable_unsolicited_during_confirm_wait_reports_unconfirmed_events() {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;

    harness.handle.database.transaction(add_binary_event);
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);

    harness
        .test_request_response(DISABLE_UNSOLICITED_SEQ0, EMPTY_RESPONSE_SEQ0)
        .await;

    harness
        .test_request_response(&read_class_1(1), G2V1_RESPONSE_SEQ1)
        .await;
    harness.check_events(&[Event::EnterSolicitedConfirmWait(1)]);

    // the event is cleared once the master confirms the response that delivered it
    harness.send_and_process(&confirm(1)).await;
    harness.check_events(&[
        Event::SolicitedConfirmReceived(1),
        Event::BeginConfirm,
        Event::Cleared(0),
        Event::EndConfirm(empty_buffer_state()),
    ]);
    harness
        .test_request_response(&read_class_1(2), EMPTY_RESPONSE_SEQ2)
        .await;
    harness.check_no_events();
}

fn update_binary_event(handle: &mut DatabaseHandle, value: bool) {
    handle.transaction(|db| {
        db.update(
            0,
            &BinaryInput::new(value, Flags::ONLINE, Time::synchronized(0)),
            UpdateOptions::default(),
        )
    });
}

/// a class 1 READ reports both class 1 events, and its confirm clears them
async fn read_and_confirm_two_class_1_events(harness: &mut OutstationHarness) {
    harness
        .test_request_response(
            &read_class_1(1),
            &[
                0xE1, 0x81, 0x80, 0x00, 0x02, 0x01, 0x28, 0x02, 0x00, 0x00, 0x00, 0x81, 0x00, 0x00,
                0x01,
            ],
        )
        .await;
    harness.check_events(&[Event::EnterSolicitedConfirmWait(1)]);

    harness.send_and_process(&confirm(1)).await;
    harness.check_events(&[
        Event::SolicitedConfirmReceived(1),
        Event::BeginConfirm,
        Event::Cleared(0),
        Event::Cleared(1),
        Event::EndConfirm(empty_buffer_state()),
    ]);
}

#[tokio::test]
async fn confirmed_read_after_unsolicited_timeout_does_not_lose_unconfirmed_events() {
    let mut harness = new_harness(config_with_one_unsolicited_attempt());
    time_out_unsolicited_event(&mut harness).await;

    // a second event while waiting to retry the unsolicited response
    update_binary_event(&mut harness.handle.database, false);

    // the READ reports both events, not just the one that was never sent
    read_and_confirm_two_class_1_events(&mut harness).await;
    harness
        .test_request_response(&read_class_1(2), EMPTY_RESPONSE_SEQ2)
        .await;
    harness.check_no_events();
}

// SELECT and OPERATE for g41v2 index 7, value 513
const SELECT_SEQ1_G41V2: &[u8] = &[0xC1, 0x03, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00];
const OPERATE_SEQ2_G41V2: &[u8] = &[0xC2, 0x04, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00];

async fn select_seq1(harness: &mut OutstationHarness) {
    harness
        .test_request_response(
            SELECT_SEQ1_G41V2,
            &[
                0xC1, 0x81, 0x80, 0x00, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00,
            ],
        )
        .await;
    harness.check_events(&selected());
}

/// the status of the single g41v2 object in a control response
async fn control_status(harness: &mut OutstationHarness) -> Option<u8> {
    harness.expect_write().await.last().copied()
}

/// IEEE 1815-2012 4.4.4 Table 4-9: the master's CONFIRM of an unsolicited response isn't a
/// request, so it doesn't end the selection (#501)
#[tokio::test]
async fn unsolicited_confirm_between_select_and_operate_keeps_the_selection() {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;

    select_seq1(&mut harness).await;
    harness.handle.database.transaction(add_binary_event);
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);
    harness.send_and_process(&unsolicited_confirm(1)).await;
    harness.check_events(&[
        Event::UnsolicitedConfirmReceived(1),
        Event::BeginConfirm,
        Event::Cleared(0),
        Event::EndConfirm(empty_buffer_state()),
    ]);

    harness.send_and_process(OPERATE_SEQ2_G41V2).await;
    assert_eq!(control_status(&mut harness).await, Some(0));
    harness.check_events(&operated(OperateType::SelectBeforeOperate));
}

#[tokio::test]
async fn operate_during_unsolicited_confirm_wait_uses_the_selection() {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;

    select_seq1(&mut harness).await;
    harness.handle.database.transaction(add_binary_event);
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);

    harness.send_and_process(OPERATE_SEQ2_G41V2).await;
    assert_eq!(control_status(&mut harness).await, Some(0));
    harness.check_events(&operated(OperateType::SelectBeforeOperate));
}

#[tokio::test]
async fn select_during_unsolicited_confirm_wait_can_be_retried_and_operated() {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;

    harness.handle.database.transaction(add_binary_event);
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);

    select_seq1(&mut harness).await;

    // the retry of the SELECT repeats its response and keeps the selection
    harness.send_and_process(SELECT_SEQ1_G41V2).await;
    assert_eq!(control_status(&mut harness).await, Some(0));
    harness.check_no_events();

    harness.send_and_process(OPERATE_SEQ2_G41V2).await;
    assert_eq!(control_status(&mut harness).await, Some(0));
    harness.check_events(&operated(OperateType::SelectBeforeOperate));
}

/// a deferred READ ends the selection, and a retry of the SELECT after it repeats the SELECT's
/// response without selecting again
#[tokio::test]
async fn select_retry_after_a_deferred_read_does_not_restore_the_selection() {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;

    select_seq1(&mut harness).await;
    harness.handle.database.transaction(add_binary_event);
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);

    // a READ that is deferred
    harness.send_and_process(&read_class_0(5)).await;
    harness.check_no_events();

    // the retry of the SELECT repeats its response
    harness.send_and_process(SELECT_SEQ1_G41V2).await;
    assert_eq!(control_status(&mut harness).await, Some(0));
    harness.check_no_events();

    // NO_SELECT
    harness.send_and_process(OPERATE_SEQ2_G41V2).await;
    assert_eq!(control_status(&mut harness).await, Some(2));
    harness.check_no_events();
}

#[tokio::test]
async fn deferred_read_of_object_that_cannot_be_read_sets_object_unknown() {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;

    harness.handle.database.transaction(add_binary_event);
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);
    // send a read of g13v1, which can't be read, that will be deferred
    harness.send_and_process(&[0xC0, 0x01, 13, 1, 0x06]).await;
    harness.check_no_events();

    // expire the unsolicited response
    tokio::time::pause();
    harness
        .wait_for_events(&[Event::UnsolicitedConfirmTimeout(1, false)])
        .await;
    // the class 1 event is still pending, and IIN2.1 (OBJECT_UNKNOWN)
    harness.expect_response(&[0xC0, 0x81, 0x82, 0x02]).await;
    harness.check_no_events();
}

#[tokio::test]
async fn deferred_read_with_more_headers_than_the_limit_sets_parameter_error() {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;

    harness.handle.database.transaction(add_binary_event);
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);
    // a read of class 0 in one more header than the default limit, which will be deferred
    let mut request = vec![0xC0, 0x01];
    for _ in 0..=OutstationConfig::DEFAULT_MAX_READ_REQUEST_HEADERS {
        request.extend_from_slice(&[60, 1, 0x06]);
    }
    harness.send_and_process(&request).await;
    harness.check_no_events();

    // expire the unsolicited response
    tokio::time::pause();
    harness
        .wait_for_events(&[Event::UnsolicitedConfirmTimeout(1, false)])
        .await;
    // the class 1 event is still pending, and IIN2.2 (PARAMETER_ERROR), ahead of the binary point
    let response = harness.expect_write().await;
    assert_eq!(response.get(..4), Some([0xC0, 0x81, 0x82, 0x04].as_slice()));
    harness.check_no_events();
}

/// longer than the confirm timeout plus the unsolicited retry delay, and shorter than the keep-alive
/// timeout, so any unsolicited response the outstation would send goes out within it
const UNSOLICITED_SILENCE: std::time::Duration = std::time::Duration::from_secs(30);

/// run the outstation for [`UNSOLICITED_SILENCE`] of paused time and check that it sends nothing
async fn expect_silence(harness: &mut OutstationHarness) {
    tokio::time::sleep(UNSOLICITED_SILENCE).await;
    harness.expect_no_response();
    harness.check_no_events();
}

#[tokio::test(start_paused = true)]
async fn handles_disable_unsolicited_during_unsolicited_confirm_wait() {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;

    harness.handle.database.transaction(add_binary_event);
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);

    // send a disable unsolicited request
    harness
        .test_request_response(DISABLE_UNSOLICITED_SEQ0, EMPTY_RESPONSE_SEQ0)
        .await;
    harness.check_no_events();

    // neither the unconfirmed event nor a new one is reported in an unsolicited response
    update_binary_event(&mut harness.handle.database, false);
    expect_silence(&mut harness).await;

    // both are still there for a READ
    read_and_confirm_two_class_1_events(&mut harness).await;
}

const ENABLE_UNSOLICITED_CLASS_2_SEQ0: &[u8] = &[0xC0, 0x14, 0x3C, 0x03, 0x06];
const DISABLE_UNSOLICITED_CLASS_1_SEQ1: &[u8] = &[0xC1, 0x15, 0x3C, 0x02, 0x06];
const EMPTY_RESPONSE_SEQ1: &[u8] = &[0xC1, 0x81, 0x80, 0x00];

/// with class 1 disabled for unsolicited responses and class 2 enabled, a class 1 event isn't
/// reported in an unsolicited response, and a class 2 event is reported on its own
async fn only_class_2_is_reported_unsolicited(harness: &mut OutstationHarness) {
    harness.handle.database.transaction(|db| {
        db.add(0, Some(EventClass::Class1), BinaryInputConfig::default());
        db.add(1, Some(EventClass::Class2), BinaryInputConfig::default());
    });

    update_binary_event(&mut harness.handle.database, true);
    expect_silence(harness).await;

    harness.handle.database.transaction(|db| {
        db.update(
            1,
            &BinaryInput::new(true, Flags::ONLINE, Time::synchronized(0)),
            UpdateOptions::default(),
        )
    });
    // IIN1.1 for the class 1 event, and the class 2 event at index 1
    harness
        .expect_response(&[
            0xF1, 0x82, 0x82, 0x00, 0x02, 0x01, 0x28, 0x01, 0x00, 0x01, 0x00, 0x81,
        ])
        .await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);
}

#[tokio::test(start_paused = true)]
async fn disable_unsolicited_of_one_class_leaves_the_others_enabled() {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;

    harness
        .test_request_response(DISABLE_UNSOLICITED_CLASS_1_SEQ1, EMPTY_RESPONSE_SEQ1)
        .await;

    only_class_2_is_reported_unsolicited(&mut harness).await;
}

#[tokio::test(start_paused = true)]
async fn enable_unsolicited_of_one_class_enables_only_that_class() {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness).await;

    harness
        .test_request_response(ENABLE_UNSOLICITED_CLASS_2_SEQ0, EMPTY_RESPONSE_SEQ0)
        .await;

    only_class_2_is_reported_unsolicited(&mut harness).await;
}

#[tokio::test]
async fn sends_unsolicited_from_one_update() {
    let mut config = get_default_unsolicited_config();
    config.unsolicited_buffer_size = BufferSize::min();
    config.event_buffer_config.max_analog = 1001;
    let mut harness = new_harness(config);
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;

    harness.handle.database.transaction(|db| {
        for i in 0..1000 {
            db.add(
                i,
                Some(EventClass::Class1),
                AnalogInputConfig {
                    s_var: StaticAnalogInputVariation::Group30Var1,
                    e_var: EventAnalogInputVariation::Group32Var1, // 5  bytes
                    deadband: 0.0,
                },
            );
        }
    });

    harness.handle.database.transaction(|db| {
        let value = AnalogInput {
            value: 98.6,
            flags: Flags { value: 0 },
            time: None,
        };

        for i in 0..1000 {
            db.update(i, &value, UpdateOptions::detect_event());
        }
    });

    let mut total_fragments: usize = 0;
    let mut total_events = 0;

    loop {
        let rx = harness.expect_write().await;
        let response = ParsedFragment::parse(ParseOptions::default(), &rx)
            .unwrap()
            .to_response()
            .unwrap();
        let mut num_events = 0;
        match response.objects.unwrap().get_only_header().unwrap() {
            ObjectHeader::CountAndPrefix16(_, PrefixedVariation::Group32Var1(seq)) => {
                num_events += seq.iter().count()
            }
            x => panic!("Unexpected header: {x:?}"),
        }

        total_fragments += 1;
        total_events += num_events;

        // confirm the fragment
        let confirm = unsolicited_confirm(response.header.control.seq.value());
        harness.send_and_process(&confirm).await;

        // terminate when the outstation indicates there are no more events
        if !response.header.iin.iin1.get_class_1_events() {
            break;
        }
    }

    assert_eq!(total_events, 1000);
    assert!(total_fragments > 10);
}

const BROADCAST_RESPONSE_SEQ1: &[u8] = &[0xC1, 0x81, 0x81, 0x00];
const BROADCAST_RESPONSE_SEQ1_CON: &[u8] = &[0xE1, 0x81, 0x81, 0x00];

async fn broadcast_record_current_time(
    harness: &mut OutstationHarness,
    mode: BroadcastConfirmMode,
) {
    harness.set_broadcast(Some(mode));
    harness.send_and_process(&record_current_time(0)).await;
    harness.set_broadcast(None);
    harness.check_events(&[Event::BroadcastReceived(
        FunctionCode::RecordCurrentTime,
        BroadcastAction::Processed,
    )]);
}

async fn broadcast_during_unsolicited_confirm_wait_is_reported_after_the_confirm(
    mode: BroadcastConfirmMode,
) {
    let mut harness = new_harness(get_default_unsolicited_config());
    harness.expect_response(&null_unsolicited(0)).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(0)]);

    broadcast_record_current_time(&mut harness, mode).await;

    // the null unsolicited response didn't carry IIN1.0, so its confirm doesn't clear it
    harness.send_and_process(&unsolicited_confirm(0)).await;
    harness.check_events(&[Event::UnsolicitedConfirmReceived(0)]);

    if mode == BroadcastConfirmMode::Mandatory {
        harness
            .test_request_response(&read_class_0(1), BROADCAST_RESPONSE_SEQ1_CON)
            .await;
        harness.send_and_process(&confirm(1)).await;
    } else {
        harness
            .test_request_response(&read_class_0(1), BROADCAST_RESPONSE_SEQ1)
            .await;
    }

    harness
        .test_request_response(&read_class_0(2), EMPTY_RESPONSE_SEQ2)
        .await;
}

#[tokio::test]
async fn mandatory_broadcast_during_unsolicited_confirm_wait_is_reported_after_the_confirm() {
    broadcast_during_unsolicited_confirm_wait_is_reported_after_the_confirm(
        BroadcastConfirmMode::Mandatory,
    )
    .await;
}

#[tokio::test]
async fn optional_broadcast_during_unsolicited_confirm_wait_is_reported_after_the_confirm() {
    broadcast_during_unsolicited_confirm_wait_is_reported_after_the_confirm(
        BroadcastConfirmMode::Optional,
    )
    .await;
}

#[tokio::test]
async fn unconfirmed_broadcast_during_unsolicited_confirm_wait_is_reported_after_the_confirm() {
    broadcast_during_unsolicited_confirm_wait_is_reported_after_the_confirm(
        BroadcastConfirmMode::NotRequired,
    )
    .await;
}

#[tokio::test]
async fn stray_solicited_confirm_does_not_clear_unreported_mandatory_broadcast() {
    let mut harness = new_harness(get_default_unsolicited_config());
    harness.expect_response(&null_unsolicited(0)).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(0)]);

    broadcast_record_current_time(&mut harness, BroadcastConfirmMode::Mandatory).await;

    // no response has carried IIN1.0 yet
    harness.send_and_process(&confirm(0)).await;
    harness.expect_no_response();

    harness
        .test_request_response(&record_current_time(1), BROADCAST_RESPONSE_SEQ1_CON)
        .await;

    // the confirm of the response that carried IIN1.0 clears it
    harness.send_and_process(&confirm(1)).await;
    harness.send_and_process(&unsolicited_confirm(0)).await;
    harness.check_events(&[Event::UnsolicitedConfirmReceived(0)]);
    harness
        .test_request_response(&read_class_0(2), EMPTY_RESPONSE_SEQ2)
        .await;
}

#[tokio::test]
async fn unsolicited_confirm_clears_mandatory_broadcast_it_reported() {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;

    broadcast_record_current_time(&mut harness, BroadcastConfirmMode::Mandatory).await;
    harness.handle.database.transaction(add_binary_event);

    harness
        .expect_response(&[
            0xF1, 0x82, 0x81, 0x00, // DEVICE_RESTART and BROADCAST
            0x02, 0x01, 0x28, 0x01, 0x00, 0x00, 0x00, 0x81,
        ])
        .await;
    harness.send_and_process(&unsolicited_confirm(1)).await;
    harness.check_events(&[
        Event::EnterUnsolicitedConfirmWait(1),
        Event::UnsolicitedConfirmReceived(1),
    ]);

    harness
        .test_request_response(
            &read_class_0(1),
            &[
                0xC1, 0x81, 0x80, 0x00, // only DEVICE_RESTART
                0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01,
            ],
        )
        .await;
}

#[tokio::test]
async fn buffer_overflow_issue() {
    let mut config = get_default_unsolicited_config();
    config.event_buffer_config = EventBufferConfig::all_types(1);
    let mut harness = new_harness(config);
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;

    fn generate_overflow(database: &mut DatabaseHandle) {
        database.transaction(|database| {
            database.update(
                0,
                &BinaryInput::new(true, Flags::ONLINE, Time::Synchronized(Timestamp::new(0))),
                UpdateOptions::default(),
            );
            database.update(
                0,
                &BinaryInput::new(false, Flags::ONLINE, Time::Synchronized(Timestamp::new(0))),
                UpdateOptions::default(),
            );
        });
    }

    // Add the point
    harness.handle.database.transaction(|database| {
        database.add(0, Some(EventClass::Class1), BinaryInputConfig::default());
    });

    generate_overflow(&mut harness.handle.database);

    // Unsolicited response with event data and EVENT_BUFFER_OVERFLOW
    harness
        .expect_response(&[
            0xF1, 0x82, 0x80, 0x08, // DEVICE_RESTART and EVENT_BUFFER_OVERFLOW asserted
            0x02, 0x01, 0x28, 0x01, 0x00, // 1 event g2v1 only
            0x00, 0x00, 0x01,
        ])
        .await;

    // THIS USED TO GENERATE A SUBTRACT OVERFLOW IN THE EVENT BUFFER (the panic occurred in the next line)
    generate_overflow(&mut harness.handle.database);

    harness.send_and_process(&unsolicited_confirm(1)).await;

    // New unsolicited response with a single event
    harness
        .expect_response(&[
            0xF2, 0x82, 0x80, 0x08, // DEVICE_RESTART and EVENT_BUFFER_OVERFLOW asserted
            0x02, 0x01, 0x28, 0x01, 0x00, // 1 event g2v1 only
            0x00, 0x00, 0x01,
        ])
        .await;
    harness.send_and_process(&unsolicited_confirm(2)).await;

    // Integrity poll response should not contain EVENT_BUFFER_OVERFLOW flag anymore
    harness
        .test_request_response(
            &[
                0xC0, 0x01, 60, 2, 0x06, 60, 3, 0x06, 60, 4, 0x06, 60, 1, 0x06,
            ],
            &[
                0xC0, 0x81, 0x80, 0x00, // Only DEVICE_RESTART
                0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, // g1v1 [0, 0]
                0x00, // Current value
            ],
        )
        .await;
}

#[tokio::test]
async fn malformed_request_during_unsolicited_confirm_wait_leaves_the_unsolicited_response() {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;

    harness.handle.database.transaction(add_binary_event);
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);

    // read, seq == 1, g1v2 with a 1-byte start/stop qualifier but no start or stop, answered with
    // restart IIN and IIN2.2 PARAMETER_ERROR
    harness
        .test_request_response(&[0xC1, 0x01, 0x01, 0x02, 0x00], &[0xC1, 0x81, 0x80, 0x04])
        .await;
    harness.check_no_events();

    // the unsolicited response is retried unchanged
    tokio::time::pause();
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::UnsolicitedConfirmTimeout(1, true)]);
    tokio::time::resume();

    // and its confirm is still accepted
    harness.send_and_process(&unsolicited_confirm(1)).await;
    harness.check_events(&[
        Event::UnsolicitedConfirmReceived(1),
        Event::BeginConfirm,
        Event::Cleared(0),
        Event::EndConfirm(empty_buffer_state()),
    ]);
}

// IEEE 1815-2012 Table 6-1 rows 20-30: a solicited response sent during the unsolicited confirm
// wait can ask for a confirm, here because of a mandatory broadcast. Only that response's confirm,
// before its confirm timeout, clears the broadcast's IIN1.0.

const BROADCAST_RESPONSE_SEQ2_CON: &[u8] = &[0xE2, 0x81, 0x81, 0x00];
const BROADCAST_RESPONSE_SEQ3_CON: &[u8] = &[0xE3, 0x81, 0x81, 0x00];
const RESPONSE_SEQ3: &[u8] = &[0xC3, 0x81, 0x80, 0x00];

/// the null unsolicited response, never confirmed, and a mandatory broadcast during its wait
async fn mandatory_broadcast_during_null_unsolicited_wait() -> OutstationHarness {
    let mut harness = new_harness(get_default_unsolicited_config());
    harness.expect_response(&null_unsolicited(0)).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(0)]);
    broadcast_record_current_time(&mut harness, BroadcastConfirmMode::Mandatory).await;
    harness
}

/// the master confirms a solicited response, which gets no answer and no callback
async fn send_solicited_confirm(harness: &mut OutstationHarness, confirm: &[u8]) {
    harness.send_and_process(confirm).await;
    harness.expect_no_response();
    harness.check_no_events();
}

/// Table 6-1 row 21
#[tokio::test]
async fn solicited_confirm_with_another_sequence_does_not_clear_a_reported_broadcast() {
    let mut harness = mandatory_broadcast_during_null_unsolicited_wait().await;

    harness
        .test_request_response(&record_current_time(1), BROADCAST_RESPONSE_SEQ1_CON)
        .await;
    send_solicited_confirm(&mut harness, &confirm(2)).await;

    // the wrong confirm didn't clear the broadcast
    harness
        .test_request_response(&record_current_time(2), BROADCAST_RESPONSE_SEQ2_CON)
        .await;

    // nor does it end the wait for the right one, which follows it with no request between
    send_solicited_confirm(&mut harness, &confirm(3)).await;
    send_solicited_confirm(&mut harness, &confirm(2)).await;
    harness
        .test_request_response(&record_current_time(3), RESPONSE_SEQ3)
        .await;
    harness.check_no_events();
}

/// Table 6-1 row 26: the next request ends the wait for the earlier response's confirm
#[tokio::test]
async fn solicited_confirm_of_an_earlier_response_does_not_clear_a_reported_broadcast() {
    let mut harness = mandatory_broadcast_during_null_unsolicited_wait().await;

    harness
        .test_request_response(&record_current_time(1), BROADCAST_RESPONSE_SEQ1_CON)
        .await;
    harness
        .test_request_response(&record_current_time(2), BROADCAST_RESPONSE_SEQ2_CON)
        .await;
    send_solicited_confirm(&mut harness, &confirm(1)).await;

    harness
        .test_request_response(&record_current_time(3), BROADCAST_RESPONSE_SEQ3_CON)
        .await;
    send_solicited_confirm(&mut harness, &confirm(3)).await;
    harness
        .test_request_response(&record_current_time(4), &[0xC4, 0x81, 0x80, 0x00])
        .await;
    harness.check_no_events();
}

/// Table 6-1 row 25: a deferred READ ends the wait for the earlier response's confirm
#[tokio::test]
async fn deferred_read_ends_the_wait_for_a_solicited_confirm() {
    let mut harness = mandatory_broadcast_during_null_unsolicited_wait().await;

    harness
        .test_request_response(&record_current_time(1), BROADCAST_RESPONSE_SEQ1_CON)
        .await;
    harness.send_and_process(&read_class_0(2)).await;
    send_solicited_confirm(&mut harness, &confirm(1)).await;

    // the deferred READ reports the broadcast and waits for its confirm
    harness.send_and_process(&unsolicited_confirm(0)).await;
    harness.expect_response(BROADCAST_RESPONSE_SEQ2_CON).await;
    harness.check_events(&[
        Event::UnsolicitedConfirmReceived(0),
        Event::EnterSolicitedConfirmWait(2),
    ]);
    harness.send_and_process(&confirm(2)).await;
    harness.check_events(&[
        Event::SolicitedConfirmReceived(2),
        Event::BeginConfirm,
        Event::EndConfirm(empty_buffer_state()),
    ]);
    harness
        .test_request_response(&record_current_time(3), RESPONSE_SEQ3)
        .await;
    harness.check_no_events();
}

/// Table 6-1 rows 22 and 33 are skipped: once the unsolicited response's wait ends, so does the
/// wait for the solicited confirm
#[tokio::test]
async fn solicited_confirm_after_the_unsolicited_confirm_wait_does_not_clear_a_reported_broadcast()
{
    let mut harness = mandatory_broadcast_during_null_unsolicited_wait().await;

    harness
        .test_request_response(&record_current_time(1), BROADCAST_RESPONSE_SEQ1_CON)
        .await;
    harness.send_and_process(&unsolicited_confirm(0)).await;
    harness.check_events(&[Event::UnsolicitedConfirmReceived(0)]);
    send_solicited_confirm(&mut harness, &confirm(1)).await;

    harness
        .test_request_response(&record_current_time(2), BROADCAST_RESPONSE_SEQ2_CON)
        .await;
    harness.check_events(&[Event::EnterSolicitedConfirmWait(2)]);
    harness.send_and_process(&confirm(2)).await;
    harness.check_events(&[
        Event::SolicitedConfirmReceived(2),
        Event::BeginConfirm,
        Event::EndConfirm(empty_buffer_state()),
    ]);
    harness
        .test_request_response(&record_current_time(3), RESPONSE_SEQ3)
        .await;
    harness.check_no_events();
}

/// an unsolicited response with one binary event that is retried until it's confirmed, then a
/// mandatory broadcast and a solicited response that reports it, which asks for a confirm
async fn solicited_response_during_unsolicited_retries(harness: &mut OutstationHarness) {
    confirm_null_unsolicited(harness).await;
    enable_unsolicited(harness).await;

    harness.handle.database.transaction(add_binary_event);
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);

    broadcast_record_current_time(harness, BroadcastConfirmMode::Mandatory).await;
    harness
        .test_request_response(&record_current_time(2), BROADCAST_RESPONSE_SEQ2_CON)
        .await;

    // the retry, in the same wait, after the solicited response's confirm timeout
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::UnsolicitedConfirmTimeout(1, true)]);
    tokio::time::advance(std::time::Duration::from_secs(1)).await;
}

/// the confirm of the unsolicited response, after which the outstation is idle
async fn confirm_unsolicited_event(harness: &mut OutstationHarness) {
    harness.send_and_process(&unsolicited_confirm(1)).await;
    harness.check_events(&[
        Event::UnsolicitedConfirmReceived(1),
        Event::BeginConfirm,
        Event::Cleared(0),
        Event::EndConfirm(empty_buffer_state()),
    ]);
}

/// Table 6-1 row 28: the repeat gets the same response, which asks for its confirm again, with
/// a new confirm timeout, as in the solicited confirm wait (row 17)
#[tokio::test(start_paused = true)]
async fn repeated_request_waits_for_the_solicited_confirm_again() {
    let mut harness = new_harness(get_default_unsolicited_config());
    solicited_response_during_unsolicited_retries(&mut harness).await;

    harness
        .test_request_response(&record_current_time(2), BROADCAST_RESPONSE_SEQ2_CON)
        .await;
    send_solicited_confirm(&mut harness, &confirm(2)).await;
    confirm_unsolicited_event(&mut harness).await;

    harness
        .test_request_response(&record_current_time(3), RESPONSE_SEQ3)
        .await;
    harness.check_no_events();
}

/// An unsolicited build that selects class 1 events but writes none of them, because the first
/// one can't fit in the unsolicited buffer, doesn't leave them selected for the next READ
#[tokio::test]
async fn read_after_an_unsolicited_build_that_wrote_nothing_reports_only_what_it_selects() {
    let mut config = get_default_unsolicited_config();
    config.unsolicited_buffer_size = BufferSize::min();
    let mut harness = new_harness(config);
    confirm_null_unsolicited(&mut harness).await;

    harness.handle.database.transaction(|db| {
        db.add(0, Some(EventClass::Class1), OctetStringConfig);
        db.add(0, Some(EventClass::Class2), BinaryInputConfig::default());
        db.update(
            0,
            &OctetString::new(&[0xAA; 255]).unwrap(),
            UpdateOptions::default(),
        );
        db.update(
            0,
            &BinaryInput::new(true, Flags::ONLINE, Time::synchronized(0)),
            UpdateOptions::default(),
        );
    });

    // enable class 1 only, after which the unsolicited build selects the octet string event
    harness
        .test_request_response(&[0xC0, 0x14, 0x3C, 0x02, 0x06], &[0xC0, 0x81, 0x86, 0x00])
        .await;

    // a class 2 READ reports the binary event, and not the class 1 octet string
    harness
        .test_request_response(
            &[0xC1, 0x01, 0x3C, 0x03, 0x06],
            &[
                0xE1, 0x81, 0x82, 0x00, 0x02, 0x01, 0x28, 0x01, 0x00, 0x00, 0x00, 0x81,
            ],
        )
        .await;
    harness.check_events(&[Event::EnterSolicitedConfirmWait(1)]);
}

const CONFIRM_TIMEOUT: Duration = OutstationConfig::DEFAULT_CONFIRM_TIMEOUT;
const RETRY_DELAY: Duration = OutstationConfig::DEFAULT_UNSOLICITED_RETRY_DELAY;
const DISABLE_UNSOLICITED_CLASS_2_SEQ1: &[u8] = &[0xC1, 0x15, 0x3C, 0x03, 0x06];

/// confirm the null unsolicited response, enable unsolicited responses, and send the class 1 event
/// in an unsolicited response that waits for its confirm
async fn send_unsolicited_event(harness: &mut OutstationHarness) {
    confirm_null_unsolicited(harness).await;
    enable_unsolicited(harness).await;

    harness.handle.database.transaction(add_binary_event);
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);
}

/// DISABLE_UNSOLICITED for classes 1, 2 and 3 sent to the broadcast address
async fn broadcast_disable_unsolicited(harness: &mut OutstationHarness) {
    harness.set_broadcast(Some(BroadcastConfirmMode::NotRequired));
    harness.send_and_process(DISABLE_UNSOLICITED_SEQ0).await;
    harness.set_broadcast(None);
    harness.check_events(&[Event::BroadcastReceived(
        FunctionCode::DisableUnsolicited,
        BroadcastAction::Processed,
    )]);
}

#[tokio::test(start_paused = true)]
async fn confirm_after_disable_unsolicited_during_unsolicited_confirm_wait() {
    watchdog(async {
        let mut harness = new_harness(get_default_unsolicited_config());
        send_unsolicited_event(&mut harness).await;

        harness
            .test_request_response(DISABLE_UNSOLICITED_SEQ0, EMPTY_RESPONSE_SEQ0)
            .await;
        harness.check_no_events();

        // the DISABLE ended the wait, so the confirm arrives in idle and doesn't clear the event
        harness.record_requests_from_idle();
        harness.send_and_process(&unsolicited_confirm(1)).await;
        harness.check_events(&[from_idle(FunctionCode::Confirm, true, 1)]);
        harness
            .test_request_response(&read_class_1(1), G2V1_RESPONSE_SEQ1)
            .await;
        harness.check_events(&[
            from_idle(FunctionCode::Read, false, 1),
            Event::EnterSolicitedConfirmWait(1),
        ]);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn timeout_after_disable_unsolicited_during_unsolicited_confirm_wait() {
    watchdog(async {
        let mut harness = new_harness(get_default_unsolicited_config());
        send_unsolicited_event(&mut harness).await;

        harness
            .test_request_response(DISABLE_UNSOLICITED_SEQ0, EMPTY_RESPONSE_SEQ0)
            .await;
        harness.check_no_events();

        // the DISABLE ended the wait, so it never times out
        expect_silence(&mut harness).await;

        // the event is still buffered
        harness
            .test_request_response(&read_class_1(1), G2V1_RESPONSE_SEQ1)
            .await;
        harness.check_events(&[Event::EnterSolicitedConfirmWait(1)]);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn timeout_after_broadcast_disable_unsolicited_during_unsolicited_confirm_wait() {
    watchdog(async {
        let mut harness = new_harness(get_default_unsolicited_config());
        send_unsolicited_event(&mut harness).await;
        let sent = Instant::now();

        broadcast_disable_unsolicited(&mut harness).await;

        // the response is retried, without a limit
        for retry in 1..=3 {
            harness.expect_response(UNSOL_G2V1_SEQ1).await;
            harness.check_events(&[Event::UnsolicitedConfirmTimeout(1, true)]);
            assert_eq!(sent.elapsed(), CONFIRM_TIMEOUT * retry);
        }
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn confirm_after_disable_unsolicited_during_null_unsolicited_confirm_wait() {
    watchdog(async {
        let mut harness = new_harness(get_default_unsolicited_config());
        harness.expect_response(&null_unsolicited(0)).await;
        harness.check_events(&[Event::EnterUnsolicitedConfirmWait(0)]);
        let start = Instant::now();

        harness
            .test_request_response(DISABLE_UNSOLICITED_SEQ0, EMPTY_RESPONSE_SEQ0)
            .await;

        // the DISABLE ended the wait, and the null response is sent again at once
        harness.expect_response(&null_unsolicited(1)).await;
        harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);
        assert_eq!(start.elapsed(), Duration::ZERO);

        // so the confirm of the first one is ignored
        harness.send_and_process(&unsolicited_confirm(0)).await;
        harness.check_no_events();
        harness
            .wait_for_events(&[Event::UnsolicitedConfirmTimeout(1, false)])
            .await;
        harness.expect_response(&null_unsolicited(2)).await;
        harness.check_events(&[Event::EnterUnsolicitedConfirmWait(2)]);
        assert_eq!(start.elapsed(), CONFIRM_TIMEOUT);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn timeout_after_disable_unsolicited_during_null_unsolicited_confirm_wait() {
    watchdog(async {
        let mut harness = new_harness(get_default_unsolicited_config());
        harness.expect_response(&null_unsolicited(0)).await;
        harness.check_events(&[Event::EnterUnsolicitedConfirmWait(0)]);
        let start = Instant::now();

        harness
            .test_request_response(DISABLE_UNSOLICITED_SEQ0, EMPTY_RESPONSE_SEQ0)
            .await;

        // the DISABLE ended the wait, and the null response is sent again at once
        harness.expect_response(&null_unsolicited(1)).await;
        harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);
        assert_eq!(start.elapsed(), Duration::ZERO);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn timeout_after_broadcast_disable_unsolicited_during_null_unsolicited_confirm_wait() {
    watchdog(async {
        let mut harness = new_harness(get_default_unsolicited_config());
        harness.expect_response(&null_unsolicited(0)).await;
        harness.check_events(&[Event::EnterUnsolicitedConfirmWait(0)]);
        let sent = Instant::now();

        broadcast_disable_unsolicited(&mut harness).await;

        // the null response is never retried, but sent again, reporting the broadcast
        harness
            .wait_for_events(&[Event::UnsolicitedConfirmTimeout(0, false)])
            .await;
        harness.expect_response(&[0xF1, 0x82, 0x81, 0x00]).await;
        harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);
        assert_eq!(sent.elapsed(), CONFIRM_TIMEOUT);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn disable_unsolicited_of_another_class_during_limited_unsolicited_retries() {
    watchdog(async {
        let mut harness = new_harness(config_with_limited_retries(2));
        send_unsolicited_event(&mut harness).await;

        // the first retry
        harness.expect_response(UNSOL_G2V1_SEQ1).await;
        harness.check_events(&[Event::UnsolicitedConfirmTimeout(1, true)]);
        let retried = Instant::now();

        harness
            .test_request_response(DISABLE_UNSOLICITED_CLASS_2_SEQ1, EMPTY_RESPONSE_SEQ1)
            .await;
        harness.check_no_events();

        // the DISABLE ended the series, and class 1 starts another after the retry delay
        harness.expect_response(UNSOL_G2V1_SEQ2).await;
        harness.check_events(&[Event::EnterUnsolicitedConfirmWait(2)]);
        assert_eq!(retried.elapsed(), RETRY_DELAY);
    })
    .await;
}
