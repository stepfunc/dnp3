use std::sync::atomic::Ordering;
use std::time::Duration;

use crate::app::Sequence;
use crate::app::{FunctionCode, RetryStrategy};
use crate::app::{Iin, Iin1, Iin2};
use crate::master::association::AssociationConfig;
use crate::master::request::{Classes, EventClasses};
use crate::master::tests::harness::AssocInfoEvent;
use crate::master::{ReadRequest, TaskError, TaskType};
use crate::util::test_support::watchdog;

use super::harness::requests::*;
use super::harness::{create_association, TestHarness};
use crate::util::test_support::watchdog;

#[tokio::test]
async fn master_startup_procedure() {
    watchdog(async {
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
    })
    .await;
}

/// Respond to the request with this sequence with CON set, and expect the master to confirm it
async fn respond_with_con(harness: &mut TestHarness, seq: &mut Sequence) {
    let current = seq.increment();
    harness
        .read_and_expect_write(
            with_con(empty_response(current)),
            solicited_confirm(current),
        )
        .await;
}

#[tokio::test]
async fn automatic_tasks_confirm_responses_with_con_before_the_next_request() {
    watchdog(async {
        let mut seq = Sequence::default();
        let mut harness = create_association(AssociationConfig::default()).await;

        harness.expect_write(disable_unsol_request(seq)).await;
        respond_with_con(&mut harness, &mut seq).await;
        harness
            .expect_write_and_respond(integrity_poll_request(seq), empty_response(seq.increment()))
            .await;
        harness.expect_write(enable_unsol_request(seq)).await;
        respond_with_con(&mut harness, &mut seq).await;

        // Unsolicited NULL response with DEVICE_RESTART IIN
        harness
            .read_and_expect_write(unsol_null(seq, true), unsol_confirm(seq))
            .await;
        harness.expect_write(clear_restart_iin(seq)).await;
        respond_with_con(&mut harness, &mut seq).await;
        harness
            .expect_write_and_respond(integrity_poll_request(seq), empty_response(seq.increment()))
            .await;
        harness
            .expect_write_and_respond(enable_unsol_request(seq), empty_response(seq.increment()))
            .await;

        harness.master.get_decode_level().await.unwrap();
        harness.assert_no_events();
    })
    .await;
}

#[tokio::test]
async fn startup_continues_when_auto_tasks_return_iin2_errors() {
    watchdog(async {
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

        // a read with IIN2 errors and no data still completes the integrity scan, so startup continues
        harness
            .expect_write_and_respond(
                integrity_poll_request(seq),
                empty_response_custom_iin(seq.increment(), iin),
            )
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
    })
    .await;
}

#[tokio::test]
async fn master_startup_procedure_skips_integrity_poll_if_none() {
    watchdog(async {
        let config = AssociationConfig {
            startup_integrity_classes: Classes::none(),
            ..Default::default()
        };
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
    })
    .await;
}

#[tokio::test]
async fn master_startup_procedure_skips_disable_unsol_if_none() {
    watchdog(async {
        let config = AssociationConfig {
            disable_unsol_classes: EventClasses::none(),
            enable_unsol_classes: EventClasses::none(),
            ..Default::default()
        };
        let mut seq = Sequence::default();
        let mut harness = create_association(config).await;

        harness
            .expect_write_and_respond(integrity_poll_request(seq), empty_response(seq.increment()))
            .await;
    })
    .await;
}

#[tokio::test]
async fn clear_restart_iin_is_higher_priority() {
    watchdog(async {
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
    })
    .await;
}

#[tokio::test]
async fn outstation_restart_procedure() {
    watchdog(async {
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
    })
    .await;
}

#[tokio::test]
async fn detect_restart_in_read_response() {
    watchdog(async {
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
                empty_response_custom_iin(
                    seq.increment(),
                    Iin::new(Iin1::new(0x80), Iin2::new(0x00)),
                ),
            )
            .await;

        assert_eq!(read_task.await.unwrap(), Ok(()));

        harness
            .expect_write_and_respond(clear_restart_iin(seq), empty_response(seq.increment()))
            .await;
    })
    .await;
}

#[tokio::test]
async fn ignore_unsolicited_response_with_data_before_first_integrity_poll() {
    watchdog(async {
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
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn master_startup_retry_procedure() {
    watchdog(async {
        let config = AssociationConfig {
            auto_tasks_retry_strategy: RetryStrategy::new(
                Duration::from_secs(1),
                Duration::from_secs(3),
            ),
            ..Default::default()
        };
        let mut seq = Sequence::default();
        let mut harness = create_association(config).await;

        // First disable unsolicited
        harness
            .expect_write(disable_unsol_request(seq.increment()))
            .await;

        // the 1 second response timeout, then the minimum retry delay
        let first_attempt = tokio::time::Instant::now();
        harness
            .expect_write(disable_unsol_request(seq.increment()))
            .await;
        TestHarness::assert_elapsed(first_attempt, Duration::from_secs(2));

        // the response timeout, then the doubled retry delay
        let second_attempt = tokio::time::Instant::now();
        harness
            .expect_write_and_respond(disable_unsol_request(seq), empty_response(seq))
            .await;
        TestHarness::assert_elapsed(second_attempt, Duration::from_secs(3));

        seq.increment();

        // continues on with the rest of the procedure
        harness
            .expect_write_and_respond(integrity_poll_request(seq), empty_response(seq.increment()))
            .await;

        harness
            .expect_write_and_respond(enable_unsol_request(seq), empty_response(seq.increment()))
            .await;
    })
    .await;
}
