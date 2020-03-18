/// custom read-only cursor
pub struct ReadCursor<'a> {
    src: &'a [u8],
}

#[derive(Debug)]
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
        match self.src {
            [a, rest @ ..] => {
                self.src = rest;
                Ok(*a)
            }
            _ => Err(ReadError),
        }
    }

    pub fn read_u16_le(&mut self) -> Result<u16, ReadError> {
        match self.src {
            [b1, b2, rest @ ..] => {
                self.src = rest;
                Ok((*b2 as u16) << 8 | (*b1 as u16))
            }
            _ => Err(ReadError),
        }
    }

    pub fn read_u32_le(&mut self) -> Result<u32, ReadError> {
        match self.src {
            [b1, b2, b3, b4, rest @ ..] => {
                self.src = rest;
                Ok((*b4 as u32) << 24 | (*b3 as u32) << 16 | (*b2 as u32) << 8 | *b1 as u32)
            }
            _ => Err(ReadError),
        }
    }

    pub fn read_u48_le(&mut self) -> Result<u64, ReadError> {
        match self.src {
            [b1, b2, b3, b4, b5, b6, rest @ ..] => {
                self.src = rest;
                Ok((*b6 as u64) << 40
                    | (*b5 as u64) << 32
                    | (*b4 as u64) << 24
                    | (*b3 as u64) << 16
                    | (*b2 as u64) << 8
                    | *b1 as u64)
            }
            _ => Err(ReadError),
        }
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

pub struct WriteCursor<'a> {
    dest: &'a mut [u8],
    pos: usize,
}

#[derive(Debug)]
pub struct WriteError;

impl<'a> WriteCursor<'a> {
    pub fn new(dest: &'a mut [u8]) -> WriteCursor<'a> {
        WriteCursor { dest, pos: 0 }
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn written_since(&'a self, pos: usize) -> Result<&'a [u8], WriteError> {
        match self.dest.get(pos..self.pos) {
            Some(x) => Ok(x),
            None => Err(WriteError),
        }
    }

    pub fn remaining(&self) -> usize {
        self.dest.len() - self.pos
    }

    pub fn write(&mut self, bytes: &[u8]) -> Result<(), WriteError> {
        if self.remaining() < bytes.len() {
            return Err(WriteError);
        }

        let new_pos = self.pos + bytes.len();
        match self.dest.get_mut(self.pos..new_pos) {
            Some(x) => x.copy_from_slice(bytes),
            None => return Err(WriteError),
        }
        self.pos = new_pos;
        Ok(())
    }

    pub fn write_u8(&mut self, value: u8) -> Result<(), WriteError> {
        match self.dest.get_mut(self.pos) {
            Some(x) => {
                *x = value;
                self.pos += 1;
                Ok(())
            }
            None => Err(WriteError),
        }
    }

    pub fn write_u16_le(&mut self, value: u16) -> Result<(), WriteError> {
        if self.remaining() < 2 {
            // don't write any bytes if there's isn't space for the whole thing
            return Err(WriteError);
        }
        let upper = ((value & 0xFF00) >> 8) as u8;
        let lower = (value & 0x00FF) as u8;
        self.write_u8(lower)?;
        self.write_u8(upper)
    }
}
