// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use std::time::Duration;

use tokio::time::Instant;

use crate::app::Sequence;
use crate::app::{ControlField, FunctionCode, Iin, Iin1, Iin2};
use crate::master::association::AssociationConfig;
use crate::master::request::EventClasses;
use crate::master::tests::harness::AssocInfoEvent;
use crate::master::{Classes, TaskError, TaskType, TimeSyncProcedure};

use super::harness::requests::*;
use super::harness::{create_association, create_association_with_handler, TestHarness};
use super::time_sync::{record_current_time, write_last_recorded_time, FixedTime, NEED_TIME};
use crate::util::test_support::watchdog;

const BUFFER_OVERFLOW: Iin = Iin::new(Iin1::new(0x00), Iin2::new(0x08));
const CLASS_1_EVENTS: Iin = Iin::new(Iin1::new(0x02), Iin2::new(0x00));
const CLASS_2_EVENTS: Iin = Iin::new(Iin1::new(0x04), Iin2::new(0x00));

#[tokio::test]
async fn auto_integrity_scan_on_buffer_overflow() {
    watchdog(async {
        let config = AssociationConfig {
            auto_integrity_scan_on_buffer_overflow: true,
            ..Default::default()
        };
        let mut seq = Sequence::default();
        let mut harness = create_association(config).await;

        startup_procedure(&mut harness, &mut seq).await;

        // Unsolicited with IIN2.3 EVENT_BUFFER_OVERFLOW set
        harness
            .read_and_expect_write(
                unsol_null_custom_iin(seq, BUFFER_OVERFLOW),
                unsol_confirm(seq),
            )
            .await;

        // Send integrity poll
        harness
            .expect_write_and_respond(integrity_poll_request(seq), empty_response(seq.increment()))
            .await;

        assert_eq!(harness.io.pop_event(), None);
    })
    .await;
}

#[tokio::test]
async fn auto_integrity_scan_on_buffer_overflow_disabled() {
    watchdog(async {
        let config = AssociationConfig {
            auto_integrity_scan_on_buffer_overflow: false,
            ..Default::default()
        };
        let mut seq = Sequence::default();
        let mut harness = create_association(config).await;

        startup_procedure(&mut harness, &mut seq).await;

        // Unsolicited with IIN2.3 EVENT_BUFFER_OVERFLOW set
        harness
            .read_and_expect_write(
                unsol_null_custom_iin(seq, BUFFER_OVERFLOW),
                unsol_confirm(seq),
            )
            .await;

        assert_eq!(harness.io.pop_event(), None);
    })
    .await;
}

#[tokio::test]
async fn auto_event_class_scan() {
    watchdog(async {
        let config = AssociationConfig {
            event_scan_on_events_available: EventClasses::all(),
            ..Default::default()
        };
        let mut seq = Sequence::default();
        let mut harness = create_association(config).await;

        startup_procedure(&mut harness, &mut seq).await;

        // notify that class 1 events are available
        harness
            .read_and_expect_write(
                unsol_null_custom_iin(seq, CLASS_1_EVENTS),
                unsol_confirm(seq),
            )
            .await;

        let class1 = Classes::new(false, EventClasses::new(true, false, false));
        harness
            .expect_write_and_respond(class_scan_request(class1, seq), empty_response(seq))
            .await;

        assert_eq!(harness.io.pop_event(), None);
    })
    .await;
}

#[tokio::test]
async fn auto_event_class_ignore_one_class_scan() {
    watchdog(async {
        let config = AssociationConfig {
            event_scan_on_events_available: EventClasses::new(false, true, true),
            ..Default::default()
        };
        let mut seq = Sequence::default();
        let mut harness = create_association(config).await;

        startup_procedure(&mut harness, &mut seq).await;

        // Unsolicited with IIN2.3 EVENT_BUFFER_OVERFLOW set
        harness
            .read_and_expect_write(
                unsol_null_custom_iin(seq, CLASS_1_EVENTS),
                unsol_confirm(seq),
            )
            .await;
        // make sure this does not result in a write
        assert_eq!(harness.io.pop_event(), None);
    })
    .await;
}

