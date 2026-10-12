use std::io::ErrorKind;

use scursor::WriteCursor;
use sfio_tokio_mock_io::Event;
use tokio::time::Instant;

use crate::app::control::{Group12Var1, OpType};
use crate::app::format::write::{start_request, start_response};
use crate::app::parse::options::ParseOptions;
use crate::app::{BufferSize, ControlField, FunctionCode, Iin, ResponseFunction, Sequence};
use crate::decode::AppDecodeLevel;
use crate::link::reader::LinkModes;
use crate::link::EndpointAddress;
use crate::master::association::{Association, AssociationConfig, AssociationMap};
use crate::master::promise::Promise;
use crate::master::task::MasterTask;
use crate::master::tasks::command_no_ack::CommandNoAckTask;
use crate::master::tasks::read::SingleReadTask;
use crate::master::tests::harness::requests::{empty_response, unsol_confirm, unsol_with_data};
use crate::master::tests::harness::{create_association, AssocInfoEvent, TestHarness};
use crate::master::Classes;
use crate::master::{
    AssociationHandle, AssociationHandler, AssociationInformation, CommandBuilder, CommandHeaders,
    CommandMode, CommandSupport, MasterChannel, MasterChannelConfig, MasterChannelType,
    ReadHandler, ReadRequest, TaskError, TaskType,
};
use crate::transport::FragmentAddr;
use crate::util::phys::PhysAddr;
use crate::util::session::Enabled;

const NO_ACK: FunctionCode = FunctionCode::DirectOperateNoResponse;

fn crob() -> CommandHeaders {
    CommandBuilder::single_header_u16(Group12Var1::from_op_type(OpType::LatchOn), 3)
}

fn command_request(function: FunctionCode, seq: Sequence) -> Vec<u8> {
    let mut buffer = [0; 64];
    let mut cursor = WriteCursor::new(&mut buffer);
    let mut writer = start_request(ControlField::request(seq), function, &mut cursor).unwrap();
    crob().write(&mut writer).unwrap();
    cursor.written().to_vec()
}

fn command_response(seq: Sequence) -> Vec<u8> {
    let mut buffer = [0; 64];
    let mut cursor = WriteCursor::new(&mut buffer);
    let mut writer = start_response(
        ControlField::response(seq, true, true, false),
        ResponseFunction::Response,
        Iin::default(),
        &mut cursor,
    )
    .unwrap();
    crob().write(&mut writer).unwrap();
    cursor.written().to_vec()
}

fn class_scan_request(seq: Sequence) -> Vec<u8> {
    crate::master::tests::harness::requests::class_scan_request(Classes::class0(), seq)
}

async fn quiet_association() -> TestHarness {
    tokio::time::pause();
    create_association(AssociationConfig::quiet()).await
}

/// Start a DIRECT_OPERATE whose response is held back, so that later tasks queue up behind it
async fn start_blocker(
    harness: &mut TestHarness,
    seq: Sequence,
) -> tokio::task::JoinHandle<Result<(), crate::master::CommandError>> {
    let mut association = harness.association.clone();
    let blocker = tokio::spawn(async move {
        association
            .operate(CommandMode::DirectOperate, crob())
            .await
    });
    harness
        .expect_write(command_request(FunctionCode::DirectOperate, seq))
        .await;
    blocker
}

async fn queue_no_ack(
    harness: &mut TestHarness,
) -> tokio::sync::oneshot::Receiver<Result<(), TaskError>> {
    let (promise, rx) = Promise::one_shot();
    harness
        .association
        .send_task(CommandNoAckTask::new(crob(), promise))
        .await
        .unwrap();
    rx
}

/// Every message sent before this returns has been processed by the master task
async fn barrier(harness: &mut TestHarness) {
    harness.master.get_decode_level().await.unwrap();
}

#[tokio::test]
async fn operate_no_ack_sends_function_code_6_and_completes_without_response() {
    let mut harness = quiet_association().await;
    let start = Instant::now();

    let request = command_request(NO_ACK, Sequence::new(0));
    assert_eq!(&request[..2], &[0xC0, 0x06]);

    let result = harness.association.operate_no_ack(crob()).await;
    assert_eq!(result, Ok(()));
    harness.expect_write(request).await;
    assert_eq!(Instant::now(), start);

    assert_eq!(
        harness.assoc_events.pop(),
        vec![
            AssocInfoEvent::TaskStart(TaskType::Command, NO_ACK, Sequence::new(0)),
            AssocInfoEvent::TaskSuccess(TaskType::Command, NO_ACK, Sequence::new(0)),
        ]
    );
}

