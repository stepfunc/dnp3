/// custom read-only cursor
pub struct ReadCursor<'a> {
    src: &'a [u8],
}

pub struct ReadError;

impl<'a> ReadCursor<'a> {
    pub fn new(src: &'a [u8]) -> ReadCursor {
        ReadCursor { src }
    }

    pub fn len(&self) -> usize {
        self.src.len()
    }

    pub fn is_empty(&self) -> bool {
        self.src.is_empty()
    }

    pub fn read_u8(&mut self) -> Result<u8, ReadError> {
        match self.src.split_first() {
            Some((first, rest)) => {
                self.src = rest;
                Ok(*first)
            }
            None => Err(ReadError),
        }
    }

    pub fn read_u16_le(&mut self) -> Result<u16, ReadError> {
        let low = self.read_u8()?;
        let high = self.read_u8()?;
        Ok((high as u16) << 8 | (low as u16))
    }

    pub fn read_bytes(&mut self, count: usize) -> Result<&'a [u8], ReadError> {
        match (self.src.get(0..count), self.src.get(count..)) {
            (Some(first), Some(rest)) => {
                self.src = rest;
                Ok(first)
            }
            _ => Err(ReadError),
        }
    }
}
