use crate::app::FileStatus;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::master::association::AssociationConfig;
use crate::master::{FileReadConfig, FileReadError, FileReader, TaskError};

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
    fn pop(&self) -> Option<Event> {
        let mut state = self.state.lock().unwrap();
        state.events.pop_front()
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
        !state.aborted
    }

    fn block_received(&mut self, block_num: u32, data: &[u8]) -> bool {
        let mut state = self.state.lock().unwrap();
        state.push(Event::Rx(block_num, data.to_vec()));
        !state.aborted
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
        Event::Abort(FileReadError::TaskError(
            TaskError::UnexpectedResponseHeaders
        ))
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
        .process_response(file_status(handle, 24, FileStatus::Success))
        .await;

    assert_eq!(events.pop().unwrap(), Event::Open(24));

    // file read
    assert_eq!(
        harness.pop_write().await[..6],
        [0xC1, 0x01, 70, 5, 0x5B, 0x01]
    );

    harness
        .process_response(file_transport(handle, 0x80_00_00_00, data))
        .await;

    assert_eq!(events.pop().unwrap(), Event::Rx(0, data.to_vec()));
    assert_eq!(events.pop().unwrap(), Event::Complete);

    // close file
    assert_eq!(
        harness.pop_write().await[..6],
        [0xC2, 26, 70, 4, 0x5B, 0x01]
    );
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
        0x01,
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

fn file_transport(file_handle: u32, block: u32, bytes: &[u8]) -> Vec<u8> {
    let fh = file_handle.to_le_bytes();
    let blk = block.to_le_bytes();

    let len: u16 = (8 + bytes.len()).try_into().unwrap();
    let len = len.to_le_bytes();

    let mut resp = [
        0xC1, 0x81, 0x00, 0x00, 70, 5, 0x5B, 0x01, len[0], // length
        len[1], fh[0], fh[1], fh[2], fh[3], blk[0], blk[1], blk[2], blk[3],
    ]
    .to_vec();

    resp.extend(bytes);

    resp
}
