#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Sequence {
    value: u8,
}

impl Sequence {
    pub const MAX: u8 = 15;
    pub const MASK: u8 = 0b0000_1111;

    fn calc_next(value: u8) -> u8 {
        if value == Self::MAX {
            0
        } else {
            value + 1
        }
    }

    fn calc_previous(value: u8) -> u8 {
        if value == 0 {
            Self::MAX
        } else {
            value - 1
        }
    }

    pub fn value(self) -> u8 {
        self.value
    }

    pub fn new(x: u8) -> Self {
        Self {
            value: x & Self::MASK,
        }
    }

    pub fn reset(&mut self) {
        self.value = 0;
    }

    pub fn next_value(self) -> u8 {
        Self::calc_next(self.value)
    }

    pub fn previous_value(self) -> u8 {
        Self::calc_previous(self.value)
    }

    pub fn increment(&mut self) -> Sequence {
        let ret = self.value;
        self.value = Self::calc_next(ret);
        Sequence { value: ret }
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
        for i in 0..16 {
            // which is really [0,15]
            assert_eq!(seq.increment(), Sequence::new(i));
        }

        assert_eq!(seq.increment(), Sequence::new(0)); // goes to zero
    }
}
