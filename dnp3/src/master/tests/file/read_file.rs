use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::app::{
    FileStatus, FunctionCode, Group70Var3, Group70Var5, MaybeAsync, Permissions, Timestamp,
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

/// Check that the read task finished, and whether it reported success or failure
fn expect_finished(harness: &mut TestHarness, success: bool) {
    let events = harness.assoc_events.pop();
    let finished = match events.as_slice() {
        [AssocInfoEvent::TaskStart(TaskType::FileRead, _, _), last] => match last {
            AssocInfoEvent::TaskSuccess(TaskType::FileRead, _, _) => success,
            AssocInfoEvent::TaskFailure(
                TaskType::FileRead,
                TaskError::UnexpectedResponseHeaders,
            ) => !success,
            _ => false,
        },
        _ => false,
    };
    assert!(finished, "{events:?}");
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
    expect_finished(&mut harness, true);
}

/// Run a read with credentials against an AUTHENTICATE response that fails it
async fn auth_failure(response: Vec<u8>) -> Vec<Event> {
    let mut harness = create_association(AssociationConfig::quiet()).await;
    let events = start_read(&mut harness, read_config(), Some(credentials())).await;
    harness.expect_write(AUTH_REQUEST.to_vec()).await;
    harness.process_response(response).await;
    harness.assert_no_events();
    expect_finished(&mut harness, true);
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

/// Run a read against an OPEN response that fails it
async fn open_failure(response: Vec<u8>) -> Vec<Event> {
    let mut harness = create_association(AssociationConfig::quiet()).await;
    let events = start_read(&mut harness, read_config(), None).await;
    harness.expect_write(open_request(0, 0)).await;
    harness.process_response(response).await;
    harness.assert_no_events();
    expect_finished(&mut harness, true);
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

/// Run a read against a READ response that fails it
async fn read_failure(config: FileReadConfig, abort: bool, response: Vec<u8>) -> Vec<Event> {
    let mut harness = create_association(AssociationConfig::quiet()).await;
    let events = start_read(&mut harness, config, None).await;
    open_file(&mut harness, &events).await;
    if abort {
        events.state().action = FileAction::Abort;
    }
    harness.process_response(response).await;
    harness.assert_no_events();
    expect_finished(&mut harness, true);
    std::iter::from_fn(|| events.pop()).collect()
}

#[tokio::test]
async fn aborts_on_unexpected_read_variation_without_close() {
    assert_eq!(
        read_failure(read_config(), false, open_response(1, FileStatus::Success)).await,
        [Event::Abort(FileError::BadResponse)]
    );
}

#[tokio::test]
async fn aborts_on_wrong_block_number() {
    assert_eq!(
        read_failure(
            read_config(),
            false,
            super::file_transport_response(1, HANDLE, 1, b"data")
        )
        .await,
        [Event::Abort(FileError::BadBlockNum)]
    );
}

#[tokio::test]
async fn aborts_when_max_file_size_is_exceeded() {
    let config = FileReadConfig {
        max_block_size: 256,
        max_file_size: 3,
    };
    assert_eq!(
        read_failure(
            config,
            false,
            super::file_transport_response(1, HANDLE, 0, b"data")
        )
        .await,
        [Event::Abort(FileError::MaxLengthExceeded)]
    );
}

#[tokio::test]
async fn aborts_when_reader_aborts_block() {
    assert_eq!(
        read_failure(
            read_config(),
            true,
            super::file_transport_response(1, HANDLE, 0, b"data")
        )
        .await,
        [
            Event::Rx(0, b"data".to_vec()),
            Event::Abort(FileError::AbortByUser)
        ]
    );
}

/// Run a read to the CLOSE and answer it with `response`, which never reaches the reader
async fn close_outcome(response: Vec<u8>, success: bool) {
    let mut harness = create_association(AssociationConfig::quiet()).await;
    let events = start_read(&mut harness, read_config(), None).await;
    read_to_close(&mut harness, &events).await;
    harness.process_response(response).await;
    harness.assert_no_events();
    assert_eq!(events.pop(), None);
    expect_finished(&mut harness, success);
}

#[tokio::test]
async fn close_success_completes_task() {
    close_outcome(
        super::file_status_response(2, HANDLE, 0, 0, FileStatus::Success),
        true,
    )
    .await;
}

#[tokio::test]
async fn close_bad_status_is_only_logged() {
    close_outcome(
        super::file_status_response(2, HANDLE, 0, 0, FileStatus::InvalidHandle),
        true,
    )
    .await;
}

#[tokio::test]
async fn close_wrong_handle_is_ignored() {
    close_outcome(
        super::file_status_response(2, HANDLE + 1, 0, 0, FileStatus::Success),
        true,
    )
    .await;
}

#[tokio::test]
async fn close_unexpected_variation_is_only_logged() {
    close_outcome(super::file_transport_response(2, HANDLE, 0, b""), true).await;
}

#[tokio::test]
async fn close_without_object_header_fails_task() {
    close_outcome(super::no_object_response(2), false).await;
}
