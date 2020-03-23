use crate::app::range::Range;
use crate::util::cursor::{ReadCursor, ReadError};

fn num_bytes_for_bits(count: usize) -> usize {
    (count + 7) / 8
}

/// zero-copy type used to iterate over a collection of bits without allocating
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct BitSequence<'a> {
    bytes: &'a [u8],
    count: usize,
}

/// zero-copy type used to iterate over a collection of bits without allocating
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct IndexedBitSequence<'a> {
    bytes: &'a [u8],
    range: Range,
}

impl<'a> BitSequence<'a> {
    pub fn parse(cursor: &mut ReadCursor<'a>, count: usize) -> Result<Self, ReadError> {
        let bytes = cursor.read_bytes(num_bytes_for_bits(count))?;

        Ok(Self { bytes, count })
    }

    pub fn iter(&self) -> BitIterator<'a> {
        BitIterator {
            bytes: self.bytes,
            count: self.count,
            pos: 0,
        }
    }
}

impl<'a> IndexedBitSequence<'a> {
    pub fn parse(cursor: &mut ReadCursor<'a>, range: Range) -> Result<Self, ReadError> {
        let bytes = cursor.read_bytes(num_bytes_for_bits(range.get_count()))?;

        Ok(Self { bytes, range })
    }

    pub fn iter(&self) -> IndexedBitIterator<'a> {
        IndexedBitIterator::<'a> {
            index: self.range.get_start(),
            inner: BitIterator::<'a> {
                bytes: self.bytes,
                count: self.range.get_count(),
                pos: 0,
            },
        }
    }
}

pub struct BitIterator<'a> {
    bytes: &'a [u8],
    count: usize,
    pos: usize,
}

pub struct IndexedBitIterator<'a> {
    index: u16,
    inner: BitIterator<'a>,
}

impl<'a> Iterator for BitIterator<'a> {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.count {
            return None;
        }

        let byte = self.pos / 8;
        let bit = (self.pos % 8) as u8;

        match self.bytes.get(byte) {
            Some(value) => {
                self.pos += 1;
                Some((*value & (1 << bit)) != 0)
            }
            None => None,
        }
    }
}

impl<'a> Iterator for IndexedBitIterator<'a> {
    type Item = (bool, u16);

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.next() {
            Some(x) => {
                let index = self.index;
                // this guard is required, b/c if the last
                // element has an index of 65535, this would
                // cause overflow
                if index < std::u16::MAX {
                    self.index += 1;
                }
                Some((x, index))
            }
            None => None,
        }
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
    fn can_parse_bit_sequence() {
        let data = [0b1111_0000, 0b0000_0001];
        let mut cursor = ReadCursor::new(&data);
        let seq = BitSequence::parse(&mut cursor, 10).unwrap();
        assert!(cursor.is_empty());
        let vec: Vec<bool> = seq.iter().collect();
        assert_eq!(
            vec,
            vec![false, false, false, false, true, true, true, true, true, false]
        );
    }

    #[test]
    fn can_parse_bit_sequence_with_index() {
        let range = Range::from(65534, 65535).unwrap();
        let data = [0b0000_0001];
        let mut cursor = ReadCursor::new(&data);
        let seq = IndexedBitSequence::parse(&mut cursor, range).unwrap();
        assert!(cursor.is_empty());
        let vec: Vec<(bool, u16)> = seq.iter().collect();
        assert_eq!(vec, vec![(true, 65534), (false, 65535)]);
    }
}
