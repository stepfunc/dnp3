use std::ops::Range;

use crate::link::error::LogicError;

pub(crate) trait SliceExtNoPanic<T> {
    fn np_split_at(&self, pos: usize) -> Result<(&[T], &[T]), LogicError>;
    fn np_split_at_no_error(&self, pos: usize) -> (&[T], &[T]);
}

pub(crate) trait MutSliceExtNoPanic<T> {
    fn np_get_mut(&mut self, range: Range<usize>) -> Result<&mut [T], LogicError>;
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
            _ => (self, &[]),
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
