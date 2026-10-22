use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::app::{
    FileStatus, FunctionCode, Group70Var3, Group70Var5, MaybeAsync, Permissions, Sequence,
    Timestamp,
};
use crate::master::association::AssociationConfig;
use crate::master::tasks::file::REQUEST_ID;
use crate::master::tests::file::auth_file::{auth_response, AUTH_REQUEST};
use crate::master::tests::file::close_file::close_request_bytes_for;
use crate::master::tests::harness::{create_association, AssocInfoEvent, TestHarness};
use crate::master::{
    FileAction, FileCredentials, FileError, FileMode, FileReadConfig, FileReader, TaskError,
    TaskType,
};

#[derive(Debug, PartialEq, Eq)]
enum Event {
    Open(u32),
    Rx(u32, Vec<u8>),
    Abort(FileError),
    Complete,
}

#[derive(Default)]
struct State {
    action: FileAction,
    events: VecDeque<Event>,
}

impl State {
    fn push(&mut self, event: Event) {
        self.events.push_back(event);
    }
}

struct EventHandle {
    state: Arc<Mutex<State>>,
}

impl EventHandle {
    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap()
    }

    fn pop(&self) -> Option<Event> {
        self.state().events.pop_front()
    }
}

struct MockReader {
    state: Arc<Mutex<State>>,
}

fn pair() -> (EventHandle, Box<dyn FileReader>) {
    let state = Arc::new(Mutex::new(State::default()));
    (
        EventHandle {
            state: state.clone(),
        },
        Box::new(MockReader { state }),
    )
}

impl FileReader for MockReader {
    fn opened(&mut self, size: u32) -> FileAction {
        let mut state = self.state.lock().unwrap();
        state.push(Event::Open(size));
        state.action
    }

    fn block_received(&mut self, block_num: u32, data: &[u8]) -> MaybeAsync<FileAction> {
        let mut state = self.state.lock().unwrap();
        state.push(Event::Rx(block_num, data.to_vec()));
        MaybeAsync::ready(state.action)
    }

    fn aborted(&mut self, err: FileError) {
        let mut state = self.state.lock().unwrap();
        state.push(Event::Abort(err));
    }

    fn completed(&mut self) {
        let mut state = self.state.lock().unwrap();
        state.push(Event::Complete);
    }
}

#[tokio::test]
async fn aborts_when_no_object_header() {
    let config = AssociationConfig::quiet();
    let mut harness = create_association(config).await;
    let (events, reader) = pair();
    harness
        .association
        .read_file(
            "./test.txt".to_string(),
            FileReadConfig::default(),
            reader,
            None,
        )
        .await
        .unwrap();

    // check that its a file open request
    assert_eq!(harness.pop_write().await[..5], [0xC0, 25, 70, 3, 0x5B]);

    harness
        .process_response([0xC0, 0x81, 0x00, 0x00].to_vec())
        .await;

    assert_eq!(
        events.pop().unwrap(),
        Event::Abort(FileError::TaskError(TaskError::UnexpectedResponseHeaders))
    );
}

#[tokio::test]
async fn closes_file_on_completion() {
    let config = AssociationConfig::quiet();
    let mut harness = create_association(config).await;
    let (events, reader) = pair();
    harness
        .association
        .read_file(
            "./test.txt".to_string(),
            FileReadConfig::default(),
            reader,
            None,
        )
        .await
        .unwrap();

    // file open
    assert_eq!(harness.pop_write().await[..5], [0xC0, 25, 70, 3, 0x5B]);

    let handle = 0xDEADCAFE;
    let data = b"data".as_slice();

    harness
        .process_response(super::file_status_response(
            0,
            handle,
            24,
            0,
            FileStatus::Success,
        ))
        .await;

    assert_eq!(events.pop().unwrap(), Event::Open(24));

    // file read
    assert_eq!(
        harness.pop_write().await[..6],
        [0xC1, 0x01, 70, 5, 0x5B, 0x01]
    );

    harness
        .process_response(super::file_transport_response(
            1,
            handle,
            0x80_00_00_00,
            data,
        ))
        .await;

    assert_eq!(events.pop().unwrap(), Event::Rx(0, data.to_vec()));
    assert_eq!(events.pop().unwrap(), Event::Complete);

    // close file
    assert_eq!(
        harness.pop_write().await[..6],
        [0xC2, 26, 70, 4, 0x5B, 0x01]
    );
}

