use std::time::Duration;

use tokio::task::JoinHandle;

use crate::app::{
    FileStatus, FunctionCode, Group70Var3, Group70Var5, Iin, Iin1, Iin2, Permissions, Sequence,
    Shutdown, Timestamp,
};
use crate::master::association::AssociationConfig;
use crate::master::tasks::file::REQUEST_ID;
use crate::master::tests::file::auth_file::{auth_response, AUTH_REQUEST};
use crate::master::tests::file::close_file::close_request_bytes_for;
use crate::master::tests::file::{ReaderEvent, RecordingReader};
use crate::master::tests::harness::requests::empty_response_custom_iin;
use crate::master::tests::harness::{create_association, AssocInfoEvent, TestHarness};
use crate::master::{
    DirReadConfig, FileAction, FileCredentials, FileError, FileMode, FileReadConfig, TaskError,
    TaskType,
};
use crate::util::session::{RunError, StopReason};
use crate::util::test_support::watchdog;

const HANDLE: u32 = 0xDEADCAFE;

fn read_config() -> FileReadConfig {
    FileReadConfig {
        max_block_size: 256,
        max_file_size: 8,
    }
}

/// Every message sent before this returns has been processed by the master task
async fn barrier(harness: &mut TestHarness) {
    harness.master.get_decode_level().await.unwrap();
}

async fn start_read(
    harness: &mut TestHarness,
    config: FileReadConfig,
    credentials: Option<FileCredentials>,
) -> RecordingReader {
    let reader = RecordingReader::default();
    harness
        .association
        .read_file("f.txt", config, Box::new(reader.clone()), credentials)
        .await
        .unwrap();
    reader
}

/// Start a directory read with the name and limits of a file read, so that it sends the same
/// requests, and return its reply with the number of entries
fn start_directory_read(
    harness: &TestHarness,
    config: FileReadConfig,
    credentials: Option<FileCredentials>,
) -> JoinHandle<Result<usize, FileError>> {
    let mut association = harness.association.clone();
    let config = DirReadConfig {
        max_block_size: config.max_block_size,
        max_file_size: config.max_file_size,
    };
    tokio::spawn(async move {
        association
            .read_directory("f.txt", config, credentials)
            .await
            .map(|entries| entries.len())
    })
}

fn credentials() -> FileCredentials {
    FileCredentials {
        user_name: "user".to_string(),
        password: "pass".to_string(),
    }
}

fn open_request(seq: u8, auth_key: u32) -> Vec<u8> {
    super::request(
        FunctionCode::OpenFile,
        seq,
        &Group70Var3 {
            time_of_creation: Timestamp::zero(),
            permissions: Permissions::default(),
            auth_key,
            file_size: 0,
            mode: FileMode::Read,
            max_block_size: 256,
            request_id: REQUEST_ID,
            file_name: "f.txt",
        },
    )
}

fn read_request(seq: u8, block: u32) -> Vec<u8> {
    super::request(
        FunctionCode::Read,
        seq,
        &Group70Var5 {
            file_handle: HANDLE,
            block_number: block,
            file_data: &[],
        },
    )
}

fn open_response(seq: u8, status: FileStatus) -> Vec<u8> {
    super::file_status_response(seq, HANDLE, 8, 128, status)
}

/// Open without credentials and expect the READ of block 0
async fn open(harness: &mut TestHarness) {
    harness.expect_write(open_request(0, 0)).await;
    harness
        .process_response(open_response(0, FileStatus::Success))
        .await;
    harness.expect_write(read_request(1, 0)).await;
}

/// [`open`], and check that the reader was told the file size
async fn open_file(harness: &mut TestHarness, reader: &RecordingReader) {
    open(harness).await;
    assert_eq!(reader.pop(), [ReaderEvent::Opened(8)]);
}

/// Open without credentials, read a single last block, and expect the CLOSE
async fn read_to_close(harness: &mut TestHarness, reader: &RecordingReader) {
    open_file(harness, reader).await;
    harness
        .process_response(super::file_transport_response(
            1,
            HANDLE,
            super::last_block(0),
            b"data",
        ))
        .await;
    assert_eq!(
        reader.pop(),
        [
            ReaderEvent::Block(0, b"data".to_vec()),
            ReaderEvent::Completed
        ]
    );
    harness
        .expect_write(close_request_bytes_for(0xC2, HANDLE))
        .await;
}

