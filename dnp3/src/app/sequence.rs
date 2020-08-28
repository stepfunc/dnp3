use crate::util::sequence::SequenceParams;

#[derive(Copy, Clone)]
pub struct AppParams;
impl SequenceParams for AppParams {
    const NUM_BITS: u8 = 4;
}

pub type Sequence = crate::util::sequence::Sequence<AppParams>;

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
