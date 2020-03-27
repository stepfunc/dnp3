#[derive(Copy, Clone)]
pub struct Sequence {
    value: u8,
}

impl Sequence {
    const MAX: u8 = 63;

    fn calc_next(value: u8) -> u8 {
        if value == Self::MAX {
            0
        } else {
            value + 1
        }
    }

    pub fn new(x: u8) -> Self {
        Self {
            value: x & super::constants::SEQ_MASK,
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

    pub fn increment(&mut self) -> u8 {
        let ret = self.value;
        self.value = Self::calc_next(ret);
        ret
    }
}

impl Default for Sequence {
    fn default() -> Self {
        Self::new(0)
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
            assert_eq!(seq.increment(), i);
        }

        assert_eq!(seq.increment(), 0); // goes to zero
    }
}
