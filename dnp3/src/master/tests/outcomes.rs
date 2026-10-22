//! The outcome of every kind of task with a response, as its requester and the
//! `AssociationInformation` see it, for each way the task can end, and how responses that don't
//! belong to the task are handled while it waits

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::task::JoinHandle;
use tokio::time::Instant;

use crate::app::control::{Group12Var1, OpType};
use crate::app::{
    FileStatus, FileType, FunctionCode, Group70Var2, Group70Var6, Group70Var7, Iin, Iin1, Iin2,
    MaybeAsync, Permissions, ResponseHeader, Sequence, Timestamp, Variation,
};
use crate::link::EndpointAddress;
use crate::master::association::AssociationConfig;
use crate::master::promise::Promise;
use crate::master::tasks::command::CommandTask;
use crate::master::tasks::read::SingleReadTask;
use crate::master::tasks::Task;
use crate::master::tests::file::{
    file_status_response, file_transport_response, last_block, response as file_response,
};
use crate::master::tests::harness::requests::{
    disable_unsol_request, empty_response, empty_response_custom_iin, integrity_poll_request,
    unsol_confirm, unsol_with_data,
};
use crate::master::tests::harness::{
    create_association, create_association_with_handler, AssocInfoEvent, ReadEvent, TestHarness,
};
use crate::master::{
    AssociationHandle, AssociationHandler, AssociationInformation, AuthKey, BlockNumber, Classes,
    CommandBuilder, CommandError, CommandHeaders, CommandMode, CommandSupport, DeadBandHeader,
    DirReadConfig, FileAction, FileCredentials, FileError, FileHandle, FileMode, FileReadConfig,
    FileReader, HeaderInfo, Headers, OpenFile, ReadHandler, ReadRequest, ReadType, TaskError,
    TaskType, TimeSyncError, TimeSyncProcedure, WriteError,
};
use crate::util::session::{RunError, StopReason};
use crate::util::test_support::watchdog;

const OUTSTATION: EndpointAddress = EndpointAddress::raw(1024);
const OTHER: EndpointAddress = EndpointAddress::raw(2048);
const UNKNOWN: EndpointAddress = EndpointAddress::raw(3000);

const HANDLE: u32 = 0xDEADCAFE;

const REJECTED: Iin = Iin::new(Iin1::new(0x00), Iin2::NO_FUNC_CODE_SUPPORT);

/// Every message sent before this returns has been processed by the master task
async fn barrier(harness: &mut TestHarness) {
    harness.master.get_decode_level().await.unwrap();
}

struct FixedTime;

impl AssociationHandler for FixedTime {
    fn get_current_time(&self) -> Option<Timestamp> {
        Some(Timestamp::new(1_700_000_000_000))
    }
}

fn crob() -> CommandHeaders {
    CommandBuilder::single_header_u16(Group12Var1::from_op_type(OpType::LatchOn), 3)
}

/// A response that echoes the objects of a request, as a control response does
fn echo(request: &[u8]) -> Vec<u8> {
    let mut response = vec![0xC0 | (request[0] & 0x0F), 0x81, 0x00, 0x00];
    response.extend_from_slice(&request[2..]);
    response
}

/// The kinds of task that wait for a response, as requested through the API
#[derive(Copy, Clone, Debug)]
enum Kind {
    Read,
    DirectOperate,
    SelectBeforeOperate,
    ColdRestart,
    EmptyResponse,
    DeadBands,
    TimeSync,
    AuthFile,
    OpenFile,
    CloseFile,
    WriteFileBlock,
    GetFileInfo,
    ReadDirectory,
}

const KINDS: [Kind; 13] = [
    Kind::Read,
    Kind::DirectOperate,
    Kind::SelectBeforeOperate,
    Kind::ColdRestart,
    Kind::EmptyResponse,
    Kind::DeadBands,
    Kind::TimeSync,
    Kind::AuthFile,
    Kind::OpenFile,
    Kind::CloseFile,
    Kind::WriteFileBlock,
    Kind::GetFileInfo,
    Kind::ReadDirectory,
];

/// What the API call returned
#[derive(Debug, PartialEq)]
enum Reply {
    Read(Result<(), TaskError>),
    Command(Result<(), CommandError>),
    Restart(Result<Duration, TaskError>),
    Write(Result<(), WriteError>),
    TimeSync(Result<(), TimeSyncError>),
    AuthKey(Result<u32, FileError>),
    Open(Result<OpenFile, FileError>),
    File(Result<(), FileError>),
    /// name and size
    Info(Result<(String, u32), FileError>),
    /// number of entries
    Directory(Result<usize, FileError>),
}

impl Kind {
    fn spawn(self, mut association: AssociationHandle) -> JoinHandle<Reply> {
        tokio::spawn(async move {
            let association = &mut association;
            match self {
                Kind::Read => Reply::Read(
                    association
                        .read(ReadRequest::ClassScan(Classes::class0()))
                        .await,
                ),
                Kind::DirectOperate => Reply::Command(
                    association
                        .operate(CommandMode::DirectOperate, crob())
                        .await,
                ),
                Kind::SelectBeforeOperate => Reply::Command(
                    association
                        .operate(CommandMode::SelectBeforeOperate, crob())
                        .await,
                ),
                Kind::ColdRestart => Reply::Restart(association.cold_restart().await),
                Kind::EmptyResponse => Reply::Write(
                    association
                        .send_and_expect_empty_response(
                            FunctionCode::ImmediateFreeze,
                            Headers::new().add_all_objects(Variation::Group20Var0),
                        )
                        .await,
                ),
                Kind::DeadBands => Reply::Write(
                    association
                        .write_dead_bands(vec![DeadBandHeader::group34_var1_u8(vec![(1, 100)])])
                        .await,
                ),
                Kind::TimeSync => {
                    Reply::TimeSync(association.synchronize_time(TimeSyncProcedure::Lan).await)
                }
                Kind::AuthFile => Reply::AuthKey(
                    association
                        .get_file_auth_key(FileCredentials {
                            user_name: "user".to_string(),
                            password: "pass".to_string(),
                        })
                        .await
                        .map(u32::from),
                ),
                Kind::OpenFile => Reply::Open(
                    association
                        .open_file(
                            "f.txt",
                            AuthKey::none(),
                            Permissions::default(),
                            0,
                            FileMode::Read,
                            1024,
                        )
                        .await,
                ),
                Kind::CloseFile => {
                    Reply::File(association.close_file(FileHandle::new(HANDLE)).await)
                }
                Kind::WriteFileBlock => Reply::File(
                    association
                        .write_file_block(
                            FileHandle::new(HANDLE),
                            BlockNumber::default(),
                            vec![1, 2, 3],
                        )
                        .await,
                ),
                Kind::GetFileInfo => Reply::Info(
                    association
                        .get_file_info("f.txt")
                        .await
                        .map(|info| (info.name, info.size)),
                ),
                Kind::ReadDirectory => Reply::Directory(
                    association
                        .read_directory("dir", DirReadConfig::default(), None)
                        .await
                        .map(|entries| entries.len()),
                ),
            }
        })
    }

