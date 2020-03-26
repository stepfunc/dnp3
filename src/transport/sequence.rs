pub struct Sequence {
    value: u8,
}

impl Sequence {
    const MAX: u8 = 63;

    pub fn calc_next(value: u8) -> u8 {
        if value == Self::MAX {
            0
        } else {
            value + 1
        }
    }

    pub fn new() -> Self {
        Self { value: 0 }
    }

    pub fn reset(&mut self) {
        self.value = 0;
    }

    pub fn next(&mut self) -> u8 {
        let ret = self.value;
        self.value = Self::calc_next(ret);
        ret
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn increments_and_wraps_as_expected() {
        let mut seq = Sequence::new();
        for i in 0..64 {
            // which is really [0,63]
            assert_eq!(seq.next(), i);
        }

        assert_eq!(seq.next(), 0); // goes to zero
    }
}
