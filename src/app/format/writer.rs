use crate::app::gen::enums::QualifierCode;
use crate::app::header::RequestHeader;
use crate::util::cursor::{WriteCursor, WriteError};
use crate::app::gen::variations::gv::Variation;

pub struct Writer<'a> {
    cursor: WriteCursor<'a>,
}

impl<'a> Writer<'a> {
    pub fn start_request(output: &'a mut [u8], header: RequestHeader) -> Result<Self, WriteError> {
        let mut cursor = WriteCursor::new(output);
        header.write(&mut cursor)?;
        Ok(Self { cursor })
    }

    pub fn write_all_objects(&mut self, headers: &[Variation]) -> Result<(), WriteError> {
        for _ in headers {
            self.cursor.write_u8(1)?;
            self.cursor.write_u8(2)?;
            self.cursor.write_u8(QualifierCode::AllObjects.as_u8())?;
        }
        Ok(())
    }

    pub fn written(&self) -> Result<&[u8], WriteError> {
        self.cursor.written_since(0)
    }
}