    fn task_type(self) -> TaskType {
        match self {
            Kind::Read => TaskType::UserRead,
            Kind::DirectOperate | Kind::SelectBeforeOperate => TaskType::Command,
            Kind::ColdRestart => TaskType::Restart,
            Kind::EmptyResponse => TaskType::GenericEmptyResponse(FunctionCode::ImmediateFreeze),
            Kind::DeadBands => TaskType::WriteDeadBands,
            Kind::TimeSync => TaskType::TimeSync,
            Kind::AuthFile => TaskType::FileAuth,
            Kind::OpenFile => TaskType::FileOpen,
            Kind::CloseFile => TaskType::FileClose,
            Kind::WriteFileBlock => TaskType::FileWriteBlock,
            Kind::GetFileInfo => TaskType::GetFileInfo,
            Kind::ReadDirectory => TaskType::FileRead,
        }
    }

    /// The function code of the first request
    fn function(self) -> FunctionCode {
        match self {
            Kind::Read => FunctionCode::Read,
            Kind::DirectOperate => FunctionCode::DirectOperate,
            Kind::SelectBeforeOperate => FunctionCode::Select,
            Kind::ColdRestart => FunctionCode::ColdRestart,
            Kind::EmptyResponse => FunctionCode::ImmediateFreeze,
            Kind::DeadBands | Kind::WriteFileBlock => FunctionCode::Write,
            Kind::TimeSync => FunctionCode::RecordCurrentTime,
            Kind::AuthFile => FunctionCode::AuthenticateFile,
            Kind::OpenFile | Kind::ReadDirectory => FunctionCode::OpenFile,
            Kind::CloseFile => FunctionCode::CloseFile,
            Kind::GetFileInfo => FunctionCode::GetFileInfo,
        }
    }

    fn is_read(self) -> bool {
        matches!(self, Kind::Read)
    }

    fn ok(self) -> Reply {
        match self {
            Kind::Read => Reply::Read(Ok(())),
            Kind::DirectOperate | Kind::SelectBeforeOperate => Reply::Command(Ok(())),
            Kind::ColdRestart => Reply::Restart(Ok(Duration::from_secs(2))),
            Kind::EmptyResponse | Kind::DeadBands => Reply::Write(Ok(())),
            Kind::TimeSync => Reply::TimeSync(Ok(())),
            Kind::AuthFile => Reply::AuthKey(Ok(42)),
            Kind::OpenFile => Reply::Open(Ok(OpenFile {
                file_handle: FileHandle::new(HANDLE),
                file_size: 8,
                max_block_size: 512,
            })),
            Kind::CloseFile | Kind::WriteFileBlock => Reply::File(Ok(())),
            Kind::GetFileInfo => Reply::Info(Ok(("f.txt".to_string(), 1234))),
            Kind::ReadDirectory => Reply::Directory(Ok(0)),
        }
    }

    fn failed(self, err: TaskError) -> Reply {
        match self {
            Kind::Read => Reply::Read(Err(err)),
            Kind::DirectOperate | Kind::SelectBeforeOperate => {
                Reply::Command(Err(CommandError::Task(err)))
            }
            Kind::ColdRestart => Reply::Restart(Err(err)),
            Kind::EmptyResponse | Kind::DeadBands => Reply::Write(Err(WriteError::Task(err))),
            Kind::TimeSync => Reply::TimeSync(Err(TimeSyncError::Task(err))),
            Kind::AuthFile => Reply::AuthKey(Err(FileError::TaskError(err))),
            Kind::OpenFile => Reply::Open(Err(FileError::TaskError(err))),
            Kind::CloseFile | Kind::WriteFileBlock => Reply::File(Err(FileError::TaskError(err))),
            Kind::GetFileInfo => Reply::Info(Err(FileError::TaskError(err))),
            Kind::ReadDirectory => Reply::Directory(Err(FileError::TaskError(err))),
        }
    }

    fn start_event(self) -> AssocInfoEvent {
        AssocInfoEvent::TaskStart(self.task_type(), self.function(), Sequence::new(0))
    }

    fn success_event(self, last: Sequence) -> AssocInfoEvent {
        AssocInfoEvent::TaskSuccess(self.task_type(), self.function(), last)
    }

    fn failure_event(self, err: TaskError) -> AssocInfoEvent {
        AssocInfoEvent::TaskFailure(self.task_type(), err)
    }

    /// Answer the first request, and any requests after it, until the task succeeds. Returns the
    /// sequence number of the last request.
    async fn finish(self, harness: &mut TestHarness, request: &[u8]) -> Sequence {
        let first = Sequence::new(0);
        match self {
            Kind::Read | Kind::EmptyResponse | Kind::DeadBands => {
                respond(harness, empty_response(first)).await;
                first
            }
            Kind::DirectOperate => {
                respond(harness, echo(request)).await;
                first
            }
            Kind::SelectBeforeOperate => {
                respond(harness, echo(request)).await;
                let operate = next_request(harness, 1, FunctionCode::Operate).await;
                respond(harness, echo(&operate)).await;
                Sequence::new(1)
            }
            Kind::ColdRestart => {
                respond(
                    harness,
                    vec![0xC0, 0x81, 0x00, 0x00, 52, 1, 0x07, 1, 0x02, 0x00],
                )
                .await;
                first
            }
            Kind::TimeSync => {
                respond(harness, empty_response(first)).await;
                next_request(harness, 1, FunctionCode::Write).await;
                respond(harness, empty_response(Sequence::new(1))).await;
                Sequence::new(1)
            }
            Kind::AuthFile => {
                let auth = Group70Var2 {
                    auth_key: 42,
                    user_name: "",
                    password: "",
                };
                respond(harness, file_response(0, &auth)).await;
                first
            }
            Kind::OpenFile => {
                respond(
                    harness,
                    file_status_response(0, HANDLE, 8, 512, FileStatus::Success),
                )
                .await;
                first
            }
            Kind::CloseFile => {
                respond(
                    harness,
                    file_status_response(0, HANDLE, 0, 0, FileStatus::Success),
                )
                .await;
                first
            }
            Kind::WriteFileBlock => {
                let status = Group70Var6 {
                    file_handle: HANDLE,
                    block_number: 0,
                    status_code: FileStatus::Success,
                    text: "",
                };
                respond(harness, file_response(0, &status)).await;
                first
            }
            Kind::GetFileInfo => {
                let info = Group70Var7 {
                    file_type: FileType::File,
                    file_size: 1234,
                    time_of_creation: Timestamp::new(5678),
                    permissions: Permissions::default(),
                    request_id: 0xCAFE,
                    file_name: "f.txt",
                };
                respond(harness, file_response(0, &info)).await;
                first
            }
            Kind::ReadDirectory => {
                respond(
                    harness,
                    file_status_response(0, HANDLE, 0, 512, FileStatus::Success),
                )
                .await;
                next_request(harness, 1, FunctionCode::Read).await;
                respond(
                    harness,
                    file_transport_response(1, HANDLE, last_block(0), &[]),
                )
                .await;
                next_request(harness, 2, FunctionCode::CloseFile).await;
                respond(
                    harness,
                    file_status_response(2, HANDLE, 0, 0, FileStatus::Success),
                )
                .await;
                Sequence::new(2)
            }
        }
    }
}

