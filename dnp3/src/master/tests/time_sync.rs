// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use scursor::WriteCursor;
use tokio::time::Instant;

use crate::app::format::write::{start_request, start_response};
use crate::app::variations::{Group50Var1, Group50Var3, Group52Var2};
use crate::app::Timestamp;
use crate::app::{
    ControlField, FunctionCode, Iin, Iin1, Iin2, ObjectParseError, ResponseFunction, Sequence,
};
use crate::master::association::AssociationConfig;
use crate::master::tests::harness::AssocInfoEvent;
use crate::master::{
    AssociationHandler, Classes, EventClasses, PollHandle, ReadRequest, TaskError, TaskType,
    TimeSyncError, TimeSyncProcedure,
};

use super::harness::requests::*;
use super::harness::{create_association_with_handler, TestHarness};
use crate::util::test_support::watchdog;

const SYSTEM_TIME: Timestamp = Timestamp::new(1_700_000_000_000);
const NEED_TIME: Iin = Iin::new(Iin1::new(0x10), Iin2::new(0x00));

struct FixedTime;

impl AssociationHandler for FixedTime {
    fn get_current_time(&self) -> Option<Timestamp> {
        Some(SYSTEM_TIME)
    }
}

/// Never has the time, and counts how often it is asked for it
struct NoTime(Arc<AtomicUsize>);

impl AssociationHandler for NoTime {
    fn get_current_time(&self) -> Option<Timestamp> {
        self.0.fetch_add(1, Ordering::SeqCst);
        None
    }
}

fn config(auto_time_sync: Option<TimeSyncProcedure>) -> AssociationConfig {
    AssociationConfig {
        auto_time_sync,
        ..AssociationConfig::quiet()
    }
}

async fn harness(auto_time_sync: Option<TimeSyncProcedure>) -> TestHarness {
    tokio::time::pause();
    create_association_with_handler(config(auto_time_sync), Box::new(FixedTime)).await
}

fn empty_request(function: FunctionCode, seq: Sequence) -> Vec<u8> {
    let mut buffer = [0; 2];
    let mut cursor = WriteCursor::new(&mut buffer);
    start_request(ControlField::request(seq), function, &mut cursor).unwrap();
    cursor.written().to_vec()
}

fn record_current_time(seq: Sequence) -> Vec<u8> {
    empty_request(FunctionCode::RecordCurrentTime, seq)
}

fn delay_measure(seq: Sequence) -> Vec<u8> {
    empty_request(FunctionCode::DelayMeasure, seq)
}

fn write_last_recorded_time(seq: Sequence) -> Vec<u8> {
    let mut buffer = [0; 32];
    let mut cursor = WriteCursor::new(&mut buffer);
    let mut writer =
        start_request(ControlField::request(seq), FunctionCode::Write, &mut cursor).unwrap();
    writer
        .write_count_of_one(Group50Var3 { time: SYSTEM_TIME })
        .unwrap();
    cursor.written().to_vec()
}

fn write_absolute_time(seq: Sequence) -> Vec<u8> {
    let mut buffer = [0; 32];
    let mut cursor = WriteCursor::new(&mut buffer);
    let mut writer =
        start_request(ControlField::request(seq), FunctionCode::Write, &mut cursor).unwrap();
    writer
        .write_count_of_one(Group50Var1 { time: SYSTEM_TIME })
        .unwrap();
    cursor.written().to_vec()
}

fn delay_measure_response(seq: Sequence, delay_ms: u16) -> Vec<u8> {
    let mut buffer = [0; 32];
    let mut cursor = WriteCursor::new(&mut buffer);
    let mut writer = start_response(
        ControlField::response(seq, true, true, false),
        ResponseFunction::Response,
        Iin::default(),
        &mut cursor,
    )
    .unwrap();
    writer
        .write_count_of_one(Group52Var2 { time: delay_ms })
        .unwrap();
    cursor.written().to_vec()
}

