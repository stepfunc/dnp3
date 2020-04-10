use crate::app::gen::enums::{FunctionCode, QualifierCode};
use crate::app::gen::variations::variation::Variation;
use crate::app::header::{Control, RequestHeader};
use crate::app::sequence::Sequence;
use crate::util::cursor::{WriteCursor, WriteError};

pub(crate) fn confirm_solicited(seq: Sequence, cursor: &mut WriteCursor) -> Result<(), WriteError> {
    RequestHeader::new(Control::request(seq), FunctionCode::Confirm).write(cursor)
}

pub(crate) fn confirm_unsolicited(
    seq: Sequence,
    cursor: &mut WriteCursor,
) -> Result<(), WriteError> {
    RequestHeader::new(Control::unsolicited(seq), FunctionCode::Confirm).write(cursor)
}

pub(crate) fn write_all_objects(
    variation: Variation,
    cursor: &mut WriteCursor,
) -> Result<(), WriteError> {
    write_gv(variation, cursor)?;
    write_qualifier(QualifierCode::AllObjects, cursor)?;
    Ok(())
}

fn write_gv(variation: Variation, cursor: &mut WriteCursor) -> Result<(), WriteError> {
    let (g, v) = variation.to_group_and_var();
    cursor.write_u8(g)?;
    cursor.write_u8(v)?;
    Ok(())
}

fn write_qualifier(qualifier: QualifierCode, cursor: &mut WriteCursor) -> Result<(), WriteError> {
    cursor.write_u8(qualifier.as_u8())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::app::sequence::Sequence;
    use crate::util::cursor::WriteCursor;

    fn read_integrity(seq: Sequence, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        RequestHeader::new(Control::request(seq), FunctionCode::Read).write(cursor)?;
        write_all_objects(Variation::Group60Var2, cursor)?;
        write_all_objects(Variation::Group60Var3, cursor)?;
        write_all_objects(Variation::Group60Var4, cursor)?;
        write_all_objects(Variation::Group60Var1, cursor)?;
        Ok(())
    }

    #[test]
    fn formats_integrity_poll() {
        let mut buffer: [u8; 100] = [0; 100];
        let mut cursor = WriteCursor::new(&mut buffer);
        read_integrity(Sequence::new(0x01), &mut cursor).unwrap();

        assert_eq!(
            cursor.written(),
            [0xC1, 0x01, 0x3C, 0x02, 0x06, 0x3C, 0x03, 0x06, 0x3C, 0x04, 0x06, 0x3C, 0x01, 0x06]
        );
    }
}