async fn respond(harness: &mut TestHarness, response: Vec<u8>) {
    harness.process_response(response).await;
}

/// Wait for the next request, check its sequence and function code, and return it
async fn next_request(harness: &mut TestHarness, seq: u8, function: FunctionCode) -> Vec<u8> {
    let request = harness.pop_write().await;
    assert_eq!(request[..2], [0xC0 | seq, function.as_u8()]);
    request
}

/// A task that has sent its first request
struct Started {
    kind: Kind,
    harness: TestHarness,
    reply: JoinHandle<Reply>,
    request: Vec<u8>,
}

async fn start(kind: Kind) -> Started {
    let mut harness =
        create_association_with_handler(AssociationConfig::quiet(), Box::new(FixedTime)).await;
    let reply = kind.spawn(harness.association.clone());
    let request = next_request(&mut harness, 0, kind.function()).await;
    assert_eq!(harness.assoc_events.pop(), [kind.start_event()], "{kind:?}");
    Started {
        kind,
        harness,
        reply,
        request,
    }
}

impl Started {
    async fn reply(&mut self) -> Reply {
        (&mut self.reply).await.unwrap()
    }

    /// Check that the task ended with this error and the master kept running
    async fn expect_failure(&mut self, err: TaskError) {
        let kind = self.kind;
        assert_eq!(self.reply().await, kind.failed(err), "{kind:?}");
        barrier(&mut self.harness).await;
        assert_eq!(
            self.harness.assoc_events.pop(),
            [kind.failure_event(err)],
            "{kind:?}"
        );
        assert!(!self.harness.task.is_finished(), "{kind:?}");
    }

    /// Check that the task is still waiting for its response, and nothing was written
    async fn expect_waiting(&mut self) {
        let kind = self.kind;
        barrier(&mut self.harness).await;
        assert!(!self.reply.is_finished(), "{kind:?}");
        assert_eq!(self.harness.io.pop_event(), None, "{kind:?}");
        assert_eq!(self.harness.assoc_events.pop(), [], "{kind:?}");
    }

    /// Answer the task and check that it succeeds
    async fn expect_success(&mut self) {
        let kind = self.kind;
        let request = self.request.clone();
        let last = kind.finish(&mut self.harness, &request).await;
        assert_eq!(self.reply().await, kind.ok(), "{kind:?}");
        barrier(&mut self.harness).await;
        assert_eq!(
            self.harness.assoc_events.pop(),
            [kind.success_event(last)],
            "{kind:?}"
        );
    }
}