#[tokio::test]
async fn auto_event_class_scan_disabled() {
    watchdog(async {
        let config = AssociationConfig {
            event_scan_on_events_available: EventClasses::none(),
            ..Default::default()
        };
        let mut seq = Sequence::default();
        let mut harness = create_association(config).await;

        startup_procedure(&mut harness, &mut seq).await;

        // Unsolicited with IIN2.3 EVENT_BUFFER_OVERFLOW set
        harness
            .read_and_expect_write(
                unsol_null_custom_iin(seq, CLASS_1_EVENTS),
                unsol_confirm(seq),
            )
            .await;
        // make sure this does not result in a write
        assert_eq!(harness.io.pop_event(), None);
    })
    .await;
}

const COLD_RESTART: u8 = 0x0D;
const RECORD_CURRENT_TIME: u8 = 0x18;
const RESTART_AND_NO_FUNC_CODE_SUPPORT: Iin = Iin::new(Iin1::new(0x80), Iin2::new(0x01));
const NEED_TIME_AND_NO_FUNC_CODE_SUPPORT: Iin = Iin::new(Iin1::new(0x10), Iin2::new(0x01));

async fn cold_restart_rejected_with(harness: &mut TestHarness, seq: &mut Sequence, iin: Iin) {
    let mut association = harness.association.clone();
    let restart = tokio::spawn(async move { association.cold_restart().await });
    harness
        .expect_write_and_respond(
            vec![0xC0 | seq.value(), COLD_RESTART],
            empty_response_custom_iin(seq.increment(), iin),
        )
        .await;
    assert_eq!(restart.await.unwrap(), Err(TaskError::RejectedByIin2(iin)));
}

/// Fail instead of waiting forever if the master never sends the request
async fn expect_write_soon(harness: &mut TestHarness, expected: Vec<u8>) {
    tokio::time::timeout(Duration::from_secs(5), harness.expect_write(expected))
        .await
        .expect("the master did not send the request");
}

#[tokio::test]
async fn restart_iin_in_a_rejected_response_schedules_clear_restart() {
    watchdog(async {
        tokio::time::pause();
        let mut seq = Sequence::default();
        let mut harness = create_association(AssociationConfig::quiet()).await;

        cold_restart_rejected_with(&mut harness, &mut seq, RESTART_AND_NO_FUNC_CODE_SUPPORT).await;

        expect_write_soon(&mut harness, clear_restart_iin(seq)).await;
        harness
            .process_response(empty_response(seq.increment()))
            .await;
    })
    .await;
}

