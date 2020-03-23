use crate::app::parser::ParseError;
use crate::util::cursor::ReadCursor;

pub struct Bytes<'a> {
    pub value: &'a [u8],
}

pub struct BytesIterator<'a> {
    cursor: ReadCursor<'a>,
    size: usize,
}

impl<'a> BytesIterator<'a> {
    pub fn parse(
        variation: u8,
        count: usize,
        cursor: &mut ReadCursor<'a>,
    ) -> Result<Self, ParseError> {
        if variation == 0 {
            return Err(ParseError::ZeroLengthOctetData);
        }

        Ok(BytesIterator {
            cursor: ReadCursor::new(cursor.read_bytes(variation as usize * count)?),
            size: variation as usize,
        })
    }
}

impl<'a> Iterator for BytesIterator<'a> {
    type Item = Bytes<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.cursor
            .read_bytes(self.size)
            .ok()
            .map(|b| Bytes { value: b })
    }
}
