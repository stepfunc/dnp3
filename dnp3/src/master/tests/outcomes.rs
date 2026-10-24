// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! The outcome of every kind of task with a response, as its requester and the
//! `AssociationInformation` see it, for each way the task can end, and how responses that don't
//! belong to the task are handled while it waits

use std::io::ErrorKind;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::task::JoinHandle;
use tokio::time::Instant;

use crate::app::attr::{var, AttrSet, BadAttribute, OwnedAttrValue, OwnedAttribute};
use crate::app::control::{Group12Var1, OpType};
use crate::app::{
    ControlField, FileStatus, FileType, FunctionCode, Group70Var2, Group70Var6, Group70Var7, Iin,
    Iin1, Iin2, MaybeAsync, ObjectParseError, Permissions, ResponseHeader, Sequence, Timestamp,
    Variation,
};
use crate::link::{EndpointAddress, LinkError};
use crate::master::association::AssociationConfig;
use crate::master::promise::Promise;
use crate::master::tasks::command::CommandTask;
use crate::master::tasks::read::SingleReadTask;
use crate::master::tasks::time::PendingTimeSync;
use crate::master::tasks::Task;
use crate::master::test_fixtures::FixedTime;
use crate::master::tests::file::{
    file_status_response, file_transport_response, last_block, response as file_response,
    ReaderEvent, RecordingReader,
};
use crate::master::tests::harness::requests::{
    analog_event, disable_unsol_request, empty_response, empty_response_custom_iin,
    integrity_poll_request, response, solicited_confirm, unsol_confirm, unsol_with_data, with_con,
};
use crate::master::tests::harness::{
    create_association, create_association_with_handler, AssocInfoEvent, ReadEvent, TestHarness,
};
use crate::master::{
    AssociationError, AssociationHandle, AssociationHandler, AssociationInformation, AuthKey,
    BadEncoding, BlockNumber, Classes, CommandBuilder, CommandError, CommandHeaders, CommandMode,
    CommandSupport, DeadBandHeader, DirReadConfig, FileCredentials, FileError, FileHandle,
    FileMode, FileReadConfig, HeaderInfo, Headers, MasterChannel, MasterChannelType, OpenFile,
    PollError, ReadHandler, ReadRequest, ReadType, TaskError, TaskType, TimeSyncError,
    TimeSyncProcedure, WriteError,
};
use crate::util::lifecycle::{RunError, StopReason};
use crate::util::test_support::watchdog;

pub(super) const OUTSTATION: EndpointAddress = EndpointAddress::raw(1024);
pub(super) const OTHER: EndpointAddress = EndpointAddress::raw(2048);
pub(super) const UNKNOWN: EndpointAddress = EndpointAddress::raw(3000);

const HANDLE: u32 = 0xDEADCAFE;