#[tokio::test]
async fn need_time_iin_in_a_rejected_response_schedules_time_sync() {
    watchdog(async {
        tokio::time::pause();
        let config = AssociationConfig {
            auto_time_sync: Some(TimeSyncProcedure::Lan),
            ..AssociationConfig::quiet()
        };
        let mut seq = Sequence::default();
        let mut harness = create_association(config).await;

        cold_restart_rejected_with(&mut harness, &mut seq, NEED_TIME_AND_NO_FUNC_CODE_SUPPORT)
            .await;

        expect_write_soon(&mut harness, vec![0xC0 | seq.value(), RECORD_CURRENT_TIME]).await;
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn rejected_clear_restart_retries_after_the_backoff() {
    watchdog(async {
        let mut seq = Sequence::default();
        let mut harness = create_association(AssociationConfig::quiet()).await;

        cold_restart_rejected_with(&mut harness, &mut seq, RESTART_AND_NO_FUNC_CODE_SUPPORT).await;

        // the outstation rejects the clear restart and still reports the restart
        expect_write_soon(&mut harness, clear_restart_iin(seq)).await;
        harness
            .process_response(empty_response_custom_iin(
                seq.increment(),
                RESTART_AND_NO_FUNC_CODE_SUPPORT,
            ))
            .await;

        // the master retries after the minimum retry delay instead of immediately
        let failed = Instant::now();
        harness
            .expect_write_and_respond(clear_restart_iin(seq), empty_response(seq.increment()))
            .await;
        TestHarness::assert_elapsed(failed, Duration::from_secs(1));
        harness.assert_no_events();
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn rejected_enable_unsolicited_with_restart_iin_is_repeated_after_the_restart_is_cleared() {
    watchdog(async {
        let config = AssociationConfig {
            enable_unsol_classes: EventClasses::all(),
            ..AssociationConfig::quiet()
        };
        let mut seq = Sequence::default();
        let mut harness = create_association(config).await;

        // the outstation rejects ENABLE_UNSOLICITED and reports a restart
        expect_write_soon(&mut harness, enable_unsol_request(seq)).await;
        harness
            .process_response(empty_response_custom_iin(
                seq.increment(),
                RESTART_AND_NO_FUNC_CODE_SUPPORT,
            ))
            .await;

        expect_write_soon(&mut harness, clear_restart_iin(seq)).await;
        harness
            .process_response(empty_response(seq.increment()))
            .await;

        // the restart starts the sequence over, so ENABLE_UNSOLICITED is sent again
        expect_write_soon(&mut harness, enable_unsol_request(seq)).await;
        harness
            .process_response(empty_response(seq.increment()))
            .await;

        assert_no_write_for(&mut harness, Duration::from_secs(60)).await;
    })
    .await;
}

/// The response timeout set by the harness plus the default minimum retry delay
const TIMEOUT_AND_FIRST_RETRY: Duration = Duration::from_secs(2);

/// Let `request` go unanswered, then expect the master to send `retry` exactly `after` the
/// original request, which was sent at the returned instant
async fn expect_retry_after(
    harness: &mut TestHarness,
    request: Vec<u8>,
    retry: Vec<u8>,
    after: Duration,
) -> Instant {
    expect_write_soon(harness, request).await;
    let sent = Instant::now();
    tokio::time::timeout(Duration::from_secs(60), harness.expect_write(retry))
        .await
        .expect("the master did not retry the request");
    TestHarness::assert_elapsed(sent, after);
    sent
}

async fn assert_no_write_for(harness: &mut TestHarness, duration: Duration) {
    let next = tokio::time::timeout(duration, harness.io.next_event()).await;
    assert_eq!(next.ok(), None);
}

/// An auto task whose first request goes unanswered
struct RetryCase {
    config: AssociationConfig,
    /// IIN reported in an unsolicited null response to start the task, when the config alone
    /// doesn't
    trigger: Option<Iin>,
    /// The request that times out and is retried
    request: fn(Sequence) -> Vec<u8>,
    /// The requests that complete the task after the retry is answered
    then: &'static [fn(Sequence) -> Vec<u8>],
}

fn class_1_scan_request(seq: Sequence) -> Vec<u8> {
    class_scan_request(
        Classes::new(false, EventClasses::new(true, false, false)),
        seq,
    )
}

fn retry_cases() -> [RetryCase; 4] {
    [
        RetryCase {
            config: AssociationConfig {
                startup_integrity_classes: Classes::all(),
                ..AssociationConfig::quiet()
            },
            trigger: None,
            request: integrity_poll_request,
            then: &[],
        },
        RetryCase {
            config: AssociationConfig {
                event_scan_on_events_available: EventClasses::all(),
                ..AssociationConfig::quiet()
            },
            trigger: Some(CLASS_1_EVENTS),
            request: class_1_scan_request,
            then: &[],
        },
        RetryCase {
            config: AssociationConfig {
                enable_unsol_classes: EventClasses::all(),
                ..AssociationConfig::quiet()
            },
            trigger: None,
            request: enable_unsol_request,
            then: &[],
        },
        RetryCase {
            config: AssociationConfig {
                auto_time_sync: Some(TimeSyncProcedure::Lan),
                ..AssociationConfig::quiet()
            },
            trigger: Some(NEED_TIME),
            request: record_current_time,
            then: &[write_last_recorded_time],
        },
    ]
}

#[tokio::test(start_paused = true)]
async fn auto_tasks_retry_after_the_backoff_on_timeout() {
    watchdog(async {
        for case in retry_cases() {
            let mut seq = Sequence::default();
            let mut harness =
                create_association_with_handler(case.config, Box::new(FixedTime)).await;

            if let Some(iin) = case.trigger {
                harness
                    .read_and_expect_write(
                        unsol_null_custom_iin(Sequence::default(), iin),
                        unsol_confirm(Sequence::default()),
                    )
                    .await;
            }

            let sent = expect_retry_after(
                &mut harness,
                (case.request)(seq.increment()),
                (case.request)(seq),
                TIMEOUT_AND_FIRST_RETRY,
            )
            .await;
            harness
                .process_response(empty_response(seq.increment()))
                .await;
            for request in case.then {
                harness
                    .expect_write_and_respond(request(seq), empty_response(seq.increment()))
                    .await;
            }
            // the rest of the task follows the retry without delay
            TestHarness::assert_elapsed(sent, TIMEOUT_AND_FIRST_RETRY);

            assert_no_write_for(&mut harness, Duration::from_secs(60)).await;
        }
    })
    .await;
}

#[tokio::test]
async fn clear_restart_backoff_grows_on_timeouts_and_resets_after_success() {
    watchdog(async {
        tokio::time::pause();
        let mut seq = Sequence::default();
        let mut unsol_seq = Sequence::default();
        let mut harness = create_association(AssociationConfig::quiet()).await;

        harness
            .read_and_expect_write(unsol_null(unsol_seq, true), unsol_confirm(unsol_seq))
            .await;
        unsol_seq.increment();

        // a timeout, then the minimum retry delay
        expect_retry_after(
            &mut harness,
            clear_restart_iin(seq.increment()),
            clear_restart_iin(seq),
            TIMEOUT_AND_FIRST_RETRY,
        )
        .await;
        seq.increment();
        // the retry times out too, and the delay doubles
        let sent = Instant::now();
        harness.expect_write(clear_restart_iin(seq)).await;
        TestHarness::assert_elapsed(sent, Duration::from_secs(3));
        harness
            .process_response(empty_response(seq.increment()))
            .await;
        assert_no_write_for(&mut harness, Duration::from_secs(60)).await;

        // a new restart starts over from the minimum retry delay
        harness
            .read_and_expect_write(unsol_null(unsol_seq, true), unsol_confirm(unsol_seq))
            .await;
        expect_retry_after(
            &mut harness,
            clear_restart_iin(seq.increment()),
            clear_restart_iin(seq),
            TIMEOUT_AND_FIRST_RETRY,
        )
        .await;
        harness.process_response(empty_response(seq)).await;

        assert_no_write_for(&mut harness, Duration::from_secs(60)).await;
    })
    .await;
}

#[tokio::test]
async fn clear_restart_rejected_without_restart_iin_is_not_repeated() {
    watchdog(async {
        tokio::time::pause();
        let mut seq = Sequence::default();
        let mut harness = create_association(AssociationConfig::quiet()).await;

        harness
            .read_and_expect_write(
                unsol_null(Sequence::default(), true),
                unsol_confirm(Sequence::default()),
            )
            .await;

        expect_write_soon(&mut harness, clear_restart_iin(seq)).await;
        harness
            .process_response(empty_response_custom_iin(
                seq.increment(),
                Iin::new(Iin1::default(), Iin2::NO_FUNC_CODE_SUPPORT),
            ))
            .await;

        assert_no_write_for(&mut harness, Duration::from_secs(60)).await;
    })
    .await;
}

const DEVICE_RESTART: Iin = Iin::new(Iin1::new(0x80), Iin2::new(0x00));

/// A response with one binary input event, reporting `iin`
fn event_response(seq: Sequence, iin: Iin) -> Vec<u8> {
    let mut response = empty_response_custom_iin(seq, iin);
    // g2v1 with an 8-bit count of 1, index 0
    response.extend_from_slice(&[2, 1, 0x17, 1, 0, 0x81]);
    response
}

/// An association that scans every event class, after the outstation reports class 1 events
async fn class_1_events_reported() -> TestHarness {
    let config = AssociationConfig {
        event_scan_on_events_available: EventClasses::all(),
        ..AssociationConfig::quiet()
    };
    let mut harness = create_association(config).await;
    harness
        .read_and_expect_write(
            unsol_null_custom_iin(Sequence::default(), CLASS_1_EVENTS),
            unsol_confirm(Sequence::default()),
        )
        .await;
    harness
}

#[tokio::test(start_paused = true)]
async fn event_scan_whose_response_still_reports_events_runs_again_at_once() {
    watchdog(async {
        let mut seq = Sequence::default();
        let mut harness = class_1_events_reported().await;

        // the response to the scan returns events, and reports that more class 1 events are available
        let class1 = Classes::new(false, EventClasses::new(true, false, false));
        expect_write_soon(&mut harness, class_scan_request(class1, seq)).await;
        harness
            .process_response(event_response(seq.increment(), CLASS_1_EVENTS))
            .await;
        let answered = Instant::now();

        // the scan runs again at once, reading the classes the last response reported
        expect_write_soon(&mut harness, class_scan_request(class1, seq)).await;
        TestHarness::assert_elapsed(answered, Duration::ZERO);
        harness
            .process_response(event_response(seq.increment(), CLASS_2_EVENTS))
            .await;
        let class2 = Classes::new(false, EventClasses::new(false, true, false));
        expect_write_soon(&mut harness, class_scan_request(class2, seq)).await;
        harness
            .process_response(empty_response(seq.increment()))
            .await;

        assert_no_write_for(&mut harness, Duration::from_secs(60)).await;
    })
    .await;
}

/// Expect the request with the next sequence, and answer it with an empty response carrying `iin`
async fn answer(
    harness: &mut TestHarness,
    seq: &mut Sequence,
    request: fn(Sequence) -> Vec<u8>,
    iin: Iin,
) {
    expect_write_soon(harness, request(*seq)).await;
    harness
        .process_response(empty_response_custom_iin(seq.increment(), iin))
        .await;
}

#[tokio::test(start_paused = true)]
async fn event_scan_whose_response_reports_events_but_returns_none_is_not_repeated() {
    watchdog(async {
        let mut seq = Sequence::default();
        let mut harness = class_1_events_reported().await;

        // the outstation keeps the class 1 bit set, but returns no events
        let class1 = Classes::new(false, EventClasses::new(true, false, false));
        expect_write_soon(&mut harness, class_scan_request(class1, seq)).await;
        harness
            .process_response(empty_response_custom_iin(seq.increment(), CLASS_1_EVENTS))
            .await;

        assert_no_write_for(&mut harness, Duration::from_secs(60)).await;
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn event_scan_rejected_with_an_iin2_request_error_is_not_repeated() {
    watchdog(async {
        let mut seq = Sequence::default();
        let mut harness = class_1_events_reported().await;

        // the outstation returns events and reports more, but also rejects part of the request
        let class1 = Classes::new(false, EventClasses::new(true, false, false));
        let object_unknown = Iin::new(CLASS_1_EVENTS.iin1, Iin2::OBJECT_UNKNOWN);
        expect_write_soon(&mut harness, class_scan_request(class1, seq)).await;
        harness
            .process_response(event_response(seq.increment(), object_unknown))
            .await;

        assert_no_write_for(&mut harness, Duration::from_secs(60)).await;
    })
    .await;
}

/// One fragment of a multi-fragment response with one binary input event, requesting confirmation
/// unless it's the last
fn event_fragment(seq: Sequence, fir: bool, fin: bool, iin: Iin) -> Vec<u8> {
    let mut response = event_response(seq, iin);
    response[0] = ControlField::response(seq, fir, fin, !fin).to_u8();
    response
}

#[tokio::test(start_paused = true)]
async fn event_scan_with_a_request_error_in_an_earlier_fragment_is_not_repeated() {
    watchdog(async {
        let mut seq = Sequence::default();
        let mut harness = class_1_events_reported().await;

        // the first fragment rejects part of the request, and the last reports more events
        let class1 = Classes::new(false, EventClasses::new(true, false, false));
        let parameter_error = Iin::new(CLASS_1_EVENTS.iin1, Iin2::PARAMETER_ERROR);
        expect_write_soon(&mut harness, class_scan_request(class1, seq)).await;
        let first = seq.increment();
        harness
            .process_response(event_fragment(first, true, false, parameter_error))
            .await;
        expect_write_soon(&mut harness, solicited_confirm(first)).await;
        harness
            .process_response(event_fragment(seq.increment(), false, true, CLASS_1_EVENTS))
            .await;

        assert_no_write_for(&mut harness, Duration::from_secs(60)).await;
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn event_scan_with_iin2_bits_other_than_request_errors_runs_again_at_once() {
    watchdog(async {
        let mut seq = Sequence::default();
        let mut harness = class_1_events_reported().await;

        // the outstation returns events and reports more, along with a corrupt configuration
        let class1 = Classes::new(false, EventClasses::new(true, false, false));
        let config_corrupt = Iin::new(CLASS_1_EVENTS.iin1, Iin2::CONFIG_CORRUPT);
        expect_write_soon(&mut harness, class_scan_request(class1, seq)).await;
        harness
            .process_response(event_response(seq.increment(), config_corrupt))
            .await;
        let answered = Instant::now();

        expect_write_soon(&mut harness, class_scan_request(class1, seq)).await;
        TestHarness::assert_elapsed(answered, Duration::ZERO);
        harness
            .process_response(empty_response(seq.increment()))
            .await;

        assert_no_write_for(&mut harness, Duration::from_secs(60)).await;
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn restart_iin_in_integrity_or_enable_unsolicited_response_starts_the_sequence_over() {
    watchdog(async {
        let config = AssociationConfig {
            startup_integrity_classes: Classes::all(),
            enable_unsol_classes: EventClasses::all(),
            ..AssociationConfig::quiet()
        };
        let mut seq = Sequence::default();
        let mut harness = create_association(config).await;
        let none = Iin::default();

        // the outstation reports a restart in its response to the startup integrity scan
        answer(
            &mut harness,
            &mut seq,
            integrity_poll_request,
            DEVICE_RESTART,
        )
        .await;
        answer(&mut harness, &mut seq, clear_restart_iin, none).await;
        // and again in its response to ENABLE_UNSOLICITED
        answer(&mut harness, &mut seq, integrity_poll_request, none).await;
        answer(&mut harness, &mut seq, enable_unsol_request, DEVICE_RESTART).await;
        answer(&mut harness, &mut seq, clear_restart_iin, none).await;
        answer(&mut harness, &mut seq, integrity_poll_request, none).await;
        answer(&mut harness, &mut seq, enable_unsol_request, none).await;

        assert_no_write_for(&mut harness, Duration::from_secs(60)).await;
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn clear_restart_answered_with_restart_iin_reports_failure_and_retries_after_the_backoff() {
    watchdog(async {
        let mut seq = Sequence::default();
        let mut harness = create_association(AssociationConfig::quiet()).await;

        harness
            .read_and_expect_write(
                unsol_null(Sequence::default(), true),
                unsol_confirm(Sequence::default()),
            )
            .await;

        // the outstation accepts the clear restart, but still reports the restart
        expect_write_soon(&mut harness, clear_restart_iin(seq)).await;
        harness
            .process_response(empty_response_custom_iin(seq.increment(), DEVICE_RESTART))
            .await;

        let failed = Instant::now();
        harness
            .expect_write_and_respond(clear_restart_iin(seq), empty_response(seq.increment()))
            .await;
        TestHarness::assert_elapsed(failed, Duration::from_secs(1));
        assert_no_write_for(&mut harness, Duration::from_secs(60)).await;

        assert_eq!(
            harness.assoc_events.pop(),
            vec![
                AssocInfoEvent::Unsolicited(false, Sequence::new(0)),
                AssocInfoEvent::TaskStart(
                    TaskType::ClearRestartBit,
                    FunctionCode::Write,
                    Sequence::new(0)
                ),
                AssocInfoEvent::TaskFailure(
                    TaskType::ClearRestartBit,
                    TaskError::UnexpectedResponseHeaders
                ),
                AssocInfoEvent::TaskStart(
                    TaskType::ClearRestartBit,
                    FunctionCode::Write,
                    Sequence::new(1)
                ),
                AssocInfoEvent::TaskSuccess(
                    TaskType::ClearRestartBit,
                    FunctionCode::Write,
                    Sequence::new(1)
                ),
            ]
        );
    })
    .await;
}
