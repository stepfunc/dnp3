use crate::app::parse::free_format::FreeFormatVariation;
use crate::app::parse::parser::{HeaderDetails, ParsedFragment};
use crate::app::{
    FileMode, FileStatus, FunctionCode, Group70Var3, Group70Var5, Group70Var6, MaybeAsync,
    PermissionSet, Permissions, RequestHeader,
};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::master::association::AssociationConfig;
use crate::master::tests::harness::create_association;
use crate::master::{Block, FileAction, FileError, FileWriteConfig, FileWriteMode, FileWriter};

#[derive(Debug, PartialEq, Eq)]
enum Event {
    Open(u32),
    NextBlock(u32, Vec<u8>),
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
    fn opened(&mut self, size: u32) -> FileAction {
        self.on_event(Event::Open(size))
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
        let open_request = harness.pop_write().await;
        let (header, var) = expect_free_format_request(&open_request);
        assert_eq!(header.function, FunctionCode::OpenFile);
        assert_matches!(
            var,
            FreeFormatVariation::Group70Var3(Group70Var3 {
                file_name: FILE_NAME,
                max_block_size: MAX_BLOCK_SIZE,
                mode: FileMode::Write,
                file_size: FILE_SIZE,
                ..
            })
        );
    }

    events.queue_block(Block {
        last: false,
        data: vec![0xDE, 0xAD],
    });
    events.queue_block(Block {
        last: true,
        data: vec![0xBE, 0xEF],
    });

    harness
        .process_response(super::file_status(22, FILE_SIZE, FileStatus::Success))
        .await;

    {
        // the first write
        let open_request = harness.pop_write().await;
        let (header, var) = expect_free_format_request(&open_request);
        assert_eq!(header.function, FunctionCode::Write);
        assert_matches!(
            var,
            FreeFormatVariation::Group70Var5(Group70Var5 {
                file_handle: 22,
                block_number: 0,
                file_data: [0xDE, 0xAD]
            })
        );
    }

    harness
        .process_response(super::response(
            01,
            &Group70Var6 {
                file_handle: 22,
                block_number: 0,
                status_code: FileStatus::Success,
                text: "",
            },
        ))
        .await;
}

fn expect_free_format_request(data: &[u8]) -> (RequestHeader, FreeFormatVariation) {
    let parsed = ParsedFragment::parse(&data).unwrap().to_request().unwrap();
    let objects = parsed.objects.unwrap();
    let header = objects.get_only_header().unwrap();
    let obj = match header.details {
        HeaderDetails::TwoByteFreeFormat(1, x) => x,
        _ => unreachable!(),
    };
    (parsed.header, obj)
}