#[tokio::test]
async fn reports_user_abort_on_open_and_closes_file() {
    let config = AssociationConfig::quiet();
    let mut harness = create_association(config).await;
    let (events, reader) = pair();
    events.state().action = FileAction::Abort;
    harness
        .association
        .read_file(
            "./test.txt".to_string(),
            FileReadConfig::default(),
            reader,
            None,
        )
        .await
        .unwrap();

    // file open
    assert_eq!(harness.pop_write().await[..5], [0xC0, 25, 70, 3, 0x5B]);

    let handle = 0xDEADCAFE;

    harness
        .process_response(super::file_status_response(
            0,
            handle,
            24,
            0,
            FileStatus::Success,
        ))
        .await;

    // close file
    assert_eq!(
        harness.pop_write().await[..6],
        [0xC1, 26, 70, 4, 0x5B, 0x01]
    );

    assert_eq!(events.pop().unwrap(), Event::Open(24));
    assert_eq!(events.pop().unwrap(), Event::Abort(FileError::AbortByUser));
    assert_eq!(events.pop(), None);

    harness
        .process_response(super::file_status_response(
            1,
            handle,
            0,
            0,
            FileStatus::Success,
        ))
        .await;

    // closing the file doesn't report a second error
    assert_eq!(events.pop(), None);
    // the user abort is local, so the task succeeds
    expect_finished(&mut harness, FunctionCode::OpenFile, Finished::Success(1)).await;
}

const HANDLE: u32 = 0xDEADCAFE;

fn read_config() -> FileReadConfig {
    FileReadConfig {
        max_block_size: 256,
        max_file_size: 8,
    }
}

