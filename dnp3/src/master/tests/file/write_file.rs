use crate::app::{
    FileStatus, FunctionCode, Group70Var3, Group70Var4, Group70Var5, Group70Var6, MaybeAsync,
    PermissionSet, Permissions, Timestamp,
};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::master::association::AssociationConfig;
use crate::master::tasks::file::REQUEST_ID;
use crate::master::tests::harness::{create_association, TestHarness};
use crate::master::{
    Block, FileAction, FileError, FileHandle, FileMode, FileWriteConfig, FileWriteMode, FileWriter,
};

#[derive(Debug, PartialEq, Eq)]
enum Event {
    Open(u32, u32), // handle and size
    Abort(FileError),
    Complete,
}

#[derive(Default)]
struct State {
    action: FileAction,
    events: VecDeque<Event>,
    blocks: VecDeque<Block>,
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
    fn pop(&self) -> Option<Event> {
        let mut state = self.state.lock().unwrap();
        state.events.pop_front()
    }

    fn queue_block(&self, block: Block) {
        let mut state = self.state.lock().unwrap();
        state.blocks.push_back(block);
    }
}

struct MockWriter {
    state: Arc<Mutex<State>>,
}

impl MockWriter {
    fn on_event(&self, event: Event) -> FileAction {
        let mut state = self.state.lock().unwrap();
        state.push(event);
        state.action
    }
}

fn pair() -> (EventHandle, Box<dyn FileWriter>) {
    let state = Arc::new(Mutex::new(State::default()));
    (
        EventHandle {
            state: state.clone(),
        },
        Box::new(MockWriter { state }),
    )
}

impl FileWriter for MockWriter {
    fn opened(&mut self, file_handle: FileHandle, file_size: u32) -> FileAction {
        self.on_event(Event::Open(file_handle.into(), file_size))
    }

    fn next_block(&mut self, _next_block_size: u16) -> MaybeAsync<Option<Block>> {
        let mut guard = self.state.lock().unwrap();
        MaybeAsync::ready(guard.blocks.pop_front())
    }

    fn aborted(&mut self, err: FileError) {
        self.on_event(Event::Abort(err));
    }

    fn completed(&mut self) {
        self.on_event(Event::Complete);
    }
}

const FILE_NAME: &str = "./test.txt";
const PERMISSIONS: Permissions = Permissions::same(PermissionSet::all());
const FILE_SIZE: u32 = 0xCAFE;
const MAX_BLOCK_SIZE: u16 = 512;

const FILE_HANDLE: u32 = 0xDEADBEEF;

#[tokio::test]
async fn aborts_when_file_cannot_be_opened() {
    let mut harness = FileWriteHarness::start_write().await;
    harness
        .respond_to_open(
            0,
            MAX_BLOCK_SIZE,
            FileStatus::FileLocked,
            Event::Abort(FileError::BadStatus(FileStatus::FileLocked)),
        )
        .await;
}

#[tokio::test]
async fn can_write_two_block_file() {
    let mut harness = FileWriteHarness::start_write().await;

    harness.events.queue_block(Block {
        last: false,
        data: vec![0xDE, 0xAD],
    });
    harness.events.queue_block(Block {
        last: true,
        data: vec![0xBE, 0xEF],
    });

    harness
        .respond_to_open(
            0,
            MAX_BLOCK_SIZE,
            FileStatus::Success,
            Event::Open(FILE_HANDLE, FILE_SIZE),
        )
        .await;
    harness.expect_write(1, 0, &[0xDE, 0xAD]).await;
    harness
        .response_to_write(1, FILE_HANDLE, 0, FileStatus::Success)
        .await;
    harness
        .expect_write(2, super::last_block(1), &[0xBE, 0xEF])
        .await;
    harness
        .response_to_write(2, FILE_HANDLE, super::last_block(1), FileStatus::Success)
        .await;
    harness.expect_close(3, Event::Complete).await;
}

struct FileWriteHarness {
    inner: TestHarness,
    events: EventHandle,
}

impl Drop for FileWriteHarness {
    fn drop(&mut self) {
        assert!(self.events.pop().is_none());
        self.inner.assert_no_events();
    }
}

impl FileWriteHarness {
    async fn start_write() -> Self {
        let config = AssociationConfig::quiet();
        let mut harness = create_association(config).await;
        let (events, writer) = pair();

        harness
            .association
            .write_file(
                FILE_NAME,
                FileWriteConfig::new(FileWriteMode::Write, PERMISSIONS, FILE_SIZE)
                    .max_block_size(MAX_BLOCK_SIZE),
                writer,
                None,
            )
            .await
            .unwrap();

        // check that it's a file open request
        let request = harness.pop_write().await;
        let expected = super::request(
            FunctionCode::OpenFile,
            0,
            &Group70Var3 {
                time_of_creation: Timestamp::zero(),
                permissions: PERMISSIONS,
                file_name: FILE_NAME,
                max_block_size: MAX_BLOCK_SIZE,
                mode: FileMode::Write,
                file_size: FILE_SIZE,
                auth_key: 0,
                request_id: REQUEST_ID,
            },
        );

        assert_eq!(request, expected);

        assert!(events.pop().is_none());

        Self {
            inner: harness,
            events,
        }
    }

    async fn respond_to_open(
        &mut self,
        seq: u8,
        max_block_size: u16,
        status: FileStatus,
        event: Event,
    ) {
        self.inner
            .process_response(super::file_status_response(
                seq,
                FILE_HANDLE,
                FILE_SIZE,
                max_block_size,
                status,
            ))
            .await;

        assert_eq!(self.events.pop().unwrap(), event);
    }

    async fn expect_write(&mut self, seq: u8, block_number: u32, file_data: &[u8]) {
        let request = self.inner.pop_write().await;
        let expected = super::request(
            FunctionCode::Write,
            seq,
            &Group70Var5 {
                file_handle: FILE_HANDLE,
                block_number,
                file_data,
            },
        );

        assert_eq!(request, expected);
    }

    async fn response_to_write(
        &mut self,
        seq: u8,
        file_handle: u32,
        block_number: u32,
        status_code: FileStatus,
    ) {
        self.inner
            .process_response(super::response(
                seq,
                &Group70Var6 {
                    file_handle,
                    block_number,
                    status_code,
                    text: "",
                },
            ))
            .await;
    }

    async fn expect_close(&mut self, seq: u8, final_event: Event) {
        // close
        let request = self.inner.pop_write().await;
        let expected = super::request(
            FunctionCode::CloseFile,
            seq,
            &Group70Var4 {
                file_handle: FILE_HANDLE,
                file_size: 0,
                max_block_size: 0,
                request_id: REQUEST_ID,
                status_code: FileStatus::Success,
                text: "",
            },
        );

        assert_eq!(request, expected);

        assert!(self.events.pop().is_none());

        self.inner
            .process_response(super::response(
                3,
                &Group70Var4 {
                    file_handle: FILE_HANDLE,
                    file_size: 0,
                    max_block_size: 0,
                    status_code: FileStatus::Success,
                    text: "",
                    request_id: 0,
                },
            ))
            .await;

        assert_eq!(self.events.pop().unwrap(), final_event);
    }
}