/// Report NEED_TIME in an unsolicited null response
async fn report_need_time(harness: &mut TestHarness, unsol_seq: Sequence) {
    harness
        .read_and_expect_write(
            unsol_null_custom_iin(unsol_seq, NEED_TIME),
            unsol_confirm(unsol_seq),
        )
        .await;
}

async fn assert_no_write_for(harness: &mut TestHarness, duration: Duration) {
    let next = tokio::time::timeout(duration, harness.io.next_event()).await;
    assert_eq!(next.ok(), None);
}

async fn complete_lan_procedure(harness: &mut TestHarness, seq: &mut Sequence) {
    tokio::time::timeout(
        Duration::from_secs(60),
        harness.expect_write(record_current_time(*seq)),
    )
    .await
    .expect("the master did not start the time sync");
    harness
        .process_response(empty_response(seq.increment()))
        .await;
    harness
        .expect_write_and_respond(
            write_last_recorded_time(*seq),
            empty_response(seq.increment()),
        )
        .await;
}

#[tokio::test]
async fn auto_lan_time_sync_completes_and_is_not_repeated() {
    watchdog(async {
        let mut harness = harness(Some(TimeSyncProcedure::Lan)).await;
        let mut seq = Sequence::default();

        report_need_time(&mut harness, Sequence::default()).await;
        complete_lan_procedure(&mut harness, &mut seq).await;

        assert_eq!(
            harness.assoc_events.pop(),
            vec![
                AssocInfoEvent::Unsolicited(false, Sequence::new(0)),
                AssocInfoEvent::TaskStart(
                    TaskType::TimeSync,
                    FunctionCode::RecordCurrentTime,
                    Sequence::new(0)
                ),
                AssocInfoEvent::TaskSuccess(
                    TaskType::TimeSync,
                    FunctionCode::RecordCurrentTime,
                    Sequence::new(1)
                ),
            ]
        );
        assert_no_write_for(&mut harness, Duration::from_secs(60)).await;
    })
    .await;
}

#[tokio::test]
async fn auto_non_lan_time_sync_completes_and_is_not_repeated() {
    watchdog(async {
        let mut harness = harness(Some(TimeSyncProcedure::NonLan)).await;
        let mut seq = Sequence::default();

        report_need_time(&mut harness, Sequence::default()).await;
        // the paused clock means no time elapses between the request and the response
        harness
            .expect_write_and_respond(
                delay_measure(seq),
                delay_measure_response(seq.increment(), 0),
            )
            .await;
        harness
            .expect_write_and_respond(write_absolute_time(seq), empty_response(seq.increment()))
            .await;

        assert_eq!(
            harness.assoc_events.pop(),
            vec![
                AssocInfoEvent::Unsolicited(false, Sequence::new(0)),
                AssocInfoEvent::TaskStart(
                    TaskType::TimeSync,
                    FunctionCode::DelayMeasure,
                    Sequence::new(0)
                ),
                AssocInfoEvent::TaskSuccess(
                    TaskType::TimeSync,
                    FunctionCode::DelayMeasure,
                    Sequence::new(1)
                ),
            ]
        );
        assert_no_write_for(&mut harness, Duration::from_secs(60)).await;
    })
    .await;
}

#[tokio::test]
async fn auto_time_sync_retries_after_the_backoff_when_the_outstation_still_needs_time() {
    watchdog(async {
        let mut harness = harness(Some(TimeSyncProcedure::Lan)).await;
        let mut seq = Sequence::default();

        report_need_time(&mut harness, Sequence::default()).await;
        harness
            .expect_write_and_respond(record_current_time(seq), empty_response(seq.increment()))
            .await;
        harness
            .expect_write_and_respond(
                write_last_recorded_time(seq),
                empty_response_custom_iin(seq.increment(), NEED_TIME),
            )
            .await;
        let failed = Instant::now();

        complete_lan_procedure(&mut harness, &mut seq).await;
        TestHarness::assert_elapsed(failed, Duration::from_secs(1));

        assert_no_write_for(&mut harness, Duration::from_secs(60)).await;
    })
    .await;
}

