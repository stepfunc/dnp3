// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Each kind of read delivers its response to the right handler with the right ReadType

use crate::app::{ControlField, Iin, Iin1, Iin2, Sequence};
use crate::master::association::AssociationConfig;
use crate::master::request::EventClasses;
use crate::master::tests::harness::{ReadEvent, ReadEventHandler};
use crate::master::{Classes, ReadRequest, ReadType};

use super::harness::create_association;
use super::harness::requests::*;
use crate::util::test_support::watchdog;

const CLASS_1_EVENTS: Iin = Iin::new(Iin1::new(0x02), Iin2::new(0x00));

/// a single fragment response, not requesting confirmation, with one analog input
fn response_with_data(seq: Sequence) -> Vec<u8> {
    response(
        ControlField::response(seq, true, true, false),
        Iin::default(),
        &analog_event(0x00, 7),
    )
}

fn fragment_with_data(read_type: ReadType) -> Vec<ReadEvent> {
    vec![
        ReadEvent::BeginFragment(read_type),
        ReadEvent::AnalogInput,
        ReadEvent::EndFragment(read_type),
    ]
}

#[tokio::test]
async fn startup_integrity_poll_reports_startup_integrity() {
    watchdog(async {
        let mut seq = Sequence::default();
        let mut harness = create_association(AssociationConfig::default()).await;

        harness
            .expect_write_and_respond(disable_unsol_request(seq), empty_response(seq.increment()))
            .await;
        harness
            .expect_write_and_respond(
                integrity_poll_request(seq),
                response_with_data(seq.increment()),
            )
            .await;
        harness
            .expect_write_and_respond(enable_unsol_request(seq), empty_response(seq.increment()))
            .await;

        assert_eq!(
            harness.read_events.pop(),
            fragment_with_data(ReadType::StartupIntegrity)
        );
    })
    .await;
}

#[tokio::test]
async fn event_scan_reports_periodic_poll() {
    watchdog(async {
        let config = AssociationConfig {
            event_scan_on_events_available: EventClasses::all(),
            ..Default::default()
        };
        let mut seq = Sequence::default();
        let mut harness = create_association(config).await;
        startup_procedure(&mut harness, &mut seq).await;
        harness.read_events.pop();

        harness
            .read_and_expect_write(
                unsol_null_custom_iin(seq, CLASS_1_EVENTS),
                unsol_confirm(seq),
            )
            .await;
        assert_eq!(
            harness.read_events.pop(),
            vec![
                ReadEvent::BeginFragment(ReadType::Unsolicited),
                ReadEvent::EndFragment(ReadType::Unsolicited),
            ]
        );

        let class1 = Classes::new(false, EventClasses::new(true, false, false));
        harness
            .expect_write_and_respond(
                class_scan_request(class1, seq),
                response_with_data(seq.increment()),
            )
            .await;

        assert_eq!(
            harness.read_events.pop(),
            fragment_with_data(ReadType::PeriodicPoll)
        );
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn periodic_poll_reports_periodic_poll() {
    watchdog(async {
        let mut seq = Sequence::default();
        let mut harness = create_association(AssociationConfig::default()).await;
        startup_procedure(&mut harness, &mut seq).await;
        harness.read_events.pop();
        let mut association = harness.association.clone();
        let _poll = association
            .add_poll(
                ReadRequest::ClassScan(Classes::class0()),
                std::time::Duration::from_secs(1),
            )
            .await
            .unwrap();

        harness
            .expect_write_and_respond(
                class_scan_request(Classes::class0(), seq),
                response_with_data(seq.increment()),
            )
            .await;

        assert_eq!(
            harness.read_events.pop(),
            fragment_with_data(ReadType::PeriodicPoll)
        );
    })
    .await;
}

#[tokio::test]
async fn single_read_reports_single_poll() {
    watchdog(async {
        let mut seq = Sequence::default();
        let mut harness = create_association(AssociationConfig::default()).await;
        startup_procedure(&mut harness, &mut seq).await;
        harness.read_events.pop();

        let mut association = harness.association.clone();
        let read = tokio::spawn(async move {
            association
                .read(ReadRequest::ClassScan(Classes::class0()))
                .await
        });
        harness
            .expect_write_and_respond(
                class_scan_request(Classes::class0(), seq),
                response_with_data(seq.increment()),
            )
            .await;

        assert_eq!(read.await.unwrap(), Ok(()));
        assert_eq!(
            harness.read_events.pop(),
            fragment_with_data(ReadType::SinglePoll)
        );
    })
    .await;
}

#[tokio::test]
async fn single_read_with_custom_handler_reports_single_poll_to_that_handler_only() {
    watchdog(async {
        let mut seq = Sequence::default();
        let mut harness = create_association(AssociationConfig::default()).await;
        startup_procedure(&mut harness, &mut seq).await;
        harness.read_events.pop();

        let handler = ReadEventHandler::new();
        let mut custom_events = handler.events();
        let mut association = harness.association.clone();
        let read = tokio::spawn(async move {
            association
                .read_with_handler(ReadRequest::ClassScan(Classes::class0()), Box::new(handler))
                .await
        });
        harness
            .expect_write_and_respond(
                class_scan_request(Classes::class0(), seq),
                response_with_data(seq.increment()),
            )
            .await;

        assert_eq!(read.await.unwrap(), Ok(()));
        assert_eq!(
            custom_events.pop(),
            fragment_with_data(ReadType::SinglePoll)
        );
        assert_eq!(harness.read_events.pop(), Vec::new());
    })
    .await;
}

#[tokio::test]
async fn unsolicited_response_reports_unsolicited() {
    watchdog(async {
        let mut seq = Sequence::default();
        let mut harness = create_association(AssociationConfig::default()).await;
        startup_procedure(&mut harness, &mut seq).await;
        harness.read_events.pop();

        harness
            .read_and_expect_write(unsol_with_data(seq, 7, false), unsol_confirm(seq))
            .await;

        assert_eq!(
            harness.read_events.pop(),
            fragment_with_data(ReadType::Unsolicited)
        );
    })
    .await;
}
