use std::collections::VecDeque;

/// A FIFO queue allocated once at construction that never holds more than `max` items
pub(crate) struct BoundedQueue<T> {
    max: usize,
    items: VecDeque<T>,
}

/// The queue already holds its maximum number of items
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Full;

impl<T> BoundedQueue<T> {
    pub(crate) fn new(max: usize) -> Self {
        Self {
            max,
            items: VecDeque::with_capacity(max),
        }
    }

    pub(crate) fn max(&self) -> usize {
        self.max
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.items.len()
    }

    pub(crate) fn try_push(&mut self, item: T) -> Result<(), Full> {
        if self.items.len() < self.max {
            self.items.push_back(item);
            Ok(())
        } else {
            Err(Full)
        }
    }

    pub(crate) fn front(&self) -> Option<&T> {
        self.items.front()
    }

    pub(crate) fn front_mut(&mut self) -> Option<&mut T> {
        self.items.front_mut()
    }

    pub(crate) fn pop_front(&mut self) -> Option<T> {
        self.items.pop_front()
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter()
    }

    pub(crate) fn clear(&mut self) {
        self.items.clear();
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
    }
}
