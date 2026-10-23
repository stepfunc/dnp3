// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Reads whose responses carry IIN2.0, IIN2.1 or IIN2.2: a well-formed response is processed, and
//! the read succeeds if it delivered data. A malformed response fails the read without delivering
//! anything.

use scursor::WriteCursor;

use crate::app::format::write::start_response;
use crate::app::measurement::AnalogInputDeadBand;
use crate::app::parse::header_identity_tests::DATALESS_COUNT_VARIATIONS;
use crate::app::variations::Group32Var2;
use crate::app::{
    ControlField, Iin, Iin1, Iin2, ObjectParseError, QualifierCode, ResponseFunction, Sequence,
    Variation,
};
use crate::master::association::AssociationConfig;
use crate::master::test_objects::{RELATIVE_TIME_EVENT, SYNCHRONIZED_CTO, TRUNCATED_ANALOG_EVENT};
use crate::master::tests::harness::{AssocInfoEvent, ReadEvent, TestHarness};
use crate::master::{Classes, ReadRequest, ReadType, TaskError, TaskType};

use super::harness::create_association;
use super::harness::requests::{
    class_scan_request, solicited_confirm, startup_procedure, unsol_confirm, unsol_null_custom_iin,
};
use crate::util::test_support::watchdog;

fn read_response(seq: Sequence, fin: bool, con: bool, iin2: Iin2, data: Option<i16>) -> Vec<u8> {
    let mut buffer = [0; 20];
    let mut cursor = WriteCursor::new(&mut buffer);
    let mut response = start_response(
        ControlField::response(seq, true, fin, con),
        ResponseFunction::Response,
        Iin::new(Iin1::default(), iin2),
        &mut cursor,
    )
    .unwrap();
    if let Some(value) = data {
        response
            .write_prefixed_items([(Group32Var2 { value, flags: 0x00 }, 0u8)].iter())
            .unwrap();
    }
    cursor.written().to_vec()
}

fn continued_response(seq: Sequence, data: Option<i16>) -> Vec<u8> {
    let mut response = read_response(seq, true, false, Iin2::default(), data);
    // clear FIR in the control octet
    response[0] &= !0x80;
    response
}

#[tokio::test]
async fn read_with_iin2_and_data_delivers_the_data_and_succeeds() {
    watchdog(async {
        let mut seq = Sequence::default();
        let mut harness = create_association(AssociationConfig::default()).await;
        startup_procedure(&mut harness, &mut seq).await;

        let mut association = harness.association.clone();
        let read = tokio::spawn(async move {
            association
                .read(ReadRequest::ClassScan(Classes::class0()))
                .await
        });

        harness
            .expect_write(class_scan_request(Classes::class0(), seq))
            .await;
        let response_seq = seq.increment();
        harness
            .process_response(read_response(
                response_seq,
                true,
                true,
                Iin2::OBJECT_UNKNOWN,
                Some(42),
            ))
            .await;
        // the events in it are confirmed
        harness.expect_write(solicited_confirm(response_seq)).await;

        assert_eq!(read.await.unwrap(), Ok(()));
        assert_eq!(harness.read_events.count(ReadEvent::AnalogInput), 1);
    })
    .await;
}

#[tokio::test]
async fn read_with_iin2_and_no_data_fails() {
    watchdog(async {
        let mut seq = Sequence::default();
        let mut harness = create_association(AssociationConfig::default()).await;
        startup_procedure(&mut harness, &mut seq).await;

        let mut association = harness.association.clone();
        let read = tokio::spawn(async move {
            association
                .read(ReadRequest::ClassScan(Classes::class0()))
                .await
        });

        harness
            .expect_write_and_respond(
                class_scan_request(Classes::class0(), seq),
                read_response(seq.increment(), true, false, Iin2::OBJECT_UNKNOWN, None),
            )
            .await;

        assert_eq!(
            read.await.unwrap(),
            Err(TaskError::RejectedByIin2(Iin::new(
                Iin1::default(),
                Iin2::OBJECT_UNKNOWN
            )))
        );
        assert_eq!(harness.read_events.count(ReadEvent::AnalogInput), 0);
    })
    .await;
}

