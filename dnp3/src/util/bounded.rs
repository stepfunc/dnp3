// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

/// A FIFO queue allocated once at construction that never holds more than `max` items
///
/// The items live in a ring of `max` slots: the `len` slots from `head`, wrapping at the end,
/// are occupied and every other slot is empty. A slice that can't grow, rather than a
/// `VecDeque`, leaves no path to a reallocation for the optimizer to keep.
pub(crate) struct BoundedQueue<T> {
    slots: Box<[Option<T>]>,
    head: usize,
    len: usize,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Full;

impl<T> BoundedQueue<T> {
    pub(crate) fn new(max: usize) -> Self {
        Self {
            slots: std::iter::repeat_with(|| None).take(max).collect(),
            head: 0,
            len: 0,
        }
    }

    pub(crate) fn max(&self) -> usize {
        self.slots.len()
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.len
    }

    /// The slot `offset` places past the head, wrapping at the end, or `None` if there are no
    /// slots
    fn slot(&self, offset: usize) -> Option<usize> {
        self.head.checked_add(offset)?.checked_rem(self.slots.len())
    }

    pub(crate) fn try_push(&mut self, item: T) -> Result<(), Full> {
        if self.len >= self.slots.len() {
            return Err(Full);
        }
        let len = self.len.checked_add(1).ok_or(Full)?;
        let slot = self
            .slot(self.len)
            .and_then(|x| self.slots.get_mut(x))
            .ok_or(Full)?;
        *slot = Some(item);
        self.len = len;
        Ok(())
    }

    pub(crate) fn front(&self) -> Option<&T> {
        self.slots.get(self.head)?.as_ref()
    }

    pub(crate) fn front_mut(&mut self) -> Option<&mut T> {
        self.slots.get_mut(self.head)?.as_mut()
    }

