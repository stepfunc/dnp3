/// custom read-only cursor
#[derive(Debug, PartialEq)]
pub struct ReadCursor<'a> {
    src: &'a [u8],
}

#[derive(Debug)]
pub struct ReadError;

impl<'a> ReadCursor<'a> {
    pub fn empty() -> Self {
        Self { src: &[] }
    }

    pub fn new(src: &'a [u8]) -> ReadCursor {
        ReadCursor { src }
    }

    pub fn len(&self) -> usize {
        self.src.len()
    }

    pub fn is_empty(&self) -> bool {
        self.src.is_empty()
    }

    pub fn read_all(&mut self) -> &'a [u8] {
        let ret = self.src;
        self.src = &[];
        ret
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

    pub fn read_i16_le(&mut self) -> Result<i16, ReadError> {
        self.read_u16_le().map(|x| x as i16)
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

    pub fn read_i32_le(&mut self) -> Result<i32, ReadError> {
        self.read_u32_le().map(|x| x as i32)
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

    pub fn read_f32_le(&mut self) -> Result<f32, ReadError> {
        let mut bytes: [u8; 4] = [0; 4];
        bytes.copy_from_slice(self.read_bytes(4)?);
        Ok(f32::from_le_bytes(bytes))
    }

    pub fn read_f64_le(&mut self) -> Result<f64, ReadError> {
        let mut bytes: [u8; 8] = [0; 8];
        bytes.copy_from_slice(self.read_bytes(8)?);
        Ok(f64::from_le_bytes(bytes))
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

    pub fn written(&self) -> &[u8] {
        &self.dest[0..self.pos]
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
        for s in [0, 8].iter() {
            let b = ((value >> *s) & 0xFF) as u8;
            self.write_u8(b)?;
        }
        Ok(())
    }

    pub fn write_i16_le(&mut self, value: i16) -> Result<(), WriteError> {
        self.write_u16_le(value as u16)
    }

    pub fn write_u32_le(&mut self, value: u32) -> Result<(), WriteError> {
        if self.remaining() < 4 {
            // don't write any bytes if there's isn't space for the whole thing
            return Err(WriteError);
        }
        for s in [0, 8, 16, 24].iter() {
            let b = ((value >> *s) & 0xFF) as u8;
            self.write_u8(b)?;
        }
        Ok(())
    }

    pub fn write_i32_le(&mut self, value: i32) -> Result<(), WriteError> {
        self.write_u32_le(value as u32)
    }

    pub fn write_u48_le(&mut self, value: u64) -> Result<(), WriteError> {
        if self.remaining() < 6 {
            // don't write any bytes if there's isn't space for the whole thing
            return Err(WriteError);
        }
        for s in [0, 8, 16, 24, 32, 40].iter() {
            let b = ((value >> *s) & 0xFF) as u8;
            self.write_u8(b)?;
        }
        Ok(())
    }

    pub fn write_f32_le(&mut self, value: f32) -> Result<(), WriteError> {
        self.write(&f32::to_le_bytes(value))
    }

    pub fn write_f64_le(&mut self, value: f64) -> Result<(), WriteError> {
        self.write(&f64::to_le_bytes(value))
    }
}
