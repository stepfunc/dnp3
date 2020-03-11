use crate::error::LogicError;
use crate::util::slice_ext::MutSliceExtNoPanic;
use std::io::Cursor;

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
            None => return Err(WriteError),
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