    pub(crate) fn pop_front(&mut self) -> Option<T> {
        let item = self.slots.get_mut(self.head)?.take()?;
        self.head = self.slot(1).unwrap_or(0);
        self.len = self.len.saturating_sub(1);
        Some(item)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &T> {
        (0..self.len).filter_map(|offset| {
            self.slot(offset)
                .and_then(|x| self.slots.get(x))
                .and_then(Option::as_ref)
        })
    }

    /// Empty the queue, visiting only the occupied slots
    pub(crate) fn clear(&mut self) {
        while self.pop_front().is_some() {}
        self.head = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holds_exactly_max_items() {
        let mut queue = BoundedQueue::new(2);
        assert_eq!(queue.try_push(1), Ok(()));
        assert_eq!(queue.try_push(2), Ok(()));
        assert_eq!(queue.try_push(3), Err(Full));
        assert_eq!(queue.iter().copied().collect::<Vec<_>>(), [1, 2]);
    }

    #[test]
    fn removing_items_makes_room() {
        let mut queue = BoundedQueue::new(1);
        assert_eq!(queue.try_push(1), Ok(()));
        assert_eq!(queue.pop_front(), Some(1));
        assert_eq!(queue.try_push(2), Ok(()));
        queue.clear();
        assert_eq!(queue.len(), 0);
        assert_eq!(queue.try_push(3), Ok(()));
    }

    #[test]
    fn zero_max_holds_nothing() {
        let mut queue = BoundedQueue::new(0);
        assert_eq!(queue.try_push(1), Err(Full));
        assert_eq!(queue.front(), None);
        assert_eq!(queue.pop_front(), None);
        assert_eq!(queue.iter().count(), 0);
        queue.clear();
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn keeps_fifo_order_across_the_end_of_the_ring() {
        let mut queue = BoundedQueue::new(3);
        for x in [1, 2, 3] {
            assert_eq!(queue.try_push(x), Ok(()));
        }
        assert_eq!(queue.pop_front(), Some(1));
        assert_eq!(queue.pop_front(), Some(2));
        // 4 and 5 wrap to the first two slots
        assert_eq!(queue.try_push(4), Ok(()));
        assert_eq!(queue.try_push(5), Ok(()));
        assert_eq!(queue.try_push(6), Err(Full));
        assert_eq!(queue.iter().copied().collect::<Vec<_>>(), [3, 4, 5]);

        assert_eq!(queue.pop_front(), Some(3));
        if let Some(front) = queue.front_mut() {
            *front = 40;
        }
        assert_eq!(queue.front(), Some(&40));
        assert_eq!(queue.iter().copied().collect::<Vec<_>>(), [40, 5]);
    }

    #[test]
    fn clearing_a_wrapped_queue_empties_it_for_reuse() {
        let mut queue = BoundedQueue::new(2);
        for x in [1, 2] {
            assert_eq!(queue.try_push(x), Ok(()));
        }
        assert_eq!(queue.pop_front(), Some(1));
        assert_eq!(queue.try_push(3), Ok(()));
        queue.clear();
        assert_eq!(queue.len(), 0);
        assert_eq!(queue.front(), None);
        assert_eq!(queue.iter().count(), 0);
        for x in [4, 5] {
            assert_eq!(queue.try_push(x), Ok(()));
        }
        assert_eq!(queue.iter().copied().collect::<Vec<_>>(), [4, 5]);
    }

    /// `BoundedQueue` against a `VecDeque` model capped at the same `max`
    ///
    /// After construction and every operation the queue must hold the model's items in order,
    /// with the same length and front, `try_push` must fail exactly when the model is full, and
    /// a slot must be occupied exactly when it is one of the `len` slots from `head`. The
    /// generator uses capacities 0 to 8 and up to 64 operations, weighted toward pushes so the
    /// queue fills and wraps, with pops, front mutation and clears between them.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the model computes ring positions"
    )]
    mod model {
        use std::collections::VecDeque;

        use proptest::prelude::*;

        use super::*;
        use crate::util::test_support::proptest_config;

        #[derive(Clone, Debug)]
        enum Op {
            Push(u8),
            Pop,
            SetFront(u8),
            Clear,
        }

        fn op() -> impl Strategy<Value = Op> {
            prop_oneof![
                4 => any::<u8>().prop_map(Op::Push),
                2 => Just(Op::Pop),
                1 => any::<u8>().prop_map(Op::SetFront),
                1 => Just(Op::Clear),
            ]
        }

        /// every slot is occupied exactly when it is one of the `len` slots from `head`
        fn check_slots(queue: &BoundedQueue<u8>) -> Result<(), TestCaseError> {
            let max = queue.slots.len();
            for (slot, item) in queue.slots.iter().enumerate() {
                let offset = (slot + max - queue.head) % max;
                prop_assert_eq!(item.is_some(), offset < queue.len, "slot {}", slot);
            }
            Ok(())
        }

        proptest! {
            #![proptest_config(proptest_config(256))]

            #[test]
            fn behaves_like_a_capped_vec_deque(
                max in 0usize..=8,
                ops in proptest::collection::vec(op(), 0..=64),
            ) {
                let mut queue = BoundedQueue::new(max);
                let mut model: VecDeque<u8> = VecDeque::new();
                prop_assert_eq!(queue.max(), max);
                check_slots(&queue)?;
                for op in ops {
                    match op {
                        Op::Push(x) => {
                            let expected = if model.len() < max {
                                model.push_back(x);
                                Ok(())
                            } else {
                                Err(Full)
                            };
                            prop_assert_eq!(queue.try_push(x), expected);
                        }
                        Op::Pop => prop_assert_eq!(queue.pop_front(), model.pop_front()),
                        Op::SetFront(x) => {
                            if let Some(front) = queue.front_mut() {
                                *front = x;
                            }
                            if let Some(front) = model.front_mut() {
                                *front = x;
                            }
                        }
                        Op::Clear => {
                            queue.clear();
                            model.clear();
                        }
                    }
                    prop_assert_eq!(queue.len(), model.len());
                    prop_assert_eq!(queue.front(), model.front());
                    prop_assert_eq!(
                        queue.iter().copied().collect::<Vec<_>>(),
                        model.iter().copied().collect::<Vec<_>>()
                    );
                    check_slots(&queue)?;
                }
            }
        }
    }
}