async fn start_read(
    harness: &mut TestHarness,
    config: FileReadConfig,
    credentials: Option<FileCredentials>,
) -> EventHandle {
    let (events, reader) = pair();
    harness
        .association
        .read_file("f.txt".to_string(), config, reader, credentials)
        .await
        .unwrap();
    events
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
async fn open_file(harness: &mut TestHarness, events: &EventHandle) {
    harness.expect_write(open_request(0, 0)).await;
    harness
        .process_response(open_response(0, FileStatus::Success))
        .await;
    assert_eq!(events.pop(), Some(Event::Open(8)));
    harness.expect_write(read_request(1, 0)).await;
}

/// Open without credentials, read a single last block, and expect the CLOSE
async fn read_to_close(harness: &mut TestHarness, events: &EventHandle) {
    open_file(harness, events).await;
    harness
        .process_response(super::file_transport_response(
            1,
            HANDLE,
            super::last_block(0),
            b"data",
        ))
        .await;
    assert_eq!(events.pop(), Some(Event::Rx(0, b"data".to_vec())));
    assert_eq!(events.pop(), Some(Event::Complete));
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

/// Check that the read task, whose first request had function code `first`, finished this way
async fn expect_finished(harness: &mut TestHarness, first: FunctionCode, finished: Finished) {
    harness.master.get_decode_level().await.unwrap();
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
}

#[tokio::test]
async fn full_transfer_with_credentials() {
    let mut harness = create_association(AssociationConfig::quiet()).await;
    let events = start_read(&mut harness, read_config(), Some(credentials())).await;

    harness.expect_write(AUTH_REQUEST.to_vec()).await;
    harness.process_response(auth_response(0, 7)).await;

    harness.expect_write(open_request(1, 7)).await;
    harness
        .process_response(open_response(1, FileStatus::Success))
        .await;
    assert_eq!(events.pop(), Some(Event::Open(8)));

    harness.expect_write(read_request(2, 0)).await;
    harness
        .process_response(super::file_transport_response(2, HANDLE, 0, b"abcd"))
        .await;
    assert_eq!(events.pop(), Some(Event::Rx(0, b"abcd".to_vec())));

    harness.expect_write(read_request(3, 1)).await;
    harness
        .process_response(super::file_transport_response(
            3,
            HANDLE,
            super::last_block(1),
            b"efgh",
        ))
        .await;
    assert_eq!(events.pop(), Some(Event::Rx(1, b"efgh".to_vec())));
    assert_eq!(events.pop(), Some(Event::Complete));

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
    assert_eq!(events.pop(), None);
    harness.assert_no_events();
    expect_finished(
        &mut harness,
        FunctionCode::AuthenticateFile,
        Finished::Success(4),
    )
    .await;
}

/// Run a read with credentials against an AUTHENTICATE response that fails it, which fails the task
async fn auth_failure(response: Vec<u8>) -> Vec<Event> {
    let mut harness = create_association(AssociationConfig::quiet()).await;
    let events = start_read(&mut harness, read_config(), Some(credentials())).await;
    harness.expect_write(AUTH_REQUEST.to_vec()).await;
    harness.process_response(response).await;
    expect_finished(
        &mut harness,
        FunctionCode::AuthenticateFile,
        Finished::Failure,
    )
    .await;
    harness.assert_no_events();
    std::iter::from_fn(|| events.pop()).collect()
}

#[tokio::test]
async fn aborts_with_no_permission_when_auth_key_is_zero() {
    assert_eq!(
        auth_failure(auth_response(0, 0)).await,
        [Event::Abort(FileError::NoPermission)]
    );
}

#[tokio::test]
async fn aborts_on_unexpected_auth_variation() {
    assert_eq!(
        auth_failure(open_response(0, FileStatus::Success)).await,
        [Event::Abort(FileError::BadResponse)]
    );
}

/// Run a read against an OPEN response that fails it, which fails the task
async fn open_failure(response: Vec<u8>) -> Vec<Event> {
    let mut harness = create_association(AssociationConfig::quiet()).await;
    let events = start_read(&mut harness, read_config(), None).await;
    harness.expect_write(open_request(0, 0)).await;
    harness.process_response(response).await;
    expect_finished(&mut harness, FunctionCode::OpenFile, Finished::Failure).await;
    harness.assert_no_events();
    std::iter::from_fn(|| events.pop()).collect()
}

#[tokio::test]
async fn aborts_on_open_bad_status_without_close() {
    assert_eq!(
        open_failure(open_response(0, FileStatus::FileNotFound)).await,
        [Event::Abort(FileError::BadStatus(FileStatus::FileNotFound))]
    );
}

#[tokio::test]
async fn aborts_on_unexpected_open_variation() {
    assert_eq!(
        open_failure(auth_response(0, 7)).await,
        [Event::Abort(FileError::BadResponse)]
    );
}

/// A READ response that ends the transfer early, after which the master closes the file
#[derive(Copy, Clone, Debug)]
enum Ending {
    /// READ is answered with a g70v4
    BlockVariation,
    /// READ returns a block other than the one requested
    BlockNumber,
    /// READ is answered without an object header
    NoObjectHeader,
    /// the block exceeds the configured maximum file size
    MaxFileSize,
    /// the reader aborts a block that isn't the last
    ReaderAbort,
    /// the reader aborts the last block
    ReaderAbortOnLastBlock,
}

const ENDINGS: [Ending; 6] = [
    Ending::BlockVariation,
    Ending::BlockNumber,
    Ending::NoObjectHeader,
    Ending::MaxFileSize,
    Ending::ReaderAbort,
    Ending::ReaderAbortOnLastBlock,
];

impl Ending {
    fn config(self) -> FileReadConfig {
        match self {
            Self::MaxFileSize => FileReadConfig {
                max_block_size: 256,
                max_file_size: 3,
            },
            Self::BlockVariation
            | Self::BlockNumber
            | Self::NoObjectHeader
            | Self::ReaderAbort
            | Self::ReaderAbortOnLastBlock => read_config(),
        }
    }

    fn reader_aborts(self) -> bool {
        match self {
            Self::ReaderAbort | Self::ReaderAbortOnLastBlock => true,
            Self::BlockVariation | Self::BlockNumber | Self::NoObjectHeader | Self::MaxFileSize => {
                false
            }
        }
    }

    fn response(self) -> Vec<u8> {
        match self {
            Self::BlockVariation => open_response(1, FileStatus::Success),
            Self::BlockNumber => super::file_transport_response(1, HANDLE, 1, b"data"),
            Self::NoObjectHeader => super::no_object_response(1),
            Self::MaxFileSize | Self::ReaderAbort => {
                super::file_transport_response(1, HANDLE, 0, b"data")
            }
            Self::ReaderAbortOnLastBlock => {
                super::file_transport_response(1, HANDLE, super::last_block(0), b"data")
            }
        }
    }

    /// What the reader is told after it opened the file, all before the CLOSE
    fn events(self) -> Vec<Event> {
        match self {
            Self::BlockVariation => vec![Event::Abort(FileError::BadResponse)],
            Self::BlockNumber => vec![Event::Abort(FileError::BadBlockNum)],
            Self::NoObjectHeader => vec![Event::Abort(FileError::TaskError(
                TaskError::UnexpectedResponseHeaders,
            ))],
            Self::MaxFileSize => vec![Event::Abort(FileError::MaxLengthExceeded)],
            Self::ReaderAbort | Self::ReaderAbortOnLastBlock => vec![
                Event::Rx(0, b"data".to_vec()),
                Event::Abort(FileError::AbortByUser),
            ],
        }
    }
}

fn close_success(seq: u8) -> Vec<u8> {
    super::file_status_response(seq, HANDLE, 0, 0, FileStatus::Success)
}

/// CLOSE responses that fail validation: a bad status, the wrong handle, the wrong variation,
/// and no object header
fn bad_close_responses(seq: u8) -> [Vec<u8>; 4] {
    [
        super::file_status_response(seq, HANDLE, 0, 0, FileStatus::InvalidHandle),
        super::file_status_response(seq, HANDLE + 1, 0, 0, FileStatus::Success),
        super::file_transport_response(seq, HANDLE, 0, b""),
        super::no_object_response(seq),
    ]
}

/// Run a read against a READ response that ends it early, check that the reader is told once and
/// before the master closes the file, answer the CLOSE with `close`, and check how the task finished
async fn read_failure(ending: Ending, close: Vec<u8>, finished: Finished) {
    let mut harness = create_association(AssociationConfig::quiet()).await;
    let events = start_read(&mut harness, ending.config(), None).await;
    open_file(&mut harness, &events).await;
    if ending.reader_aborts() {
        events.state().action = FileAction::Abort;
    }
    harness.process_response(ending.response()).await;
    harness
        .expect_write(close_request_bytes_for(0xC2, HANDLE))
        .await;
    assert_eq!(
        std::iter::from_fn(|| events.pop()).collect::<Vec<_>>(),
        ending.events(),
        "{ending:?}"
    );
    harness.process_response(close).await;
    expect_finished(&mut harness, FunctionCode::OpenFile, finished).await;
    harness.assert_no_events();
    assert_eq!(events.pop(), None, "{ending:?}");
}

#[tokio::test]
async fn aborts_on_unexpected_read_variation_and_closes_file() {
    read_failure(Ending::BlockVariation, close_success(2), Finished::Failure).await;
}

#[tokio::test]
async fn aborts_on_wrong_block_number_and_closes_file() {
    read_failure(Ending::BlockNumber, close_success(2), Finished::Failure).await;
}

#[tokio::test]
async fn aborts_on_read_response_without_object_header_and_closes_file() {
    read_failure(Ending::NoObjectHeader, close_success(2), Finished::Failure).await;
}

#[tokio::test]
async fn aborts_when_max_file_size_is_exceeded_and_closes_file() {
    read_failure(Ending::MaxFileSize, close_success(2), Finished::Success(2)).await;
}

#[tokio::test]
async fn aborts_when_reader_aborts_block_and_closes_file() {
    read_failure(Ending::ReaderAbort, close_success(2), Finished::Success(2)).await;
}

#[tokio::test]
async fn aborts_when_reader_aborts_last_block_and_closes_file() {
    read_failure(
        Ending::ReaderAbortOnLastBlock,
        close_success(2),
        Finished::Success(2),
    )
    .await;
}

#[tokio::test]
async fn bad_close_after_an_early_end_fails_task_without_another_callback() {
    for ending in ENDINGS {
        for close in bad_close_responses(2) {
            read_failure(ending, close, Finished::Failure).await;
        }
    }
}

/// Run a read whose reader aborts when the file opens, and answer the CLOSE with `close`
async fn open_abort(close: Vec<u8>, finished: Finished) {
    let mut harness = create_association(AssociationConfig::quiet()).await;
    let events = start_read(&mut harness, read_config(), None).await;
    events.state().action = FileAction::Abort;
    harness.expect_write(open_request(0, 0)).await;
    harness
        .process_response(open_response(0, FileStatus::Success))
        .await;
    harness
        .expect_write(close_request_bytes_for(0xC1, HANDLE))
        .await;
    assert_eq!(events.pop(), Some(Event::Open(8)));
    assert_eq!(events.pop(), Some(Event::Abort(FileError::AbortByUser)));
    assert_eq!(events.pop(), None);
    harness.process_response(close).await;
    expect_finished(&mut harness, FunctionCode::OpenFile, finished).await;
    harness.assert_no_events();
    assert_eq!(events.pop(), None);
}

#[tokio::test]
async fn bad_close_after_open_abort_fails_task_without_another_callback() {
    for close in bad_close_responses(1) {
        open_abort(close, Finished::Failure).await;
    }
}

/// Run a read to the CLOSE and answer it with `response`, which never reaches the reader
async fn close_outcome(response: Vec<u8>, finished: Finished) {
    let mut harness = create_association(AssociationConfig::quiet()).await;
    let events = start_read(&mut harness, read_config(), None).await;
    read_to_close(&mut harness, &events).await;
    harness.process_response(response).await;
    expect_finished(&mut harness, FunctionCode::OpenFile, finished).await;
    harness.assert_no_events();
    assert_eq!(events.pop(), None);
}

#[tokio::test]
async fn close_success_completes_task() {
    close_outcome(close_success(2), Finished::Success(2)).await;
}

#[tokio::test]
async fn close_bad_status_fails_task() {
    close_outcome(
        super::file_status_response(2, HANDLE, 0, 0, FileStatus::InvalidHandle),
        Finished::Failure,
    )
    .await;
}

#[tokio::test]
async fn close_wrong_handle_fails_task() {
    close_outcome(
        super::file_status_response(2, HANDLE + 1, 0, 0, FileStatus::Success),
        Finished::Failure,
    )
    .await;
}

#[tokio::test]
async fn close_unexpected_variation_fails_task() {
    close_outcome(
        super::file_transport_response(2, HANDLE, 0, b""),
        Finished::Failure,
    )
    .await;
}

#[tokio::test]
async fn close_without_object_header_fails_task() {
    close_outcome(super::no_object_response(2), Finished::Failure).await;
}
