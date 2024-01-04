use crate::app::{FileStatus, MaybeAsync};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::master::association::AssociationConfig;
use crate::master::tests::harness::create_association;
use crate::master::{FileAction, FileError, FileReadConfig, FileReader, TaskError};

#[derive(Debug, PartialEq, Eq)]
enum Event {
    Open(u32),
    Rx(u32, Vec<u8>),
    Abort(FileError),
    Complete,
}

impl Default for FileAction {
    fn default() -> Self {
        FileAction::Continue
    }
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
