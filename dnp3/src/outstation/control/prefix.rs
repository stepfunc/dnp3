use crate::app::parse::traits::{FixedSizeVariation, Index};

use scursor::{WriteCursor, WriteError};

#[derive(Copy, Clone)]
struct State<I>
where
    I: Index,
{
    count: I,
    count_pos: usize,
}

pub(crate) struct PrefixWriter<I, V>
where
    I: Index,
    V: FixedSizeVariation,
{
    state: Option<State<I>>,
    _phantom: std::marker::PhantomData<V>,
}

impl<I, V> PrefixWriter<I, V>
where
    I: Index,
    V: FixedSizeVariation,
{
    pub(crate) fn new() -> Self {
        Self {
            state: None,
            _phantom: std::marker::PhantomData,
        }
    }

    pub(crate) fn write(
        &mut self,
        cursor: &mut WriteCursor,
        value: V,
        index: I,
    ) -> Result<(), WriteError> {
        let next_state = cursor.transaction(|cur| self.write_inner(cur, value, index))?;
        self.state = Some(next_state);
        Ok(())
    }

    fn write_inner(
        &self,
        cursor: &mut WriteCursor,
        value: V,
        index: I,
    ) -> Result<State<I>, WriteError> {
        // if this is successful, what will the next state be?
        let next_state: State<I> = match self.state {
            // need to write the header
            None => {
                V::VARIATION.write(cursor)?;
                I::COUNT_AND_PREFIX_QUALIFIER.write(cursor)?;
                let count_pos = cursor.position();
                cursor.skip(I::SIZE as usize)?;
                State {
                    count: I::one(),
                    count_pos,
                }
            }
            Some(s) => State {
                count: s.count.next(),
                count_pos: s.count_pos,
            },
        };

        index.write(cursor)?;
        value.write(cursor)?;
        cursor.at_pos(next_state.count_pos, |cur| next_state.count.write(cur))?;

        Ok(next_state)
    }
}
