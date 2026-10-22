use std::time::Duration;

use tokio::time::Instant;

use crate::app::Sequence;
use crate::app::{Iin, Iin1, Iin2};
use crate::master::association::AssociationConfig;
use crate::master::request::EventClasses;
use crate::master::{Classes, TaskError, TimeSyncProcedure};

use super::harness::requests::*;
use super::harness::{create_association, TestHarness};

const BUFFER_OVERFLOW: Iin = Iin::new(Iin1::new(0x00), Iin2::new(0x08));
const CLASS_1_EVENTS: Iin = Iin::new(Iin1::new(0x02), Iin2::new(0x00));

#[tokio::test]
async fn auto_integrity_scan_on_buffer_overflow() {
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
}

#[tokio::test]
async fn auto_integrity_scan_on_buffer_overflow_disabled() {
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
}

#[tokio::test]
async fn auto_event_class_scan() {
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
}

#[tokio::test]
async fn auto_event_class_ignore_one_class_scan() {
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
}

#[tokio::test]
async fn auto_event_class_scan_disabled() {
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
    tokio::time::pause();
    let mut seq = Sequence::default();
    let mut harness = create_association(AssociationConfig::quiet()).await;

    cold_restart_rejected_with(&mut harness, &mut seq, RESTART_AND_NO_FUNC_CODE_SUPPORT).await;

    expect_write_soon(&mut harness, clear_restart_iin(seq)).await;
    harness
        .process_response(empty_response(seq.increment()))
        .await;
}

#[tokio::test]
async fn need_time_iin_in_a_rejected_response_schedules_time_sync() {
    tokio::time::pause();
    let config = AssociationConfig {
        auto_time_sync: Some(TimeSyncProcedure::Lan),
        ..AssociationConfig::quiet()
    };
    let mut seq = Sequence::default();
    let mut harness = create_association(config).await;

    cold_restart_rejected_with(&mut harness, &mut seq, NEED_TIME_AND_NO_FUNC_CODE_SUPPORT).await;

    expect_write_soon(&mut harness, vec![0xC0 | seq.value(), RECORD_CURRENT_TIME]).await;
}

#[tokio::test]
async fn rejected_clear_restart_retries_after_the_backoff() {
    tokio::time::pause();
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
    let elapsed = failed.elapsed();
    assert!(elapsed >= Duration::from_secs(1), "{elapsed:?}");
    harness.assert_no_events();
}

#[tokio::test]
async fn rejected_enable_unsolicited_with_restart_iin_is_not_repeated() {
    tokio::time::pause();
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

    // the rejected ENABLE_UNSOLICITED isn't sent again
    let next = tokio::time::timeout(Duration::from_secs(5), harness.io.next_event()).await;
    assert_eq!(next.ok(), None);
}
