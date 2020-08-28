use std::fmt::{Debug, Formatter};

pub trait SequenceParams {
    const NUM_BITS: u8;
    const MAX_VALUE: u8 = (1 << Self::NUM_BITS) - 1;
}

pub struct Sequence<T>
where
    T: SequenceParams,
{
    value: u8,
    phantom: std::marker::PhantomData<T>,
}

// can't automatically derive these b/c of generic parameter
impl<T> Copy for Sequence<T> where T: SequenceParams {}
impl<T> Clone for Sequence<T>
where
    T: SequenceParams,
{
    fn clone(&self) -> Self {
        Self {
            value: self.value,
            phantom: std::marker::PhantomData {},
        }
    }
}
impl<T> Debug for Sequence<T>
where
    T: SequenceParams,
{
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}
impl<T> PartialEq for Sequence<T>
where
    T: SequenceParams,
{
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T> Sequence<T>
where
    T: SequenceParams,
{
    pub fn new(x: u8) -> Self {
        Self {
            value: x & T::MAX_VALUE, // MAX_VALUE doubles as a mask
            phantom: std::marker::PhantomData {},
        }
    }

    fn calc_next(value: u8) -> u8 {
        if value == T::MAX_VALUE {
            0
        } else {
            value + 1
        }
    }

    fn calc_previous(value: u8) -> u8 {
        if value == 0 {
            T::MAX_VALUE
        } else {
            value - 1
        }
    }

    pub fn reset(&mut self) {
        self.value = 0;
    }

    pub fn value(self) -> u8 {
        self.value
    }

    pub fn next(self) -> u8 {
        Self::calc_next(self.value)
    }

    pub fn previous(self) -> u8 {
        Self::calc_previous(self.value)
    }

    pub fn increment(&mut self) -> Sequence<T> {
        let ret = self.value;
        self.value = Self::calc_next(ret);
        Self {
            value: ret,
            phantom: std::marker::PhantomData {},
        }
    }
}

impl<T> Default for Sequence<T>
where
    T: SequenceParams,
{
    fn default() -> Self {
        Self::new(0)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    struct MockSeq;
    impl SequenceParams for MockSeq {
        const NUM_BITS: u8 = 3;
    }

    #[test]
    fn increments_and_wraps_as_expected() {
        let mut seq = Sequence::<MockSeq>::default();
        for i in 0..8 {
            // 8 == 2^3
            // which is really [0,7]
            assert_eq!(seq.increment().value, i);
        }

        assert_eq!(seq.increment().value, 0); // goes to zero
    }
}
