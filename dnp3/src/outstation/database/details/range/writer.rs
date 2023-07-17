use crate::app::measurement::DoubleBit;
use crate::app::variations::Variation;
use crate::app::QualifierCode;
use crate::outstation::database::details::range::traits::{
    FixedWriteFn, ToBit, ToDoubleBit, WriteInfo, WriteType,
};

use crate::util::BadWrite;
use scursor::{WriteCursor, WriteError};

trait BitConverter {
    const NUM_BITS: u8;
    fn to_mask(&self) -> u8;
}

impl BitConverter for bool {
    const NUM_BITS: u8 = 1;

    fn to_mask(&self) -> u8 {
        u8::from(*self)
    }
}

impl BitConverter for DoubleBit {
    const NUM_BITS: u8 = 2;

    fn to_mask(&self) -> u8 {
        self.to_byte()
    }
}

struct BitState<T>
where
    T: BitConverter,
{
    /// the position of next item within the current byte [0,7]
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
                bit_pos: self.bit_pos + T::NUM_BITS,
                acc: self.acc | value.to_mask() << self.bit_pos,
                byte_pos: self.byte_pos,
                _phantom: self._phantom,
            })
        } else {
            None
        }
    }
}

enum TypeState<T> {
    Fixed(FixedWriteFn<T>),
    Bit(ToBit<T>, BitState<bool>),
    DoubleBit(ToDoubleBit<T>, BitState<DoubleBit>),
}

struct HeaderState<T> {
    /// last index value written
    index: u16,
    /// immutable position of the u16 stop field within the cursor
    stop_pos: usize,
    /// header type specific state
    state: TypeState<T>,
}

impl<T> HeaderState<T> {
    fn new(index: u16, stop_pos: usize, state: TypeState<T>) -> HeaderState<T> {
        Self {
            index,
            stop_pos,
            state,
        }
    }
}

//#[derive(Debug)]
enum State<T> {
    Start,
    Header(Variation, HeaderState<T>),
    Full,
}

pub(crate) struct RangeWriter<T> {
    state: State<T>,
}

impl<T> RangeWriter<T> {
    pub(crate) fn new() -> Self {
        Self {
            state: State::Start,
        }
    }

    pub(crate) fn write(
        &mut self,
        cursor: &mut WriteCursor,
        index: u16,
        value: &T,
        info: WriteInfo<T>,
    ) -> Result<(), BadWrite> {
        match self.try_write(cursor, index, value, info) {
            Ok(state) => {
                self.state = state;
                //println!("state is {:?}", self.state);
                Ok(())
            }
            Err(_) => {
                self.state = State::Full;
                Err(BadWrite)
            }
        }
    }

    fn try_write(
        &self,
        cursor: &mut WriteCursor,
        index: u16,
        value: &T,
        info: WriteInfo<T>,
    ) -> Result<State<T>, BadWrite> {
        let variation = info.variation;
        let state = match &self.state {
            State::Full => return Err(BadWrite),
            State::Start => Self::start_header(cursor, index, value, info)?,
            State::Header(variation, header) => {
                if *variation == info.variation && is_consecutive(header.index, index) {
                    Self::write_next_value(cursor, header, index, value)?
                } else {
                    Self::start_header(cursor, index, value, info)?
                }
            }
        };

        Ok(State::Header(variation, state))
    }

    fn start_header(
        cursor: &mut WriteCursor,
        index: u16,
        value: &T,
        info: WriteInfo<T>,
    ) -> Result<HeaderState<T>, BadWrite> {
        let ret = cursor.transaction(|cur| {
            let stop_pos = write_header(cur, info.variation, index)?;
            Ok(HeaderState::new(
                index,
                stop_pos,
                info.write_type.write_first_value(cur, value)?,
            ))
        })?;
        Ok(ret)
    }

    fn write_next_value(
        cursor: &mut WriteCursor,
        header: &HeaderState<T>,
        index: u16,
        value: &T,
    ) -> Result<HeaderState<T>, WriteError> {
        let ret = cursor.transaction(|cur| {
            let next_state = header.state.write_next_value(cur, value)?;
            // update the stop field
            cur.at_pos(header.stop_pos, |cur| cur.write_u16_le(index))?;
            Ok(HeaderState::new(index, header.stop_pos, next_state))
        })?;
        Ok(ret)
    }
}

