// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::measurement::DoubleBit;
use crate::app::parse::traits::FixedSizeVariation;
use crate::app::variations::Variation;
use crate::app::QualifierCode;
use crate::outstation::database::details::range::traits::{
    StaticEncoder, StaticVariation, ToBit, ToDoubleBit, ToOctets,
};

use crate::app::measurement::ToVariation;
use crate::util::BadWrite;
use scursor::{WriteCursor, WriteError};

/// A value packed into the bytes of a header, `NUM_BITS` at a time
trait BitConverter: Copy {
    const NUM_BITS: u8;
    fn to_mask(&self) -> u8;

    /// the packing of a header of these values
    fn packing(state: BitState<Self>) -> Packing;

    /// the bit state of a header packed with these values, if it is
    fn state(packing: &Packing) -> Option<BitState<Self>>;
}

impl BitConverter for bool {
    const NUM_BITS: u8 = 1;

    fn to_mask(&self) -> u8 {
        u8::from(*self)
    }

    fn packing(state: BitState<Self>) -> Packing {
        Packing::Bits(state)
    }

    fn state(packing: &Packing) -> Option<BitState<Self>> {
        match packing {
            Packing::Bits(state) => Some(*state),
            Packing::Fixed | Packing::DoubleBits(_) => None,
        }
    }
}

impl BitConverter for DoubleBit {
    const NUM_BITS: u8 = 2;

    fn to_mask(&self) -> u8 {
        self.to_byte()
    }

    fn packing(state: BitState<Self>) -> Packing {
        Packing::DoubleBits(state)
    }

    fn state(packing: &Packing) -> Option<BitState<Self>> {
        match packing {
            Packing::DoubleBits(state) => Some(*state),
            Packing::Fixed | Packing::Bits(_) => None,
        }
    }
}

#[derive(Copy, Clone)]
struct BitState<T>
where
    T: BitConverter,
{
    /// the position of next item within the current byte (0 through 7)
    bit_pos: u8,
    /// accumulated value of the current byte
    acc: u8,
    /// the position of the current byte within the cursor
    byte_pos: usize,
    /// just for the typing
    _phantom: std::marker::PhantomData<T>,
}

impl<T> BitState<T>
where
    T: BitConverter,
{
    fn new(acc: u8, byte_pos: usize) -> Self {
        Self {
            bit_pos: T::NUM_BITS,
            acc,
            byte_pos,
            _phantom: std::marker::PhantomData,
        }
    }

    fn next(&self, value: T) -> Option<Self> {
        if self.bit_pos < 8 {
            Some(Self {
                bit_pos: self.bit_pos.checked_add(T::NUM_BITS)?,
                acc: self.acc | (value.to_mask() << self.bit_pos),
                byte_pos: self.byte_pos,
                _phantom: self._phantom,
            })
        } else {
            None
        }
    }
}

/// How the values of a header are laid out
#[derive(Copy, Clone)]
enum Packing {
    /// one object per value
    Fixed,
    Bits(BitState<bool>),
    DoubleBits(BitState<DoubleBit>),
}

#[derive(Copy, Clone)]
struct HeaderState {
    /// last index value written
    index: u16,
    /// immutable position of the u16 stop field within the cursor
    stop_pos: usize,
    /// how the header's values are laid out so far
    packing: Packing,
}

impl HeaderState {
    fn new(index: u16, stop_pos: usize, packing: Packing) -> HeaderState {
        Self {
            index,
            stop_pos,
            packing,
        }
    }
}

enum State {
    Start,
    Header(Variation, HeaderState),
    Full,
}

/// Writes the values of a range of points as 16-bit start and stop headers, starting a new
/// header when the variation changes or an index is skipped
pub(crate) struct RangeWriter {
    state: State,
}

impl RangeWriter {
    pub(crate) fn new() -> Self {
        Self {
            state: State::Start,
        }
    }

