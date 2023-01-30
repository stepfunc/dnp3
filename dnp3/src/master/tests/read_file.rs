use crate::app::FileStatus;
use std::sync::{Arc, Mutex};

use crate::master::association::AssociationConfig;
use crate::master::{FileReadError, FileReader, TaskError};

use super::harness::create_association;

#[derive(Debug, PartialEq, Eq)]
enum Event {
    Open(u32),
    Rx(u32, Vec<u8>),
    Abort(FileReadError),
    Complete,
}

#[derive(Default)]
struct State {
    aborted: bool,
    events: Vec<Event>,
}

impl State {
    fn push(&mut self, event: Event) {
        self.events.push(event);
    }
}

struct EventHandle {
    state: Arc<Mutex<State>>,
}

impl EventHandle {
    fn pop_all(&self) -> Vec<Event> {
        let mut state = self.state.lock().unwrap();
        let mut ret = Vec::new();
        std::mem::swap(&mut state.events, &mut ret);
        ret
    }

    fn expect_one(&self) -> Event {
        let mut events = self.pop_all();
        assert_eq!(events.len(), 1);
        events.remove(0)
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
    fn opened(&mut self, size: u32) -> bool {
        let mut state = self.state.lock().unwrap();
        state.push(Event::Open(size));
        state.aborted
    }

    fn block_received(&mut self, block_num: u32, data: &[u8]) -> bool {
        let mut state = self.state.lock().unwrap();
        state.push(Event::Rx(block_num, data.to_vec()));
        state.aborted
    }

    fn aborted(&mut self, err: FileReadError) {
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
        .read_file("./test.txt".to_string(), 1024, reader)
        .await
        .unwrap();

    // check that its a file open request
    assert_eq!(harness.pop_write().await[..5], [0xC0, 25, 70, 3, 0x5B]);

    harness
        .process_response([0xC0, 0x81, 0x00, 0x00].to_vec())
        .await;

    assert_eq!(
        events.expect_one(),
        Event::Abort(FileReadError::TaskError(
            TaskError::UnexpectedResponseHeaders
        ))
    );
}

#[tokio::test]
async fn can_read_file() {
    let config = AssociationConfig::quiet();
    let mut harness = create_association(config).await;
    let (events, reader) = pair();
    harness
        .association
        .read_file("./test.txt".to_string(), 1024, reader)
        .await
        .unwrap();

    // check that its a file open request
    assert_eq!(harness.pop_write().await[..5], [0xC0, 25, 70, 3, 0x5B]);

    harness
        .process_response(file_status(0xDEADCAFE, 24, FileStatus::Success))
        .await;

    assert_eq!(events.expect_one(), Event::Open(24),);
}

fn file_status(file_handle: u32, file_size: u32, status: FileStatus) -> Vec<u8> {
    let fh_bytes = file_handle.to_le_bytes();
    let fs_bytes = file_size.to_le_bytes();

    [
        0xC0,
        0x81,
        0x00,
        0x00,
        70,
        4,
        0x5B,
        13, // length
        00,
        fh_bytes[0], // file handle
        fh_bytes[1],
        fh_bytes[2],
        fh_bytes[3],
        fs_bytes[0], // file size
        fs_bytes[1],
        fs_bytes[2],
        fs_bytes[3],
        0x00, // max block size
        0x40,
        0x00, // request id
        0x00,
        status.to_u8(),
    ]
    .to_vec()
}
