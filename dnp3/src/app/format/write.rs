// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::attr::{AttrWriteError, OwnedAttribute};
use crate::app::header::{ControlField, RequestHeader};
#[cfg(test)]
use crate::app::header::{Iin, ResponseFunction, ResponseHeader};
#[cfg(test)]
use crate::app::parse::parser::ParsedFragment;
use crate::app::parse::traits::{FixedSizeVariation, Index};
use crate::app::variations::Variation;
use crate::app::{FunctionCode, QualifierCode};

use crate::app::format::free_format::FreeFormat;
use scursor::WriteCursor;

pub(crate) struct HeaderWriter<'a, 'b> {
    cursor: &'b mut WriteCursor<'a>,
}

pub(crate) fn start_request<'a, 'b>(
    control: ControlField,
    function: FunctionCode,
    cursor: &'b mut WriteCursor<'a>,
) -> Result<HeaderWriter<'a, 'b>, scursor::WriteError> {
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
) -> Result<HeaderWriter<'a, 'b>, scursor::WriteError> {
    let header = ResponseHeader::new(control, function, iin);
    cursor.write_bytes(&header.to_bytes())?;
    Ok(HeaderWriter { cursor })
}

impl<'a, 'b> HeaderWriter<'a, 'b> {
    #[cfg(test)]
    pub(crate) fn write_class1230(&mut self) -> Result<(), scursor::WriteError> {
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
    ) -> Result<(), scursor::WriteError> {
        variation.write(self.cursor)?;
        QualifierCode::AllObjects.write(self.cursor)?;
        Ok(())
    }

    pub(crate) fn write_range_only<T>(
        &mut self,
        variation: Variation,
        start: T,
        stop: T,
    ) -> Result<(), scursor::WriteError>
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
    ) -> Result<(), scursor::WriteError>
    where
        T: Index,
    {
        variation.write(self.cursor)?;
        T::LIMITED_COUNT_QUALIFIER.write(self.cursor)?;
        count.write(self.cursor)?;
        Ok(())
    }

    pub(crate) fn write_clear_restart(&mut self) -> Result<(), scursor::WriteError> {
        self.write_range_only(Variation::Group80Var1, 7u8, 7u8)?;
        self.cursor.write_u8(0)?;
        Ok(())
    }

    pub(crate) fn write_prefixed_items<'c, V, I>(
        &mut self,
        iter: impl Iterator<Item = &'c (V, I)>,
    ) -> Result<(), scursor::WriteError>
    where
        V: FixedSizeVariation + 'c,
        I: Index + 'c,
    {
        V::VARIATION.write(self.cursor)?;
        I::COUNT_AND_PREFIX_QUALIFIER.write(self.cursor)?;
        let pos_of_count = self.cursor.position();
        self.cursor.skip(I::SIZE.into())?;

        let mut count = I::zero();
        for (v, i) in iter {
            count = count.next().ok_or(scursor::WriteError::NumericOverflow)?;
            i.write(self.cursor)?;
            v.write(self.cursor)?;
        }

        self.cursor.at_pos(pos_of_count, |cur| count.write(cur))
    }

    pub(crate) fn write_count_of_one<V>(&mut self, item: V) -> Result<(), scursor::WriteError>
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

    pub(crate) fn write_free_format<T: FreeFormat>(
        &mut self,
        value: &T,
    ) -> Result<(), crate::app::format::WriteError> {
        T::VARIATION.write(self.cursor)?;
        QualifierCode::FreeFormat16.write(self.cursor)?;
        self.cursor.write_u8(1)?; // count of 1
        let length_pos = self.cursor.position();
        self.cursor.skip(2)?; // come back and write this after
        let object_start = self.cursor.position();
        value.write(self.cursor)?;
        let length = self
            .cursor
            .position()
            .checked_sub(object_start)
            .ok_or(crate::app::format::WriteError::Overflow)?;
        let length = crate::app::format::to_u16(length)?;
        self.cursor
            .at_pos(length_pos, |cur| cur.write_u16_le(length))?;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn to_parsed(&'a self) -> ParsedFragment<'a> {
        ParsedFragment::parse(
            crate::app::parse::options::ParseOptions::default(),
            self.cursor.written(),
        )
        .unwrap()
    }
}

impl Variation {
    pub(crate) fn write(self, cursor: &mut WriteCursor) -> Result<(), scursor::WriteError> {
        let (g, v) = self.to_group_and_var();
        cursor.write_u8(g)?;
        cursor.write_u8(v)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::app::file::Group70Var5;
    use crate::app::sequence::Sequence;
    use scursor::WriteCursor;

    use super::*;

    #[test]
    fn prefixed_items_fail_when_the_count_overflows_the_index_type() {
        let items: Vec<(crate::app::variations::Group41Var2, u8)> = (0..=u8::MAX)
            .map(|i| (crate::app::variations::Group41Var2::new(513), i))
            .collect();
        let mut buffer = [0u8; 2048];

        let mut cursor = WriteCursor::new(&mut buffer);
        let mut writer = HeaderWriter::new(&mut cursor);
        writer.write_prefixed_items(items.iter().take(255)).unwrap();

        let mut cursor = WriteCursor::new(&mut buffer);
        let mut writer = HeaderWriter::new(&mut cursor);
        assert_eq!(
            writer.write_prefixed_items(items.iter()),
            Err(scursor::WriteError::NumericOverflow)
        );
    }

    #[test]
    fn formats_integrity_poll() {
        let mut buffer: [u8; 100] = [0; 100];
        let mut cursor = WriteCursor::new(&mut buffer);
        start_request(
            ControlField::request(Sequence::new(0x01)),
            FunctionCode::Read,
            &mut cursor,
        )
        .unwrap()
        .write_class1230()
        .unwrap();

        assert_eq!(
            cursor.written(),
            [0xC1, 0x01, 0x3C, 0x02, 0x06, 0x3C, 0x03, 0x06, 0x3C, 0x04, 0x06, 0x3C, 0x01, 0x06]
        );
    }

    #[test]
    fn formats_clear_restart_write() {
        let mut buffer = [0; 8];
        let mut cursor = WriteCursor::new(&mut buffer);
        start_request(
            ControlField::request(Sequence::new(1)),
            FunctionCode::Write,
            &mut cursor,
        )
        .unwrap()
        .write_clear_restart()
        .unwrap();

        assert_eq!(cursor.written(), [0xC1, 0x02, 80, 1, 0, 7, 7, 0]);
    }

    #[test]
    fn writes_free_format() {
        let mut buffer: [u8; 100] = [0; 100];
        let mut cursor = WriteCursor::new(&mut buffer);

        let mut writer = HeaderWriter::new(&mut cursor);
        let data = Group70Var5 {
            file_handle: 0xFFEEDDCC,
            block_number: 0x01ABCDEF,
            file_data: b"hi",
        };

        writer.write_free_format(&data).unwrap();

        assert_eq!(
            cursor.written(),
            [
                70, 5, 0x5B, // var and qualifier
                0x01, // count
                10, 0, // length
                0xCC, 0xDD, 0xEE, 0xFF, 0xEF, 0xCD, 0xAB, 0x01, b'h', b'i'
            ]
        );
    }
}