    /// Write `value` at `index` in `variation`, failing once a value doesn't fit
    pub(crate) fn write<T, V>(
        &mut self,
        cursor: &mut WriteCursor,
        index: u16,
        value: &T,
        variation: V,
    ) -> Result<(), BadWrite>
    where
        V: StaticVariation<T>,
    {
        let next = variation.encode(Value {
            writer: self,
            cursor,
            index,
            value,
        });
        match next {
            Ok(state) => {
                self.state = state;
                Ok(())
            }
            Err(_) => {
                self.state = State::Full;
                Err(BadWrite)
            }
        }
    }

    /// The header to continue for `index` in `variation`, or `None` to start one
    fn current(&self, variation: Variation, index: u16) -> Result<Option<HeaderState>, BadWrite> {
        match &self.state {
            State::Full => Err(BadWrite),
            State::Start => Ok(None),
            State::Header(current, header) => {
                if *current == variation && is_consecutive(header.index, index) {
                    Ok(Some(*header))
                } else {
                    Ok(None)
                }
            }
        }
    }

    /// write one object, by `write`, at `index`
    fn write_object(
        &self,
        cursor: &mut WriteCursor,
        index: u16,
        variation: Variation,
        write: impl FnOnce(&mut WriteCursor) -> Result<(), WriteError>,
    ) -> Result<State, BadWrite> {
        let header = match self.current(variation, index)? {
            Some(header) if matches!(header.packing, Packing::Fixed) => {
                cursor.transaction(|cur| {
                    write(cur)?;
                    // update the stop field
                    cur.at_pos(header.stop_pos, |cur| cur.write_u16_le(index))?;
                    Ok(HeaderState::new(index, header.stop_pos, Packing::Fixed))
                })?
            }
            _ => cursor.transaction(|cur| {
                let stop_pos = write_header(cur, variation, index)?;
                write(cur)?;
                Ok(HeaderState::new(index, stop_pos, Packing::Fixed))
            })?,
        };
        Ok(State::Header(variation, header))
    }

    /// pack `value` at `index` into the current byte, or a new one
    fn write_packed<B>(
        &self,
        cursor: &mut WriteCursor,
        index: u16,
        variation: Variation,
        value: B,
    ) -> Result<State, BadWrite>
    where
        B: BitConverter,
    {
        let continued = self
            .current(variation, index)?
            .and_then(|header| B::state(&header.packing).map(|state| (header, state)));
        let header = match continued {
            Some((header, state)) => cursor.transaction(|cur| {
                let state = match state.next(value) {
                    Some(state) => {
                        cur.at_pos(state.byte_pos, |c| c.write_u8(state.acc))?;
                        state
                    }
                    None => {
                        let pos = cur.position();
                        let acc = value.to_mask();
                        cur.write_u8(acc)?;
                        BitState::new(acc, pos)
                    }
                };
                // update the stop field
                cur.at_pos(header.stop_pos, |cur| cur.write_u16_le(index))?;
                Ok(HeaderState::new(index, header.stop_pos, B::packing(state)))
            })?,
            None => cursor.transaction(|cur| {
                let stop_pos = write_header(cur, variation, index)?;
                let pos = cur.position();
                let acc = value.to_mask();
                cur.write_u8(acc)?;
                Ok(HeaderState::new(
                    index,
                    stop_pos,
                    B::packing(BitState::new(acc, pos)),
                ))
            })?,
        };
        Ok(State::Header(variation, header))
    }
}

/// Writes one value through the encoding its variation chooses
struct Value<'a, 'b, 'c, T> {
    writer: &'a RangeWriter,
    cursor: &'a mut WriteCursor<'b>,
    index: u16,
    value: &'c T,
}

