// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use std::io::ErrorKind;

use scursor::WriteCursor;
use sfio_tokio_mock_io::Event;
use tokio::sync::oneshot::error::TryRecvError;
use tokio::time::Instant;

use crate::app::control::{ControlCode, Group12Var1, Group41Var1, OpType, TripCloseCode};
use crate::app::format::write::{start_request, start_response};
use crate::app::{BufferSize, ControlField, FunctionCode, Iin, ResponseFunction, Sequence};
use crate::decode::AppDecodeLevel;
use crate::link::reader::LinkModes;
use crate::link::{EndpointAddress, LinkError, LinkErrorMode};
use crate::master::association::{AssociationConfig, AssociationMap};
use crate::master::promise::Promise;
use crate::master::task::MasterTask;
use crate::master::tasks::command_no_ack::CommandNoAckTask;
use crate::master::tasks::read::SingleReadTask;
use crate::master::test_fixtures::{association, null};
use crate::master::tests::harness::requests::{empty_response, unsol_confirm, unsol_with_data};
use crate::master::tests::harness::{create_association, AssocInfoEvent, ReadEvent, TestHarness};
use crate::master::Classes;
use crate::master::{
    AssociationHandle, BadEncoding, CommandBuilder, CommandError, CommandHeaders, CommandMode,
    CommandSupport, MasterChannel, MasterChannelConfig, MasterChannelType, ReadRequest, TaskError,
    TaskType,
};
use crate::util::lifecycle::{Enabled, RunError};
use crate::util::test_support::watchdog;

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

