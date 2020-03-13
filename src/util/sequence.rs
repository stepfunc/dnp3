pub struct Sequence {
    value: u8,
    max: u8,
}

impl Sequence {
    const MAX_TRANSPORT_SEQ: u8 = 63;

    fn calc_next(value: u8, max: u8) -> u8 {
        if value >= max {
            0
        } else {
            value + 1
        }
    }

    pub fn calc_next_transport(value: u8) -> u8 {
        Self::calc_next(value, Self::MAX_TRANSPORT_SEQ)
    }

    pub fn new(value: u8, max: u8) -> Self {
        Self { value, max }
    }

    pub fn transport() -> Self {
        Sequence::new(0, Self::MAX_TRANSPORT_SEQ)
    }

    pub fn reset(&mut self) {
        self.value = 0;
    }

    pub fn next(&mut self) -> u8 {
        let ret = self.value;
        self.value = Self::calc_next(ret, self.max);
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