impl<T> StaticEncoder<T> for Value<'_, '_, '_, T> {
    type Output = Result<State, BadWrite>;

    fn fixed<V>(self) -> Self::Output
    where
        V: FixedSizeVariation,
        T: ToVariation<V>,
    {
        let value = self.value;
        self.writer
            .write_object(self.cursor, self.index, V::VARIATION, |cur| {
                value.to_variation().write(cur)
            })
    }

    fn bits(self, variation: Variation) -> Self::Output
    where
        T: ToBit,
    {
        self.writer
            .write_packed(self.cursor, self.index, variation, self.value.to_bit())
    }

    fn double_bits(self, variation: Variation) -> Self::Output
    where
        T: ToDoubleBit,
    {
        self.writer.write_packed(
            self.cursor,
            self.index,
            variation,
            self.value.to_double_bit(),
        )
    }

    fn octet_string(self) -> Self::Output
    where
        T: ToOctets,
    {
        let bytes = self.value.to_octets();
        let len = u8::try_from(bytes.len()).map_err(|_| BadWrite)?;
        self.writer
            .write_object(self.cursor, self.index, Variation::Group110(len), |cur| {
                cur.write_bytes(bytes)
            })
    }
}

fn is_consecutive(last: u16, next: u16) -> bool {
    last.checked_add(1) == Some(next)
}

fn write_header(
    cursor: &mut WriteCursor,
    variation: Variation,
    start: u16,
) -> Result<usize, WriteError> {
    variation.write(cursor)?;
    QualifierCode::Range16.write(cursor)?;
    cursor.write_u16_le(start)?;
    let stop_pos = cursor.position();
    cursor.write_u16_le(start)?;
    Ok(stop_pos)
}

#[cfg(test)]
mod tests {
    use crate::app::measurement::*;
    use crate::outstation::database::variations::*;

    use super::*;

    fn binary(value: bool) -> BinaryInput {
        BinaryInput::new(value, Flags::ONLINE, Time::synchronized(0))
    }

    fn double_bit(bit: DoubleBit) -> DoubleBitBinaryInput {
        DoubleBitBinaryInput::new(bit, Flags::ONLINE, Time::synchronized(0))
    }

    #[test]
    fn can_write_two_bytes_of_g1v1() {
        let mut buffer = [0u8; 64];
        let mut cursor = WriteCursor::new(buffer.as_mut());

        let mut writer = RangeWriter::new();

        let g1v1 = StaticBinaryInputVariation::Group1Var1;

        fn is_odd(index: u16) -> bool {
            index % 2 == 1
        }

        for index in 1..=9 {
            let value = binary(is_odd(index));
            writer.write(&mut cursor, index, &value, g1v1).unwrap();
        }

        assert_eq!(
            cursor.written(),
            [
                // g1v1 - 16-bit start/stop
                1,
                1,
                0x01,
                // start
                1,
                0,
                // stop
                9,
                0,
                // first byte
                0b0101_0101,
                // second byte
                0b000_0001,
            ]
        )
    }

    #[test]
    fn can_write_two_bytes_of_g3v1() {
        let mut buffer = [0u8; 64];
        let mut cursor = WriteCursor::new(buffer.as_mut());
        let mut writer = RangeWriter::new();

        fn db_modulo(i: u16) -> DoubleBit {
            match i % 4 {
                0 => DoubleBit::Intermediate,
                1 => DoubleBit::DeterminedOff,
                2 => DoubleBit::DeterminedOn,
                _ => DoubleBit::Indeterminate,
            }
        }

        let g3v1 = StaticDoubleBitBinaryInputVariation::Group3Var1;

        for index in 1..=5 {
            let value = double_bit(db_modulo(index));
            writer.write(&mut cursor, index, &value, g3v1).unwrap();
        }

        assert_eq!(
            cursor.written(),
            [
                // g3v1 - 16-bit start/stop
                3,
                1,
                0x01,
                // start
                1,
                0,
                // stop
                5,
                0,
                // first byte
                0b0011_1001,
                // second byte
                0b000_0001,
            ]
        )
    }

