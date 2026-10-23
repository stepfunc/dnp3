// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

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

    pub(crate) fn write_cursor(&mut self) -> WriteCursor<'_> {
        WriteCursor::new(self.inner.as_mut())
    }

    #[cfg(test)]
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
