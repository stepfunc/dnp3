// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use std::sync::{Arc, Mutex, MutexGuard};

use scursor::WriteCursor;

use crate::app::format::free_format::FreeFormat;
use crate::app::format::write::HeaderWriter;
use crate::app::{FileStatus, FunctionCode, Group70Var4, Group70Var5, MaybeAsync};
use crate::master::{FileAction, FileError, FileReader};

mod auth_file;
mod close_file;
mod get_info;
mod open_file;
mod read_file;
mod write_block;

/// What a file reader was told, in order
#[derive(Clone, Debug, PartialEq)]
pub(super) enum ReaderEvent {
    Opened(u32),
    Block(u32, Vec<u8>),
    Aborted(FileError),
    Completed,
}

#[derive(Default)]
struct ReaderState {
    action: FileAction,
    events: Vec<ReaderEvent>,
}

/// A file reader that records what it's told, and answers each file and block with an action
#[derive(Clone, Default)]
pub(super) struct RecordingReader(Arc<Mutex<ReaderState>>);

impl RecordingReader {
    /// Take what the reader was told since the last call
    pub(super) fn pop(&self) -> Vec<ReaderEvent> {
        std::mem::take(&mut self.lock().events)
    }

    /// Answer the file and the blocks that follow with `action`
    pub(super) fn set_action(&self, action: FileAction) {
        self.lock().action = action;
    }

    fn lock(&self) -> MutexGuard<'_, ReaderState> {
        self.0.lock().unwrap()
    }

    fn record(&self, event: ReaderEvent) -> FileAction {
        let mut state = self.lock();
        state.events.push(event);
        state.action
    }
}

impl FileReader for RecordingReader {
    fn opened(&mut self, size: u32) -> FileAction {
        self.record(ReaderEvent::Opened(size))
    }

    fn block_received(&mut self, block_num: u32, data: &[u8]) -> MaybeAsync<FileAction> {
        MaybeAsync::ready(self.record(ReaderEvent::Block(block_num, data.to_vec())))
    }

    fn aborted(&mut self, err: FileError) {
        self.record(ReaderEvent::Aborted(err));
    }

    fn completed(&mut self) {
        self.record(ReaderEvent::Completed);
    }
}

pub(super) fn last_block(block: u32) -> u32 {
    (1 << 31) | block
}

pub(super) fn fir_and_fin(seq: u8) -> u8 {
    0b1100_0000 | seq
}

pub(super) fn file_status_response(
    seq: u8,
    file_handle: u32,
    file_size: u32,
    max_block_size: u16,
    status_code: FileStatus,
) -> Vec<u8> {
    response(
        seq,
        &Group70Var4 {
            file_handle,
            file_size,
            max_block_size,
            request_id: 0,
            status_code,
            text: "",
        },
    )
}

pub(super) fn file_transport_response(
    seq: u8,
    file_handle: u32,
    block_number: u32,
    file_data: &[u8],
) -> Vec<u8> {
    response(
        seq,
        &Group70Var5 {
            file_handle,
            block_number,
            file_data,
        },
    )
}

pub(super) fn no_object_response(seq: u8) -> Vec<u8> {
    [fir_and_fin(seq), 0x81, 0x00, 0x00].to_vec()
}

pub(crate) fn response<T: FreeFormat>(seq: u8, variation: &T) -> Vec<u8> {
    let mut response = [fir_and_fin(seq), 0x81, 0x00, 0x00].to_vec();

    let mut buffer: [u8; 64] = [0; 64];
    let mut cursor = WriteCursor::new(&mut buffer);
    let mut writer = HeaderWriter::new(&mut cursor);
    writer.write_free_format(variation).unwrap();

    response.extend_from_slice(cursor.written());

    response
}

pub(crate) fn request<T: FreeFormat>(function: FunctionCode, seq: u8, variation: &T) -> Vec<u8> {
    let mut response = [fir_and_fin(seq), function.as_u8()].to_vec();

    let mut buffer: [u8; 64] = [0; 64];
    let mut cursor = WriteCursor::new(&mut buffer);
    let mut writer = HeaderWriter::new(&mut cursor);
    writer.write_free_format(variation).unwrap();

    response.extend_from_slice(cursor.written());

    response
}