#[tokio::test]
async fn operate_no_ack_does_not_delay_the_next_task() {
    let mut harness = quiet_association().await;
    let start = Instant::now();

    // followed by a read
    harness.association.operate_no_ack(crob()).await.unwrap();
    harness
        .expect_write(command_request(NO_ACK, Sequence::new(0)))
        .await;
    let mut association = harness.association.clone();
    let read = tokio::spawn(async move {
        association
            .read(ReadRequest::class_scan(Classes::class0()))
            .await
    });
    harness
        .expect_write_and_respond(
            class_scan_request(Sequence::new(1)),
            empty_response(Sequence::new(1)),
        )
        .await;
    assert_eq!(read.await.unwrap(), Ok(()));

    // followed by a command that does expect a response
    harness.association.operate_no_ack(crob()).await.unwrap();
    harness
        .expect_write(command_request(NO_ACK, Sequence::new(2)))
        .await;
    let mut association = harness.association.clone();
    let operate = tokio::spawn(async move {
        association
            .operate(CommandMode::DirectOperate, crob())
            .await
    });
    harness
        .expect_write_and_respond(
            command_request(FunctionCode::DirectOperate, Sequence::new(3)),
            command_response(Sequence::new(3)),
        )
        .await;
    assert_eq!(operate.await.unwrap(), Ok(()));

    assert_eq!(Instant::now(), start);
}

#[tokio::test]
async fn queued_tasks_run_back_to_back_after_operate_no_ack_and_ignore_its_sequence() {
    let mut harness = quiet_association().await;
    let blocker = start_blocker(&mut harness, Sequence::new(0)).await;

    let mut no_ack = queue_no_ack(&mut harness).await;
    let (promise, mut read) = Promise::one_shot();
    harness
        .association
        .send_task(SingleReadTask::new(
            ReadRequest::class_scan(Classes::class0()),
            promise,
        ))
        .await
        .unwrap();
    barrier(&mut harness).await;

    let start = Instant::now();
    harness
        .process_response(command_response(Sequence::new(0)))
        .await;
    assert_eq!(blocker.await.unwrap(), Ok(()));

    harness
        .expect_write(command_request(NO_ACK, Sequence::new(1)))
        .await;
    harness
        .expect_write(class_scan_request(Sequence::new(2)))
        .await;
    assert_eq!(no_ack.try_recv(), Ok(Ok(())));
    assert_eq!(Instant::now(), start);

    // a response carrying the DIRECT_OPERATE_NO_ACK's sequence doesn't complete the read
    harness
        .process_response(empty_response(Sequence::new(1)))
        .await;
    tokio::task::yield_now().await;
    assert!(read.try_recv().is_err());

    harness
        .process_response(empty_response(Sequence::new(2)))
        .await;
    assert_eq!(read.await.unwrap(), Ok(()));
}

#[tokio::test]
async fn response_to_operate_no_ack_while_idle_is_ignored() {
    let mut harness = quiet_association().await;

    harness.association.operate_no_ack(crob()).await.unwrap();
    harness
        .expect_write(command_request(NO_ACK, Sequence::new(0)))
        .await;
    harness.assoc_events.pop();

    harness
        .process_response(empty_response(Sequence::new(0)))
        .await;
    barrier(&mut harness).await;
    harness.assert_no_events();
    assert_eq!(harness.assoc_events.pop(), Vec::new());

    harness.association.operate_no_ack(crob()).await.unwrap();
    harness
        .expect_write(command_request(NO_ACK, Sequence::new(1)))
        .await;
}

