pub struct Sequence {
    value: u8,
    max: u8,
}

impl Sequence {
    pub fn new(value: u8, max: u8) -> Self {
        Self { value, max }
    }

    pub fn transport() -> Self {
        Sequence::new(0, 63)
    }

    pub fn reset(&mut self) {
        self.value = 0;
    }

    pub fn next(&mut self) -> u8 {
        let ret = self.value;

        self.value = if self.value == self.max {
            0
        } else {
            self.value + 1
        };

        ret
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn increments_and_wraps_as_expected() {
        let mut seq = Sequence::transport();
        for i in 0..64 {
            // which is really [0,63]
            assert_eq!(seq.next(), i);
        }

        assert_eq!(seq.next(), 0); // goes to zero
    }
}
