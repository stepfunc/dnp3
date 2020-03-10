use crate::error::LogicError;
use std::ops::Range;

pub trait SliceExt<T> {
    fn split_at_no_panic(&self, pos: usize) -> Result<(&[T], &[T]), LogicError>;
}

pub trait MutSliceExt<T> {
    fn get_mut_no_panic(&mut self, range: std::ops::Range<usize>) -> Result<&mut [T], LogicError>;
}

impl<T> SliceExt<T> for &[T] {
    fn split_at_no_panic(&self, pos: usize) -> Result<(&[T], &[T]), LogicError> {
        match (self.get(0..pos), self.get(pos..)) {
            (Some(left), Some(right)) => Ok((left, right)),
            _ => Err(LogicError::BadSize),
        }
    }
}

impl<T> MutSliceExt<T> for &mut [T] {
    fn get_mut_no_panic(&mut self, range: Range<usize>) -> Result<&mut [T], LogicError> {
        match self.get_mut(range) {
            Some(x) => Ok(x),
            None => Err(LogicError::BadSize),
        }
    }
}