/// How the `AssociationInformation` sees the read task end
#[derive(Copy, Clone, Debug)]
enum Finished {
    /// `task_success`, with the sequence number of the last request
    Success(u8),
    /// `task_fail` with `UnexpectedResponseHeaders`
    Failure,
}

/// Check that the read task, whose first request had function code `first`, finished this way and
/// that the master kept running
async fn expect_finished(harness: &mut TestHarness, first: FunctionCode, finished: Finished) {
    barrier(harness).await;
    let last = match finished {
        Finished::Success(seq) => {
            AssocInfoEvent::TaskSuccess(TaskType::FileRead, first, Sequence::new(seq))
        }
        Finished::Failure => {
            AssocInfoEvent::TaskFailure(TaskType::FileRead, TaskError::UnexpectedResponseHeaders)
        }
    };
    assert_eq!(
        harness.assoc_events.pop(),
        [
            AssocInfoEvent::TaskStart(TaskType::FileRead, first, Sequence::new(0)),
            last
        ]
    );
    assert!(!harness.task.is_finished());
}

#[tokio::test]
async fn full_transfer_with_credentials() {
    let mut harness = create_association(AssociationConfig::quiet()).await;
    let reader = start_read(&mut harness, read_config(), Some(credentials())).await;

    harness.expect_write(AUTH_REQUEST.to_vec()).await;
    harness.process_response(auth_response(0, 7)).await;

    harness.expect_write(open_request(1, 7)).await;
    harness
        .process_response(open_response(1, FileStatus::Success))
        .await;
    assert_eq!(reader.pop(), [ReaderEvent::Opened(8)]);

    harness.expect_write(read_request(2, 0)).await;
    harness
        .process_response(super::file_transport_response(2, HANDLE, 0, b"abcd"))
        .await;
    assert_eq!(reader.pop(), [ReaderEvent::Block(0, b"abcd".to_vec())]);

    harness.expect_write(read_request(3, 1)).await;
    harness
        .process_response(super::file_transport_response(
            3,
            HANDLE,
            super::last_block(1),
            b"efgh",
        ))
        .await;
    assert_eq!(
        reader.pop(),
        [
            ReaderEvent::Block(1, b"efgh".to_vec()),
            ReaderEvent::Completed
        ]
    );

    harness
        .expect_write(close_request_bytes_for(0xC4, HANDLE))
        .await;
    harness
        .process_response(super::file_status_response(
            4,
            HANDLE,
            0,
            0,
            FileStatus::Success,
        ))
        .await;
    assert_eq!(reader.pop(), []);
    harness.assert_no_events();
    expect_finished(
        &mut harness,
        FunctionCode::AuthenticateFile,
        Finished::Success(4),
    )
    .await;
}

/// An AUTHENTICATE or OPEN response that fails a read before the file is open, so nothing is closed
#[derive(Copy, Clone, Debug)]
enum OpenFailure {
    /// AUTHENTICATE returns auth key 0
    NoPermission,
    /// AUTHENTICATE is answered with a g70v4
    AuthVariation,
    /// OPEN returns an unsuccessful status
    OpenStatus,
    /// OPEN is answered with a g70v2
    OpenVariation,
    /// OPEN is answered without an object header
    NoObjectHeader,
}

const OPEN_FAILURES: [OpenFailure; 5] = [
    OpenFailure::NoPermission,
    OpenFailure::AuthVariation,
    OpenFailure::OpenStatus,
    OpenFailure::OpenVariation,
    OpenFailure::NoObjectHeader,
];

impl OpenFailure {
    fn credentials(self) -> Option<FileCredentials> {
        match self {
            Self::NoPermission | Self::AuthVariation => Some(credentials()),
            Self::OpenStatus | Self::OpenVariation | Self::NoObjectHeader => None,
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
            Self::AuthVariation | Self::OpenVariation => FileError::BadResponse,
            Self::OpenStatus => FileError::BadStatus(FileStatus::FileNotFound),
            Self::NoObjectHeader => FileError::TaskError(TaskError::UnexpectedResponseHeaders),
        }
    }

