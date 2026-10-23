// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::measurement::DoubleBit;
use crate::app::parse::range::Range;
use crate::util::bit::Bitfield;

use scursor::{ReadCursor, ReadError};

fn num_bytes_for_bits(count: usize) -> usize {
    count.div_ceil(8)
}

fn num_bytes_for_double_bits(count: usize) -> usize {
    count.div_ceil(4)
}

/// A value packed several to a byte, least significant bits first
pub(crate) trait Packed: Sized {
    /// the number of bytes that hold `count` values
    fn num_bytes(count: usize) -> usize;

    /// the value at `pos`, or `None` past the end of `bytes`
    fn get(bytes: &[u8], pos: usize) -> Option<Self>;
}

impl Packed for bool {
    fn num_bytes(count: usize) -> usize {
        num_bytes_for_bits(count)
    }

    fn get(bytes: &[u8], pos: usize) -> Option<Self> {
        let value = bytes.get(pos / 8)?;
        Some((*value & (1 << (pos % 8))) != 0)
    }
}

impl Packed for DoubleBit {
    fn num_bytes(count: usize) -> usize {
        num_bytes_for_double_bits(count)
    }

    fn get(bytes: &[u8], pos: usize) -> Option<Self> {
        let shift = match pos % 4 {
            0 => 0,
            1 => 2,
            2 => 4,
            _ => 6,
        };
        let bits = *bytes.get(pos / 4)? >> shift;
        Some(DoubleBit::from(bits.bit_1(), bits.bit_0()))
    }
}

/// zero-copy type used to iterate over a range of packed values without allocating
#[derive(Debug, PartialEq, Copy, Clone)]
pub(crate) struct PackedSequence<'a, P> {
    bytes: &'a [u8],
    range: Range,
    phantom: std::marker::PhantomData<P>,
}

/// a range of bits
pub(crate) type BitSequence<'a> = PackedSequence<'a, bool>;

/// a range of double bits
pub(crate) type DoubleBitSequence<'a> = PackedSequence<'a, DoubleBit>;

impl<'a, P: Packed> PackedSequence<'a, P> {
    pub(crate) fn parse(range: Range, cursor: &mut ReadCursor<'a>) -> Result<Self, ReadError> {
        let bytes = cursor.read_bytes(P::num_bytes(range.get_count()))?;
        Ok(Self {
            bytes,
            range,
            phantom: std::marker::PhantomData,
        })
    }

    pub(crate) fn iter(&self) -> PackedIterator<'a, P> {
        PackedIterator {
            index: self.range.get_start(),
            bytes: self.bytes,
            count: self.range.get_count(),
            pos: 0,
            phantom: std::marker::PhantomData,
        }
    }
}

pub(crate) struct PackedIterator<'a, P> {
    index: u16,
    bytes: &'a [u8],
    count: usize,
    pos: usize,
    phantom: std::marker::PhantomData<P>,
}

impl<P: Packed> Iterator for PackedIterator<'_, P> {
    type Item = (P, u16);

    #[cfg_attr(all(test, dnp3_no_panic), no_panic::no_panic)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.count {
            return None;
        }

        let value = P::get(self.bytes, self.pos)?;
        let index = self.index;
        self.pos = self.pos.saturating_add(1);
        self.index = self.index.saturating_add(1);
        Some((value, index))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let count = self.count.saturating_sub(self.pos);
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
    fn can_parse_bits_across_two_bytes() {
        let range = Range::from(3, 12).unwrap(); // ten values
        let data = [0b1010_0110, 0b0000_0010];
        let mut cursor = ReadCursor::new(&data);
        let seq = BitSequence::parse(range, &mut cursor).unwrap();
        assert!(cursor.is_empty());
        assert_eq!(seq.iter().size_hint(), (10, Some(10)));
        let vec: Vec<(bool, u16)> = seq.iter().collect();
        assert_eq!(
            vec,
            vec![
                (false, 3),
                (true, 4),
                (true, 5),
                (false, 6),
                (false, 7),
                (true, 8),
                (false, 9),
                (true, 10),
                (false, 11),
                (true, 12),
            ]
        );
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