#[tokio::test(start_paused = true)]
async fn every_task_succeeds_and_reports_its_type_and_sequence() {
    watchdog(async {
        for kind in KINDS {
            start(kind).await.expect_success().await;
        }
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn every_task_times_out_one_response_timeout_after_its_request() {
    watchdog(async {
        for kind in KINDS {
            let mut started = start(kind).await;
            let now = Instant::now();
            started.expect_failure(TaskError::ResponseTimeout).await;
            TestHarness::assert_elapsed(now, Duration::from_secs(1));
        }
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn every_task_fails_when_its_response_is_rejected_by_iin2() {
    watchdog(async {
        for kind in KINDS {
            let mut started = start(kind).await;
            respond(
                &mut started.harness,
                empty_response_custom_iin(Sequence::new(0), REJECTED),
            )
            .await;
            started
                .expect_failure(TaskError::RejectedByIin2(REJECTED))
                .await;
        }
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn every_task_fails_on_a_response_without_fin_or_con() {
    watchdog(async {
        for kind in KINDS {
            let mut started = start(kind).await;
            // FIR, but neither FIN nor CON
            respond(&mut started.harness, vec![0x80, 0x81, 0x00, 0x00]).await;
            let err = if kind.is_read() {
                TaskError::NonFinWithoutCon
            } else {
                TaskError::MultiFragmentResponse
            };
            started.expect_failure(err).await;
        }
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn every_task_fails_on_a_response_with_unexpected_objects() {
    watchdog(async {
        for kind in KINDS.into_iter().filter(|kind| !kind.is_read()) {
            let mut started = start(kind).await;
            let expects_objects =
                !matches!(kind, Kind::EmptyResponse | Kind::DeadBands | Kind::TimeSync);
            let response = if expects_objects {
                empty_response(Sequence::new(0))
            } else {
                // a g52v1 time delay
                vec![0xC0, 0x81, 0x00, 0x00, 52, 1, 0x07, 1, 0x02, 0x00]
            };
            respond(&mut started.harness, response).await;
            let unexpected = TaskError::UnexpectedResponseHeaders;
            let reply = match kind {
                Kind::DirectOperate | Kind::SelectBeforeOperate => {
                    Reply::Command(Err(CommandError::Response(
                        crate::master::CommandResponseError::HeaderCountMismatch,
                    )))
                }
                _ => kind.failed(unexpected),
            };
            assert_eq!(started.reply().await, reply, "{kind:?}");
            barrier(&mut started.harness).await;
            assert_eq!(
                started.harness.assoc_events.pop(),
                [kind.failure_event(unexpected)],
                "{kind:?}"
            );
            assert_eq!(started.harness.io.pop_event(), None, "{kind:?}");
        }
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn every_task_fails_on_a_fragment_too_short_to_be_a_response() {
    watchdog(async {
        for kind in KINDS {
            let mut started = start(kind).await;
            respond(&mut started.harness, vec![0xC0, 0x81, 0x00]).await;
            started.expect_failure(TaskError::Transport).await;
        }
    })
    .await;
}

/// Fragments that aren't valid responses: one too short for the IIN, and a solicited response with
/// the UNS bit
const MALFORMED: [&[u8]; 2] = [&[0xC0, 0x81, 0x00], &[0xD0, 0x81, 0x00, 0x00]];

#[tokio::test(start_paused = true)]
async fn every_task_fails_on_a_solicited_response_with_the_uns_bit() {
    watchdog(async {
        for kind in KINDS {
            let mut started = start(kind).await;
            respond(&mut started.harness, vec![0xD0, 0x81, 0x00, 0x00]).await;
            started.expect_failure(TaskError::Transport).await;
        }
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn every_task_ignores_malformed_fragments_from_other_outstations() {
    watchdog(async {
        for kind in KINDS {
            let mut started = start(kind).await;
            let harness = &mut started.harness;
            harness.set_source(OTHER);
            for fragment in MALFORMED {
                respond(harness, fragment.to_vec()).await;
            }
            harness.set_source(OUTSTATION);
            started.expect_waiting().await;
            started.expect_success().await;
        }
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn malformed_fragments_from_other_outstations_do_not_extend_any_task_deadline() {
    watchdog(async {
        for kind in KINDS {
            let mut started = start(kind).await;
            let now = Instant::now();
            let harness = &mut started.harness;
            harness.set_source(OTHER);
            for fragment in MALFORMED {
                tokio::time::advance(Duration::from_millis(250)).await;
                respond(harness, fragment.to_vec()).await;
            }
            harness.set_source(OUTSTATION);
            let err = TaskError::ResponseTimeout;
            assert_eq!(started.reply().await, kind.failed(err), "{kind:?}");
            assert_eq!(now.elapsed(), Duration::from_secs(1), "{kind:?}");
            barrier(&mut started.harness).await;
            assert_eq!(
                started.harness.assoc_events.pop(),
                [kind.failure_event(err)],
                "{kind:?}"
            );
        }
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn every_task_fails_with_disabled_when_the_channel_is_disabled() {
    watchdog(async {
        for kind in KINDS {
            let mut started = start(kind).await;
            started.harness.master.disable().await.unwrap();
            assert_eq!(
                started.reply().await,
                kind.failed(TaskError::Disabled),
                "{kind:?}"
            );
            assert_eq!(
                (&mut started.harness.task).await.unwrap(),
                RunError::Stop(StopReason::Disable),
                "{kind:?}"
            );
            assert_eq!(
                started.harness.assoc_events.pop(),
                [kind.failure_event(TaskError::Disabled)],
                "{kind:?}"
            );
        }
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn every_task_fails_with_shutdown_when_the_master_shuts_down() {
    watchdog(async {
        for kind in KINDS {
            let mut started = start(kind).await;
            // the requester holds a handle to the master, so it has to go away first
            started.reply.abort();
            assert!((&mut started.reply).await.is_err(), "{kind:?}");
            drop(started.harness.association);
            drop(started.harness.master);
            assert_eq!(
                (&mut started.harness.task).await.unwrap(),
                RunError::Stop(StopReason::Shutdown),
                "{kind:?}"
            );
            assert_eq!(
                started.harness.assoc_events.pop(),
                [kind.failure_event(TaskError::Shutdown)],
                "{kind:?}"
            );
        }
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn running_and_queued_tasks_complete_their_promises_on_shutdown() {
    watchdog(async {
        let mut harness = create_association(AssociationConfig::quiet()).await;

        // tasks queued directly, so that no requester holds a handle to the master and the
        // receivers tell a completed promise from a dropped one
        let (promise, running) = Promise::one_shot();
        harness
            .association
            .send_task(CommandTask::from_mode(
                CommandMode::DirectOperate,
                crob(),
                promise,
            ))
            .await
            .unwrap();
        next_request(&mut harness, 0, FunctionCode::DirectOperate).await;

        let (promise, queued_command) = Promise::one_shot();
        harness
            .association
            .send_task(CommandTask::from_mode(
                CommandMode::SelectBeforeOperate,
                crob(),
                promise,
            ))
            .await
            .unwrap();
        let (promise, queued_read) = Promise::one_shot();
        harness
            .association
            .send_task(SingleReadTask::new(
                ReadRequest::ClassScan(Classes::class0()),
                promise,
            ))
            .await
            .unwrap();
        let (promise, queued_link_status) = Promise::one_shot();
        harness
            .association
            .send_task(Task::LinkStatus(promise))
            .await
            .unwrap();
        let reader = RecordingReader::default();
        harness
            .association
            .read_file(
                "f.txt",
                FileReadConfig::default(),
                Box::new(reader.clone()),
                None,
            )
            .await
            .unwrap();
        barrier(&mut harness).await;

        drop(harness.association);
        drop(harness.master);
        assert_eq!(
            (&mut harness.task).await.unwrap(),
            RunError::Stop(StopReason::Shutdown)
        );

        let shutdown = Err(CommandError::Task(TaskError::Shutdown));
        assert_eq!(running.await, Ok(shutdown));
        assert_eq!(queued_command.await, Ok(shutdown));
        assert_eq!(queued_read.await, Ok(Err(TaskError::Shutdown)));
        assert_eq!(queued_link_status.await, Ok(Err(TaskError::Shutdown)));
        assert_eq!(
            reader.pop(),
            [ReaderEvent::Aborted(FileError::TaskError(
                TaskError::Shutdown
            ))]
        );
        assert_eq!(
            harness.assoc_events.pop(),
            [
                AssocInfoEvent::TaskStart(
                    TaskType::Command,
                    FunctionCode::DirectOperate,
                    Sequence::new(0)
                ),
                AssocInfoEvent::TaskFailure(TaskType::Command, TaskError::Shutdown),
            ]
        );
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn every_task_ignores_a_response_with_the_wrong_sequence() {
    watchdog(async {
        for kind in KINDS {
            let mut started = start(kind).await;
            respond(&mut started.harness, empty_response(Sequence::new(1))).await;
            started.expect_waiting().await;
            started.expect_success().await;
        }
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn every_task_processes_and_confirms_unsolicited_responses_while_waiting() {
    watchdog(async {
        for kind in KINDS {
            let mut started = start(kind).await;
            let harness = &mut started.harness;
            respond(harness, unsol_with_data(Sequence::new(5), 7, false)).await;
            harness.expect_write(unsol_confirm(Sequence::new(5))).await;
            barrier(harness).await;
            assert_eq!(
                harness.read_events.pop(),
                [
                    ReadEvent::BeginFragment(ReadType::Unsolicited),
                    ReadEvent::AnalogInput,
                    ReadEvent::EndFragment(ReadType::Unsolicited),
                ],
                "{kind:?}"
            );
            assert_eq!(
                harness.assoc_events.pop(),
                [AssocInfoEvent::Unsolicited(false, Sequence::new(5))],
                "{kind:?}"
            );
            started.expect_waiting().await;
            started.expect_success().await;
        }
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn every_task_attributes_fragments_from_other_outstations_to_them() {
    watchdog(async {
        for kind in KINDS {
            let mut started = start(kind).await;
            let other = Log::default();
            let harness = &mut started.harness;
            harness
                .master
                .add_association(
                    OTHER,
                    AssociationConfig::quiet(),
                    Box::new(other.clone()),
                    Box::new(other.clone()),
                    Box::new(other.clone()),
                )
                .await
                .unwrap();

            // a solicited response with the expected sequence, from another outstation
            harness.set_source(OTHER);
            respond(harness, empty_response(Sequence::new(0))).await;
            // an unsolicited response from another outstation is processed by its association
            respond(harness, unsol_with_data(Sequence::new(3), 7, false)).await;
            harness.expect_write(unsol_confirm(Sequence::new(3))).await;
            // and one from an address without an association is neither processed nor confirmed
            harness.set_source(UNKNOWN);
            respond(harness, unsol_with_data(Sequence::new(4), 7, false)).await;
            respond(harness, empty_response(Sequence::new(0))).await;
            harness.set_source(OUTSTATION);

            started.expect_waiting().await;
            assert_eq!(started.harness.read_events.pop(), [], "{kind:?}");
            assert_eq!(
                other.pop(),
                [
                    Entry::Read(ReadEvent::BeginFragment(ReadType::Unsolicited)),
                    Entry::Read(ReadEvent::AnalogInput),
                    Entry::Read(ReadEvent::EndFragment(ReadType::Unsolicited)),
                    Entry::Info(AssocInfoEvent::Unsolicited(false, Sequence::new(3))),
                ],
                "{kind:?}"
            );
            started.expect_success().await;
            assert_eq!(other.pop(), [], "{kind:?}");
        }
    })
    .await;
}

/// The callbacks of one association, in the order they were made
#[derive(Copy, Clone, Debug, PartialEq)]
enum Entry {
    Read(ReadEvent),
    Info(AssocInfoEvent),
}

#[derive(Clone, Default)]
struct Log(Arc<Mutex<Vec<Entry>>>);

impl Log {
    fn push(&self, entry: Entry) {
        self.0.lock().unwrap().push(entry);
    }

    fn pop(&self) -> Vec<Entry> {
        std::mem::take(&mut *self.0.lock().unwrap())
    }
}

impl AssociationHandler for Log {}

impl ReadHandler for Log {
    fn begin_fragment(&mut self, read_type: ReadType, _header: ResponseHeader) -> MaybeAsync<()> {
        self.push(Entry::Read(ReadEvent::BeginFragment(read_type)));
        MaybeAsync::ready(())
    }

    fn end_fragment(&mut self, read_type: ReadType, _header: ResponseHeader) -> MaybeAsync<()> {
        self.push(Entry::Read(ReadEvent::EndFragment(read_type)));
        MaybeAsync::ready(())
    }

    fn handle_analog_input(
        &mut self,
        _info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (crate::app::measurement::AnalogInput, u16)>,
    ) {
        iter.for_each(drop);
        self.push(Entry::Read(ReadEvent::AnalogInput));
    }
}

impl AssociationInformation for Log {
    fn task_start(&mut self, task_type: TaskType, fc: FunctionCode, seq: Sequence) {
        self.push(Entry::Info(AssocInfoEvent::TaskStart(task_type, fc, seq)));
    }

    fn task_success(&mut self, task_type: TaskType, fc: FunctionCode, seq: Sequence) {
        self.push(Entry::Info(AssocInfoEvent::TaskSuccess(task_type, fc, seq)));
    }

    fn task_fail(&mut self, task_type: TaskType, error: TaskError) {
        self.push(Entry::Info(AssocInfoEvent::TaskFailure(task_type, error)));
    }

    fn unsolicited_response(&mut self, is_duplicate: bool, seq: Sequence) {
        self.push(Entry::Info(AssocInfoEvent::Unsolicited(is_duplicate, seq)));
    }
}

/// A read response fragment with one analog input event
fn read_fragment(seq: Sequence, fir: bool, fin: bool, con: bool) -> Vec<u8> {
    let control = (u8::from(fir) << 7) | (u8::from(fin) << 6) | (u8::from(con) << 5);
    // g32v2, 1-byte count and prefix, one event at index 0 with value 7
    vec![
        control | seq.value(),
        0x81,
        0x00,
        0x00,
        32,
        2,
        0x17,
        1,
        0,
        0x01,
        7,
        0,
    ]
}

fn solicited_confirm(seq: Sequence) -> Vec<u8> {
    vec![0xC0 | seq.value(), 0x00]
}

#[tokio::test(start_paused = true)]
async fn read_callbacks_and_sequence_numbers_follow_the_fragments_in_order() {
    watchdog(async {
        let mut harness = create_association(AssociationConfig::quiet()).await;
        let log = Log::default();
        let mut association = harness
            .master
            .add_association(
                OTHER,
                AssociationConfig::quiet(),
                Box::new(log.clone()),
                Box::new(log.clone()),
                Box::new(log.clone()),
            )
            .await
            .unwrap();
        harness.set_source(OTHER);

        let read = tokio::spawn(async move {
            association
                .read(ReadRequest::ClassScan(Classes::class0()))
                .await
        });
        next_request(&mut harness, 0, FunctionCode::Read).await;

        // the first fragment asks for a confirm, and the next one has the next sequence number
        respond(
            &mut harness,
            read_fragment(Sequence::new(0), true, false, true),
        )
        .await;
        harness
            .expect_write(solicited_confirm(Sequence::new(0)))
            .await;
        // an unsolicited response between fragments is processed on its own
        respond(&mut harness, unsol_with_data(Sequence::new(9), 7, false)).await;
        harness.expect_write(unsol_confirm(Sequence::new(9))).await;
        // a fragment that repeats the first sequence number is ignored
        respond(
            &mut harness,
            read_fragment(Sequence::new(0), false, true, false),
        )
        .await;
        respond(
            &mut harness,
            read_fragment(Sequence::new(1), false, true, false),
        )
        .await;

        assert_eq!(read.await.unwrap(), Ok(()));
        assert_eq!(
            log.pop(),
            [
                Entry::Info(AssocInfoEvent::TaskStart(
                    TaskType::UserRead,
                    FunctionCode::Read,
                    Sequence::new(0)
                )),
                Entry::Read(ReadEvent::BeginFragment(ReadType::SinglePoll)),
                Entry::Read(ReadEvent::AnalogInput),
                Entry::Read(ReadEvent::EndFragment(ReadType::SinglePoll)),
                Entry::Read(ReadEvent::BeginFragment(ReadType::Unsolicited)),
                Entry::Read(ReadEvent::AnalogInput),
                Entry::Read(ReadEvent::EndFragment(ReadType::Unsolicited)),
                Entry::Info(AssocInfoEvent::Unsolicited(false, Sequence::new(9))),
                Entry::Read(ReadEvent::BeginFragment(ReadType::SinglePoll)),
                Entry::Read(ReadEvent::AnalogInput),
                Entry::Read(ReadEvent::EndFragment(ReadType::SinglePoll)),
                Entry::Info(AssocInfoEvent::TaskSuccess(
                    TaskType::UserRead,
                    FunctionCode::Read,
                    Sequence::new(1)
                )),
            ]
        );
        // nothing reached the harness association
        assert_eq!(harness.read_events.pop(), []);
        assert_eq!(harness.assoc_events.pop(), []);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn read_fails_if_the_first_fragment_is_not_fir() {
    watchdog(async {
        let mut started = start(Kind::Read).await;
        respond(
            &mut started.harness,
            read_fragment(Sequence::new(0), false, true, false),
        )
        .await;
        started.expect_failure(TaskError::NeverReceivedFir).await;
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn read_fails_if_a_later_fragment_is_fir() {
    watchdog(async {
        let mut started = start(Kind::Read).await;
        let harness = &mut started.harness;
        respond(harness, read_fragment(Sequence::new(0), true, false, true)).await;
        harness
            .expect_write(solicited_confirm(Sequence::new(0)))
            .await;
        respond(harness, read_fragment(Sequence::new(1), true, true, false)).await;
        harness.read_events.pop();
        started.expect_failure(TaskError::UnexpectedFir).await;
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn read_times_out_one_response_timeout_after_the_last_fragment() {
    watchdog(async {
        let mut started = start(Kind::Read).await;
        tokio::time::advance(Duration::from_millis(500)).await;
        let harness = &mut started.harness;
        respond(harness, read_fragment(Sequence::new(0), true, false, true)).await;
        harness
            .expect_write(solicited_confirm(Sequence::new(0)))
            .await;
        let now = Instant::now();
        started.expect_failure(TaskError::ResponseTimeout).await;
        TestHarness::assert_elapsed(now, Duration::from_secs(1));
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn select_before_operate_reports_the_select_and_the_operate_sequence() {
    watchdog(async {
        // SELECT and OPERATE are one task: it starts with the SELECT and succeeds with the OPERATE
        let mut started = start(Kind::SelectBeforeOperate).await;
        started.expect_success().await;
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn select_before_operate_times_out_waiting_for_the_operate_response() {
    watchdog(async {
        let mut started = start(Kind::SelectBeforeOperate).await;
        let echo = echo(&started.request);
        respond(&mut started.harness, echo).await;
        next_request(&mut started.harness, 1, FunctionCode::Operate).await;
        let now = Instant::now();
        started.expect_failure(TaskError::ResponseTimeout).await;
        TestHarness::assert_elapsed(now, Duration::from_secs(1));
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn select_before_operate_fails_without_operate_if_the_select_response_differs() {
    watchdog(async {
        let mut started = start(Kind::SelectBeforeOperate).await;
        // the outstation answers with no objects
        respond(&mut started.harness, empty_response(Sequence::new(0))).await;
        assert_eq!(
            started.reply().await,
            Reply::Command(Err(CommandError::Response(
                crate::master::CommandResponseError::HeaderCountMismatch
            )))
        );
        barrier(&mut started.harness).await;
        assert_eq!(
            started.harness.assoc_events.pop(),
            [AssocInfoEvent::TaskFailure(
                TaskType::Command,
                TaskError::UnexpectedResponseHeaders
            )]
        );
        assert_eq!(started.harness.io.pop_event(), None);
    })
    .await;
}

/// Disable or shut down the master while an automatic task waits for its response: the
/// DISABLE_UNSOLICITED that starts up the association, or the integrity poll after it
async fn interrupt_auto_task(integrity: bool, disable: bool) {
    let config = AssociationConfig {
        disable_unsol_classes: crate::master::EventClasses::all(),
        startup_integrity_classes: Classes::all(),
        ..AssociationConfig::quiet()
    };
    let mut harness = create_association(config).await;
    harness
        .expect_write(disable_unsol_request(Sequence::new(0)))
        .await;
    let disable_unsol = AssocInfoEvent::TaskStart(
        TaskType::DisableUnsolicited,
        FunctionCode::DisableUnsolicited,
        Sequence::new(0),
    );
    let task_type = if integrity {
        respond(&mut harness, empty_response(Sequence::new(0))).await;
        harness
            .expect_write(integrity_poll_request(Sequence::new(1)))
            .await;
        assert_eq!(
            harness.assoc_events.pop(),
            [
                disable_unsol,
                AssocInfoEvent::TaskSuccess(
                    TaskType::DisableUnsolicited,
                    FunctionCode::DisableUnsolicited,
                    Sequence::new(0),
                ),
                AssocInfoEvent::TaskStart(
                    TaskType::StartupIntegrity,
                    FunctionCode::Read,
                    Sequence::new(1)
                ),
            ]
        );
        TaskType::StartupIntegrity
    } else {
        assert_eq!(harness.assoc_events.pop(), [disable_unsol]);
        TaskType::DisableUnsolicited
    };

    let (stop, err) = if disable {
        harness.master.disable().await.unwrap();
        (StopReason::Disable, TaskError::Disabled)
    } else {
        drop(harness.association);
        drop(harness.master);
        (StopReason::Shutdown, TaskError::Shutdown)
    };
    assert_eq!((&mut harness.task).await.unwrap(), RunError::Stop(stop));
    assert_eq!(
        harness.assoc_events.pop(),
        [AssocInfoEvent::TaskFailure(task_type, err)]
    );
}

#[tokio::test(start_paused = true)]
async fn automatic_tasks_fail_with_disabled_when_the_channel_is_disabled() {
    watchdog(async {
        interrupt_auto_task(false, true).await;
        interrupt_auto_task(true, true).await;
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn automatic_tasks_fail_with_shutdown_when_the_master_shuts_down() {
    watchdog(async {
        interrupt_auto_task(false, false).await;
        interrupt_auto_task(true, false).await;
    })
    .await;
}

/// What a file reader was told, in order
#[derive(Clone, Debug, PartialEq)]
enum ReaderEvent {
    Opened(u32),
    Block(u32, Vec<u8>),
    Aborted(FileError),
    Completed,
}

#[derive(Clone, Default)]
struct RecordingReader(Arc<Mutex<Vec<ReaderEvent>>>);

impl RecordingReader {
    fn pop(&self) -> Vec<ReaderEvent> {
        std::mem::take(&mut *self.0.lock().unwrap())
    }

    fn push(&self, event: ReaderEvent) {
        self.0.lock().unwrap().push(event);
    }
}

impl FileReader for RecordingReader {
    fn opened(&mut self, size: u32) -> FileAction {
        self.push(ReaderEvent::Opened(size));
        FileAction::Continue
    }

    fn block_received(&mut self, block_num: u32, data: &[u8]) -> MaybeAsync<FileAction> {
        self.push(ReaderEvent::Block(block_num, data.to_vec()));
        MaybeAsync::ready(FileAction::Continue)
    }

    fn aborted(&mut self, err: FileError) {
        self.push(ReaderEvent::Aborted(err));
    }

    fn completed(&mut self) {
        self.push(ReaderEvent::Completed);
    }
}

/// Start a file read, open the file, and wait for the request for the first block
async fn read_first_block() -> (TestHarness, RecordingReader) {
    let mut harness = create_association(AssociationConfig::quiet()).await;
    let reader = RecordingReader::default();
    harness
        .association
        .read_file(
            "f.txt",
            FileReadConfig::default(),
            Box::new(reader.clone()),
            None,
        )
        .await
        .unwrap();
    next_request(&mut harness, 0, FunctionCode::OpenFile).await;
    respond(
        &mut harness,
        file_status_response(0, HANDLE, 4, 512, FileStatus::Success),
    )
    .await;
    next_request(&mut harness, 1, FunctionCode::Read).await;
    assert_eq!(reader.pop(), [ReaderEvent::Opened(4)]);
    assert_eq!(
        harness.assoc_events.pop(),
        [AssocInfoEvent::TaskStart(
            TaskType::FileRead,
            FunctionCode::OpenFile,
            Sequence::new(0)
        )]
    );
    (harness, reader)
}

#[tokio::test(start_paused = true)]
async fn file_read_completes_the_reader_once_and_then_closes_the_file() {
    watchdog(async {
        let (mut harness, reader) = read_first_block().await;
        respond(
            &mut harness,
            file_transport_response(1, HANDLE, last_block(0), b"data"),
        )
        .await;
        next_request(&mut harness, 2, FunctionCode::CloseFile).await;
        assert_eq!(
            reader.pop(),
            [
                ReaderEvent::Block(0, b"data".to_vec()),
                ReaderEvent::Completed
            ]
        );
        respond(
            &mut harness,
            file_status_response(2, HANDLE, 0, 0, FileStatus::Success),
        )
        .await;
        barrier(&mut harness).await;
        assert_eq!(
            harness.assoc_events.pop(),
            [AssocInfoEvent::TaskSuccess(
                TaskType::FileRead,
                FunctionCode::OpenFile,
                Sequence::new(2)
            )]
        );
        assert_eq!(reader.pop(), []);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn file_read_aborts_the_reader_once_on_timeout_without_closing() {
    watchdog(async {
        let (mut harness, reader) = read_first_block().await;
        tokio::time::sleep(Duration::from_millis(999)).await;
        barrier(&mut harness).await;
        assert_eq!(reader.pop(), []);
        tokio::time::sleep(Duration::from_millis(2)).await;
        barrier(&mut harness).await;
        assert_eq!(
            reader.pop(),
            [ReaderEvent::Aborted(FileError::TaskError(
                TaskError::ResponseTimeout
            ))]
        );
        assert_eq!(
            harness.assoc_events.pop(),
            [AssocInfoEvent::TaskFailure(
                TaskType::FileRead,
                TaskError::ResponseTimeout
            )]
        );
        assert_eq!(harness.io.pop_event(), None);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn file_read_aborts_the_reader_once_when_rejected_by_iin2() {
    watchdog(async {
        let (mut harness, reader) = read_first_block().await;
        respond(
            &mut harness,
            empty_response_custom_iin(Sequence::new(1), REJECTED),
        )
        .await;
        barrier(&mut harness).await;
        assert_eq!(
            reader.pop(),
            [ReaderEvent::Aborted(FileError::TaskError(
                TaskError::RejectedByIin2(REJECTED)
            ))]
        );
        assert_eq!(
            harness.assoc_events.pop(),
            [AssocInfoEvent::TaskFailure(
                TaskType::FileRead,
                TaskError::RejectedByIin2(REJECTED)
            )]
        );
        assert_eq!(harness.io.pop_event(), None);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn file_read_aborts_the_reader_once_when_the_channel_is_disabled() {
    watchdog(async {
        let (mut harness, reader) = read_first_block().await;
        harness.master.disable().await.unwrap();
        assert_eq!(
            (&mut harness.task).await.unwrap(),
            RunError::Stop(StopReason::Disable)
        );
        assert_eq!(
            reader.pop(),
            [ReaderEvent::Aborted(FileError::TaskError(
                TaskError::Disabled
            ))]
        );
        assert_eq!(
            harness.assoc_events.pop(),
            [AssocInfoEvent::TaskFailure(
                TaskType::FileRead,
                TaskError::Disabled
            )]
        );
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn file_read_aborts_the_reader_once_when_the_master_shuts_down() {
    watchdog(async {
        let (mut harness, reader) = read_first_block().await;
        drop(harness.association);
        drop(harness.master);
        assert_eq!(
            (&mut harness.task).await.unwrap(),
            RunError::Stop(StopReason::Shutdown)
        );
        assert_eq!(
            reader.pop(),
            [ReaderEvent::Aborted(FileError::TaskError(
                TaskError::Shutdown
            ))]
        );
        assert_eq!(
            harness.assoc_events.pop(),
            [AssocInfoEvent::TaskFailure(
                TaskType::FileRead,
                TaskError::Shutdown
            )]
        );
    })
    .await;
}

/// A response from the outstation that ends a file read
#[derive(Copy, Clone, Debug)]
enum ContentFailure {
    /// AUTHENTICATE returns auth key 0
    NoPermission,
    /// AUTHENTICATE is answered with a g70v4
    AuthVariation,
    /// OPEN returns an unsuccessful status
    OpenStatus,
    /// OPEN is answered with a g70v2
    OpenVariation,
    /// READ is answered with a g70v4
    BlockVariation,
    /// READ returns a block other than the one requested
    BlockNumber,
}

const CONTENT_FAILURES: [ContentFailure; 6] = [
    ContentFailure::NoPermission,
    ContentFailure::AuthVariation,
    ContentFailure::OpenStatus,
    ContentFailure::OpenVariation,
    ContentFailure::BlockVariation,
    ContentFailure::BlockNumber,
];

impl ContentFailure {
    fn credentials(self) -> Option<FileCredentials> {
        match self {
            Self::NoPermission | Self::AuthVariation => Some(FileCredentials {
                user_name: "user".to_string(),
                password: "pass".to_string(),
            }),
            Self::OpenStatus | Self::OpenVariation | Self::BlockVariation | Self::BlockNumber => {
                None
            }
        }
    }

    /// The function code of the first request
    fn first(self) -> FunctionCode {
        match self.credentials() {
            Some(_) => FunctionCode::AuthenticateFile,
            None => FunctionCode::OpenFile,
        }
    }

    fn error(self) -> FileError {
        match self {
            Self::NoPermission => FileError::NoPermission,
            Self::AuthVariation | Self::OpenVariation | Self::BlockVariation => {
                FileError::BadResponse
            }
            Self::OpenStatus => FileError::BadStatus(FileStatus::FileNotFound),
            Self::BlockNumber => FileError::BadBlockNum,
        }
    }

    /// What the reader is told, in order
    fn reader_events(self) -> Vec<ReaderEvent> {
        let aborted = ReaderEvent::Aborted(self.error());
        match self {
            Self::NoPermission | Self::AuthVariation | Self::OpenStatus | Self::OpenVariation => {
                vec![aborted]
            }
            Self::BlockVariation | Self::BlockNumber => vec![ReaderEvent::Opened(4), aborted],
        }
    }

    /// Answer the first request, and any requests after it, until this response ends the read
    async fn respond(self, harness: &mut TestHarness) {
        let auth = |auth_key| Group70Var2 {
            auth_key,
            user_name: "",
            password: "",
        };
        let opened = file_status_response(0, HANDLE, 4, 512, FileStatus::Success);
        match self {
            Self::NoPermission => respond(harness, file_response(0, &auth(0))).await,
            Self::AuthVariation => respond(harness, opened).await,
            Self::OpenStatus => {
                respond(
                    harness,
                    file_status_response(0, HANDLE, 0, 0, FileStatus::FileNotFound),
                )
                .await
            }
            Self::OpenVariation => respond(harness, file_response(0, &auth(42))).await,
            Self::BlockVariation => {
                respond(harness, opened).await;
                next_request(harness, 1, FunctionCode::Read).await;
                respond(
                    harness,
                    file_status_response(1, HANDLE, 0, 0, FileStatus::Success),
                )
                .await;
            }
            Self::BlockNumber => {
                respond(harness, opened).await;
                next_request(harness, 1, FunctionCode::Read).await;
                respond(harness, file_transport_response(1, HANDLE, 1, b"data")).await;
            }
        }
    }

    /// Check that the association saw the read fail, and that the master sent nothing after it
    async fn expect_task_failure(self, harness: &mut TestHarness) {
        barrier(harness).await;
        assert_eq!(
            harness.assoc_events.pop(),
            [
                AssocInfoEvent::TaskStart(TaskType::FileRead, self.first(), Sequence::new(0)),
                AssocInfoEvent::TaskFailure(
                    TaskType::FileRead,
                    TaskError::UnexpectedResponseHeaders
                )
            ],
            "{self:?}"
        );
        assert_eq!(harness.io.pop_event(), None, "{self:?}");
        assert!(!harness.task.is_finished(), "{self:?}");
    }
}

#[tokio::test(start_paused = true)]
async fn file_read_content_failures_abort_the_reader_and_fail_the_task() {
    watchdog(async {
        for failure in CONTENT_FAILURES {
            let mut harness = create_association(AssociationConfig::quiet()).await;
            let reader = RecordingReader::default();
            harness
                .association
                .read_file(
                    "f.txt",
                    FileReadConfig::default(),
                    Box::new(reader.clone()),
                    failure.credentials(),
                )
                .await
                .unwrap();
            next_request(&mut harness, 0, failure.first()).await;
            failure.respond(&mut harness).await;
            failure.expect_task_failure(&mut harness).await;
            assert_eq!(reader.pop(), failure.reader_events(), "{failure:?}");
        }
    })
    .await;
}

/// Start a directory read and wait for its first request
async fn start_directory_read(
    harness: &mut TestHarness,
    config: DirReadConfig,
    credentials: Option<FileCredentials>,
    first: FunctionCode,
) -> JoinHandle<Result<usize, FileError>> {
    let mut association = harness.association.clone();
    let reply = tokio::spawn(async move {
        association
            .read_directory("dir", config, credentials)
            .await
            .map(|entries| entries.len())
    });
    next_request(harness, 0, first).await;
    reply
}

/// Check that a directory read that started with an OPEN reported `task_success` with the sequence
/// number of its last request
async fn expect_directory_task_success(harness: &mut TestHarness, last: u8) {
    barrier(harness).await;
    assert_eq!(
        harness.assoc_events.pop(),
        [
            AssocInfoEvent::TaskStart(TaskType::FileRead, FunctionCode::OpenFile, Sequence::new(0)),
            AssocInfoEvent::TaskSuccess(
                TaskType::FileRead,
                FunctionCode::OpenFile,
                Sequence::new(last)
            )
        ]
    );
}

#[tokio::test(start_paused = true)]
async fn directory_read_content_failures_fail_the_read_and_the_task() {
    watchdog(async {
        for failure in CONTENT_FAILURES {
            let mut harness = create_association(AssociationConfig::quiet()).await;
            let reply = start_directory_read(
                &mut harness,
                DirReadConfig::default(),
                failure.credentials(),
                failure.first(),
            )
            .await;
            failure.respond(&mut harness).await;
            assert_eq!(reply.await.unwrap(), Err(failure.error()), "{failure:?}");
            failure.expect_task_failure(&mut harness).await;
        }
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn directory_read_over_its_size_limit_fails_the_read_but_not_the_task() {
    watchdog(async {
        let mut harness = create_association(AssociationConfig::quiet()).await;
        let config = DirReadConfig {
            max_block_size: 512,
            max_file_size: 3,
        };
        let reply = start_directory_read(&mut harness, config, None, FunctionCode::OpenFile).await;
        respond(
            &mut harness,
            file_status_response(0, HANDLE, 4, 512, FileStatus::Success),
        )
        .await;
        next_request(&mut harness, 1, FunctionCode::Read).await;
        respond(
            &mut harness,
            file_transport_response(1, HANDLE, last_block(0), b"data"),
        )
        .await;
        assert_eq!(reply.await.unwrap(), Err(FileError::MaxLengthExceeded));
        expect_directory_task_success(&mut harness, 1).await;
        assert_eq!(harness.io.pop_event(), None);
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn directory_read_with_unparseable_entries_fails_the_read_but_not_the_task() {
    watchdog(async {
        let mut harness = create_association(AssociationConfig::quiet()).await;
        let reply = start_directory_read(
            &mut harness,
            DirReadConfig::default(),
            None,
            FunctionCode::OpenFile,
        )
        .await;
        respond(
            &mut harness,
            file_status_response(0, HANDLE, 1, 512, FileStatus::Success),
        )
        .await;
        next_request(&mut harness, 1, FunctionCode::Read).await;
        respond(
            &mut harness,
            file_transport_response(1, HANDLE, last_block(0), &[0x01]),
        )
        .await;
        assert_eq!(reply.await.unwrap(), Err(FileError::BadResponse));
        next_request(&mut harness, 2, FunctionCode::CloseFile).await;
        respond(
            &mut harness,
            file_status_response(2, HANDLE, 0, 0, FileStatus::Success),
        )
        .await;
        expect_directory_task_success(&mut harness, 2).await;
    })
    .await;
}