#[tokio::test]
async fn auto_time_sync_retries_after_the_backoff_on_timeout() {
    watchdog(async {
        let mut harness = harness(Some(TimeSyncProcedure::Lan)).await;
        let mut seq = Sequence::default();

        report_need_time(&mut harness, Sequence::default()).await;
        harness
            .expect_write(record_current_time(seq.increment()))
            .await;
        let sent = Instant::now();

        // the response timeout, then the minimum retry delay
        complete_lan_procedure(&mut harness, &mut seq).await;
        TestHarness::assert_elapsed(sent, Duration::from_secs(2));

        assert_no_write_for(&mut harness, Duration::from_secs(60)).await;
    })
    .await;
}

#[tokio::test]
async fn auto_time_sync_without_system_time_reports_failure_and_retries_with_a_growing_backoff() {
    watchdog(async {
        tokio::time::pause();
        let count = Arc::new(AtomicUsize::new(0));
        let mut harness = create_association_with_handler(
            config(Some(TimeSyncProcedure::Lan)),
            Box::new(NoTime(count.clone())),
        )
        .await;

        report_need_time(&mut harness, Sequence::default()).await;
        let start = Instant::now();

        // asked at once, then after 1, 2 and 4 more seconds, and each attempt is reported as a
        // task that fails without sending a request
        let attempt = [
            AssocInfoEvent::TaskStart(
                TaskType::TimeSync,
                FunctionCode::RecordCurrentTime,
                Sequence::new(0),
            ),
            AssocInfoEvent::TaskFailure(TaskType::TimeSync, TaskError::SystemTimeNotAvailable),
        ];
        let mut events = vec![AssocInfoEvent::Unsolicited(false, Sequence::new(0))];
        for (at_ms, expected) in [(500, 1), (1500, 2), (3500, 3), (7500, 4)] {
            tokio::time::sleep_until(start + Duration::from_millis(at_ms)).await;
            assert_eq!(count.load(Ordering::SeqCst), expected, "at {at_ms} ms");
            events.extend(attempt);
            assert_eq!(harness.assoc_events.pop(), events, "at {at_ms} ms");
            events.clear();
        }

        harness.assert_no_events();
    })
    .await;
}

#[tokio::test]
async fn user_lan_time_sync_completes_the_request() {
    watchdog(async {
        let mut harness = harness(None).await;
        let mut seq = Sequence::default();

        let mut association = harness.association.clone();
        let sync =
            tokio::spawn(async move { association.synchronize_time(TimeSyncProcedure::Lan).await });
        complete_lan_procedure(&mut harness, &mut seq).await;

        assert_eq!(sync.await.unwrap(), Ok(()));
        assert_no_write_for(&mut harness, Duration::from_secs(60)).await;
    })
    .await;
}

#[tokio::test]
async fn user_non_lan_time_sync_completes_the_request() {
    watchdog(async {
        let mut harness = harness(None).await;
        let mut seq = Sequence::default();

        let mut association = harness.association.clone();
        let sync = tokio::spawn(async move {
            association
                .synchronize_time(TimeSyncProcedure::NonLan)
                .await
        });
        harness
            .expect_write_and_respond(
                delay_measure(seq),
                delay_measure_response(seq.increment(), 0),
            )
            .await;
        harness
            .expect_write_and_respond(write_absolute_time(seq), empty_response(seq.increment()))
            .await;

        assert_eq!(sync.await.unwrap(), Ok(()));
        assert_no_write_for(&mut harness, Duration::from_secs(60)).await;
    })
    .await;
}

