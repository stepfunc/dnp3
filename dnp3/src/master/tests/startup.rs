use std::sync::atomic::Ordering;
use std::time::Duration;

use crate::app::Sequence;
use crate::app::{FunctionCode, RetryStrategy};
use crate::app::{Iin, Iin1, Iin2};
use crate::master::association::AssociationConfig;
use crate::master::request::{Classes, EventClasses};
use crate::master::tests::harness::AssocInfoEvent;
use crate::master::{ReadRequest, TaskError, TaskType};

use super::harness::create_association;
use super::harness::requests::*;

#[tokio::test]
async fn master_startup_procedure() {
    let config = AssociationConfig::default();
    let mut seq = Sequence::default();
    let mut harness = create_association(config).await;

    assert_eq!(
        harness.assoc_events.pop().as_slice(),
        &[AssocInfoEvent::TaskStart(
            TaskType::DisableUnsolicited,
            FunctionCode::DisableUnsolicited,
            Sequence::new(0)
        )]
    );

    harness
        .expect_write_and_respond(disable_unsol_request(seq), empty_response(seq.increment()))
        .await;

    assert_eq!(
        harness.assoc_events.pop().as_slice(),
        &[
            AssocInfoEvent::TaskSuccess(
                TaskType::DisableUnsolicited,
                FunctionCode::DisableUnsolicited,
                Sequence::new(0)
            ),
            AssocInfoEvent::TaskStart(
                TaskType::StartupIntegrity,
                FunctionCode::Read,
                Sequence::new(1)
            ),
        ]
    );

    harness
        .expect_write_and_respond(integrity_poll_request(seq), empty_response(seq.increment()))
        .await;

    assert_eq!(
        harness.assoc_events.pop().as_slice(),
        &[
            AssocInfoEvent::TaskSuccess(
                TaskType::StartupIntegrity,
                FunctionCode::Read,
                Sequence::new(1)
            ),
            AssocInfoEvent::TaskStart(
                TaskType::EnableUnsolicited,
                FunctionCode::EnableUnsolicited,
                Sequence::new(2)
            ),
        ]
    );

    harness
        .expect_write_and_respond(enable_unsol_request(seq), empty_response(seq.increment()))
        .await;

    assert_eq!(
        harness.assoc_events.pop().as_slice(),
        &[AssocInfoEvent::TaskSuccess(
            TaskType::EnableUnsolicited,
            FunctionCode::EnableUnsolicited,
            Sequence::new(2)
        ),]
    );
}

#[tokio::test]
async fn master_calls_task_fail_when_auto_tasks_returns_iin2_errors() {
    let config = AssociationConfig::default();
    let mut seq = Sequence::default();
    let mut harness = create_association(config).await;

    assert_eq!(
        harness.assoc_events.pop().as_slice(),
        &[AssocInfoEvent::TaskStart(
            TaskType::DisableUnsolicited,
            FunctionCode::DisableUnsolicited,
            Sequence::new(0)
        )]
    );

    let iin = Iin::new(Iin1::default(), Iin2::NO_FUNC_CODE_SUPPORT);

    harness
        .expect_write_and_respond(
            disable_unsol_request(seq),
            empty_response_custom_iin(seq.increment(), iin),
        )
        .await;

    assert_eq!(
        harness.assoc_events.pop().as_slice(),
        &[
            AssocInfoEvent::TaskFailure(
                TaskType::DisableUnsolicited,
                TaskError::RejectedByIin2(iin),
            ),
            // even though disable unsolicited fails, the master should continue performing the startup tasks
            AssocInfoEvent::TaskStart(
                TaskType::StartupIntegrity,
                FunctionCode::Read,
                Sequence::new(1)
            ),
        ]
    );

    let iin = Iin::new(Iin1::default(), Iin2::PARAMETER_ERROR);

    harness
        .expect_write_and_respond(
            integrity_poll_request(seq),
            empty_response_custom_iin(seq.increment(), iin),
        )
        .await;

    assert_eq!(
        harness.assoc_events.pop().as_slice(),
        &[
            AssocInfoEvent::TaskFailure(TaskType::StartupIntegrity, TaskError::RejectedByIin2(iin),) // the master will NOT start another task right away if the integrity poll fails
        ]
    );

    // auto advance time
    tokio::time::pause();

    harness.expect_write(integrity_poll_request(seq)).await;

    assert_eq!(
        harness.assoc_events.pop().as_slice(),
        &[AssocInfoEvent::TaskStart(
            TaskType::StartupIntegrity,
            FunctionCode::Read,
            Sequence::new(2)
        )]
    );
}

