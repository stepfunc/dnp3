use crate::app::attr::{AttrWriteError, OwnedAttribute};
use crate::app::header::{ControlField, RequestHeader};
#[cfg(test)]
use crate::app::header::{Iin, ResponseFunction, ResponseHeader};
#[cfg(test)]
use crate::app::parse::parser::ParsedFragment;
use crate::app::parse::traits::{FixedSizeVariation, Index};
use crate::app::sequence::Sequence;
use crate::app::variations::Variation;
use crate::app::{FunctionCode, QualifierCode};

use scursor::{WriteCursor, WriteError};

pub(crate) struct HeaderWriter<'a, 'b> {
    cursor: &'b mut WriteCursor<'a>,
}

pub(crate) fn start_request<'a, 'b>(
    control: ControlField,
    function: FunctionCode,
    cursor: &'b mut WriteCursor<'a>,
) -> Result<HeaderWriter<'a, 'b>, WriteError> {
    let header = RequestHeader::new(control, function);
    header.write(cursor)?;
    Ok(HeaderWriter { cursor })
}

#[cfg(test)]
pub(crate) fn start_response<'a, 'b>(
    control: ControlField,
    function: ResponseFunction,
    iin: Iin,
    cursor: &'b mut WriteCursor<'a>,
) -> Result<HeaderWriter<'a, 'b>, WriteError> {
    let header = ResponseHeader::new(control, function, iin);
    header.write(cursor)?;
    Ok(HeaderWriter { cursor })
}

impl<'a, 'b> HeaderWriter<'a, 'b> {
    #[cfg(test)]
    pub(crate) fn inner(self) -> &'b mut WriteCursor<'a> {
        self.cursor
    }

    #[cfg(test)]
    pub(crate) fn write_class1230(&mut self) -> Result<(), WriteError> {
        self.write_all_objects_header(Variation::Group60Var2)?;
        self.write_all_objects_header(Variation::Group60Var3)?;
        self.write_all_objects_header(Variation::Group60Var4)?;
        self.write_all_objects_header(Variation::Group60Var1)?;
        Ok(())
    }

    pub(crate) fn new(cursor: &'b mut WriteCursor<'a>) -> Self {
        Self { cursor }
    }

    pub(crate) fn write_all_objects_header(
        &mut self,
        variation: Variation,
    ) -> Result<(), WriteError> {
        variation.write(self.cursor)?;
        QualifierCode::AllObjects.write(self.cursor)?;
        Ok(())
    }

    pub(crate) fn write_range_only<T>(
        &mut self,
        variation: Variation,
        start: T,
        stop: T,
    ) -> Result<(), WriteError>
    where
        T: Index,
    {
        variation.write(self.cursor)?;
        T::RANGE_QUALIFIER.write(self.cursor)?;
        start.write(self.cursor)?;
        stop.write(self.cursor)?;
        Ok(())
    }

    pub(crate) fn write_limited_count<T>(
        &mut self,
        variation: Variation,
        count: T,
    ) -> Result<(), WriteError>
    where
        T: Index,
    {
        variation.write(self.cursor)?;
        T::LIMITED_COUNT_QUALIFIER.write(self.cursor)?;
        count.write(self.cursor)?;
        Ok(())
    }

    pub(crate) fn write_clear_restart(&mut self) -> Result<(), WriteError> {
        self.write_range_only(Variation::Group80Var1, 7u8, 7u8)?;
        self.cursor.write_u8(0)?;
        Ok(())
    }

    pub(crate) fn write_prefixed_items<'c, V, I>(
        &mut self,
        iter: impl Iterator<Item = &'c (V, I)>,
    ) -> Result<(), WriteError>
    where
        V: FixedSizeVariation + 'c,
        I: Index + 'c,
    {
        V::VARIATION.write(self.cursor)?;
        I::COUNT_AND_PREFIX_QUALIFIER.write(self.cursor)?;
        let pos_of_count = self.cursor.position();
        self.cursor.skip(I::SIZE as usize)?;

        let mut count = I::zero();
        for (v, i) in iter {
            i.write(self.cursor)?;
            v.write(self.cursor)?;
            count.increment();
        }

        self.cursor.at_pos(pos_of_count, |cur| count.write(cur))
    }

    pub(crate) fn write_count_of_one<V>(&mut self, item: V) -> Result<(), WriteError>
    where
        V: FixedSizeVariation,
    {
        V::VARIATION.write(self.cursor)?;
        QualifierCode::Count8.write(self.cursor)?;
        self.cursor.write_u8(1)?;
        item.write(self.cursor)?;
        Ok(())
    }

    pub(crate) fn write_attribute(&mut self, attr: &OwnedAttribute) -> Result<(), AttrWriteError> {
        let variation = Variation::Group0(attr.variation);
        variation.write(self.cursor)?;
        QualifierCode::Range8.write(self.cursor)?;
        let start_stop: u8 = attr.set.value();
        self.cursor.write_u8(start_stop)?;
        self.cursor.write_u8(start_stop)?;
        attr.value.write(self.cursor)?;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn to_parsed(&'a self) -> ParsedFragment<'a> {
        ParsedFragment::parse(self.cursor.written()).unwrap()
    }
}

pub(crate) fn confirm_solicited(seq: Sequence, cursor: &mut WriteCursor) -> Result<(), WriteError> {
    start_request(ControlField::request(seq), FunctionCode::Confirm, cursor).map(|_| {})
}

pub(crate) fn confirm_unsolicited(
    seq: Sequence,
    cursor: &mut WriteCursor,
) -> Result<(), WriteError> {
    start_request(
        ControlField::unsolicited(seq),
        FunctionCode::Confirm,
        cursor,
    )
    .map(|_| {})
}

impl Variation {
    pub(crate) fn write(self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        let (g, v) = self.to_group_and_var();
        cursor.write_u8(g)?;
        cursor.write_u8(v)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::app::sequence::Sequence;
    use scursor::WriteCursor;

    use super::*;

    fn read_integrity(seq: Sequence, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        let mut writer = start_request(ControlField::request(seq), FunctionCode::Read, cursor)?;
        writer.write_all_objects_header(Variation::Group60Var2)?;
        writer.write_all_objects_header(Variation::Group60Var3)?;
        writer.write_all_objects_header(Variation::Group60Var4)?;
        writer.write_all_objects_header(Variation::Group60Var1)?;
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
