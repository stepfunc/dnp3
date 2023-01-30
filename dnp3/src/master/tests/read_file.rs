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
        println!("Event: {:?}", event);
        self.events.push(event);
    }
}

struct EventHandle {
    state: Arc<Mutex<State>>,
}

impl EventHandle {
    fn events(&self) -> Vec<Event> {
        let mut state = self.state.lock().unwrap();
        let mut ret = Vec::new();
        std::mem::swap(&mut state.events, &mut ret);
        ret
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

fn file_open() -> Vec<u8> {
    [
        0xC0, 25, // open file
        70, 3, 0x5B, // free format
        36,   // 36- bytes
    ]
    .to_vec()
}

fn file_open_response() -> Vec<u8> {
    [0xC0, 0x81, 0x00, 0x00].to_vec()
}

#[tokio::test]
async fn can_read_file() {
    let config = AssociationConfig::quiet();
    let mut harness = create_association(config).await;
    let (handle, reader) = pair();
    harness
        .association
        .read_file("./test.txt".to_string(), 1024, reader)
        .await
        .unwrap();

    // check that its a file open request
    assert_eq!(harness.pop_write().await[..5], [0xC0, 25, 70, 3, 0x5B]);

    harness.process_response(file_open_response()).await;

    assert_eq!(
        handle.events().as_slice(),
        [Event::Abort(FileReadError::TaskError(
            TaskError::UnexpectedResponseHeaders
        ))]
        .as_slice()
    );
}
