use crate::app::gen::enums::QualifierCode;
use crate::app::gen::variations::gv::Variation;
use crate::app::header::RequestHeader;
use crate::util::cursor::{WriteCursor, WriteError};

pub struct Writer<'a> {
    cursor: WriteCursor<'a>,
}

impl<'a> Writer<'a> {
    pub fn start_request(output: &'a mut [u8], header: RequestHeader) -> Result<Self, WriteError> {
        let mut cursor = WriteCursor::new(output);
        header.write(&mut cursor)?;
        Ok(Self { cursor })
    }

    pub fn write_all_objects(&mut self, variation: Variation) -> Result<(), WriteError> {
        let (g, v) = variation.to_group_and_var();
        self.cursor.write_u8(g)?;
        self.cursor.write_u8(v)?;
        self.cursor.write_u8(QualifierCode::AllObjects.as_u8())?;
        Ok(())
    }

    pub fn write_integrity(&mut self) -> Result<(), WriteError> {
        self.write_all_objects(Variation::Group60Var2)?;
        self.write_all_objects(Variation::Group60Var3)?;
        self.write_all_objects(Variation::Group60Var4)?;
        self.write_all_objects(Variation::Group60Var1)?;
        Ok(())
    }

    pub fn written(&self) -> Result<&[u8], WriteError> {
        self.cursor.written_since(0)
    }
}

#[cfg(test)]
mod test {
    use crate::app::format::writer::Writer;
    use crate::app::gen::enums::FunctionCode;
    use crate::app::header::{Control, RequestHeader};
    use crate::app::sequence::Sequence;

    #[test]
    fn formats_integrity_poll() {
        let mut buffer: [u8; 100] = [0; 100];
        let mut writer = Writer::start_request(
            &mut buffer,
            RequestHeader::new(Control::request(Sequence::new(0x01)), FunctionCode::Read),
        )
        .unwrap();

        writer.write_integrity().unwrap();
        assert_eq!(
            writer.written().unwrap(),
            [0xC1, 0x01, 0x3C, 0x02, 0x06, 0x3C, 0x03, 0x06, 0x3C, 0x04, 0x06, 0x3C, 0x01, 0x06]
        );
    }
}
