use crate::app::measurement::DoubleBit;
use crate::app::parse::range::Range;
use crate::util::bit::Bitfield;

use scursor::{ReadCursor, ReadError};

fn num_bytes_for_bits(count: usize) -> usize {
    (count + 7) / 8
}

fn num_bytes_for_double_bits(count: usize) -> usize {
    (count + 3) / 4
}

/// zero-copy type used to iterate over a collection of bits without allocating
#[derive(Debug, PartialEq, Copy, Clone)]
pub(crate) struct BitSequence<'a> {
    bytes: &'a [u8],
    range: Range,
}

/// zero-copy type used to iterate over a collection of double bits without allocating
#[derive(Debug, PartialEq, Copy, Clone)]
pub(crate) struct DoubleBitSequence<'a> {
    bytes: &'a [u8],
    range: Range,
}

impl<'a> BitSequence<'a> {
    pub(crate) fn empty() -> Self {
        Self {
            bytes: &[],
            range: Range::empty(),
        }
    }

    pub(crate) fn parse(range: Range, cursor: &mut ReadCursor<'a>) -> Result<Self, ReadError> {
        let bytes = cursor.read_bytes(num_bytes_for_bits(range.get_count()))?;

        Ok(Self { bytes, range })
    }

    pub(crate) fn iter(&self) -> BitIterator<'a> {
        BitIterator::<'a> {
            index: self.range.get_start(),
            bytes: self.bytes,
            count: self.range.get_count(),
            pos: 0,
        }
    }
}

impl<'a> DoubleBitSequence<'a> {
    pub(crate) fn empty() -> Self {
        Self {
            bytes: &[],
            range: Range::empty(),
        }
    }

    pub(crate) fn parse(range: Range, cursor: &mut ReadCursor<'a>) -> Result<Self, ReadError> {
        let bytes = cursor.read_bytes(num_bytes_for_double_bits(range.get_count()))?;
        Ok(Self { bytes, range })
    }

    pub(crate) fn iter(&self) -> DoubleBitIterator<'a> {
        DoubleBitIterator::<'a> {
            index: self.range.get_start(),
            bytes: self.bytes,
            count: self.range.get_count(),
            pos: 0,
        }
    }
}

pub(crate) struct BitIterator<'a> {
    index: u16,
    bytes: &'a [u8],
    count: usize,
    pos: usize,
}

pub(crate) struct DoubleBitIterator<'a> {
    index: u16,
    bytes: &'a [u8],
    count: usize,
    pos: usize,
}

impl Iterator for BitIterator<'_> {
    type Item = (bool, u16);

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.count {
            return None;
        }

        let byte = self.pos / 8;
        let bit = (self.pos % 8) as u8; // [0,7]

        match self.bytes.get(byte) {
            Some(value) => {
                let value = (*value & (1 << bit)) != 0;
                let index = self.index;
                self.pos += 1;
                if self.pos < self.count {
                    // protect from overflow
                    self.index += 1;
                }

                Some((value, index))
            }
            None => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let count = self.count - self.pos;
        (count, Some(count))
    }
}

impl Iterator for DoubleBitIterator<'_> {
    type Item = (DoubleBit, u16);

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.count {
            return None;
        }

        let byte = self.pos / 4;
        let shift = 2 * (self.pos % 4) as u8; // [0,2,4,6]

        match self.bytes.get(byte) {
            Some(x) => {
                let bits = *x >> shift;
                let value = DoubleBit::from(bits.bit_1(), bits.bit_0());
                let index = self.index;
                self.pos += 1;
                if self.pos < self.count {
                    // protect from overflow
                    self.index += 1;
                }
                Some((value, index))
            }
            None => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let count = self.count - self.pos;
        (count, Some(count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correctly_calculates_bytes_for_bits() {
        assert_eq!(num_bytes_for_bits(0), 0);
        assert_eq!(num_bytes_for_bits(1), 1);
        assert_eq!(num_bytes_for_bits(8), 1);
        assert_eq!(num_bytes_for_bits(7), 1);
        assert_eq!(num_bytes_for_bits(9), 2);
        assert_eq!(num_bytes_for_bits(15), 2);
        assert_eq!(num_bytes_for_bits(16), 2);
    }

    #[test]
    fn correctly_calculates_bytes_for_double_bits() {
        assert_eq!(num_bytes_for_double_bits(0), 0);
        assert_eq!(num_bytes_for_double_bits(1), 1);
        assert_eq!(num_bytes_for_double_bits(3), 1);
        assert_eq!(num_bytes_for_double_bits(4), 1);
        assert_eq!(num_bytes_for_double_bits(5), 2);
        assert_eq!(num_bytes_for_double_bits(8), 2);
        assert_eq!(num_bytes_for_double_bits(9), 3);
    }

    #[test]
    fn can_parse_bit_sequence_at_max_index() {
        let range = Range::from(65534, 65535).unwrap();
        let data = [0b0000_0001];
        let mut cursor = ReadCursor::new(&data);
        let seq = BitSequence::parse(range, &mut cursor).unwrap();
        assert!(cursor.is_empty());
        assert_eq!(seq.iter().size_hint(), (2, Some(2)));
        let vec: Vec<(bool, u16)> = seq.iter().collect();
        assert_eq!(vec, vec![(true, 65534), (false, 65535)]);
    }

    #[test]
    fn can_parse_single_byte_of_double_bit() {
        let range = Range::from(1, 4).unwrap();
        let data = [0b11_10_01_00];
        let mut cursor = ReadCursor::new(&data);
        let seq = DoubleBitSequence::parse(range, &mut cursor).unwrap();
        assert!(cursor.is_empty());
        assert_eq!(seq.iter().size_hint(), (4, Some(4)));
        let vec: Vec<(DoubleBit, u16)> = seq.iter().collect();
        assert_eq!(
            vec,
            vec![
                (DoubleBit::Intermediate, 1),
                (DoubleBit::DeterminedOff, 2),
                (DoubleBit::DeterminedOn, 3),
                (DoubleBit::Indeterminate, 4),
            ]
        );
    }

    #[test]
    fn can_parse_double_bit_sequence_at_max_index() {
        let range = Range::from(65531, 65535).unwrap(); // five values!
        let data = [0b1000_1101, 0b0000_0011];
        let mut cursor = ReadCursor::new(&data);
        let seq = DoubleBitSequence::parse(range, &mut cursor).unwrap();
        assert!(cursor.is_empty());
        assert_eq!(seq.iter().size_hint(), (5, Some(5)));
        let vec: Vec<(DoubleBit, u16)> = seq.iter().collect();
        assert_eq!(
            vec,
            vec![
                (DoubleBit::DeterminedOff, 65531),
                (DoubleBit::Indeterminate, 65532),
                (DoubleBit::Intermediate, 65533),
                (DoubleBit::DeterminedOn, 65534),
                (DoubleBit::Indeterminate, 65535),
            ]
        );
    }
}
