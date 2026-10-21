//! Reads whose responses carry IIN2.0, IIN2.1 or IIN2.2: a well-formed response is processed, and
//! the read succeeds if it delivered data

use scursor::WriteCursor;

use crate::app::format::write::{start_request, start_response};
use crate::app::variations::Group32Var2;
use crate::app::{ControlField, FunctionCode, Iin, Iin1, Iin2, ResponseFunction, Sequence};
use crate::master::association::AssociationConfig;
use crate::master::tests::harness::{AssocInfoEvent, TestHarness};
use crate::master::{Classes, ReadRequest, TaskError, TaskType};

use super::harness::create_association;
use super::harness::requests::{class_scan_request, startup_procedure};

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

fn solicited_confirm(seq: Sequence) -> Vec<u8> {
    let mut buffer = [0; 2];
    let mut cursor = WriteCursor::new(&mut buffer);
    start_request(
        ControlField::request(seq),
        FunctionCode::Confirm,
        &mut cursor,
    )
    .unwrap();
    cursor.written().to_vec()
}

#[tokio::test]
async fn read_with_iin2_and_data_delivers_the_data_and_succeeds() {
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
    assert_eq!(harness.num_requests(), 1);
}

#[tokio::test]
async fn read_with_iin2_and_no_data_fails() {
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
    assert_eq!(harness.num_requests(), 0);
}

#[tokio::test]
async fn multi_fragment_read_with_iin2_in_the_first_fragment_reads_every_fragment() {
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
    assert_eq!(harness.num_requests(), 2);
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
}

#[tokio::test]
async fn read_with_iin2_and_data_only_in_the_first_fragment_succeeds() {
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
    assert_eq!(harness.num_requests(), 1);
}

#[tokio::test]
async fn read_with_iin2_and_an_empty_header_fails() {
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
}

#[tokio::test]
async fn periodic_poll_with_iin2_and_no_data_fails() {
    let mut seq = Sequence::default();
    let mut harness = create_association(AssociationConfig::default()).await;
    startup_procedure(&mut harness, &mut seq).await;
    drop(harness.assoc_events.pop());

    tokio::time::pause();
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
}
