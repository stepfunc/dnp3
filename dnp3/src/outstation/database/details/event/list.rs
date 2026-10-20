use std::collections::VecDeque;

#[derive(Copy, Clone)]
struct MetaData {
    version: u64,
    prev: Option<usize>,
    next: Option<usize>,
}

impl MetaData {
    fn new(version: u64, prev: Option<usize>, next: Option<usize>) -> Self {
        Self {
            version,
            prev,
            next,
        }
    }

    fn create_index(&self, index: usize) -> Index {
        Index {
            version: self.version,
            value: index,
        }
    }
}

struct Entry<T> {
    data: T,
    is_free: bool,
    metadata: MetaData,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) struct Index {
    version: u64,
    value: usize,
}

impl Index {
    fn new(version: u64, value: usize) -> Self {
        Self { version, value }
    }
}

impl<T> Entry<T> {
    fn last(version: u64, data: T, prev: Option<usize>) -> Self {
        Self {
            data,
            is_free: false,
            metadata: MetaData::new(version, prev, None),
        }
    }

    fn first(version: u64, data: T) -> Self {
        Self {
            data,
            is_free: false,
            metadata: MetaData::new(version, None, None),
        }
    }

    fn create_index(&self, idx: usize) -> Index {
        Index::new(self.metadata.version, idx)
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

    fn append(&self, new_tail: usize) -> State {
        Self {
            head: self.head,
            tail: new_tail,
            size: self.size + 1,
        }
    }

    fn single(index: usize) -> State {
        State::new(index, index, 1)
    }
}

pub(crate) struct VecList<T> {
    // the maximum number of entries, which `storage.capacity()` may exceed
    capacity: usize,
    version: u64,
    storage: Vec<Entry<T>>,
    free_stack: VecDeque<usize>,
    state: Option<State>,
}

pub(crate) struct ListIterator<'a, T> {
    list: &'a VecList<T>,
    current: Option<usize>,
}

impl<'a, T> Iterator for ListIterator<'a, T> {
    type Item = (Index, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.current?;
        let entry = &self.list.storage[idx];
        self.current = entry.metadata.next;
        Some((entry.metadata.create_index(idx), &entry.data))
    }
}