#[tokio::test]
async fn multi_fragment_read_with_iin2_in_the_first_fragment_reads_every_fragment() {
    watchdog(async {
        let mut seq = Sequence::default();
        let mut harness = create_association(AssociationConfig::default()).await;
        startup_procedure(&mut harness, &mut seq).await;

        let mut association = harness.association.clone();
        let read = tokio::spawn(async move {
            association
                .read(ReadRequest::ClassScan(Classes::class0()))
                .await
        });

        harness
            .expect_write(class_scan_request(Classes::class0(), seq))
            .await;
        let first = seq.increment();
        harness
            .process_response(read_response(
                first,
                false,
                true,
                Iin2::PARAMETER_ERROR,
                Some(1),
            ))
            .await;
        harness.expect_write(solicited_confirm(first)).await;
        harness
            .process_response(continued_response(seq.increment(), Some(2)))
            .await;

        assert_eq!(read.await.unwrap(), Ok(()));
        assert_eq!(harness.read_events.count(ReadEvent::AnalogInput), 2);
    })
    .await;
}

/// a class 0 read from a started association, returning the task and the sequence of the response
async fn start_read(
    harness: &mut TestHarness,
    seq: &mut Sequence,
) -> (tokio::task::JoinHandle<Result<(), TaskError>>, Sequence) {
    startup_procedure(harness, seq).await;
    let mut association = harness.association.clone();
    let read = tokio::spawn(async move {
        association
            .read(ReadRequest::ClassScan(Classes::class0()))
            .await
    });
    harness
        .expect_write(class_scan_request(Classes::class0(), *seq))
        .await;
    (read, seq.increment())
}

#[tokio::test]
async fn multi_fragment_read_with_iin2_and_no_data_fails() {
    watchdog(async {
        let mut seq = Sequence::default();
        let mut harness = create_association(AssociationConfig::default()).await;
        let (read, first) = start_read(&mut harness, &mut seq).await;

        harness
            .process_response(read_response(
                first,
                false,
                true,
                Iin2::OBJECT_UNKNOWN,
                None,
            ))
            .await;
        harness.expect_write(solicited_confirm(first)).await;
        // the error in the first fragment is kept
        harness
            .process_response(continued_response(seq.increment(), None))
            .await;

        assert_eq!(
            read.await.unwrap(),
            Err(TaskError::RejectedByIin2(Iin::new(
                Iin1::default(),
                Iin2::OBJECT_UNKNOWN
            )))
        );
    })
    .await;
}

#[tokio::test]
async fn read_with_iin2_and_data_only_in_the_first_fragment_succeeds() {
    watchdog(async {
        let mut seq = Sequence::default();
        let mut harness = create_association(AssociationConfig::default()).await;
        let (read, first) = start_read(&mut harness, &mut seq).await;

        harness
            .process_response(read_response(
                first,
                false,
                true,
                Iin2::PARAMETER_ERROR,
                Some(1),
            ))
            .await;
        harness.expect_write(solicited_confirm(first)).await;
        harness
            .process_response(continued_response(seq.increment(), None))
            .await;

        assert_eq!(read.await.unwrap(), Ok(()));
        assert_eq!(harness.read_events.count(ReadEvent::AnalogInput), 1);
    })
    .await;
}

