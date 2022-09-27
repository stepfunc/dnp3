use scursor::WriteCursor;

pub(crate) struct Buffer {
    inner: Box<[u8]>,
}

impl Buffer {
    pub(crate) fn new(size: usize) -> Self {
        Self {
            inner: vec![0; size].into_boxed_slice(),
        }
    }

    pub(crate) fn write_cursor(&mut self) -> WriteCursor {
        WriteCursor::new(self.inner.as_mut())
    }

    pub(crate) fn get(&self, length: usize) -> Option<&[u8]> {
        self.inner.get(0..length)
    }

    #[cfg(test)]
    pub(crate) fn get_mut(&mut self, length: usize) -> Option<&mut [u8]> {
        self.inner.get_mut(0..length)
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.inner.len()
    }
}