#[tokio::test]
async fn master_startup_procedure_skips_integrity_poll_if_none() {
    let mut config = AssociationConfig::default();
    config.startup_integrity_classes = Classes::none();
    let mut seq = Sequence::default();
    let mut harness = create_association(config).await;

    // Disable unsolicited
    harness
        .expect_write_and_respond(disable_unsol_request(seq), empty_response(seq.increment()))
        .await;

    // NO Integrity poll

    // Enable unsolicited
    harness
        .expect_write_and_respond(enable_unsol_request(seq), empty_response(seq.increment()))
        .await;

    // Unsolicited NULL response with RESTART IIN
    harness
        .read_and_expect_write(unsol_null(seq, true), unsol_confirm(seq))
        .await;

    // Clear the restart flag
    harness
        .expect_write_and_respond(clear_restart_iin(seq), empty_response(seq.increment()))
        .await;

    // NO Integrity poll

    // Enable unsolicited
    harness
        .expect_write_and_respond(enable_unsol_request(seq), empty_response(seq.increment()))
        .await;
}

#[tokio::test]
async fn master_startup_procedure_skips_disable_unsol_if_none() {
    let mut config = AssociationConfig::default();
    config.disable_unsol_classes = EventClasses::none();
    config.enable_unsol_classes = EventClasses::none();
    let mut seq = Sequence::default();
    let mut harness = create_association(config).await;

    harness
        .expect_write_and_respond(integrity_poll_request(seq), empty_response(seq.increment()))
        .await;
}

#[tokio::test]
async fn clear_restart_iin_is_higher_priority() {
    let config = AssociationConfig::default();
    let mut seq = Sequence::default();
    let mut harness = create_association(config).await;

    // Disable unsolicited
    harness.expect_write(disable_unsol_request(seq)).await;

    // Never respond to it, send unsolicited NULL response with RESTART IIN
    harness
        .read_and_expect_write(unsol_null(seq, true), unsol_confirm(seq))
        .await;

    // Respond to the DISABLE_UNSOLICITED
    harness
        .process_response(empty_response(seq.increment()))
        .await;

    // Now clear the restart flag
    harness
        .expect_write_and_respond(clear_restart_iin(seq), empty_response(seq.increment()))
        .await;

    // Proceed with the rest of the startup sequence
    harness
        .expect_write_and_respond(integrity_poll_request(seq), empty_response(seq.increment()))
        .await;
    harness
        .expect_write_and_respond(enable_unsol_request(seq), empty_response(seq.increment()))
        .await;
}

#[tokio::test]
async fn outstation_restart_procedure() {
    let config = AssociationConfig::default();
    let mut seq = Sequence::default();
    let mut harness = create_association(config).await;

    startup_procedure(&mut harness, &mut seq).await;

    // Unsolicited NULL response with DEVICE_RESTART IIN
    harness
        .read_and_expect_write(unsol_null(seq, true), unsol_confirm(seq))
        .await;

    // Clear the restart flag
    harness
        .expect_write_and_respond(clear_restart_iin(seq), empty_response(seq.increment()))
        .await;

    // Integrity poll
    harness
        .expect_write_and_respond(integrity_poll_request(seq), empty_response(seq.increment()))
        .await;
    // Enable unsolicited
    harness
        .expect_write_and_respond(enable_unsol_request(seq), empty_response(seq.increment()))
        .await;
}

#[tokio::test]
async fn detect_restart_in_read_response() {
    let config = AssociationConfig::default();
    let mut seq = Sequence::default();
    let mut harness = create_association(config).await;

    startup_procedure(&mut harness, &mut seq).await;

    // Spawn a task that sends an integrity scan
    let mut assoc = harness.association.clone();
    let read_task =
        tokio::spawn(async move { assoc.read(ReadRequest::ClassScan(Classes::all())).await });

    // expect the integrity scan and respond with DEVICE_RESTART
    harness
        .expect_write_and_respond(
            integrity_poll_request(seq),
            empty_response_custom_iin(seq.increment(), Iin::new(Iin1::new(0x80), Iin2::new(0x00))),
        )
        .await;

    assert_eq!(read_task.await.unwrap(), Ok(()));

    harness
        .expect_write_and_respond(clear_restart_iin(seq), empty_response(seq.increment()))
        .await;
}