    /// Expect the first request, and answer it so that the read fails
    async fn answer(self, harness: &mut TestHarness) {
        let (request, response) = match self {
            Self::NoPermission => (AUTH_REQUEST.to_vec(), auth_response(0, 0)),
            Self::AuthVariation => (AUTH_REQUEST.to_vec(), open_response(0, FileStatus::Success)),
            Self::OpenStatus => (
                open_request(0, 0),
                open_response(0, FileStatus::FileNotFound),
            ),
            Self::OpenVariation => (open_request(0, 0), auth_response(0, 7)),
            Self::NoObjectHeader => (open_request(0, 0), super::no_object_response(0)),
        };
        harness.expect_write(request).await;
        harness.process_response(response).await;
    }
}

#[tokio::test]
async fn open_failures_abort_the_reader_and_fail_the_task_without_a_close() {
    for failure in OPEN_FAILURES {
        let mut harness = create_association(AssociationConfig::quiet()).await;
        let reader = start_read(&mut harness, read_config(), failure.credentials()).await;
        failure.answer(&mut harness).await;
        expect_finished(&mut harness, failure.first(), Finished::Failure).await;
        harness.assert_no_events();
        assert_eq!(
            reader.pop(),
            [ReaderEvent::Aborted(failure.error())],
            "{failure:?}"
        );
    }
}

/// A READ response that ends the transfer early, after which the master closes the file
#[derive(Copy, Clone, Debug)]
enum Ending {
    /// READ is answered with a g70v4
    BlockVariation,
    /// READ returns a block other than the one requested
    BlockNumber,
    /// READ returns the last block of a file handle other than the one opened
    Handle,
    /// READ is answered without an object header
    NoObjectHeader,
    /// the block exceeds the configured maximum file size
    MaxFileSize,
    /// the reader aborts a block that isn't the last
    ReaderAbort,
    /// the reader aborts the last block
    ReaderAbortOnLastBlock,
}

const ENDINGS: [Ending; 7] = [
    Ending::BlockVariation,
    Ending::BlockNumber,
    Ending::Handle,
    Ending::NoObjectHeader,
    Ending::MaxFileSize,
    Ending::ReaderAbort,
    Ending::ReaderAbortOnLastBlock,
];

/// An ending by the outstation and one by the master, to end a read before its CLOSE fails
const CLEANUP_ENDINGS: [Ending; 2] = [Ending::BlockNumber, Ending::MaxFileSize];

impl Ending {
    fn config(self) -> FileReadConfig {
        match self {
            Self::MaxFileSize => FileReadConfig {
                max_block_size: 256,
                max_file_size: 3,
            },
            Self::BlockVariation
            | Self::BlockNumber
            | Self::Handle
            | Self::NoObjectHeader
            | Self::ReaderAbort
            | Self::ReaderAbortOnLastBlock => read_config(),
        }
    }

    fn reader_aborts(self) -> bool {
        match self {
            Self::ReaderAbort | Self::ReaderAbortOnLastBlock => true,
            Self::BlockVariation
            | Self::BlockNumber
            | Self::Handle
            | Self::NoObjectHeader
            | Self::MaxFileSize => false,
        }
    }

    /// How the task ends when its CLOSE ends it with `closed`: an outstation that ended the
    /// transfer fails the task, while the master ending it leaves the outcome to the CLOSE
    fn finished(self, closed: Finished) -> Finished {
        match self {
            Self::MaxFileSize | Self::ReaderAbort | Self::ReaderAbortOnLastBlock => closed,
            Self::BlockVariation | Self::BlockNumber | Self::Handle | Self::NoObjectHeader => {
                Finished::Failure
            }
        }
    }

    fn response(self) -> Vec<u8> {
        match self {
            Self::BlockVariation => open_response(1, FileStatus::Success),
            Self::BlockNumber => super::file_transport_response(1, HANDLE, 1, b"data"),
            Self::Handle => {
                super::file_transport_response(1, HANDLE + 1, super::last_block(0), b"data")
            }
            Self::NoObjectHeader => super::no_object_response(1),
            Self::MaxFileSize | Self::ReaderAbort => {
                super::file_transport_response(1, HANDLE, 0, b"data")
            }
            Self::ReaderAbortOnLastBlock => {
                super::file_transport_response(1, HANDLE, super::last_block(0), b"data")
            }
        }
    }