    #[test]
    fn can_write_three_bytes_of_g3v1() {
        let mut buffer = [0u8; 64];
        let mut cursor = WriteCursor::new(buffer.as_mut());
        let mut writer = RangeWriter::new();

        fn db_modulo(i: u16) -> DoubleBit {
            match i % 4 {
                0 => DoubleBit::Intermediate,
                1 => DoubleBit::DeterminedOff,
                2 => DoubleBit::DeterminedOn,
                _ => DoubleBit::Indeterminate,
            }
        }

        let g3v1 = StaticDoubleBitBinaryInputVariation::Group3Var1;

        for index in 1..=9 {
            let value = double_bit(db_modulo(index));
            writer.write(&mut cursor, index, &value, g3v1).unwrap();
        }

        assert_eq!(
            cursor.written(),
            [
                // g3v1 - 16-bit start/stop
                3,
                1,
                0x01,
                // start
                1,
                0,
                // stop
                9,
                0,
                // first byte
                0b0011_1001,
                // second byte
                0b0011_1001,
                // third byte
                0b0000_0001,
            ]
        )
    }

    #[test]
    #[rustfmt::skip]
    fn switches_headers_with_same_index() {
        let mut buffer = [0u8; 64];
        let mut cursor = WriteCursor::new(buffer.as_mut());
        let mut writer = RangeWriter::new();

        let g1v2 = StaticBinaryInputVariation::Group1Var2;

        writer.write(&mut cursor, 2, &binary(true), g1v2).unwrap();
        writer.write(&mut cursor, 2, &binary(true), g1v2).unwrap();


        assert_eq!(
            cursor.written(),
            [
                // g1v2 - 16-bit start/stop
                1, 2, 0x01, 2, 0, 2, 0, 0x81,
                // same header
                1, 2, 0x01, 2, 0, 2, 0, 0x81,
            ]
        )
    }

    #[test]
    #[rustfmt::skip]
    fn index_zero_does_not_follow_the_last_index() {
        let mut buffer = [0u8; 64];
        let mut cursor = WriteCursor::new(buffer.as_mut());

        let g1v2 = StaticBinaryInputVariation::Group1Var2;
        let mut writer = RangeWriter::new();
        writer.write(&mut cursor, 65535, &binary(true), g1v2).unwrap();
        writer.write(&mut cursor, 0, &binary(true), g1v2).unwrap();

        let g1v1 = StaticBinaryInputVariation::Group1Var1;
        let mut writer = RangeWriter::new();
        writer.write(&mut cursor, 65535, &binary(true), g1v1).unwrap();
        writer.write(&mut cursor, 0, &binary(true), g1v1).unwrap();

        assert_eq!(
            cursor.written(),
            [
                // g1v2 at 65535, then a new header at 0
                1, 2, 0x01, 255, 255, 255, 255, 0x81,
                1, 2, 0x01, 0, 0, 0, 0, 0x81,
                // g1v1 the same
                1, 1, 0x01, 255, 255, 255, 255, 0x01,
                1, 1, 0x01, 0, 0, 0, 0, 0x01,
            ]
        )
    }

    #[test]
    #[rustfmt::skip]
    fn switches_headers_with_non_consecutive_indices() {
        let mut buffer = [0u8; 64];
        let mut cursor = WriteCursor::new(buffer.as_mut());
        let mut writer = RangeWriter::new();

        let g1v2 = StaticBinaryInputVariation::Group1Var2;

        writer.write(&mut cursor, 1, &binary(true), g1v2).unwrap();
        writer.write(&mut cursor, 3, &binary(true), g1v2).unwrap();

        assert_eq!(
            cursor.written(),
            [
                // g1v2 - 16-bit start/stop
                1, 2, 0x01, 1, 0, 1, 0, 0x81,
                // same header but with index == 3
                1, 2, 0x01, 3, 0, 3, 0, 0x81,
            ]
        )
    }
}