const REJECTED: Iin = Iin::new(Iin1::new(0x00), Iin2::NO_FUNC_CODE_SUPPORT);

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
pub(super) enum Kind {
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
pub(super) enum Reply {
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
    pub(super) fn spawn(self, mut association: AssociationHandle) -> JoinHandle<Reply> {
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

    pub(super) fn task_type(self) -> TaskType {
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
    pub(super) fn function(self) -> FunctionCode {
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

    pub(super) fn ok(self) -> Reply {
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

    pub(super) fn failed(self, err: TaskError) -> Reply {
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

    /// The successful response to the first request
    pub(super) fn first_response(self, request: &[u8]) -> Vec<u8> {
        match self {
            Kind::Read | Kind::EmptyResponse | Kind::DeadBands | Kind::TimeSync => {
                empty_response(Sequence::new(0))
            }
            Kind::DirectOperate | Kind::SelectBeforeOperate => echo(request),
            Kind::ColdRestart => vec![0xC0, 0x81, 0x00, 0x00, 52, 1, 0x07, 1, 0x02, 0x00],
            Kind::AuthFile => file_response(
                0,
                &Group70Var2 {
                    auth_key: 42,
                    user_name: "",
                    password: "",
                },
            ),
            Kind::OpenFile => file_status_response(0, HANDLE, 8, 512, FileStatus::Success),
            Kind::CloseFile => file_status_response(0, HANDLE, 0, 0, FileStatus::Success),
            Kind::WriteFileBlock => file_response(
                0,
                &Group70Var6 {
                    file_handle: HANDLE,
                    block_number: 0,
                    status_code: FileStatus::Success,
                    text: "",
                },
            ),
            Kind::GetFileInfo => file_response(
                0,
                &Group70Var7 {
                    file_type: FileType::File,
                    file_size: 1234,
                    time_of_creation: Timestamp::new(5678),
                    permissions: Permissions::default(),
                    request_id: 0xCAFE,
                    file_name: "f.txt",
                },
            ),
            Kind::ReadDirectory => file_status_response(0, HANDLE, 0, 512, FileStatus::Success),
        }
    }

    /// True if the response to the first request must have exactly one object header
    fn expects_one_header(self) -> bool {
        match self {
            Kind::ColdRestart
            | Kind::AuthFile
            | Kind::OpenFile
            | Kind::CloseFile
            | Kind::WriteFileBlock
            | Kind::GetFileInfo
            | Kind::ReadDirectory => true,
            Kind::Read
            | Kind::DirectOperate
            | Kind::SelectBeforeOperate
            | Kind::EmptyResponse
            | Kind::DeadBands
            | Kind::TimeSync => false,
        }
    }

    /// Answer the first request, and any requests after it, until the task succeeds, with CON set
    /// in every response if `con`. Returns the sequence number of the last request.
    async fn finish(self, harness: &mut TestHarness, request: &[u8], con: bool) -> Sequence {
        answer(harness, con, self.first_response(request)).await;
        match self {
            Kind::Read
            | Kind::DirectOperate
            | Kind::ColdRestart
            | Kind::EmptyResponse
            | Kind::DeadBands
            | Kind::AuthFile
            | Kind::OpenFile
            | Kind::CloseFile
            | Kind::WriteFileBlock
            | Kind::GetFileInfo => Sequence::new(0),
            Kind::SelectBeforeOperate => {
                let operate = next_request(harness, 1, FunctionCode::Operate).await;
                answer(harness, con, echo(&operate)).await;
                Sequence::new(1)
            }
            Kind::TimeSync => {
                next_request(harness, 1, FunctionCode::Write).await;
                answer(harness, con, empty_response(Sequence::new(1))).await;
                Sequence::new(1)
            }
            Kind::ReadDirectory => {
                next_request(harness, 1, FunctionCode::Read).await;
                answer(
                    harness,
                    con,
                    file_transport_response(1, HANDLE, last_block(0), &[]),
                )
                .await;
                next_request(harness, 2, FunctionCode::CloseFile).await;
                answer(
                    harness,
                    con,
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

/// Respond, with CON set if `con`, and check that the master confirms a response with CON before it
/// writes anything else
async fn answer(harness: &mut TestHarness, con: bool, response: Vec<u8>) {
    if !con {
        respond(harness, response).await;
        return;
    }
    let seq = ControlField::from(response[0]).seq;
    respond(harness, with_con(response)).await;
    assert_eq!(harness.pop_write().await, solicited_confirm(seq));
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
        self.harness.barrier().await;
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
        self.harness.barrier().await;
        assert!(!self.reply.is_finished(), "{kind:?}");
        assert_eq!(self.harness.io.pop_event(), None, "{kind:?}");
        assert_eq!(self.harness.assoc_events.pop(), [], "{kind:?}");
    }

    /// Answer the task and check that it succeeds
    async fn expect_success(&mut self) {
        self.expect_success_with(false).await;
    }

    /// Answer the task, with CON set in every response if `con`, and check that it succeeds
    async fn expect_success_with(&mut self, con: bool) {
        let kind = self.kind;
        let request = self.request.clone();
        let last = kind.finish(&mut self.harness, &request, con).await;
        assert_eq!(self.reply().await, kind.ok(), "{kind:?}");
        self.harness.barrier().await;
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

/// A parseable response with the wrong objects is still confirmed if it has CON
#[tokio::test(start_paused = true)]
async fn every_task_fails_on_a_response_with_unexpected_objects() {
    watchdog(async {
        for con in [false, true] {
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
                answer(&mut started.harness, con, response).await;
                let unexpected = TaskError::UnexpectedResponseHeaders;
                let reply = match kind {
                    Kind::DirectOperate | Kind::SelectBeforeOperate => {
                        Reply::Command(Err(CommandError::Response(
                            crate::master::CommandResponseError::HeaderCountMismatch,
                        )))
                    }
                    _ => kind.failed(unexpected),
                };
                assert_eq!(started.reply().await, reply, "{kind:?} con: {con}");
                started.harness.barrier().await;
                assert_eq!(
                    started.harness.assoc_events.pop(),
                    [kind.failure_event(unexpected)],
                    "{kind:?} con: {con}"
                );
                assert_eq!(started.harness.io.pop_event(), None, "{kind:?} con: {con}");
            }
        }
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn every_task_confirms_responses_with_con_before_its_next_request() {
    watchdog(async {
        for con in [false, true] {
            for kind in KINDS {
                let mut started = start(kind).await;
                started.expect_success_with(con).await;
                started.harness.barrier().await;
                assert_eq!(started.harness.io.pop_event(), None, "{kind:?} con: {con}");
            }
        }
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn every_task_confirms_a_response_with_con_rejected_by_iin2() {
    watchdog(async {
        for kind in KINDS {
            let mut started = start(kind).await;
            let response = empty_response_custom_iin(Sequence::new(0), REJECTED);
            answer(&mut started.harness, true, response).await;
            started
                .expect_failure(TaskError::RejectedByIin2(REJECTED))
                .await;
            assert_eq!(started.harness.io.pop_event(), None, "{kind:?}");
        }
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn no_task_confirms_a_response_with_con_whose_objects_do_not_parse() {
    watchdog(async {
        for kind in KINDS {
            let mut started = start(kind).await;
            // a valid fragment whose object header is cut short after its group
            let response = with_con(vec![0xC0, 0x81, 0x00, 0x00, 0x01]);
            respond(&mut started.harness, response).await;
            started
                .expect_failure(TaskError::MalformedResponse(
                    ObjectParseError::InsufficientBytes,
                ))
                .await;
            assert_eq!(started.harness.io.pop_event(), None, "{kind:?}");
        }
    })
    .await;
}

/// A task whose response must have one object header fails if it has two, here the expected
/// header twice. The response is still confirmed if it has CON.
#[tokio::test(start_paused = true)]
async fn every_single_header_task_fails_on_a_response_with_two_headers() {
    watchdog(async {
        for con in [false, true] {
            for kind in KINDS.into_iter().filter(|kind| kind.expects_one_header()) {
                let mut started = start(kind).await;
                let mut response = kind.first_response(&started.request);
                let header = response[4..].to_vec();
                response.extend_from_slice(&header);
                answer(&mut started.harness, con, response).await;

                let err = TaskError::UnexpectedResponseHeaders;
                assert_eq!(
                    started.reply().await,
                    kind.failed(err),
                    "{kind:?} con: {con}"
                );
                started.harness.barrier().await;
                assert_eq!(
                    started.harness.assoc_events.pop(),
                    [kind.failure_event(err)],
                    "{kind:?} con: {con}"
                );
                assert_eq!(started.harness.io.pop_event(), None, "{kind:?} con: {con}");
            }
        }
    })
    .await;
}

/// Fragments that aren't valid responses: one too short for the IIN, and a solicited response with
/// the UNS bit
pub(super) const MALFORMED: [&[u8]; 2] = [&[0xC0, 0x81, 0x00], &[0xD0, 0x81, 0x00, 0x00]];

#[tokio::test(start_paused = true)]
async fn every_task_fails_on_a_malformed_fragment_from_its_outstation() {
    watchdog(async {
        for fragment in MALFORMED {
            for kind in KINDS {
                let mut started = start(kind).await;
                respond(&mut started.harness, fragment.to_vec()).await;
                started.expect_failure(TaskError::Transport).await;
            }
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
            started.harness.barrier().await;
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
async fn every_task_fails_with_a_link_error_when_the_connection_drops() {
    watchdog(async {
        let err = LinkError::Stdio(ErrorKind::ConnectionReset);
        for kind in KINDS {
            let mut started = start(kind).await;
            // queued directly, so that the receiver tells a completed promise from a dropped one
            let (promise, queued) = Promise::one_shot();
            started
                .harness
                .association
                .send_task(SingleReadTask::new(
                    ReadRequest::ClassScan(Classes::class0()),
                    promise,
                ))
                .await
                .unwrap();
            started.harness.barrier().await;

            started.harness.io.read_error(ErrorKind::ConnectionReset);
            assert_eq!(
                started.reply().await,
                kind.failed(TaskError::Link(err)),
                "{kind:?}"
            );
            assert_eq!(queued.await, Ok(Err(TaskError::Link(err))), "{kind:?}");
            assert_eq!(
                (&mut started.harness.task).await.unwrap(),
                RunError::Link(err),
                "{kind:?}"
            );
            assert_eq!(
                started.harness.assoc_events.pop(),
                [kind.failure_event(TaskError::Link(err))],
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
        harness.barrier().await;

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
async fn queued_tasks_fail_with_no_such_association_when_the_association_is_removed() {
    watchdog(async {
        let mut harness =
            create_association_with_handler(AssociationConfig::quiet(), Box::new(FixedTime)).await;

        let (promise, _running) = Promise::one_shot();
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

        // tasks queued directly, so that the receivers tell a completed promise from a dropped one
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
        let (promise, queued_time_sync) = Promise::one_shot();
        harness
            .association
            .queue_task(PendingTimeSync::user(TimeSyncProcedure::Lan, promise).into())
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
        harness.barrier().await;
        harness.assoc_events.pop();

        harness.master.remove_association(OUTSTATION).await.unwrap();
        harness.barrier().await;

        let err = TaskError::NoSuchAssociation(OUTSTATION);
        assert_eq!(queued_command.await, Ok(Err(CommandError::Task(err))));
        assert_eq!(queued_read.await, Ok(Err(err)));
        assert_eq!(queued_link_status.await, Ok(Err(err)));
        assert_eq!(queued_time_sync.await, Ok(Err(TimeSyncError::Task(err))));
        assert_eq!(
            reader.pop(),
            [ReaderEvent::Aborted(FileError::TaskError(err))]
        );
        assert_eq!(harness.assoc_events.pop(), []);
        assert!(!harness.task.is_finished());

        // the reader isn't told again once the running task ends
        tokio::time::sleep(Duration::from_secs(2)).await;
        harness.barrier().await;
        assert_eq!(reader.pop(), []);
        assert!(!harness.task.is_finished());
    })
    .await;
}

/// A response with CON isn't confirmed either, since it's for an association that no longer exists
#[tokio::test(start_paused = true)]
async fn every_task_fails_with_no_such_association_when_removed_while_it_waits() {
    watchdog(async {
        for con in [false, true] {
            for kind in KINDS {
                let mut started = start(kind).await;
                started
                    .harness
                    .master
                    .remove_association(OUTSTATION)
                    .await
                    .unwrap();
                started.harness.barrier().await;
                let response = kind.first_response(&started.request);
                let response = if con { with_con(response) } else { response };
                respond(&mut started.harness, response).await;

                let err = TaskError::NoSuchAssociation(OUTSTATION);
                assert_eq!(
                    started.reply().await,
                    kind.failed(err),
                    "{kind:?} con: {con}"
                );
                started.harness.barrier().await;
                assert_eq!(started.harness.read_events.pop(), [], "{kind:?} con: {con}");
                assert_eq!(
                    started.harness.assoc_events.pop(),
                    [],
                    "{kind:?} con: {con}"
                );
                assert_eq!(started.harness.io.pop_event(), None, "{kind:?} con: {con}");
                assert!(!started.harness.task.is_finished(), "{kind:?} con: {con}");
            }
        }
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn requests_to_a_removed_association_fail_with_no_such_association() {
    watchdog(async {
        for via_handle in [false, true] {
            let mut harness =
                create_association_with_handler(AssociationConfig::quiet(), Box::new(FixedTime))
                    .await;
            let mut stale = harness.association.clone();
            if via_handle {
                harness.association.clone().remove().await.unwrap();
            } else {
                harness.master.remove_association(OUTSTATION).await.unwrap();
            }
            harness.barrier().await;

            let err = TaskError::NoSuchAssociation(OUTSTATION);
            for kind in KINDS {
                let reply = kind.spawn(stale.clone()).await.unwrap();
                assert_eq!(reply, kind.failed(err), "{kind:?} {via_handle}");
            }
            let poll = stale
                .add_poll(
                    ReadRequest::ClassScan(Classes::class0()),
                    Duration::from_secs(1),
                )
                .await;
            assert_eq!(
                poll.err(),
                Some(PollError::NoSuchAssociation(OUTSTATION)),
                "{via_handle}"
            );
            assert_eq!(harness.assoc_events.pop(), [], "{via_handle}");

            // the master still serves another association, and the first thing it writes is that
            // association's request
            let log = Log::default();
            let mut other = harness
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
            let read =
                tokio::spawn(
                    async move { other.read(ReadRequest::ClassScan(Classes::class0())).await },
                );
            next_request(&mut harness, 0, FunctionCode::Read).await;
            respond(&mut harness, empty_response(Sequence::new(0))).await;
            assert_eq!(read.await.unwrap(), Ok(()), "{via_handle}");
        }
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn adding_an_association_fails_for_a_used_address_or_the_wrong_channel_type() {
    watchdog(async {
        let mut started = start(Kind::Read).await;
        let log = Log::default();
        let duplicate = started
            .harness
            .master
            .add_association(
                OUTSTATION,
                AssociationConfig::quiet(),
                Box::new(log.clone()),
                Box::new(log.clone()),
                Box::new(log.clone()),
            )
            .await;
        assert_eq!(
            duplicate.err(),
            Some(AssociationError::DuplicateAddress(OUTSTATION))
        );

        let destination = "127.0.0.1:20000".parse().unwrap();
        let udp = started
            .harness
            .master
            .add_udp_association(
                OTHER,
                destination,
                AssociationConfig::quiet(),
                Box::new(log.clone()),
                Box::new(log.clone()),
                Box::new(log.clone()),
            )
            .await;
        assert_eq!(
            udp.err(),
            Some(AssociationError::WrongChannelType {
                actual: MasterChannelType::Stream,
                required: MasterChannelType::Udp,
            })
        );

        let (tx, _rx) = crate::util::channel::request_channel();
        let mut udp_channel = MasterChannel::new(tx, MasterChannelType::Udp);
        let stream = udp_channel
            .add_association(
                OTHER,
                AssociationConfig::quiet(),
                Box::new(log.clone()),
                Box::new(log.clone()),
                Box::new(log.clone()),
            )
            .await;
        assert_eq!(
            stream.err(),
            Some(AssociationError::WrongChannelType {
                actual: MasterChannelType::Udp,
                required: MasterChannelType::Stream,
            })
        );

        // the existing association still handles its outstation
        started.expect_success().await;
        assert_eq!(
            started.harness.read_events.pop(),
            [
                ReadEvent::BeginFragment(ReadType::SinglePoll),
                ReadEvent::EndFragment(ReadType::SinglePoll),
            ]
        );
        assert_eq!(log.pop(), []);
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
            harness.barrier().await;
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
pub(super) enum Entry {
    Read(ReadEvent),
    Info(AssocInfoEvent),
}

#[derive(Clone, Default)]
pub(super) struct Log(Arc<Mutex<Vec<Entry>>>);

impl Log {
    fn push(&self, entry: Entry) {
        self.0.lock().unwrap().push(entry);
    }

    pub(super) fn pop(&self) -> Vec<Entry> {
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

    fn handle_binary_input(
        &mut self,
        _info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (crate::app::measurement::BinaryInput, u16)>,
    ) {
        iter.for_each(drop);
        self.push(Entry::Read(ReadEvent::BinaryInput));
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
    response(
        ControlField::response(seq, fir, fin, con),
        Iin::default(),
        &analog_event(0x01, 7),
    )
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

#[tokio::test(start_paused = true)]
async fn writing_an_attribute_whose_variation_cannot_carry_a_value_sends_nothing() {
    watchdog(async {
        let mut harness = create_association(AssociationConfig::quiet()).await;

        for variation in [
            0,
            var::ALL_ATTRIBUTES_REQUEST,
            var::LIST_OF_ATTRIBUTE_VARIATIONS,
        ] {
            let headers = Headers::new().add_attribute(OwnedAttribute::new(
                AttrSet::Default,
                variation,
                OwnedAttrValue::UnsignedInt(0),
            ));
            assert_eq!(
                harness
                    .association
                    .send_and_expect_empty_response(FunctionCode::Write, headers)
                    .await,
                Err(WriteError::Task(TaskError::BadEncoding(
                    BadEncoding::Attribute(BadAttribute::BadVariation(variation))
                )))
            );
            harness.assert_no_events();
        }
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn reading_group0_var0_sends_nothing() {
    watchdog(async {
        let mut harness = create_association(AssociationConfig::quiet()).await;
        let g0v0 = Variation::Group0(0);
        let err = TaskError::BadEncoding(BadEncoding::UndefinedVariation(g0v0));

        assert_eq!(
            harness
                .association
                .read(ReadRequest::all_objects(g0v0))
                .await,
            Err(err)
        );
        harness.assert_no_events();

        assert_eq!(
            harness
                .association
                .send_and_expect_empty_response(
                    FunctionCode::Read,
                    Headers::new().add_all_objects(g0v0)
                )
                .await,
            Err(WriteError::Task(err))
        );
        harness.assert_no_events();

        let mut poll = harness
            .association
            .add_poll(ReadRequest::all_objects(g0v0), Duration::from_secs(3600))
            .await
            .unwrap();
        harness.assoc_events.pop();
        poll.demand().await.unwrap();
        harness.expect_idle_for(Duration::from_secs(1)).await;
        assert_eq!(
            harness.assoc_events.pop(),
            [
                AssocInfoEvent::TaskStart(
                    TaskType::PeriodicPoll,
                    FunctionCode::Read,
                    Sequence::new(2)
                ),
                AssocInfoEvent::TaskFailure(TaskType::PeriodicPoll, err),
            ]
        );
    })
    .await;
}