    /// The error that ends the transfer
    fn error(self) -> FileError {
        match self {
            Self::BlockVariation => FileError::BadResponse,
            Self::BlockNumber => FileError::BadBlockNum,
            Self::Handle => FileError::WrongHandle,
            Self::NoObjectHeader => FileError::TaskError(TaskError::UnexpectedResponseHeaders),
            Self::MaxFileSize => FileError::MaxLengthExceeded,
            Self::ReaderAbort | Self::ReaderAbortOnLastBlock => FileError::AbortByUser,
        }
    }

    /// What the reader is told after it opened the file, all before the CLOSE
    fn events(self) -> Vec<ReaderEvent> {
        let aborted = ReaderEvent::Aborted(self.error());
        if self.reader_aborts() {
            vec![ReaderEvent::Block(0, b"data".to_vec()), aborted]
        } else {
            vec![aborted]
        }
    }
}

fn close_success(seq: u8) -> Vec<u8> {
    super::file_status_response(seq, HANDLE, 0, 0, FileStatus::Success)
}

/// Every CLOSE response with how it ends a task that it decides: success, and the responses that
/// fail validation (a bad status, the wrong handle, the wrong variation, and no object header)
fn close_responses(seq: u8) -> [(Vec<u8>, Finished); 5] {
    [
        (close_success(seq), Finished::Success(seq)),
        (
            super::file_status_response(seq, HANDLE, 0, 0, FileStatus::InvalidHandle),
            Finished::Failure,
        ),
        (
            super::file_status_response(seq, HANDLE + 1, 0, 0, FileStatus::Success),
            Finished::Failure,
        ),
        (
            super::file_transport_response(seq, HANDLE, 0, b""),
            Finished::Failure,
        ),
        (super::no_object_response(seq), Finished::Failure),
    ]
}

/// Start a read and end it early with `ending`, and check that the reader is told once and before
/// the master closes the file
async fn end_file_read(harness: &mut TestHarness, ending: Ending) -> RecordingReader {
    let reader = start_read(harness, ending.config(), None).await;
    open_file(harness, &reader).await;
    if ending.reader_aborts() {
        reader.set_action(FileAction::Abort);
    }
    harness.process_response(ending.response()).await;
    harness
        .expect_write(close_request_bytes_for(0xC2, HANDLE))
        .await;
    assert_eq!(reader.pop(), ending.events(), "{ending:?}");
    reader
}

#[tokio::test]
async fn early_end_closes_the_file_without_another_callback() {
    for ending in ENDINGS {
        for (close, closed) in close_responses(2) {
            let mut harness = create_association(AssociationConfig::quiet()).await;
            let reader = end_file_read(&mut harness, ending).await;
            harness.process_response(close).await;
            expect_finished(
                &mut harness,
                FunctionCode::OpenFile,
                ending.finished(closed),
            )
            .await;
            harness.assert_no_events();
            assert_eq!(reader.pop(), [], "{ending:?}");
        }
    }
}

#[tokio::test]
async fn reader_abort_on_open_closes_the_file_without_another_callback() {
    for (close, finished) in close_responses(1) {
        let mut harness = create_association(AssociationConfig::quiet()).await;
        let reader = start_read(&mut harness, read_config(), None).await;
        reader.set_action(FileAction::Abort);
        harness.expect_write(open_request(0, 0)).await;
        harness
            .process_response(open_response(0, FileStatus::Success))
            .await;
        harness
            .expect_write(close_request_bytes_for(0xC1, HANDLE))
            .await;
        assert_eq!(
            reader.pop(),
            [
                ReaderEvent::Opened(8),
                ReaderEvent::Aborted(FileError::AbortByUser)
            ]
        );
        harness.process_response(close).await;
        expect_finished(&mut harness, FunctionCode::OpenFile, finished).await;
        harness.assert_no_events();
        assert_eq!(reader.pop(), []);
    }
}

#[tokio::test]
async fn close_after_the_last_block_decides_the_task_and_never_reaches_the_reader() {
    for (close, finished) in close_responses(2) {
        let mut harness = create_association(AssociationConfig::quiet()).await;
        let reader = start_read(&mut harness, read_config(), None).await;
        read_to_close(&mut harness, &reader).await;
        harness.process_response(close).await;
        expect_finished(&mut harness, FunctionCode::OpenFile, finished).await;
        harness.assert_no_events();
        assert_eq!(reader.pop(), []);
    }
}

