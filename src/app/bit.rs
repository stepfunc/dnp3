use crate::util::cursor::{ReadCursor, ReadError};

fn num_bytes_for_bits(count: usize) -> usize {
    (count + 7) / 8
}

/// zero-copy type used to iterate over a collection of bits without allocating
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct BitCollection<'a> {
    bytes: &'a [u8],
    count: usize,
}

/// zero-copy type used to iterate over a collection of bits without allocating
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct IndexedBitCollection<'a> {
    bytes: &'a [u8],
    count: usize,
    start: u16,
}

impl<'a> BitCollection<'a> {
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

impl<'a> IndexedBitCollection<'a> {
    pub fn parse(cursor: &mut ReadCursor<'a>, count: usize, start: u16) -> Result<Self, ReadError> {
        let bytes = cursor.read_bytes(num_bytes_for_bits(count))?;

        Ok(Self {
            bytes,
            count,
            start,
        })
    }

    pub fn iter(&self) -> IndexedBitIterator<'a> {
        IndexedBitIterator::<'a> {
            index: self.start,
            inner: BitIterator::<'a> {
                bytes: self.bytes,
                count: self.count,
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
                self.index += 1;
                Some((x, index))
            }
            None => None,
        }
    }
}
