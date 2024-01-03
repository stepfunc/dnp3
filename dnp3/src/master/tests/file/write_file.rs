use crate::app::{
    FileMode, FileStatus, FunctionCode, Group70Var3, Group70Var4, Group70Var5, Group70Var6,
    MaybeAsync, PermissionSet, Permissions, Timestamp,
};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::master::association::AssociationConfig;
use crate::master::tasks::file::REQUEST_ID;
use crate::master::tests::harness::create_association;
use crate::master::{Block, FileAction, FileError, FileWriteConfig, FileWriteMode, FileWriter};

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
    fn opened(&mut self, file_handle: u32, file_size: u32) -> FileAction {
        self.on_event(Event::Open(file_handle, file_size))
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

#[tokio::test]
async fn can_write_a_file() {
    let config = AssociationConfig::quiet();
    let mut harness = create_association(config).await;
    let (events, reader) = pair();

    const FILE_NAME: &str = "./test.txt";
    const PERMISSIONS: Permissions = Permissions::same(PermissionSet::all());
    const FILE_SIZE: u32 = 0xCAFE;
    const MAX_BLOCK_SIZE: u16 = 512;

    const FILE_HANDLE: u32 = 0xDEADBEEF;

    harness
        .association
        .write_file(
            FILE_NAME,
            FileWriteConfig::new(FileWriteMode::Write, PERMISSIONS, FILE_SIZE)
                .max_block_size(MAX_BLOCK_SIZE),
            reader,
            None,
        )
        .await
        .unwrap();

    {
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
    }

    events.queue_block(Block {
        last: false,
        data: vec![0xDE, 0xAD],
    });
    events.queue_block(Block {
        last: true,
        data: vec![0xBE, 0xEF],
    });

    assert!(events.pop().is_none());

    harness
        .process_response(super::file_status(
            FILE_HANDLE,
            FILE_SIZE,
            FileStatus::Success,
        ))
        .await;

    assert_eq!(events.pop().unwrap(), Event::Open(FILE_HANDLE, FILE_SIZE));

    {
        let request = harness.pop_write().await;
        let expected = super::request(
            FunctionCode::Write,
            1,
            &Group70Var5 {
                file_handle: FILE_HANDLE,
                block_number: 0,
                file_data: &[0xDE, 0xAD],
            },
        );

        assert_eq!(request, expected);
    }

    harness
        .process_response(super::response(
            1,
            &Group70Var6 {
                file_handle: FILE_HANDLE,
                block_number: 0,
                status_code: FileStatus::Success,
                text: "",
            },
        ))
        .await;

    {
        // the second and final write
        let request = harness.pop_write().await;
        let expected = super::request(
            FunctionCode::Write,
            2,
            &Group70Var5 {
                file_handle: FILE_HANDLE,
                block_number: super::last_block(1),
                file_data: &[0xBE, 0xEF],
            },
        );

        assert_eq!(request, expected);
    }

    harness
        .process_response(super::response(
            2,
            &Group70Var6 {
                file_handle: FILE_HANDLE,
                block_number: super::last_block(1),
                status_code: FileStatus::Success,
                text: "",
            },
        ))
        .await;

    {
        // close
        let request = harness.pop_write().await;
        let expected = super::request(
            FunctionCode::CloseFile,
            3,
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
    }

    assert!(events.pop().is_none());

    harness
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

    assert_eq!(events.pop().unwrap(), Event::Complete);
}
