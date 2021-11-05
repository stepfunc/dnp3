#[derive(Copy, Clone, Default)]
pub(crate) struct Sequence {
    value: u8,
}

impl Sequence {
    const MAX_VALUE: u8 = 0b0011_1111;

    pub(crate) fn new(value: u8) -> Self {
        Self {
            value: value & Self::MAX_VALUE,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.value = 0;
    }

    pub(crate) fn value(&self) -> u8 {
        self.value
    }

    fn calc_next(value: u8) -> u8 {
        if value == Self::MAX_VALUE {
            0
        } else {
            value + 1
        }
    }

    pub(crate) fn next(self) -> u8 {
        Self::calc_next(self.value)
    }

    pub(crate) fn increment(&mut self) -> Sequence {
        let ret = self.value;
        self.value = Self::calc_next(ret);
        Self { value: ret }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn increments_and_wraps_as_expected() {
        let mut seq = Sequence::default();
        for i in 0..64 {
            // which is really [0,63]
            assert_eq!(seq.increment().value(), i);
        }

        assert_eq!(seq.increment().value(), 0); // goes to zero
    }
}