/// The request a file read waits on when something other than its response ends it
#[derive(Copy, Clone, Debug)]
enum Waiting {
    /// the READ of block 0
    Read,
    /// the CLOSE after the transfer ended this way
    Close(Ending),
}

/// What ends a request other than its response
#[derive(Copy, Clone, Debug)]
enum Interruption {
    Timeout,
    RejectedByIin2,
    /// a fragment from the outstation that isn't a valid response
    Malformed(&'static [u8]),
    Disabled,
    Shutdown,
}

const INTERRUPTIONS: [Interruption; 6] = [
    Interruption::Timeout,
    Interruption::RejectedByIin2,
    // too short for the IIN
    Interruption::Malformed(&[0xC0, 0x81, 0x00]),
    // a solicited response with the UNS bit
    Interruption::Malformed(&[0xD0, 0x81, 0x00, 0x00]),
    Interruption::Disabled,
    Interruption::Shutdown,
];

#[tokio::test(start_paused = true)]
async fn interrupted_file_read_fails_the_task_and_tells_the_reader_once_without_a_close() {
    const REJECTED: Iin = Iin::new(Iin1::new(0x00), Iin2::NO_FUNC_CODE_SUPPORT);
    watchdog(async {
        let waits = [Waiting::Read]
            .into_iter()
            .chain(CLEANUP_ENDINGS.map(Waiting::Close));
        for waiting in waits {
            for interruption in INTERRUPTIONS {
                let mut harness = create_association(AssociationConfig::quiet()).await;
                let (reader, seq) = match waiting {
                    Waiting::Read => {
                        let reader = start_read(&mut harness, read_config(), None).await;
                        open_file(&mut harness, &reader).await;
                        (reader, 1)
                    }
                    Waiting::Close(ending) => (end_file_read(&mut harness, ending).await, 2),
                };
                let err = match interruption {
                    Interruption::Timeout => {
                        tokio::time::sleep(Duration::from_millis(999)).await;
                        barrier(&mut harness).await;
                        assert_eq!(reader.pop(), [], "{waiting:?}");
                        tokio::time::sleep(Duration::from_millis(2)).await;
                        barrier(&mut harness).await;
                        TaskError::ResponseTimeout
                    }
                    Interruption::RejectedByIin2 => {
                        harness
                            .process_response(empty_response_custom_iin(
                                Sequence::new(seq),
                                REJECTED,
                            ))
                            .await;
                        barrier(&mut harness).await;
                        TaskError::RejectedByIin2(REJECTED)
                    }
                    Interruption::Malformed(fragment) => {
                        harness.process_response(fragment.to_vec()).await;
                        barrier(&mut harness).await;
                        TaskError::Transport
                    }
                    Interruption::Disabled => {
                        harness.master.disable().await.unwrap();
                        assert_eq!(
                            (&mut harness.task).await.unwrap(),
                            RunError::Stop(StopReason::Disable)
                        );
                        TaskError::Disabled
                    }
                    Interruption::Shutdown => {
                        drop(harness.association);
                        drop(harness.master);
                        assert_eq!(
                            (&mut harness.task).await.unwrap(),
                            RunError::Stop(StopReason::Shutdown)
                        );
                        TaskError::Shutdown
                    }
                };
                let context = format!("{waiting:?} {interruption:?}");
                assert_eq!(
                    harness.assoc_events.pop(),
                    [
                        AssocInfoEvent::TaskStart(
                            TaskType::FileRead,
                            FunctionCode::OpenFile,
                            Sequence::new(0)
                        ),
                        AssocInfoEvent::TaskFailure(TaskType::FileRead, err)
                    ],
                    "{context}"
                );
                let told = match waiting {
                    Waiting::Read => vec![ReaderEvent::Aborted(FileError::TaskError(err))],
                    Waiting::Close(_) => vec![],
                };
                assert_eq!(reader.pop(), told, "{context}");
                assert_eq!(harness.io.pop_event(), None, "{context}");
            }
        }
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn file_read_requested_after_the_master_stops_aborts_the_reader_once() {
    watchdog(async {
        let mut harness = create_association(AssociationConfig::quiet()).await;
        harness.task.abort();
        assert!((&mut harness.task).await.unwrap_err().is_cancelled());

        // the request is dropped unsent, with the reader
        let reader = RecordingReader::default();
        let result = harness
            .association
            .read_file(
                "f.txt",
                FileReadConfig::default(),
                Box::new(reader.clone()),
                None,
            )
            .await;
        assert!(matches!(result, Err(Shutdown)));
        assert_eq!(
            reader.pop(),
            [ReaderEvent::Aborted(FileError::TaskError(
                TaskError::Shutdown
            ))]
        );
    })
    .await;
}

/// Start a directory read and end it early with `ending`, and check that it returned before the
/// master closes the file
async fn end_directory_read(harness: &mut TestHarness, ending: Ending) {
    let reply = start_directory_read(harness, ending.config(), None);
    open(harness).await;
    harness.process_response(ending.response()).await;
    harness
        .expect_write(close_request_bytes_for(0xC2, HANDLE))
        .await;
    assert_eq!(reply.await.unwrap(), Err(ending.error()), "{ending:?}");
}

#[tokio::test]
async fn directory_read_fails_with_the_error_that_ended_the_transfer() {
    for failure in OPEN_FAILURES {
        let mut harness = create_association(AssociationConfig::quiet()).await;
        let reply = start_directory_read(&harness, read_config(), failure.credentials());
        failure.answer(&mut harness).await;
        assert_eq!(reply.await.unwrap(), Err(failure.error()), "{failure:?}");
        expect_finished(&mut harness, failure.first(), Finished::Failure).await;
        harness.assert_no_events();
    }
    for ending in ENDINGS.into_iter().filter(|ending| !ending.reader_aborts()) {
        let mut harness = create_association(AssociationConfig::quiet()).await;
        end_directory_read(&mut harness, ending).await;
        harness.process_response(close_success(2)).await;
        let finished = ending.finished(Finished::Success(2));
        expect_finished(&mut harness, FunctionCode::OpenFile, finished).await;
        harness.assert_no_events();
    }
}

#[tokio::test]
async fn directory_read_with_unparseable_entries_fails_the_read_but_not_the_task() {
    // a g70v7 entry for a file named "f", followed by a byte that isn't an entry
    const ENTRY_THEN_JUNK: &[u8] = &[
        20, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, b'f', 0x01,
    ];
    let config = FileReadConfig {
        max_block_size: 256,
        max_file_size: 64,
    };
    for data in [[0x01].as_slice(), ENTRY_THEN_JUNK] {
        let mut harness = create_association(AssociationConfig::quiet()).await;
        let reply = start_directory_read(&harness, config, None);
        open(&mut harness).await;
        harness
            .process_response(super::file_transport_response(
                1,
                HANDLE,
                super::last_block(0),
                data,
            ))
            .await;
        assert_eq!(
            reply.await.unwrap(),
            Err(FileError::BadResponse),
            "{data:?}"
        );
        harness
            .expect_write(close_request_bytes_for(0xC2, HANDLE))
            .await;
        harness.process_response(close_success(2)).await;
        expect_finished(&mut harness, FunctionCode::OpenFile, Finished::Success(2)).await;
    }
}

#[tokio::test(start_paused = true)]
async fn directory_read_cleanup_that_fails_keeps_the_original_error() {
    watchdog(async {
        for ending in CLEANUP_ENDINGS {
            for timeout in [false, true] {
                let mut harness = create_association(AssociationConfig::quiet()).await;
                end_directory_read(&mut harness, ending).await;
                let err = if timeout {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    TaskError::ResponseTimeout
                } else {
                    harness
                        .process_response(super::file_status_response(
                            2,
                            HANDLE,
                            0,
                            0,
                            FileStatus::InvalidHandle,
                        ))
                        .await;
                    TaskError::UnexpectedResponseHeaders
                };
                barrier(&mut harness).await;
                assert_eq!(
                    harness.assoc_events.pop(),
                    [
                        AssocInfoEvent::TaskStart(
                            TaskType::FileRead,
                            FunctionCode::OpenFile,
                            Sequence::new(0)
                        ),
                        AssocInfoEvent::TaskFailure(TaskType::FileRead, err)
                    ],
                    "{ending:?} {timeout}"
                );
                harness.assert_no_events();
            }
        }
    })
    .await;
}
