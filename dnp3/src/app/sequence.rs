// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

/// Application-layer sequence number
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct Sequence {
    value: u8,
}

impl Sequence {
    const MAX_VALUE: u8 = 0b0000_1111;

    /// retrieve the underlying value of the sequence number
    pub fn value(&self) -> u8 {
        self.value
    }

    fn calc_next(value: u8) -> u8 {
        value.wrapping_add(1) & Self::MAX_VALUE
    }

    pub(crate) fn next(self) -> u8 {
        Self::calc_next(self.value)
    }

    pub(crate) fn new(x: u8) -> Self {
        Self {
            value: x & Self::MAX_VALUE,
        }
    }

    pub(crate) fn increment(&mut self) -> Sequence {
        let value = self.value;
        self.value = Self::calc_next(value);
        Self { value }
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
            assert_eq!(seq.increment().value(), i);
        }

        assert_eq!(seq.increment().value(), 0); // goes to zero
    }
}
