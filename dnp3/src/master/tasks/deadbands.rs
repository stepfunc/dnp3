use crate::app::format::write::HeaderWriter;
use crate::app::parse::parser::Response;
use crate::app::parse::traits::Index;
use crate::app::FunctionCode;
use crate::master::tasks::empty_response::expect_empty;
use crate::master::tasks::{NonReadTask, OneShot, OneShotRequest};
use crate::master::{
    DeadBandHeader, DeadBandHeaderVariants, DeadBands, TaskError, TaskType, WriteError,
};

pub(crate) struct DeadBandsRequest {
    headers: Vec<DeadBandHeader>,
}

impl DeadBandHeaderVariants {
    fn write(&self, writer: &mut HeaderWriter) -> Result<(), scursor::WriteError> {
        match self {
            Self::U8(x) => x.write(writer),
            Self::U16(x) => x.write(writer),
        }
    }
}

impl<I: Index> DeadBands<I> {
    fn write(&self, writer: &mut HeaderWriter) -> Result<(), scursor::WriteError> {
        match self {
            Self::V1(x) => writer.write_prefixed_items(x.iter()),
            Self::V2(x) => writer.write_prefixed_items(x.iter()),
            Self::V3(x) => writer.write_prefixed_items(x.iter()),
        }
    }
}

impl DeadBandsRequest {
    pub(crate) fn new(headers: Vec<DeadBandHeader>) -> Self {
        Self { headers }
    }
}

impl OneShotRequest for DeadBandsRequest {
    type Output = ();
    type Error = WriteError;

    fn function(&self) -> FunctionCode {
        FunctionCode::Write
    }

    fn task_type(&self) -> TaskType {
        TaskType::WriteDeadBands
    }

    fn write(&self, writer: &mut HeaderWriter) -> Result<(), TaskError> {
        for header in self.headers.iter() {
            header.inner.write(writer)?;
        }

        Ok(())
    }

    fn parse(&self, response: Response<'_>) -> Result<(), WriteError> {
        expect_empty(response)
    }

    fn wrap(task: OneShot<Self>) -> NonReadTask {
        NonReadTask::DeadBands(task)
    }
}

#[cfg(test)]
mod tests {
    use scursor::WriteCursor;

    use super::*;

    #[test]
    fn writes_each_header_with_its_variation_and_index_width() {
        let request = DeadBandsRequest::new(vec![
            DeadBandHeader::group34_var1_u8(vec![(1, 0x0201), (2, 0x0403)]),
            DeadBandHeader::group34_var1_u16(vec![(0x0102, 0x0605)]),
            DeadBandHeader::group34_var2_u8(vec![(3, 0x0A09_0807)]),
            DeadBandHeader::group34_var2_u16(vec![(0x0304, 0x0E0D_0C0B)]),
            DeadBandHeader::group34_var3_u8(vec![(5, 1.0)]),
            DeadBandHeader::group34_var3_u16(vec![(0x0506, -2.5), (0x0607, 0.0)]),
        ]);

        let mut buffer = [0; 128];
        let mut cursor = WriteCursor::new(&mut buffer);
        request.write(&mut HeaderWriter::new(&mut cursor)).unwrap();

        assert_eq!(
            cursor.written(),
            [
                // g34v1, 1-byte count and prefix: two values
                34, 1, 0x17, 2, 1, 0x01, 0x02, 2, 0x03, 0x04, //
                // g34v1, 2-byte count and prefix
                34, 1, 0x28, 1, 0, 0x02, 0x01, 0x05, 0x06, //
                // g34v2, 1-byte count and prefix
                34, 2, 0x17, 1, 3, 0x07, 0x08, 0x09, 0x0A, //
                // g34v2, 2-byte count and prefix
                34, 2, 0x28, 1, 0, 0x04, 0x03, 0x0B, 0x0C, 0x0D, 0x0E, //
                // g34v3, 1-byte count and prefix: 1.0
                34, 3, 0x17, 1, 5, 0x00, 0x00, 0x80, 0x3F, //
                // g34v3, 2-byte count and prefix: -2.5 and 0.0
                34, 3, 0x28, 2, 0, 0x06, 0x05, 0x00, 0x00, 0x20, 0xC0, 0x07, 0x06, 0x00, 0x00, 0x00,
                0x00,
            ]
        );
    }
}