impl<T> VecList<T> {
    pub(crate) fn iter(&self) -> ListIterator<'_, T> {
        ListIterator {
            list: self,
            current: self.state.map(|x| x.head),
        }
    }

    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            capacity,
            version: 0,
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

    pub(crate) fn add(&mut self, item: T) -> Option<Index> {
        if self.is_full() {
            return None;
        }

        // do we have an entry already?
        let index: Index = match self.state {
            Some(current) => {
                if let Some(idx) = self.free_stack.pop_front() {
                    self.storage[idx] = Entry::last(self.version, item, Some(current.tail));

                    // update the previous tail
                    self.storage[current.tail].metadata.next = Some(idx);

                    self.state = Some(current.append(idx));
                    Index::new(self.version, idx)
                } else {
                    let idx = self.storage.len();
                    self.storage
                        .push(Entry::last(self.version, item, Some(current.tail)));

                    // update the previous tail
                    self.storage[current.tail].metadata.next = Some(idx);

                    // set the new tail
                    self.state = Some(current.append(idx));
                    Index::new(self.version, idx)
                }
            }
            None => {
                if let Some(idx) = self.free_stack.pop_front() {
                    self.storage[idx] = Entry::first(self.version, item);
                    self.state = Some(State::single(idx));
                    Index::new(self.version, idx)
                } else {
                    let idx = self.storage.len();
                    self.storage.push(Entry::first(self.version, item));
                    self.state = Some(State::single(idx));
                    Index::new(self.version, idx)
                }
            }
        };

        self.version = self.version.wrapping_add(1);
        Some(index)
    }

    pub(crate) fn remove_first<F>(&mut self, predicate: F) -> Option<&T>
    where
        F: Fn(&T) -> bool,
    {
        let index = self.find_first(&predicate)?;

        if self.remove_at(index) {
            return self.storage.get(index.value).map(|x| &x.data);
        }

        None
    }

    pub(crate) fn remove_all<F>(&mut self, mut predicate: F) -> usize
    where
        F: FnMut(&T) -> bool,
    {
        let mut count = 0;
        let mut current = match self.state {
            None => return count,
            Some(x) => x.head,
        };

        loop {
            let (next, entry_to_remove) = {
                let entry = &self.storage[current];
                let entry_to_delete = match predicate(&entry.data) {
                    true => Some(entry.create_index(current)),
                    false => None,
                };
                (entry.metadata.next, entry_to_delete)
            };

            if let Some(index) = entry_to_remove {
                self.remove_at(index);
                count += 1;
            }

            match next {
                Some(next) => {
                    current = next;
                }
                None => return count,
            }
        }
    }

    fn find_first<F>(&self, predicate: &F) -> Option<Index>
    where
        F: Fn(&T) -> bool,
    {
        self.state
            .and_then(|x| self.find_first_from(x.head, predicate))
    }

    fn find_first_from<F>(&self, start: usize, predicate: &F) -> Option<Index>
    where
        F: Fn(&T) -> bool,
    {
        let mut current = start;

        loop {
            let entry = &self.storage[current];
            if predicate(&entry.data) {
                return Some(entry.create_index(current));
            }

            current = entry.metadata.next?;
        }
    }

    pub(crate) fn remove_at(&mut self, index: Index) -> bool {
        let current = match self.state {
            None => return false,
            Some(x) => x,
        };

        let metadata = match self.storage.get_mut(index.value) {
            None => return false,
            Some(x) => {
                if x.is_free {
                    // already free
                    return false;
                }
                if x.metadata.version != index.version {
                    // bad version
                    return false;
                }
                x.is_free = true;
                x.metadata
            }
        };

        // return this index to the free list
        self.free_stack.push_back(index.value);

        if let Some(prev) = metadata.prev {
            self.storage[prev].metadata.next = metadata.next;
        }

        if let Some(next) = metadata.next {
            self.storage[next].metadata.prev = metadata.prev;
        }

        // the removed entry's neighbors determine the new ends
        self.state = match (metadata.prev, metadata.next) {
            // it was the only entry
            (None, None) => None,
            // it was the tail
            (Some(prev), None) => Some(State::new(current.head, prev, current.size - 1)),
            // it was the head
            (None, Some(next)) => Some(State::new(next, current.tail, current.size - 1)),
            // it was in the middle
            (Some(_), Some(_)) => Some(State::new(current.head, current.tail, current.size - 1)),
        };
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cannot_add_entries_past_capacity() {
        let mut list = VecList::new(2);
        assert_eq!(list.add("hello"), Some(Index::new(0, 0)));
        assert_eq!(list.add("world"), Some(Index::new(1, 1)));
        assert_eq!(list.len(), 2);
        assert_eq!(list.add("NOPE"), None);
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn can_remove_first_entry_matching_predicate() {
        let mut list = VecList::new(2);

        // run this a few times to ensure the data structure
        // isn't put into a bad state
        for _ in 0..3 {
            assert_eq!(list.add("hello").map(|x| x.value), Some(0));
            assert_eq!(list.add("world").map(|x| x.value), Some(1));
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
        assert_eq!(list.add("hello"), Some(Index::new(0, 0)));
        assert_eq!(list.add("my"), Some(Index::new(1, 1)));
        assert_eq!(list.add("friends"), Some(Index::new(2, 2)));

        assert!(list.remove_at(Index::new(1, 1)));
        assert_eq!(list.add("yolo"), Some(Index::new(3, 1)));
    }

    #[test]
    fn can_remove_with_bad_version() {
        let mut list = VecList::new(3);
        let index = list.add("hello").unwrap();
        assert!(!list.remove_at(Index::new(index.version + 1, index.value)));
    }

    mod model {
        use super::*;

        use proptest::prelude::*;

        #[derive(Debug, Clone)]
        enum Op {
            Add(u8),
            // remove the live entry at this position, modulo the number of live entries
            RemoveLive(usize),
            // remove an entry that was already removed, modulo the number of removed entries
            RemoveStale(usize),
            // remove an index past the end of storage: this offset past it, or usize::MAX for None
            RemoveOutOfRange(Option<usize>),
            RemoveFirst(u8),
            RemoveAll(u8),
        }

        // a small range of values, so that predicates match more than one entry
        fn value() -> impl Strategy<Value = u8> {
            0u8..4
        }

        fn op() -> impl Strategy<Value = Op> {
            prop_oneof![
                3 => value().prop_map(Op::Add),
                2 => any::<usize>().prop_map(Op::RemoveLive),
                1 => any::<usize>().prop_map(Op::RemoveStale),
                1 => prop::option::of(0usize..4).prop_map(Op::RemoveOutOfRange),
                1 => value().prop_map(Op::RemoveFirst),
                1 => value().prop_map(Op::RemoveAll),
            ]
        }

        // the list must behave like a bounded queue of (index, value) pairs in insertion order
        struct Model {
            capacity: usize,
            live: VecDeque<(Index, u8)>,
            removed: Vec<Index>,
        }

        impl Model {
            fn apply(&mut self, list: &mut VecList<u8>, op: Op) {
                match op {
                    Op::Add(value) => {
                        let result = list.add(value);
                        if self.live.len() == self.capacity {
                            assert_eq!(result, None);
                        } else {
                            let index = result.expect("add failed below capacity");
                            assert!(self.live.iter().all(|(live, _)| *live != index));
                            self.live.push_back((index, value));
                        }
                    }
                    Op::RemoveLive(position) => {
                        if self.live.is_empty() {
                            return;
                        }
                        let (index, _) = self
                            .live
                            .remove(position % self.live.len())
                            .expect("position is in range");
                        assert!(list.remove_at(index));
                        self.removed.push(index);
                    }
                    Op::RemoveStale(position) => {
                        if self.removed.is_empty() {
                            return;
                        }
                        let index = self.removed[position % self.removed.len()];
                        assert!(!list.remove_at(index));
                    }
                    Op::RemoveOutOfRange(offset) => {
                        let value = match offset {
                            Some(offset) => list.storage.len() + offset,
                            None => usize::MAX,
                        };
                        assert!(!list.remove_at(Index::new(list.version, value)));
                    }
                    Op::RemoveFirst(value) => {
                        let result = list.remove_first(|x| *x == value).copied();
                        match self.live.iter().position(|(_, x)| *x == value) {
                            Some(position) => {
                                let (index, _) =
                                    self.live.remove(position).expect("position is in range");
                                assert_eq!(result, Some(value));
                                self.removed.push(index);
                            }
                            None => assert_eq!(result, None),
                        }
                    }
                    Op::RemoveAll(value) => {
                        let count = list.remove_all(|x| *x == value);
                        let before = self.live.len();
                        let removed = &mut self.removed;
                        self.live.retain(|(index, x)| {
                            let keep = *x != value;
                            if !keep {
                                removed.push(*index);
                            }
                            keep
                        });
                        assert_eq!(count, before - self.live.len());
                    }
                }
            }

            fn check(&self, list: &VecList<u8>) {
                assert_eq!(list.len(), self.live.len());
                assert_eq!(list.is_full(), self.live.len() == self.capacity);
                // bounded, so that a cycle in the links fails here instead of hanging
                let entries: Vec<(Index, u8)> = list
                    .iter()
                    .take(self.live.len() + 1)
                    .map(|(i, x)| (i, *x))
                    .collect();
                assert_eq!(entries, self.live.iter().copied().collect::<Vec<_>>());
                // the stored ends and size, which iteration alone doesn't fully check
                let expected = match (self.live.front(), self.live.back()) {
                    (Some((head, _)), Some((tail, _))) => {
                        Some((head.value, tail.value, self.live.len()))
                    }
                    _ => None,
                };
                assert_eq!(list.state.map(|s| (s.head, s.tail, s.size)), expected);
            }
        }

        proptest! {
            #[test]
            fn behaves_like_a_bounded_queue(
                capacity in 0usize..8,
                near_version_wraparound in any::<bool>(),
                ops in prop::collection::vec(op(), 0..64),
            ) {
                let mut list = VecList::new(capacity);
                if near_version_wraparound {
                    list.version = u64::MAX - 4;
                }
                let mut model = Model {
                    capacity,
                    live: VecDeque::new(),
                    removed: Vec::new(),
                };
                model.check(&list);
                for op in ops {
                    model.apply(&mut list, op);
                    model.check(&list);
                }
            }
        }
    }

    #[test]
    fn can_iterate_over_values() {
        let mut list = VecList::new(3);

        let index_a = list.add("A").unwrap();
        let index_b = list.add("B").unwrap();
        let index_c = list.add("C").unwrap();

        let mut iter = list.iter();
        assert_eq!(iter.next(), Some((index_a, &"A")));
        assert_eq!(iter.next(), Some((index_b, &"B")));
        assert_eq!(iter.next(), Some((index_c, &"C")));
        assert_eq!(iter.next(), None);
    }
}
