use crate::app::format::free_format::FreeFormat;
use crate::app::format::write::HeaderWriter;
use crate::app::format::WriteError;
use crate::app::{FileStatus, FunctionCode, Group70Var4, Group70Var5, Group70Var6, Variation};
use scursor::WriteCursor;

mod close_file;
mod open_file;
mod read_file;

impl<'a> FreeFormat for Group70Var6<'a> {
    const VARIATION: Variation = Variation::Group70Var6;

    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        self.write(cursor)
    }
}

pub(super) fn last_block(block: u32) -> u32 {
    1 << 31 | block
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
