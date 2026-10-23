// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::parse::traits::{FixedSize, FixedSizeVariation, Index};

use scursor::*;

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Prefix<I, V>
where
    I: Index,
    V: FixedSizeVariation,
{
    pub(crate) index: I,
    pub(crate) value: V,
}

impl<I, V> FixedSize for Prefix<I, V>
where
    I: Index,
    V: FixedSizeVariation,
{
    const SIZE: u8 = I::SIZE + V::SIZE;

    fn read(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(Prefix {
            index: I::read(cursor)?,
            value: V::read(cursor)?,
        })
    }

    fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        self.index.write(cursor)?;
        self.value.write(cursor)
    }
}
