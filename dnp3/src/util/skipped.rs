/// What a loop over the headers of one fragment skipped: the first item and how many, so that it
/// logs one line for the fragment instead of one per header
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Skipped<T> {
    first: Option<T>,
    count: usize,
}

/// How many items were skipped, displayed as nothing for one and as " (first of N)" for more
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Count(usize);

impl<T: Copy> Skipped<T> {
    pub(crate) const fn new() -> Self {
        Self {
            first: None,
            count: 0,
        }
    }

    pub(crate) fn add(&mut self, item: T) {
        self.first.get_or_insert(item);
        self.count = self.count.saturating_add(1);
    }

    /// The first item skipped and how many, or `None` if none were
    pub(crate) fn first(&self) -> Option<(T, Count)> {
        self.first.map(|first| (first, Count(self.count)))
    }
}

impl std::fmt::Display for Count {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            0 | 1 => Ok(()),
            count => write!(f, " (first of {count})"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_first_item_and_counts_them_all() {
        let mut skipped = Skipped::new();
        assert_eq!(skipped.first(), None);
        skipped.add(3);
        assert_eq!(skipped.first(), Some((3, Count(1))));
        skipped.add(4);
        skipped.add(5);
        assert_eq!(skipped.first(), Some((3, Count(3))));
    }
}