#[tokio::test]
async fn ignore_unsolicited_response_with_data_before_first_integrity_poll() {
    let config = AssociationConfig::default();
    let mut seq = Sequence::default();
    let mut unsol_seq = Sequence::default();
    let mut harness = create_association(config).await;

    startup_procedure(&mut harness, &mut seq).await;

    // Unsolicited NULL response with DEVICE_RESTART IIN
    harness
        .read_and_expect_write(
            unsol_null(unsol_seq, true),
            unsol_confirm(unsol_seq.increment()),
        )
        .await;

    // Clear the restart flag
    harness
        .expect_write_and_respond(clear_restart_iin(seq), empty_response(seq.increment()))
        .await;

    // Integrity poll (never respond to)
    harness
        .expect_write(integrity_poll_request(seq.increment()))
        .await;

    // Send unsolicited with data
    harness
        .process_response(unsol_with_data(unsol_seq, 42, true))
        .await;

    // DO NOT CONFIRM AND IGNORE PAYLOAD
    assert_eq!(harness.num_requests.fetch_add(0, Ordering::Relaxed), 0);

    tokio::time::pause(); // auto advance time
    harness
        .expect_write_and_respond(clear_restart_iin(seq), empty_response(seq.increment()))
        .await;
    harness
        .expect_write_and_respond(integrity_poll_request(seq), empty_response(seq.increment()))
        .await;
    harness
        .expect_write_and_respond(enable_unsol_request(seq), empty_response(seq.increment()))
        .await;
}

#[tokio::test]
async fn ignore_duplicate_unsolicited_response() {
    let config = AssociationConfig::default();
    let mut seq = Sequence::default();
    let mut unsol_seq = Sequence::default();
    let mut harness = create_association(config).await;

    startup_procedure(&mut harness, &mut seq).await;

    // Send unsolicited with data
    harness
        .read_and_expect_write(
            unsol_with_data(unsol_seq, 42, false),
            unsol_confirm(unsol_seq),
        )
        .await;
    assert_eq!(harness.num_requests(), 1);

    // Send exact same unsolicited response
    harness
        .read_and_expect_write(
            unsol_with_data(unsol_seq, 42, false),
            unsol_confirm(unsol_seq.increment()),
        )
        .await;
    assert_eq!(harness.num_requests(), 1);

    // Send different data
    harness
        .read_and_expect_write(
            unsol_with_data(unsol_seq, 43, false),
            unsol_confirm(unsol_seq.increment()),
        )
        .await;
    assert_eq!(harness.num_requests(), 2);

    // Send different sequence number
    harness
        .read_and_expect_write(
            unsol_with_data(unsol_seq, 43, false),
            unsol_confirm(unsol_seq.increment()),
        )
        .await;
    assert_eq!(harness.num_requests(), 3);
}

#[tokio::test]
async fn master_startup_retry_procedure() {
    let mut config = AssociationConfig::default();
    config.auto_tasks_retry_strategy =
        RetryStrategy::new(Duration::from_secs(1), Duration::from_secs(3));
    let mut seq = Sequence::default();
    let mut harness = create_association(config).await;

    // First disable unsolicited
    harness
        .expect_write(disable_unsol_request(seq.increment()))
        .await;

    // Wait for the timeout
    let first_attempt = tokio::time::Instant::now();
    tokio::time::pause();
    harness
        .expect_write(disable_unsol_request(seq.increment()))
        .await;
    let second_attempt = tokio::time::Instant::now();

    let elapsed = second_attempt - first_attempt;
    assert!(elapsed >= Duration::from_secs(2));

    harness
        .expect_write_and_respond(disable_unsol_request(seq), empty_response(seq))
        .await;
    let third_attempt = tokio::time::Instant::now();
    let elapsed = third_attempt - second_attempt;
    assert!(elapsed >= Duration::from_secs(3));
    tokio::time::resume();

    seq.increment();

    // continues on with the rest of the procedure
    harness
        .expect_write_and_respond(integrity_poll_request(seq), empty_response(seq.increment()))
        .await;

    harness
        .expect_write_and_respond(enable_unsol_request(seq), empty_response(seq.increment()))
        .await;
}
