// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::parse::range::Range;
use crate::app::parse::traits::FixedSize;
use crate::app::parse_error::ObjectParseError;

use crate::app::parse::options::ParseOptions;
use scursor::ReadCursor;

/// Wrapper around an underlying u8 slice
#[derive(Debug, PartialEq)]
pub(crate) struct Bytes<'a> {
    /// underlying slice
    pub(crate) value: &'a [u8],
}

impl<'a> Bytes<'a> {
    pub(crate) fn new(value: &'a [u8]) -> Self {
        Self { value }
    }
}

impl std::fmt::Display for Bytes<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.value.len() <= 3 {
            return write!(f, "{:02X?}", self.value);
        }

        if let Some(s) = self.value.get(0..3) {
            return write!(f, "length = {}, {:02X?} ...", self.value.len(), s);
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct RangedBytesSequence<'a> {
    bytes: &'a [u8],
    range: Range,
    size: usize,
}

#[derive(Debug)]
pub(crate) struct RangedBytesIterator<'a> {
    cursor: ReadCursor<'a>,
    index: u16,
    size: usize,
    remaining: usize,
}

#[derive(Debug)]
pub(crate) struct PrefixedBytesSequence<'a, T>
where
    T: FixedSize,
{
    bytes: &'a [u8],
    size: usize,
    count: usize,
    phantom: std::marker::PhantomData<T>,
}

pub(crate) struct PrefixedBytesIterator<'a, T>
where
    T: FixedSize,
{
    cursor: ReadCursor<'a>,
    size: usize,
    remaining: usize,
    phantom: std::marker::PhantomData<T>,
}

impl<'a> RangedBytesSequence<'a> {
    pub(crate) fn parse(
        options: ParseOptions,
        variation: u8,
        range: Range,
        cursor: &mut ReadCursor<'a>,
    ) -> Result<Self, ObjectParseError> {
        if variation == 0 && !options.parse_zero_length_strings {
            return Err(ObjectParseError::ZeroLengthOctetData);
        }

        let size = usize::from(variation);
        let num_bytes = size
            .checked_mul(range.get_count())
            .ok_or(ObjectParseError::InsufficientBytes)?;

        Ok(RangedBytesSequence {
            bytes: cursor.read_bytes(num_bytes)?,
            range,
            size,
        })
    }

    pub(crate) fn iter(&self) -> RangedBytesIterator<'a> {
        RangedBytesIterator {
            cursor: ReadCursor::new(self.bytes),
            index: self.range.get_start(),
            size: self.size,
            remaining: self.range.get_count(),
        }
    }
}

impl<'a, T> PrefixedBytesSequence<'a, T>
where
    T: FixedSize,
{
    pub(crate) fn parse(
        options: ParseOptions,
        variation: u8,
        count: u16,
        cursor: &mut ReadCursor<'a>,
    ) -> Result<Self, ObjectParseError> {
        if variation == 0 && !options.parse_zero_length_strings {
            return Err(ObjectParseError::ZeroLengthOctetData);
        }

        let size = usize::from(variation);
        let count = usize::from(count);
        let num_bytes = size
            .checked_add(T::SIZE.into())
            .and_then(|n| n.checked_mul(count))
            .ok_or(ObjectParseError::InsufficientBytes)?;

        Ok(PrefixedBytesSequence {
            bytes: cursor.read_bytes(num_bytes)?,
            size,
            count,
            phantom: std::marker::PhantomData {},
        })
    }

    pub(crate) fn iter(&self) -> PrefixedBytesIterator<'a, T> {
        PrefixedBytesIterator {
            cursor: ReadCursor::new(self.bytes),
            size: self.size,
            remaining: self.count,
            phantom: std::marker::PhantomData {},
        }
    }
}

impl<'a> Iterator for RangedBytesIterator<'a> {
    type Item = (&'a [u8], u16);

    #[cfg_attr(all(test, dnp3_no_panic), no_panic::no_panic)]
    fn next(&mut self) -> Option<Self::Item> {
        let remaining = self.remaining.checked_sub(1)?;
        let bytes = self.cursor.read_bytes(self.size).ok()?;
        let index = self.index;
        self.index = self.index.saturating_add(1);
        self.remaining = remaining;
        Some((bytes, index))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<'a, T> Iterator for PrefixedBytesIterator<'a, T>
where
    T: FixedSize,
{
    type Item = (&'a [u8], T);

    #[cfg_attr(all(test, dnp3_no_panic), no_panic::no_panic)]
    fn next(&mut self) -> Option<Self::Item> {
        let remaining = self.remaining.checked_sub(1)?;

        let index = match T::read(&mut self.cursor) {
            Ok(x) => x,
            _ => return None,
        };
        let bytes = match self.cursor.read_bytes(self.size) {
            Ok(x) => x,
            _ => return None,
        };

        self.remaining = remaining;

        Some((bytes, index))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn bytes_formats_as_expected() {
        let short = Bytes::new(&[0x01, 0x02, 0x03]);
        let long = Bytes::new(&[0x01, 0x02, 0x03, 0x04]);

        assert_eq!(format!("{short}"), "[01, 02, 03]");
        assert_eq!(format!("{long}"), "length = 4, [01, 02, 03] ...");
    }
}
