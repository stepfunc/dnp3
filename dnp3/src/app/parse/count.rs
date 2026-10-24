// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::parse::traits::FixedSize;

use scursor::{ReadCursor, ReadError};

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct CountSequence<'a, T>
where
    T: FixedSize,
{
    count: usize,
    data: &'a [u8],
    phantom: std::marker::PhantomData<T>,
}

impl<'a, T> CountSequence<'a, T>
where
    T: FixedSize,
{
    pub(crate) fn parse(
        count: u16,
        cursor: &mut ReadCursor<'a>,
    ) -> Result<CountSequence<'a, T>, ReadError> {
        let count = usize::from(count);
        let num_bytes = usize::from(T::SIZE).checked_mul(count).ok_or(ReadError)?;
        Ok(Self::new(count, cursor.read_bytes(num_bytes)?))
    }

    pub(crate) fn single(&self) -> Option<T> {
        if self.count != 1 {
            return None;
        }

        self.iter().next()
    }

    fn new(count: usize, data: &'a [u8]) -> Self {
        Self {
            count,
            data,
            phantom: std::marker::PhantomData {},
        }
    }

    pub(crate) fn iter(&self) -> CountIterator<'a, T> {
        CountIterator {
            remaining: self.count,
            cursor: ReadCursor::new(self.data),
            phantom: std::marker::PhantomData {},
        }
    }
}

pub(crate) struct CountIterator<'a, T> {
    cursor: ReadCursor<'a>,
    remaining: usize,
    phantom: std::marker::PhantomData<T>,
}

impl<T> Iterator for CountIterator<'_, T>
where
    T: FixedSize,
{
    type Item = T;

    #[cfg_attr(all(test, dnp3_no_panic), no_panic::no_panic)]
    fn next(&mut self) -> Option<Self::Item> {
        self.remaining = self.remaining.checked_sub(1)?;
        T::read(&mut self.cursor).ok()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::parse::range::assert_yields_exactly;
    use crate::app::variations::Group50Var1;
    use crate::app::Timestamp;

    use proptest::prelude::*;

    /// The count sequence of `values`, encoded as `objects` and followed by `trailing`, reads
    /// exactly `objects` and yields each value once, then stays exhausted; only a count of one has
    /// a single value; one byte less is an error
    fn check<T>(values: &[T], objects: &[u8], trailing: &[u8])
    where
        T: FixedSize + PartialEq + std::fmt::Debug,
    {
        let bytes = [objects, trailing].concat();
        let count = u16::try_from(values.len()).unwrap();

        let mut cursor = ReadCursor::new(&bytes);
        let seq = CountSequence::<T>::parse(count, &mut cursor).unwrap();
        assert_eq!(cursor.remaining(), trailing.len());
        assert_yields_exactly(seq.iter(), values);
        let single = match values {
            [x] => Some(*x),
            _ => None,
        };
        assert_eq!(seq.single(), single);

        if let Some(short) = objects.len().checked_sub(1) {
            CountSequence::<T>::parse(count, &mut ReadCursor::new(&objects[..short])).unwrap_err();
        }
    }

    #[test]
    fn count_sequence_examples() {
        // a trailing byte the sequence leaves unread
        check::<u16>(&[1, 2, 3], &[0x01, 0x00, 0x02, 0x00, 0x03, 0x00], &[0xFF]);
        // two objects: no single value, and rejected one byte short
        check::<u16>(&[1, 2], &[0x01, 0x00, 0x02, 0x00], &[]);
        // a count of zero consumes nothing
        check::<u16>(&[], &[], &[0xAA]);
        // a count of one is a single value
        check::<u16>(&[1], &[0x01, 0x00], &[0x02, 0x00]);
    }

    proptest! {
        #![proptest_config(crate::util::test_support::proptest_config(256))]

        /// A count sequence reads exactly its objects and yields each once, then stays
        /// exhausted; one byte less is an error
        #[test]
        fn count_sequence_yields_each_object_once(
            times in prop_oneof![1 => Just(0), 1 => Just(1), 6 => 0..40usize]
                .prop_flat_map(|len| prop::collection::vec(0..=Timestamp::MAX_VALUE, len)),
            trailing in prop::collection::vec(any::<u8>(), 0..4),
        ) {
            let objects: Vec<u8> = times.iter().flat_map(|x| x.to_le_bytes()[..6].to_vec()).collect();
            let values: Vec<Group50Var1> = times
                .iter()
                .map(|x| Group50Var1 { time: Timestamp::new(*x) })
                .collect();
            check(&values, &objects, &trailing);
        }
    }
}