#[tokio::test]
async fn unsolicited_response_is_processed_after_a_queued_batch_drains() {
    let mut harness = quiet_association().await;
    let blocker = start_blocker(&mut harness, Sequence::new(0)).await;

    let mut batch = Vec::new();
    for _ in 0..3 {
        batch.push(queue_no_ack(&mut harness).await);
    }
    barrier(&mut harness).await;

    harness
        .process_response(command_response(Sequence::new(0)))
        .await;
    assert_eq!(blocker.await.unwrap(), Ok(()));
    harness
        .io
        .read(&unsol_with_data(Sequence::new(0), 42, false));

    for seq in 1..=3 {
        harness
            .expect_write(command_request(NO_ACK, Sequence::new(seq)))
            .await;
    }
    for mut rx in batch {
        assert_eq!(rx.try_recv(), Ok(Ok(())));
    }

    assert_eq!(harness.io.next_event().await, Event::Read);
    harness.expect_write(unsol_confirm(Sequence::new(0))).await;
    assert_eq!(harness.num_requests(), 1);
}

#[tokio::test]
async fn operate_no_ack_fails_if_the_write_fails() {
    let mut harness = quiet_association().await;

    harness.io.write_error(ErrorKind::BrokenPipe);
    let result = harness.association.operate_no_ack(crob()).await;
    assert!(matches!(result, Err(TaskError::Link(_))), "{result:?}");

    let events = harness.assoc_events.pop();
    assert!(
        events
            .iter()
            .all(|e| !matches!(e, AssocInfoEvent::TaskSuccess(..))),
        "{events:?}"
    );
    assert!(events
        .iter()
        .any(|e| matches!(e, AssocInfoEvent::TaskFailure(TaskType::Command, _))));
}

#[tokio::test]
async fn operate_no_ack_fails_if_the_request_does_not_fit() {
    let mut harness = quiet_association().await;

    // more CROBs than fit in the minimum transmit buffer
    let mut builder = CommandBuilder::new();
    for index in 0..(BufferSize::<249, 2048>::min().value() as u16) {
        builder.add_u16(Group12Var1::from_op_type(OpType::LatchOn), index);
    }

    let result = harness.association.operate_no_ack(builder.build()).await;
    assert_eq!(result, Err(TaskError::WriteError));
    harness.assert_no_events();
    assert_eq!(
        harness.assoc_events.pop(),
        vec![
            AssocInfoEvent::TaskStart(TaskType::Command, NO_ACK, Sequence::new(0)),
            AssocInfoEvent::TaskFailure(TaskType::Command, TaskError::WriteError),
        ]
    );
}

#[tokio::test]
async fn queued_operate_no_ack_fails_when_the_channel_is_disabled() {
    let mut harness = quiet_association().await;
    let blocker = start_blocker(&mut harness, Sequence::new(0)).await;

    let no_ack = queue_no_ack(&mut harness).await;
    barrier(&mut harness).await;
    harness.master.disable().await.unwrap();

    assert_eq!(no_ack.await.unwrap(), Err(TaskError::Disabled));
    assert!(blocker.await.unwrap().is_err());
    harness.assert_no_events();
}

struct NullHandler;
impl ReadHandler for NullHandler {}
impl AssociationHandler for NullHandler {}
impl AssociationInformation for NullHandler {}

#[tokio::test]
async fn operate_no_ack_fails_without_a_connection() {
    let address = EndpointAddress::try_new(1024).unwrap();
    let mut associations = AssociationMap::new();
    associations
        .register(Association::new(
            FragmentAddr {
                link: address,
                phys: PhysAddr::None,
            },
            AssociationConfig::quiet(),
            Box::new(NullHandler),
            Box::new(NullHandler),
            Box::new(NullHandler),
        ))
        .unwrap();

    let (tx, rx) = crate::util::channel::request_channel();
    let mut task = MasterTask::with_associations(
        Enabled::Yes,
        associations,
        LinkModes::serial(),
        ParseOptions::default(),
        MasterChannelConfig {
            master_address: EndpointAddress::try_new(1).unwrap(),
            decode_level: AppDecodeLevel::Nothing.into(),
            tx_buffer_size: BufferSize::min(),
            rx_buffer_size: BufferSize::min(),
        },
        rx,
    );
    let mut association =
        AssociationHandle::new(address, MasterChannel::new(tx, MasterChannelType::Stream));

    let operate = tokio::spawn(async move { association.operate_no_ack(crob()).await });
    task.process_next_message().await.unwrap();

    assert_eq!(operate.await.unwrap(), Err(TaskError::NoConnection));
}