#[tokio::test]
async fn read_with_iin2_and_an_empty_header_fails() {
    watchdog(async {
        let mut seq = Sequence::default();
        let mut harness = create_association(AssociationConfig::default()).await;
        let (read, response_seq) = start_read(&mut harness, &mut seq).await;

        // a g32v2 header with a count of zero
        let mut response = read_response(response_seq, true, false, Iin2::OBJECT_UNKNOWN, None);
        response.extend_from_slice(&[32, 2, 0x17, 0]);
        harness.process_response(response).await;

        assert_eq!(
            read.await.unwrap(),
            Err(TaskError::RejectedByIin2(Iin::new(
                Iin1::default(),
                Iin2::OBJECT_UNKNOWN
            )))
        );
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn periodic_poll_with_iin2_and_no_data_fails() {
    watchdog(async {
        let mut seq = Sequence::default();
        let mut harness = create_association(AssociationConfig::default()).await;
        startup_procedure(&mut harness, &mut seq).await;
        drop(harness.assoc_events.pop());
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
                read_response(seq.increment(), true, false, Iin2::OBJECT_UNKNOWN, None),
            )
            .await;

        let iin = Iin::new(Iin1::default(), Iin2::OBJECT_UNKNOWN);
        assert!(harness
            .assoc_events
            .pop()
            .contains(&AssocInfoEvent::TaskFailure(
                TaskType::PeriodicPoll,
                TaskError::RejectedByIin2(iin)
            )));
    })
    .await;
}

#[tokio::test]
async fn read_with_counted_event_objects_is_malformed() {
    watchdog(async {
        let mut seq = Sequence::default();
        let mut harness = create_association(AssociationConfig::default()).await;
        let (read, response_seq) = start_read(&mut harness, &mut seq).await;
        harness.read_events.pop();

        let mut response = read_response(response_seq, true, false, Iin2::default(), None);
        // g2v1 with a count of 8 and its 8 bytes of object data, which also read as a g32v2 event
        response.extend_from_slice(&[
            0x02, 0x01, 0x07, 0x08, 0x20, 0x02, 0x17, 0x01, 0x00, 0x01, 0x07, 0x00,
        ]);
        harness.process_response(response).await;

        assert_eq!(
            read.await.unwrap(),
            Err(TaskError::MalformedResponse(
                ObjectParseError::InvalidQualifierForVariation(
                    Variation::Group2Var1,
                    QualifierCode::Count8
                )
            ))
        );
        assert_eq!(harness.read_events.pop(), Vec::new());
    })
    .await;
}

#[tokio::test]
async fn read_with_a_count_of_objects_that_carry_no_data_is_malformed() {
    watchdog(async {
        for variation in DATALESS_COUNT_VARIATIONS {
            let (group, var) = variation.to_group_and_var();
            let counts: [(QualifierCode, &[u8]); 2] = [
                (QualifierCode::Count8, &[0x07, 0x01]),
                (QualifierCode::Count16, &[0x08, 0x01, 0x00]),
            ];
            for (qualifier, count) in counts {
                let mut seq = Sequence::default();
                let mut harness = create_association(AssociationConfig::default()).await;
                let (read, response_seq) = start_read(&mut harness, &mut seq).await;
                harness.read_events.pop();

                let mut response =
                    read_response(response_seq, true, false, Iin2::default(), Some(7));
                response.extend_from_slice(&[group, var]);
                response.extend_from_slice(count);
                harness.process_response(response).await;

                assert_eq!(
                    read.await.unwrap(),
                    Err(TaskError::MalformedResponse(
                        ObjectParseError::InvalidQualifierForVariation(variation, qualifier)
                    ))
                );
                assert_eq!(harness.read_events.pop(), Vec::new());
            }
        }
    })
    .await;
}

#[tokio::test]
async fn unsolicited_response_with_counted_event_objects_delivers_nothing() {
    watchdog(async {
        let mut seq = Sequence::default();
        let unsol_seq = Sequence::default();
        let mut harness = create_association(AssociationConfig::default()).await;
        startup_procedure(&mut harness, &mut seq).await;
        harness.read_events.pop();
        harness.assoc_events.pop();

        let mut response = unsol_null_custom_iin(unsol_seq, Iin::default());
        // g2v1 with a count of 8 and its 8 bytes of object data, which also read as a g32v2 event
        response.extend_from_slice(&[
            0x02, 0x01, 0x07, 0x08, 0x20, 0x02, 0x17, 0x01, 0x00, 0x01, 0x07, 0x00,
        ]);
        harness.process_response(response).await;
        // a malformed unsolicited response isn't confirmed, so the next write confirms a valid one
        let next = Sequence::new(1);
        harness
            .read_and_expect_write(
                unsol_null_custom_iin(next, Iin::default()),
                unsol_confirm(next),
            )
            .await;

        assert_eq!(
            harness.read_events.pop(),
            vec![
                ReadEvent::BeginFragment(ReadType::Unsolicited),
                ReadEvent::EndFragment(ReadType::Unsolicited),
            ]
        );
        assert_eq!(
            harness.assoc_events.pop(),
            vec![AssocInfoEvent::Unsolicited(false, next)]
        );
    })
    .await;
}

#[tokio::test]
async fn read_with_relative_time_without_cto_fails_without_confirm() {
    watchdog(async {
        let mut seq = Sequence::default();
        let mut harness = create_association(AssociationConfig::default()).await;
        let (read, response_seq) = start_read(&mut harness, &mut seq).await;
        harness.read_events.pop();

        let mut response = read_response(response_seq, true, true, Iin2::default(), None);
        response.extend_from_slice(RELATIVE_TIME_EVENT);
        harness.process_response(response).await;

        assert_eq!(
            read.await.unwrap(),
            Err(TaskError::UnexpectedResponseHeaders)
        );
        assert_eq!(harness.read_events.pop(), Vec::new());
        harness.assert_no_events();
    })
    .await;
}

#[tokio::test]
async fn read_with_relative_time_after_cto_succeeds() {
    watchdog(async {
        let mut seq = Sequence::default();
        let mut harness = create_association(AssociationConfig::default()).await;
        let (read, response_seq) = start_read(&mut harness, &mut seq).await;
        harness.read_events.pop();

        let mut response = read_response(response_seq, true, true, Iin2::default(), None);
        response.extend_from_slice(SYNCHRONIZED_CTO);
        response.extend_from_slice(RELATIVE_TIME_EVENT);
        harness.process_response(response).await;
        harness.expect_write(solicited_confirm(response_seq)).await;

        assert_eq!(read.await.unwrap(), Ok(()));
        assert_eq!(
            harness.read_events.pop(),
            vec![
                ReadEvent::BeginFragment(ReadType::SinglePoll),
                ReadEvent::BinaryInput,
                ReadEvent::EndFragment(ReadType::SinglePoll),
            ]
        );
    })
    .await;
}

#[tokio::test]
async fn multi_fragment_read_with_cto_only_in_the_first_fragment_fails() {
    watchdog(async {
        let mut seq = Sequence::default();
        let mut harness = create_association(AssociationConfig::default()).await;
        let (read, first) = start_read(&mut harness, &mut seq).await;
        harness.read_events.pop();

        let mut response = read_response(first, false, true, Iin2::default(), Some(1));
        response.extend_from_slice(SYNCHRONIZED_CTO);
        harness.process_response(response).await;
        harness.expect_write(solicited_confirm(first)).await;

        let mut response = continued_response(seq.increment(), None);
        // set CON
        response[0] |= 0x20;
        response.extend_from_slice(RELATIVE_TIME_EVENT);
        harness.process_response(response).await;

        assert_eq!(
            read.await.unwrap(),
            Err(TaskError::UnexpectedResponseHeaders)
        );
        // only the first fragment was delivered
        assert_eq!(
            harness.read_events.pop(),
            vec![
                ReadEvent::BeginFragment(ReadType::SinglePoll),
                ReadEvent::AnalogInput,
                ReadEvent::EndFragment(ReadType::SinglePoll),
            ]
        );
        harness.assert_no_events();
    })
    .await;
}

#[tokio::test]
async fn read_with_valid_measurements_before_relative_time_without_cto_delivers_nothing() {
    watchdog(async {
        let mut seq = Sequence::default();
        let mut harness = create_association(AssociationConfig::default()).await;
        let (read, response_seq) = start_read(&mut harness, &mut seq).await;
        harness.read_events.pop();

        let mut response = read_response(response_seq, true, true, Iin2::default(), Some(1));
        response.extend_from_slice(RELATIVE_TIME_EVENT);
        harness.process_response(response).await;

        assert_eq!(
            read.await.unwrap(),
            Err(TaskError::UnexpectedResponseHeaders)
        );
        assert_eq!(harness.read_events.pop(), Vec::new());
        harness.assert_no_events();
    })
    .await;
}

#[tokio::test]
async fn read_with_g34v0_start_stop_headers_ignores_them_and_processes_the_rest() {
    watchdog(async {
        // g34v0 over indices 0 to 1 with 1-byte and 2-byte start-stop, which carry no objects
        let ignored: [&[u8]; 2] = [&[34, 0, 0x00, 0, 1], &[34, 0, 0x01, 0, 0, 1, 0]];

        for header in ignored {
            let mut seq = Sequence::default();
            let mut harness = create_association(AssociationConfig::default()).await;
            let (read, response_seq) = start_read(&mut harness, &mut seq).await;
            harness.read_events.pop();

            let mut response = read_response(response_seq, true, true, Iin2::default(), None);
            response.extend_from_slice(header);
            // g34v1 with a 1-byte start-stop of index 3, a deadband of 0x1234
            response.extend_from_slice(&[34, 1, 0x00, 3, 3, 0x34, 0x12]);
            harness.process_response(response).await;

            assert_eq!(read.await.unwrap(), Ok(()));
            // the response requested confirmation, and is confirmed
            harness.expect_write(solicited_confirm(response_seq)).await;
            assert_eq!(
                harness.read_events.pop(),
                vec![
                    ReadEvent::BeginFragment(ReadType::SinglePoll),
                    ReadEvent::AnalogInputDeadBand(AnalogInputDeadBand::U16(0x1234), 3),
                    ReadEvent::EndFragment(ReadType::SinglePoll),
                ]
            );
            harness.assert_no_events();
        }
    })
    .await;
}

/// Read a response that requests confirmation and carries `objects`, which the master must reject
/// with `expected` without confirming the response or delivering anything
async fn expect_read_rejected(objects: &[&[u8]], expected: TaskError) {
    let mut seq = Sequence::default();
    let mut harness = create_association(AssociationConfig::default()).await;
    let (read, response_seq) = start_read(&mut harness, &mut seq).await;
    harness.read_events.pop();

    let mut response = read_response(response_seq, true, true, Iin2::default(), None);
    for x in objects {
        response.extend_from_slice(x);
    }
    harness.process_response(response).await;

    assert_eq!(read.await.unwrap(), Err(expected), "{objects:02X?}");
    assert_eq!(harness.read_events.pop(), Vec::new(), "{objects:02X?}");
    harness.assert_no_events();
}

#[tokio::test]
async fn read_with_relative_time_without_cto_then_a_parse_error_is_malformed() {
    watchdog(async {
        expect_read_rejected(
            &[RELATIVE_TIME_EVENT, TRUNCATED_ANALOG_EVENT],
            TaskError::MalformedResponse(ObjectParseError::InsufficientBytes),
        )
        .await;
    })
    .await;
}

#[tokio::test]
async fn read_with_empty_relative_time_headers_without_cto_passes_them_to_the_handler() {
    watchdog(async {
        let mut seq = Sequence::default();
        let mut harness = create_association(AssociationConfig::default()).await;
        let (read, response_seq) = start_read(&mut harness, &mut seq).await;
        harness.read_events.pop();

        let mut response = read_response(response_seq, true, true, Iin2::default(), None);
        // g2v3 and g4v3 with a count of 0, with a one-byte then a two-byte count and prefix
        response.extend_from_slice(&[0x02, 0x03, 0x17, 0x00, 0x04, 0x03, 0x17, 0x00]);
        response.extend_from_slice(&[0x02, 0x03, 0x28, 0x00, 0x00, 0x04, 0x03, 0x28, 0x00, 0x00]);
        harness.process_response(response).await;
        harness.expect_write(solicited_confirm(response_seq)).await;

        assert_eq!(read.await.unwrap(), Ok(()));
        assert_eq!(
            harness.read_events.pop(),
            vec![
                ReadEvent::BeginFragment(ReadType::SinglePoll),
                ReadEvent::BinaryInput,
                ReadEvent::DoubleBitBinaryInput,
                ReadEvent::BinaryInput,
                ReadEvent::DoubleBitBinaryInput,
                ReadEvent::EndFragment(ReadType::SinglePoll),
            ]
        );
        harness.assert_no_events();
    })
    .await;
}
