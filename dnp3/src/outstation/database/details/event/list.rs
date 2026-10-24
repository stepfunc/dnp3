// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use std::collections::VecDeque;
use std::ops::ControlFlow;

#[derive(Copy, Clone)]
struct MetaData {
    prev: Option<usize>,
    next: Option<usize>,
}

struct Entry<T> {
    data: T,
    metadata: MetaData,
}

impl<T> Entry<T> {
    /// an entry at the end of the list, after `prev`
    fn last(data: T, prev: Option<usize>) -> Self {
        Self {
            data,
            metadata: MetaData { prev, next: None },
        }
    }
}

#[derive(Copy, Clone)]
struct State {
    head: usize,
    tail: usize,
    size: usize,
}

impl State {
    fn new(head: usize, tail: usize, size: usize) -> State {
        State { head, tail, size }
    }

    fn append(&self, new_tail: usize) -> Option<State> {
        Some(Self {
            head: self.head,
            tail: new_tail,
            size: self.size.checked_add(1)?,
        })
    }

    fn single(index: usize) -> State {
        State::new(index, index, 1)
    }
}

pub(crate) struct VecList<T> {
    // the maximum number of entries, which `storage.capacity()` may exceed
    capacity: usize,
    storage: Vec<Entry<T>>,
    free_stack: VecDeque<usize>,
    state: Option<State>,
}