#[tokio::test(start_paused = true)]
async fn operate_no_ack_sends_function_code_6_and_completes_without_response() {
    watchdog(async {
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
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn operate_no_ack_does_not_delay_the_next_task() {
    watchdog(async {
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
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn queued_tasks_run_back_to_back_after_operate_no_ack_and_ignore_its_sequence() {
    watchdog(async {
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
        harness.barrier().await;

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
        assert_eq!(read.try_recv(), Err(TryRecvError::Empty));

        harness
            .process_response(empty_response(Sequence::new(2)))
            .await;
        assert_eq!(read.await.unwrap(), Ok(()));
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn requests_beyond_the_queue_limit_fail_with_too_many_requests() {
    watchdog(async {
        let config = AssociationConfig {
            max_queued_user_requests: 1,
            ..AssociationConfig::quiet()
        };
        let mut harness = create_association(config).await;
        let blocker = start_blocker(&mut harness, Sequence::new(0)).await;

        let mut queued = queue_no_ack(&mut harness).await;
        let rejected = queue_no_ack(&mut harness).await;
        assert_eq!(rejected.await.unwrap(), Err(TaskError::TooManyRequests));
        assert_eq!(queued.try_recv(), Err(TryRecvError::Empty));

        harness
            .process_response(command_response(Sequence::new(0)))
            .await;
        assert_eq!(blocker.await.unwrap(), Ok(()));
        harness
            .expect_write(command_request(NO_ACK, Sequence::new(1)))
            .await;
        assert_eq!(queued.await.unwrap(), Ok(()));
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn response_to_operate_no_ack_while_idle_is_ignored() {
    watchdog(async {
        let mut harness = quiet_association().await;

        harness.association.operate_no_ack(crob()).await.unwrap();
        harness
            .expect_write(command_request(NO_ACK, Sequence::new(0)))
            .await;
        harness.assoc_events.pop();

        harness
            .process_response(empty_response(Sequence::new(0)))
            .await;
        harness.barrier().await;
        harness.assert_no_events();
        assert_eq!(harness.assoc_events.pop(), Vec::new());

        harness.association.operate_no_ack(crob()).await.unwrap();
        harness
            .expect_write(command_request(NO_ACK, Sequence::new(1)))
            .await;
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn unsolicited_response_is_processed_after_a_queued_batch_drains() {
    watchdog(async {
        let mut harness = quiet_association().await;
        let blocker = start_blocker(&mut harness, Sequence::new(0)).await;

        let mut batch = Vec::new();
        for _ in 0..3 {
            batch.push(queue_no_ack(&mut harness).await);
        }
        harness.barrier().await;

        // the unsolicited response is already waiting when the batch starts
        harness.io.read(&command_response(Sequence::new(0)));
        harness
            .io
            .read(&unsol_with_data(Sequence::new(0), 42, false));

        assert_eq!(harness.io.next_event().await, Event::Read);
        for seq in 1..=3 {
            harness
                .expect_write(command_request(NO_ACK, Sequence::new(seq)))
                .await;
        }
        for mut rx in batch {
            assert_eq!(rx.try_recv(), Ok(Ok(())));
        }
        assert_eq!(blocker.await.unwrap(), Ok(()));

        assert_eq!(harness.io.next_event().await, Event::Read);
        harness.expect_write(unsol_confirm(Sequence::new(0))).await;
        assert_eq!(harness.read_events.count(ReadEvent::AnalogInput), 1);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn operate_no_ack_fails_if_the_write_fails() {
    watchdog(async {
        let mut harness = quiet_association().await;

        harness.io.write_error(ErrorKind::BrokenPipe);
        let result = harness.association.operate_no_ack(crob()).await;
        let err = LinkError::Stdio(ErrorKind::BrokenPipe);
        assert_eq!(result, Err(TaskError::Link(err)));

        // the link error ends the session
        assert_eq!((&mut harness.task).await.unwrap(), RunError::Link(err));
        assert_eq!(
            harness.assoc_events.pop(),
            [
                AssocInfoEvent::TaskStart(TaskType::Command, NO_ACK, Sequence::new(0)),
                AssocInfoEvent::TaskFailure(TaskType::Command, TaskError::Link(err)),
            ]
        );
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn operate_no_ack_fails_if_the_request_does_not_fit() {
    watchdog(async {
        let mut harness = quiet_association().await;

        // more CROBs than fit in the minimum transmit buffer
        let mut builder = CommandBuilder::new();
        for index in 0..u16::try_from(BufferSize::<249, 2048>::min().value()).unwrap() {
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
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn queued_operate_no_ack_fails_when_the_channel_is_disabled() {
    watchdog(async {
        let mut harness = quiet_association().await;
        let blocker = start_blocker(&mut harness, Sequence::new(0)).await;

        let no_ack = queue_no_ack(&mut harness).await;
        harness.barrier().await;
        harness.master.disable().await.unwrap();

        assert_eq!(no_ack.await.unwrap(), Err(TaskError::Disabled));
        assert!(blocker.await.unwrap().is_err());
        harness.assert_no_events();
    })
    .await;
}

#[tokio::test]
async fn operate_no_ack_fails_without_a_connection() {
    watchdog(async {
        let address = EndpointAddress::try_new(1024).unwrap();
        let mut associations = AssociationMap::new();
        associations
            .register(association(address, AssociationConfig::quiet(), null()))
            .unwrap();

        let (tx, rx) = crate::util::channel::request_channel();
        let mut task = MasterTask::with_associations(
            Enabled::Yes,
            associations,
            LinkModes::stream(LinkErrorMode::Discard),
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
    })
    .await;
}

/// Codes whose fields encode to a byte that parses back to a different `ControlCode`
fn non_canonical_codes() -> [ControlCode; 3] {
    [
        // encodes as PULSE_ON
        ControlCode::from_op_type(OpType::Unknown(0x11)),
        // encodes as CLOSE + PULSE_ON
        ControlCode::from_tcc_and_op_type(TripCloseCode::Unknown(5), OpType::PulseOn),
        // encodes as PULSE_ON, which parses back as the named variant
        ControlCode::from_op_type(OpType::Unknown(1)),
    ]
}

/// Requests that carry `bad` alone, after a valid command in the same header, or in a later
/// header, with both index widths
fn requests_containing(bad: ControlCode) -> [CommandHeaders; 7] {
    let good = Group12Var1::from_op_type(OpType::LatchOn);
    let bad = Group12Var1::from_code(bad);

    let same_header_u8 = {
        let mut builder = CommandBuilder::new();
        builder.add_u8(good, 1);
        builder.add_u8(bad, 2);
        builder.build()
    };
    let same_header_u16 = {
        let mut builder = CommandBuilder::new();
        builder.add_u16(good, 1);
        builder.add_u16(bad, 2);
        builder.build()
    };
    let later_header_u8 = {
        let mut builder = CommandBuilder::new();
        builder.add_u8(good, 1);
        builder.finish_header();
        builder.add_u8(bad, 2);
        builder.build()
    };
    let later_header_u16 = {
        let mut builder = CommandBuilder::new();
        builder.add_u16(good, 1);
        builder.finish_header();
        builder.add_u16(bad, 2);
        builder.build()
    };
    let after_analog_header = {
        let mut builder = CommandBuilder::new();
        builder.add_u16(Group41Var1::new(7), 1);
        builder.add_u8(bad, 2);
        builder.build()
    };

    [
        CommandBuilder::single_header_u8(bad, 3),
        CommandBuilder::single_header_u16(bad, 3),
        same_header_u8,
        same_header_u16,
        later_header_u8,
        later_header_u16,
        after_analog_header,
    ]
}

#[tokio::test(start_paused = true)]
async fn non_canonical_control_codes_are_rejected_before_anything_is_sent() {
    watchdog(async {
        let mut harness = quiet_association().await;

        for code in non_canonical_codes() {
            let expected = TaskError::BadEncoding(BadEncoding::ControlCode(code));

            for mode in [CommandMode::DirectOperate, CommandMode::SelectBeforeOperate] {
                for headers in requests_containing(code) {
                    assert_eq!(
                        harness.association.operate(mode, headers).await,
                        Err(CommandError::Task(expected))
                    );
                    harness.assert_no_events();
                }
            }

            for headers in requests_containing(code) {
                assert_eq!(
                    harness.association.operate_no_ack(headers).await,
                    Err(expected)
                );
                harness.assert_no_events();
            }
        }
    })
    .await;
}