impl<T> TypeState<T> {
    fn write_next_value(
        &self,
        cursor: &mut WriteCursor,
        value: &T,
    ) -> Result<TypeState<T>, WriteError> {
        let state = match self {
            TypeState::Fixed(func) => {
                (func)(cursor, value)?;
                TypeState::Fixed(*func)
            }
            TypeState::Bit(conv, bs) => {
                let bit = conv(value);
                match bs.next(bit) {
                    Some(bs) => {
                        cursor.at_pos(bs.byte_pos, |c| c.write_u8(bs.acc))?;
                        TypeState::Bit(*conv, bs)
                    }
                    None => {
                        let pos = cursor.position();
                        let acc = bit.to_mask();
                        cursor.write_u8(acc)?;
                        TypeState::Bit(*conv, BitState::new(acc, pos))
                    }
                }
            }
            TypeState::DoubleBit(conv, bs) => {
                let bits = conv(value);
                match bs.next(bits) {
                    Some(bs) => {
                        cursor.at_pos(bs.byte_pos, |c| c.write_u8(bs.acc))?;
                        TypeState::DoubleBit(*conv, bs)
                    }
                    None => {
                        let pos = cursor.position();
                        let acc = bits.to_mask();
                        cursor.write_u8(acc)?;
                        TypeState::DoubleBit(*conv, BitState::new(acc, pos))
                    }
                }
            }
        };
        Ok(state)
    }
}

impl<T> WriteType<T> {
    fn write_first_value(
        &self,
        cursor: &mut WriteCursor,
        value: &T,
    ) -> Result<TypeState<T>, WriteError> {
        match self {
            WriteType::Fixed(write) => {
                (write)(cursor, value)?;
                Ok(TypeState::Fixed(*write))
            }
            WriteType::Bits(conv) => {
                let pos = cursor.position();
                let byte = (conv)(value).to_mask();
                cursor.write_u8(byte)?;
                Ok(TypeState::Bit(*conv, BitState::new(byte, pos)))
            }
            WriteType::DoubleBits(conv) => {
                let pos = cursor.position();
                let byte = (conv)(value).to_mask();
                cursor.write_u8(byte)?;
                Ok(TypeState::DoubleBit(*conv, BitState::new(byte, pos)))
            }
        }
    }
}

fn is_consecutive(last: u16, next: u16) -> bool {
    if next > last {
        next == last + 1
    } else {
        false
    }
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
    use crate::outstation::database::config::*;
    use crate::outstation::database::details::range::traits::StaticVariation;

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

        let g1v1 = StaticBinaryInputVariation::Group1Var1.get_write_info(&binary(false));

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
                01,
                00,
                // stop
                09,
                00,
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

        let g3v1 = StaticDoubleBitBinaryInputVariation::Group3Var1
            .get_write_info(&double_bit(DoubleBit::DeterminedOff));

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
                01,
                00,
                // stop
                05,
                00,
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

        let g3v1 = StaticDoubleBitBinaryInputVariation::Group3Var1
            .get_write_info(&double_bit(DoubleBit::DeterminedOff));

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
                01,
                00,
                // stop
                09,
                00,
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

        let g1v2 = StaticBinaryInputVariation::Group1Var2.get_write_info(&binary(false));

        writer.write(&mut cursor, 2, &binary(true), g1v2).unwrap();
        writer.write(&mut cursor, 2, &binary(true), g1v2).unwrap();


        assert_eq!(
            cursor.written(),
            [
                // g1v2 - 16-bit start/stop
                1, 2, 0x01, 02, 00, 02, 00, 0x81,
                // same header
                1, 2, 0x01, 02, 00, 02, 00, 0x81,
            ]
        )
    }

    #[test]
    #[rustfmt::skip]
    fn switches_headers_with_non_consecutive_indices() {
        let mut buffer = [0u8; 64];
        let mut cursor = WriteCursor::new(buffer.as_mut());
        let mut writer = RangeWriter::new();

        let g1v2 = StaticBinaryInputVariation::Group1Var2.get_write_info(&binary(false));

        writer.write(&mut cursor, 1, &binary(true), g1v2).unwrap();
        writer.write(&mut cursor, 3, &binary(true), g1v2).unwrap();

        assert_eq!(
            cursor.written(),
            [
                // g1v2 - 16-bit start/stop
                1, 2, 0x01, 01, 00, 01, 00, 0x81,
                // same header but with index == 3
                1, 2, 0x01, 03, 00, 03, 00, 0x81,
            ]
        )
    }
}