/// Objects of delay measure responses that don't hold exactly one g52v2
const BAD_DELAYS: [&[u8]; 5] = [
    // g52v1 (coarse, in seconds) with a count of one
    &[52, 1, 0x07, 1, 0x00, 0x00],
    // g52v2 with a count of zero
    &[52, 2, 0x07, 0],
    // g52v2 with a count of two
    &[52, 2, 0x07, 2, 0x00, 0x00, 0x00, 0x00],
    // two g52v2 headers, each with a count of one
    &[52, 2, 0x07, 1, 0x00, 0x00, 52, 2, 0x07, 1, 0x00, 0x00],
    // a CTO instead of a time delay
    &[51, 1, 0x07, 1, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
];

#[tokio::test(start_paused = true)]
async fn non_lan_time_sync_fails_without_a_write_if_the_delay_is_not_one_g52v2() {
    watchdog(async {
        let mut harness = create_association_with_handler(config(None), Box::new(FixedTime)).await;
        let mut seq = Sequence::default();

        for objects in BAD_DELAYS {
            let mut association = harness.association.clone();
            let sync = tokio::spawn(async move {
                association
                    .synchronize_time(TimeSyncProcedure::NonLan)
                    .await
            });
            // the delay measure is the next write, so nothing followed the last failure
            harness.expect_write(delay_measure(seq)).await;
            let mut response = empty_response(seq.increment());
            response.extend_from_slice(objects);
            harness.process_response(response).await;

            assert_eq!(
                sync.await.unwrap(),
                Err(TimeSyncError::Task(TaskError::UnexpectedResponseHeaders)),
                "{objects:?}"
            );
        }

        // a valid delay measure is answered with a WRITE
        let mut association = harness.association.clone();
        let sync = tokio::spawn(async move {
            association
                .synchronize_time(TimeSyncProcedure::NonLan)
                .await
        });
        harness
            .expect_write_and_respond(
                delay_measure(seq),
                delay_measure_response(seq.increment(), 0),
            )
            .await;
        harness
            .expect_write_and_respond(write_absolute_time(seq), empty_response(seq.increment()))
            .await;
        assert_eq!(sync.await.unwrap(), Ok(()));
    })
    .await;
}

#[tokio::test]
async fn user_time_sync_timeout_is_reported_to_the_caller_and_not_retried() {
    watchdog(async {
        let mut harness = harness(Some(TimeSyncProcedure::Lan)).await;

        let mut association = harness.association.clone();
        let sync =
            tokio::spawn(async move { association.synchronize_time(TimeSyncProcedure::Lan).await });
        harness
            .expect_write(record_current_time(Sequence::default()))
            .await;

        assert_eq!(
            sync.await.unwrap(),
            Err(TimeSyncError::Task(TaskError::ResponseTimeout))
        );
        assert_no_write_for(&mut harness, Duration::from_secs(60)).await;
    })
    .await;
}

#[tokio::test]
async fn user_time_sync_without_system_time_fails_without_scheduling_a_retry() {
    watchdog(async {
        tokio::time::pause();
        let count = Arc::new(AtomicUsize::new(0));
        let mut harness = create_association_with_handler(
            config(Some(TimeSyncProcedure::Lan)),
            Box::new(NoTime(count.clone())),
        )
        .await;

        let result = harness
            .association
            .synchronize_time(TimeSyncProcedure::Lan)
            .await;
        assert_eq!(result, Err(TimeSyncError::SystemTimeNotAvailable));

        tokio::time::sleep(Duration::from_secs(60)).await;
        assert_eq!(count.load(Ordering::SeqCst), 1);
        harness.assert_no_events();
    })
    .await;
}

#[tokio::test]
async fn user_time_sync_that_still_needs_time_fails_and_starts_the_auto_time_sync() {
    watchdog(async {
        let mut harness = harness(Some(TimeSyncProcedure::Lan)).await;
        let mut seq = Sequence::default();

        let mut association = harness.association.clone();
        let sync =
            tokio::spawn(async move { association.synchronize_time(TimeSyncProcedure::Lan).await });
        harness
            .expect_write_and_respond(record_current_time(seq), empty_response(seq.increment()))
            .await;
        harness
            .expect_write_and_respond(
                write_last_recorded_time(seq),
                empty_response_custom_iin(seq.increment(), NEED_TIME),
            )
            .await;
        assert_eq!(sync.await.unwrap(), Err(TimeSyncError::StillNeedsTime));

        // the NEED_TIME bit in the response schedules the automatic time sync right away
        let failed = Instant::now();
        complete_lan_procedure(&mut harness, &mut seq).await;
        TestHarness::assert_elapsed(failed, Duration::ZERO);

        assert_eq!(
            harness.assoc_events.pop(),
            vec![
                AssocInfoEvent::TaskStart(
                    TaskType::TimeSync,
                    FunctionCode::RecordCurrentTime,
                    Sequence::new(0)
                ),
                AssocInfoEvent::TaskFailure(
                    TaskType::TimeSync,
                    TaskError::UnexpectedResponseHeaders
                ),
                AssocInfoEvent::TaskStart(
                    TaskType::TimeSync,
                    FunctionCode::RecordCurrentTime,
                    Sequence::new(2)
                ),
                AssocInfoEvent::TaskSuccess(
                    TaskType::TimeSync,
                    FunctionCode::RecordCurrentTime,
                    Sequence::new(3)
                ),
            ]
        );
        assert_no_write_for(&mut harness, Duration::from_secs(60)).await;
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn user_non_lan_time_sync_fails_with_the_cause_when_the_delay_measure_response_does_not_parse(
) {
    watchdog(async {
        let cases: [(&[u8], ObjectParseError); 2] = [
            // g52v2 cut short after its qualifier
            (&[52, 2, 0x07], ObjectParseError::InsufficientBytes),
            // a variation of g52 that doesn't exist
            (
                &[52, 9, 0x07, 1, 0x00, 0x00],
                ObjectParseError::UnknownGroupVariation(52, 9),
            ),
        ];
        for (objects, cause) in cases {
            let mut harness =
                create_association_with_handler(config(None), Box::new(FixedTime)).await;
            let seq = Sequence::default();

            let mut association = harness.association.clone();
            let sync = tokio::spawn(async move {
                association
                    .synchronize_time(TimeSyncProcedure::NonLan)
                    .await
            });
            let mut response = empty_response(seq);
            response.extend_from_slice(objects);
            harness
                .expect_write_and_respond(delay_measure(seq), response)
                .await;

            let err = TaskError::MalformedResponse(cause);
            assert_eq!(
                sync.await.unwrap(),
                Err(TimeSyncError::Task(err)),
                "{objects:?}"
            );
            assert_no_write_for(&mut harness, Duration::from_secs(60)).await;
            assert_eq!(
                harness.assoc_events.pop(),
                [
                    AssocInfoEvent::TaskStart(TaskType::TimeSync, FunctionCode::DelayMeasure, seq),
                    AssocInfoEvent::TaskFailure(TaskType::TimeSync, err),
                ],
                "{objects:?}"
            );
        }
    })
    .await;
}

/// A WRITE response with objects that parse is rejected as unexpected, and one with objects that
/// don't parse as malformed, with the cause
#[tokio::test(start_paused = true)]
async fn user_time_sync_fails_when_the_write_response_has_objects() {
    watchdog(async {
        let cases: [(&[u8], TaskError); 3] = [
            // a g52v2 time delay
            (
                &[52, 2, 0x07, 1, 0x00, 0x00],
                TaskError::UnexpectedResponseHeaders,
            ),
            // a header cut short after its group
            (
                &[0x01],
                TaskError::MalformedResponse(ObjectParseError::InsufficientBytes),
            ),
            // a variation of g50 that doesn't exist
            (
                &[50, 9, 0x07, 1],
                TaskError::MalformedResponse(ObjectParseError::UnknownGroupVariation(50, 9)),
            ),
        ];
        let procedures = [
            TimeSyncProcedure::Lan,
            TimeSyncProcedure::NonLan,
            TimeSyncProcedure::DirectWriteAbsTime,
        ];
        for procedure in procedures {
            for (objects, err) in cases {
                let mut harness =
                    create_association_with_handler(config(None), Box::new(FixedTime)).await;
                let mut seq = Sequence::default();

                let mut association = harness.association.clone();
                let sync =
                    tokio::spawn(async move { association.synchronize_time(procedure).await });
                let (first, write) = match procedure {
                    TimeSyncProcedure::Lan => {
                        harness
                            .expect_write_and_respond(
                                record_current_time(seq),
                                empty_response(seq.increment()),
                            )
                            .await;
                        (
                            FunctionCode::RecordCurrentTime,
                            write_last_recorded_time(seq),
                        )
                    }
                    TimeSyncProcedure::NonLan => {
                        harness
                            .expect_write_and_respond(
                                delay_measure(seq),
                                delay_measure_response(seq.increment(), 0),
                            )
                            .await;
                        (FunctionCode::DelayMeasure, write_absolute_time(seq))
                    }
                    TimeSyncProcedure::DirectWriteAbsTime => {
                        (FunctionCode::Write, write_absolute_time(seq))
                    }
                };
                let mut response = empty_response(seq);
                response.extend_from_slice(objects);
                harness.expect_write_and_respond(write, response).await;

                assert_eq!(
                    sync.await.unwrap(),
                    Err(TimeSyncError::Task(err)),
                    "{procedure:?} {objects:?}"
                );
                assert_no_write_for(&mut harness, Duration::from_secs(60)).await;
                assert_eq!(
                    harness.assoc_events.pop(),
                    [
                        AssocInfoEvent::TaskStart(TaskType::TimeSync, first, Sequence::new(0)),
                        AssocInfoEvent::TaskFailure(TaskType::TimeSync, err),
                    ],
                    "{procedure:?} {objects:?}"
                );
            }
        }
    })
    .await;
}

const DEVICE_RESTART_AND_NEED_TIME: Iin = Iin::new(Iin1::new(0x90), Iin2::new(0x00));

/// The default startup procedure with the automatic LAN time sync
async fn startup_harness() -> TestHarness {
    let config = AssociationConfig {
        auto_time_sync: Some(TimeSyncProcedure::Lan),
        ..AssociationConfig::default()
    };
    create_association_with_handler(config, Box::new(FixedTime)).await
}

#[tokio::test(start_paused = true)]
async fn auto_time_sync_runs_before_the_integrity_scan_at_startup() {
    watchdog(async {
        let mut harness = startup_harness().await;
        let mut seq = Sequence::default();

        // the outstation needs the time
        harness
            .expect_write_and_respond(
                disable_unsol_request(seq),
                empty_response_custom_iin(seq.increment(), NEED_TIME),
            )
            .await;
        complete_lan_procedure(&mut harness, &mut seq).await;
        harness
            .expect_write_and_respond(integrity_poll_request(seq), empty_response(seq.increment()))
            .await;
        harness
            .expect_write_and_respond(enable_unsol_request(seq), empty_response(seq.increment()))
            .await;

        assert_no_write_for(&mut harness, Duration::from_secs(60)).await;
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn auto_time_sync_runs_before_the_integrity_scan_after_a_restart() {
    watchdog(async {
        let mut harness = startup_harness().await;
        let mut seq = Sequence::default();
        startup_procedure(&mut harness, &mut seq).await;

        // the outstation restarts and needs the time
        harness
            .read_and_expect_write(
                unsol_null_custom_iin(Sequence::default(), DEVICE_RESTART_AND_NEED_TIME),
                unsol_confirm(Sequence::default()),
            )
            .await;
        harness
            .expect_write_and_respond(clear_restart_iin(seq), empty_response(seq.increment()))
            .await;
        complete_lan_procedure(&mut harness, &mut seq).await;
        harness
            .expect_write_and_respond(integrity_poll_request(seq), empty_response(seq.increment()))
            .await;
        harness
            .expect_write_and_respond(enable_unsol_request(seq), empty_response(seq.increment()))
            .await;

        assert_no_write_for(&mut harness, Duration::from_secs(60)).await;
    })
    .await;
}

const CLASS_1_EVENTS: Iin = Iin::new(Iin1::new(0x02), Iin2::new(0x00));

/// The default startup procedure with the automatic LAN time sync and event scans, and the
/// handler that provides the system time
async fn busy_harness(handler: Box<dyn AssociationHandler>) -> TestHarness {
    let config = AssociationConfig {
        auto_time_sync: Some(TimeSyncProcedure::Lan),
        event_scan_on_events_available: EventClasses::all(),
        ..AssociationConfig::default()
    };
    create_association_with_handler(config, handler).await
}

fn class_1() -> Classes {
    Classes::new(false, EventClasses::new(true, false, false))
}

fn class_3() -> Classes {
    Classes::new(false, EventClasses::new(false, false, true))
}

/// Add a poll of class 3 events
async fn add_class_3_poll(harness: &mut TestHarness, period: Duration) -> PollHandle {
    harness
        .association
        .add_poll(ReadRequest::ClassScan(class_3()), period)
        .await
        .unwrap()
}

/// Answer the startup integrity scan, ENABLE_UNSOLICITED, and the event scan that the response to
/// ENABLE_UNSOLICITED asks for, expecting each request as soon as the previous one is answered
async fn finish_startup(harness: &mut TestHarness, seq: &mut Sequence) {
    harness.expect_write(integrity_poll_request(*seq)).await;
    let answered = Instant::now();
    harness
        .process_response(empty_response(seq.increment()))
        .await;
    harness
        .expect_write_and_respond(
            enable_unsol_request(*seq),
            empty_response_custom_iin(seq.increment(), CLASS_1_EVENTS),
        )
        .await;
    harness
        .expect_write_and_respond(
            class_scan_request(class_1(), *seq),
            empty_response(seq.increment()),
        )
        .await;
    TestHarness::assert_elapsed(answered, Duration::ZERO);
}

#[tokio::test(start_paused = true)]
async fn auto_time_sync_without_system_time_does_not_hold_up_the_association() {
    watchdog(async {
        let count = Arc::new(AtomicUsize::new(0));
        let mut harness = busy_harness(Box::new(NoTime(count.clone()))).await;
        let mut seq = Sequence::default();
        let start = Instant::now();
        let _poll = add_class_3_poll(&mut harness, Duration::from_secs(5)).await;

        // the outstation needs the time, but the master doesn't have it
        harness
            .expect_write_and_respond(
                disable_unsol_request(seq),
                empty_response_custom_iin(seq.increment(), NEED_TIME),
            )
            .await;

        // the time sync fails without a request, and the rest of the startup goes ahead of its
        // retry
        finish_startup(&mut harness, &mut seq).await;
        TestHarness::assert_elapsed(start, Duration::ZERO);
        assert_eq!(count.load(Ordering::SeqCst), 1);

        // the poll runs on its schedule, while the time sync retries on its own after 1 and 2 s
        harness
            .expect_write_and_respond(
                class_scan_request(class_3(), seq),
                empty_response(seq.increment()),
            )
            .await;
        TestHarness::assert_elapsed(start, Duration::from_secs(5));
        assert_eq!(count.load(Ordering::SeqCst), 3);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn auto_time_sync_that_times_out_does_not_hold_up_the_association() {
    watchdog(async {
        let mut harness = busy_harness(Box::new(FixedTime)).await;
        let mut seq = Sequence::default();
        let start = Instant::now();
        let _poll = add_class_3_poll(&mut harness, Duration::from_secs(4)).await;

        // the outstation needs the time, but never answers the time sync
        harness
            .expect_write_and_respond(
                disable_unsol_request(seq),
                empty_response_custom_iin(seq.increment(), NEED_TIME),
            )
            .await;
        harness.expect_write(record_current_time(seq)).await;
        seq.increment();

        // the rest of the startup goes as soon as the time sync times out, ahead of its retry
        finish_startup(&mut harness, &mut seq).await;
        TestHarness::assert_elapsed(start, Duration::from_secs(1));

        // the time sync retries after its 1 s backoff, and times out again
        harness.expect_write(record_current_time(seq)).await;
        seq.increment();
        TestHarness::assert_elapsed(start, Duration::from_secs(2));

        // the poll runs on its schedule while the time sync waits 2 s for its next retry
        harness
            .expect_write_and_respond(
                class_scan_request(class_3(), seq),
                empty_response(seq.increment()),
            )
            .await;
        TestHarness::assert_elapsed(start, Duration::from_secs(4));
        complete_lan_procedure(&mut harness, &mut seq).await;
        TestHarness::assert_elapsed(start, Duration::from_secs(5));
    })
    .await;
}
