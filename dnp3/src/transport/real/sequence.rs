use crate::util::sequence::SequenceParams;

pub(crate) struct TransportParams {}
impl SequenceParams for TransportParams {
    const NUM_BITS: u8 = 6; // 2^6 == 64
}

pub(crate) type Sequence = crate::util::sequence::Sequence<TransportParams>;

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
