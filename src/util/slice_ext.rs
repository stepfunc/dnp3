use crate::error::LogicError;
use std::ops::Range;

pub trait SliceExtNoPanic<T> {
    fn np_split_at(&self, pos: usize) -> Result<(&[T], &[T]), LogicError>;
    fn np_split_at_no_error(&self, pos: usize) -> (&[T], &[T]);
    fn np_get(&self, range: Range<usize>) -> Result<&[T], LogicError>;
    fn np_take(&self, count: usize) -> Result<&[T], LogicError> {
        self.np_get(0..count)
    }
}

pub trait MutSliceExtNoPanic<T> {
    fn np_get_mut(&mut self, range: std::ops::Range<usize>) -> Result<&mut [T], LogicError>;
}

impl<T> SliceExtNoPanic<T> for &[T] {
    fn np_split_at(&self, pos: usize) -> Result<(&[T], &[T]), LogicError> {
        match (self.get(0..pos), self.get(pos..)) {
            (Some(left), Some(right)) => Ok((left, right)),
            _ => Err(LogicError::BadSize),
        }
    }

    fn np_split_at_no_error(&self, pos: usize) -> (&[T], &[T]) {
        match (self.get(0..pos), self.get(pos..)) {
            (Some(left), Some(right)) => (left, right),
            _ => (self.as_ref(), &[]),
        }
    }

    fn np_get(&self, range: Range<usize>) -> Result<&[T], LogicError> {
        match self.get(range) {
            Some(x) => Ok(x),
            None => Err(LogicError::BadSize),
        }
    }
}

impl<T> MutSliceExtNoPanic<T> for &mut [T] {
    fn np_get_mut(&mut self, range: Range<usize>) -> Result<&mut [T], LogicError> {
        match self.get_mut(range) {
            Some(x) => Ok(x),
            None => Err(LogicError::BadSize),
        }
    }
}