impl<T> VecList<T> {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            capacity,
            storage: Vec::with_capacity(capacity),
            free_stack: VecDeque::with_capacity(capacity),
            state: None,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.state.map_or(0, |x| x.size)
    }

    pub(crate) fn is_full(&self) -> bool {
        self.len() == self.capacity
    }

    /// append an entry, returning false if the list is full
    #[must_use]
    pub(crate) fn add(&mut self, item: T) -> bool {
        if self.is_full() {
            return false;
        }

        // reuse a free slot, or add one
        let reused = self.free_stack.front().copied();
        let idx = reused.unwrap_or(self.storage.len());

        let state = match self.state {
            Some(current) => current.append(idx),
            None => Some(State::single(idx)),
        };
        let Some(state) = state else {
            return false;
        };
        let entry = Entry::last(item, self.state.map(|x| x.tail));

        match reused {
            Some(idx) => {
                let Some(slot) = self.storage.get_mut(idx) else {
                    return false;
                };
                *slot = entry;
                self.free_stack.pop_front();
            }
            None => self.storage.push(entry),
        }

        // link the previous tail to the new one
        if let Some(tail) = self.state.and_then(|x| self.storage.get_mut(x.tail)) {
            tail.metadata.next = Some(idx);
        }
        self.state = Some(state);
        true
    }

    /// remove the first entry matching the predicate, returning its data, which stays in the free
    /// slot until reused
    pub(crate) fn remove_first<F>(&mut self, predicate: F) -> Option<&T>
    where
        F: Fn(&T) -> bool,
    {
        let mut current = self.state.map(|x| x.head);
        while let Some(idx) = current {
            let entry = self.storage.get(idx)?;
            let metadata = entry.metadata;
            if predicate(&entry.data) {
                if !self.unlink(idx, metadata) {
                    return None;
                }
                return self.storage.get(idx).map(|x| &x.data);
            }
            current = metadata.next;
        }
        None
    }

    /// remove every entry matching the predicate, passing each one to `removed` once it's removed
    ///
    /// Each entry is unlinked where the walk finds it, before `removed` sees it, so a panic in
    /// either closure leaves the list consistent with the entries removed so far.
    pub(crate) fn remove_all<F, R>(&mut self, mut predicate: F, mut removed: R) -> usize
    where
        F: FnMut(&T) -> bool,
        R: FnMut(&T),
    {
        let mut count: usize = 0;
        let mut current = self.state.map(|x| x.head);
        while let Some(idx) = current {
            let Some(entry) = self.storage.get(idx) else {
                break;
            };
            let metadata = entry.metadata;
            current = metadata.next;

            if !predicate(&entry.data) {
                continue;
            }
            if !self.unlink(idx, metadata) {
                break;
            }
            if let Some(entry) = self.storage.get(idx) {
                removed(&entry.data);
            }
            count = count.saturating_add(1);
        }
        count
    }

    /// visit the entries in order, mutably, until the visitor breaks
    pub(crate) fn for_each_mut<F>(&mut self, mut visit: F)
    where
        F: FnMut(&mut T) -> ControlFlow<()>,
    {
        let mut current = self.state.map(|x| x.head);
        while let Some(entry) = current.and_then(|idx| self.storage.get_mut(idx)) {
            if visit(&mut entry.data).is_break() {
                return;
            }
            current = entry.metadata.next;
        }
    }

    /// unlink the in-use entry at `idx`, whose links are `metadata`, and free its slot
    fn unlink(&mut self, idx: usize, metadata: MetaData) -> bool {
        let Some(current) = self.state else {
            return false;
        };

        // the removed entry's neighbors determine the new ends
        let Some(size) = current.size.checked_sub(1) else {
            return false;
        };
        let state = match (metadata.prev, metadata.next) {
            // it was the only entry
            (None, None) => None,
            // it was the tail
            (Some(prev), None) => Some(State::new(current.head, prev, size)),
            // it was the head
            (None, Some(next)) => Some(State::new(next, current.tail, size)),
            // it was in the middle
            (Some(_), Some(_)) => Some(State::new(current.head, current.tail, size)),
        };

        // return this index to the free list
        self.free_stack.push_back(idx);

        if let Some(prev) = metadata.prev.and_then(|i| self.storage.get_mut(i)) {
            prev.metadata.next = metadata.next;
        }

        if let Some(next) = metadata.next.and_then(|i| self.storage.get_mut(i)) {
            next.metadata.prev = metadata.prev;
        }

        self.state = state;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ListIterator<'a, T> {
        list: &'a VecList<T>,
        current: Option<usize>,
    }

    impl<'a, T> Iterator for ListIterator<'a, T> {
        // the slot of each entry and its data
        type Item = (usize, &'a T);

        fn next(&mut self) -> Option<Self::Item> {
            let idx = self.current?;
            let entry = self.list.storage.get(idx)?;
            self.current = entry.metadata.next;
            Some((idx, &entry.data))
        }
    }

    impl<T> VecList<T> {
        fn iter(&self) -> ListIterator<'_, T> {
            ListIterator {
                list: self,
                current: self.state.map(|x| x.head),
            }
        }
    }

    #[test]
    fn cannot_add_entries_past_capacity() {
        let mut list = VecList::new(2);
        assert!(list.add("hello"));
        assert!(list.add("world"));
        assert_eq!(list.len(), 2);
        assert!(!list.add("NOPE"));
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn can_remove_first_entry_matching_predicate() {
        let mut list = VecList::new(2);

        // run this a few times to ensure the data structure
        // isn't put into a bad state
        for _ in 0..3 {
            assert!(list.add("hello"));
            assert!(list.add("world"));
            assert_eq!(list.remove_first(|x| x == &"test"), None);
            assert_eq!(list.len(), 2);
            assert_eq!(list.remove_first(|x| x == &"hello"), Some(&"hello"));
            assert_eq!(list.len(), 1);
            assert_eq!(list.remove_first(|x| x == &"world"), Some(&"world"));
            assert_eq!(list.len(), 0);
        }
    }

    #[test]
    fn can_add_after_remove() {
        let mut list = VecList::new(3);
        assert!(list.add("hello"));
        assert!(list.add("my"));
        assert!(list.add("friends"));

        assert_eq!(list.remove_first(|x| x == &"my"), Some(&"my"));
        assert!(list.add("yolo"));
        // the new entry takes the freed slot, at the end of the list
        assert_eq!(
            list.iter().map(|(slot, x)| (slot, *x)).collect::<Vec<_>>(),
            [(0, "hello"), (2, "friends"), (1, "yolo")]
        );
    }

    /// `VecList` against a queue model
    ///
    /// After every generated op (add, remove_first, remove_all by value or by position), the list
    /// must hold the model's items in insertion order, forward and backward through the same
    /// slots, with the model's ends, size and fullness. `remove_all` must call its closures in
    /// walk order, unlinking each entry before `removed` sees it.
    ///
    /// The model is independent of the list. It looks inside only to check that the storage never
    /// grows past the capacity and that each slot is either live or free, never both; which free
    /// slot is reused first is the list's choice.
    ///
    /// The generator uses capacities 0..8, up to 64 ops, values from a small range so that
    /// predicates match several entries, and position masks for remove_all. It is built to reach
    /// a full list (and capacity 0), slot reuse, remove_first at the head and further in or with
    /// no match, and remove_all of none, all, adjacent, interior and both-end entries. Not
    /// generated: panicking closures (the two examples below) and `for_each_mut` (its example
    /// below, and the event buffer property).
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the model picks entries and counts them"
    )]
    mod model {
        use super::*;

        use std::cell::RefCell;
        use std::panic::{catch_unwind, AssertUnwindSafe};

        use proptest::prelude::*;

        // every entry is distinct, with a value from a small range for predicates to match
        #[derive(Copy, Clone, Debug, PartialEq)]
        struct Item {
            id: u32,
            value: u8,
        }

        // a call from remove_all into one of its closures
        #[derive(Copy, Clone, Debug, PartialEq)]
        enum Call {
            Predicate(u32),
            Removed(u32),
        }

        #[derive(Debug, Clone)]
        enum Op {
            Add(u8),
            RemoveFirst(u8),
            RemoveAll(u8),
            // remove_all of the live entries at the positions whose bits are set
            RemovePositions(u8),
        }

        // a small range of values, so that predicates match more than one entry
        fn value() -> impl Strategy<Value = u8> {
            0u8..4
        }

        fn op() -> impl Strategy<Value = Op> {
            prop_oneof![
                3 => value().prop_map(Op::Add),
                2 => value().prop_map(Op::RemoveFirst),
                1 => value().prop_map(Op::RemoveAll),
                2 => any::<u8>().prop_map(Op::RemovePositions),
            ]
        }

        fn at_positions(mask: u8) -> impl Fn(usize, &Item) -> bool {
            move |position, _| position < 8 && mask & (1 << position) != 0
        }

        // the list must behave like a bounded queue of items in insertion order
        struct Model {
            capacity: usize,
            next_id: u32,
            live: VecDeque<Item>,
        }

        impl Model {
            fn new(list: &VecList<Item>) -> Self {
                Self {
                    capacity: list.capacity,
                    next_id: 0,
                    live: VecDeque::new(),
                }
            }

            fn apply(&mut self, list: &mut VecList<Item>, op: Op) {
                match op {
                    Op::Add(value) => {
                        let item = Item {
                            id: self.next_id,
                            value,
                        };
                        self.next_id += 1;
                        let added = list.add(item);
                        let has_room = self.live.len() < self.capacity;
                        assert_eq!(added, has_room);
                        if has_room {
                            self.live.push_back(item);
                        }
                    }
                    Op::RemoveFirst(value) => {
                        let result = list.remove_first(|x| x.value == value).copied();
                        match self.live.iter().position(|x| x.value == value) {
                            Some(position) => assert_eq!(result, self.live.remove(position)),
                            None => assert_eq!(result, None),
                        }
                    }
                    Op::RemoveAll(value) => {
                        self.remove_all(list, |_, item| item.value == value, None);
                    }
                    Op::RemovePositions(mask) => {
                        self.remove_all(list, at_positions(mask), None);
                    }
                }
            }

            // remove_all of the live entries `select` picks by position, panicking in the
            // closure call `panic_at` if it's reached
            fn remove_all<S>(&mut self, list: &mut VecList<Item>, select: S, panic_at: Option<Call>)
            where
                S: Fn(usize, &Item) -> bool,
            {
                let selected: Vec<u32> = self
                    .live
                    .iter()
                    .enumerate()
                    .filter(|(position, item)| select(*position, item))
                    .map(|(_, item)| item.id)
                    .collect();

                // the calls expected in order, up to the one that panics, and the entries removed
                let mut expected = Vec::new();
                let mut removed = Vec::new();
                let mut panics = false;
                for item in self.live.iter() {
                    expected.push(Call::Predicate(item.id));
                    if panic_at == Some(Call::Predicate(item.id)) {
                        panics = true;
                        break;
                    }
                    if selected.contains(&item.id) {
                        removed.push(item.id);
                        expected.push(Call::Removed(item.id));
                        if panic_at == Some(Call::Removed(item.id)) {
                            panics = true;
                            break;
                        }
                    }
                }

                let calls = RefCell::new(Vec::new());
                let result = catch_unwind(AssertUnwindSafe(|| {
                    list.remove_all(
                        |x| {
                            calls.borrow_mut().push(Call::Predicate(x.id));
                            if panic_at == Some(Call::Predicate(x.id)) {
                                panic!("predicate panics");
                            }
                            selected.contains(&x.id)
                        },
                        |x| {
                            calls.borrow_mut().push(Call::Removed(x.id));
                            if panic_at == Some(Call::Removed(x.id)) {
                                panic!("callback panics");
                            }
                        },
                    )
                }));

                assert_eq!(calls.into_inner(), expected);
                match panics {
                    true => assert!(result.is_err()),
                    false => assert_eq!(result.ok(), Some(removed.len())),
                }

                self.live.retain(|item| !removed.contains(&item.id));
            }

            fn check(&self, list: &VecList<Item>) {
                assert_eq!(list.len(), self.live.len());
                assert_eq!(list.is_full(), self.live.len() == self.capacity);

                // forward, bounded, so that a cycle in the links fails here instead of hanging
                let forward: Vec<(usize, Item)> = list
                    .iter()
                    .take(self.live.len() + 1)
                    .map(|(slot, x)| (slot, *x))
                    .collect();
                assert_eq!(
                    forward.iter().map(|(_, x)| *x).collect::<Vec<_>>(),
                    self.live.iter().copied().collect::<Vec<_>>()
                );

                // backward from the tail, through the same slots
                let mut backward = Vec::new();
                let mut current = list.state.map(|s| s.tail);
                while let Some(idx) = current {
                    if backward.len() > self.live.len() {
                        break;
                    }
                    let entry = &list.storage[idx];
                    backward.push((idx, entry.data));
                    current = entry.metadata.prev;
                }
                backward.reverse();
                assert_eq!(backward, forward);

                // the stored ends and size
                let expected = match (forward.first(), forward.last()) {
                    (Some((head, _)), Some((tail, _))) => Some((*head, *tail, forward.len())),
                    _ => None,
                };
                assert_eq!(list.state.map(|s| (s.head, s.tail, s.size)), expected);

                // the storage never grows past the capacity, and every slot in it is either live
                // or free, never both
                assert!(list.storage.len() <= self.capacity);
                let mut slots: Vec<usize> = forward
                    .iter()
                    .map(|(slot, _)| *slot)
                    .chain(list.free_stack.iter().copied())
                    .collect();
                slots.sort_unstable();
                assert_eq!(slots, (0..list.storage.len()).collect::<Vec<_>>());
            }
        }

        // a full list of `capacity` entries
        fn full(capacity: usize) -> (VecList<Item>, Model) {
            let mut list = VecList::new(capacity);
            let mut model = Model::new(&list);
            for value in (0u8..4).cycle().take(capacity) {
                model.apply(&mut list, Op::Add(value));
                model.check(&list);
            }
            (list, model)
        }

        // add until the list is full, then once more
        fn refill(list: &mut VecList<Item>, model: &mut Model) {
            while !list.is_full() {
                model.apply(list, Op::Add(0));
                model.check(list);
            }
            model.apply(list, Op::Add(0));
            model.check(list);
        }

        const PATTERNS: [u8; 8] = [
            0b000000, // none
            0b111111, // all
            0b000011, // prefix
            0b110000, // suffix
            0b011110, // consecutive, in the middle
            0b010101, // alternating, from the head
            0b101010, // alternating, to the tail
            0b100001, // both ends
        ];

        #[test]
        fn removes_each_pattern_and_reuses_the_freed_slots() {
            for mask in PATTERNS {
                let (mut list, mut model) = full(6);
                model.apply(&mut list, Op::RemovePositions(mask));
                model.check(&list);
                refill(&mut list, &mut model);
                // and again, with the slots now out of order
                model.apply(&mut list, Op::RemovePositions(!mask));
                model.check(&list);
                refill(&mut list, &mut model);
            }
        }

        #[test]
        fn a_panicking_predicate_leaves_the_earlier_removals_complete() {
            // panic on the entry at this position, after removing the selected ones before it
            for (mask, position) in [(0b010101, 3), (0b000111, 3), (0b111111, 5), (0b111110, 0)] {
                let (mut list, mut model) = full(6);
                let id = model.live[position].id;
                model.remove_all(&mut list, at_positions(mask), Some(Call::Predicate(id)));
                model.check(&list);
                refill(&mut list, &mut model);
            }
        }

        #[test]
        fn a_panicking_callback_leaves_its_entry_removed() {
            // panic in the callback of the entry at this position, which must be removed already
            for (mask, position) in [(0b010101, 2), (0b000011, 0), (0b110000, 5), (0b111111, 3)] {
                let (mut list, mut model) = full(6);
                let id = model.live[position].id;
                model.remove_all(&mut list, at_positions(mask), Some(Call::Removed(id)));
                model.check(&list);
                refill(&mut list, &mut model);
            }
        }

        proptest! {
            #![proptest_config(crate::util::test_support::proptest_config(256))]

            #[test]
            fn behaves_like_a_bounded_queue(
                capacity in 0usize..8,
                ops in prop::collection::vec(op(), 0..64),
            ) {
                let mut list = VecList::new(capacity);
                let mut model = Model::new(&list);
                model.check(&list);
                for op in ops {
                    model.apply(&mut list, op);
                    model.check(&list);
                }
            }
        }
    }

    #[test]
    fn can_visit_values_mutably_until_break() {
        let mut list = VecList::new(3);
        assert!(list.add(1));
        assert!(list.add(2));
        assert!(list.add(3));

        let mut visited = Vec::new();
        list.for_each_mut(|x| {
            visited.push(*x);
            *x *= 10;
            if *x == 20 {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        });

        assert_eq!(visited, [1, 2]);
        let values: Vec<i32> = list.iter().map(|(_, x)| *x).collect();
        assert_eq!(values, [10, 20, 3]);
    }

    #[test]
    fn can_iterate_over_values() {
        let mut list = VecList::new(3);

        assert!(list.add("A"));
        assert!(list.add("B"));
        assert!(list.add("C"));

        let mut iter = list.iter();
        assert_eq!(iter.next(), Some((0, &"A")));
        assert_eq!(iter.next(), Some((1, &"B")));
        assert_eq!(iter.next(), Some((2, &"C")));
        assert_eq!(iter.next(), None);
    }
}
