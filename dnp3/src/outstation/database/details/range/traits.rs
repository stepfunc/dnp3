use crate::app::flags::Flags;
use crate::app::measurement::*;
use crate::app::parse::traits::{FixedSize, FixedSizeVariation};
use crate::app::types::DoubleBit;
use crate::app::variations::*;
use crate::outstation::database::config::*;
use crate::util::bit::bits::{BIT_6, BIT_7};
use crate::util::cursor::{WriteCursor, WriteError};

pub(crate) type FixedWriteFn<T> = fn(&mut WriteCursor, &T) -> Result<(), WriteError>;
pub(crate) type ToBit<T> = fn(&T) -> bool;
pub(crate) type ToDoubleBit<T> = fn(&T) -> DoubleBit;

fn fixed_type<T, V>() -> WriteInfo<T>
where
    V: FixedSizeVariation,
    T: ToVariation<V>,
{
    fn write<T, V>(cursor: &mut WriteCursor, value: &T) -> Result<(), WriteError>
    where
        V: FixedSize,
        T: ToVariation<V>,
    {
        value.to_variation().write(cursor)
    }
    WriteInfo {
        variation: V::VARIATION,
        write_type: WriteType::Fixed(write),
    }
}

fn bit_type<T>(variation: Variation, func: fn(&T) -> bool) -> WriteInfo<T> {
    WriteInfo {
        variation,
        write_type: WriteType::Bits(func),
    }
}

fn double_bit_type<T>(variation: Variation, func: fn(&T) -> DoubleBit) -> WriteInfo<T> {
    WriteInfo {
        variation,
        write_type: WriteType::DoubleBits(func),
    }
}

#[derive(Copy, Clone)]
pub(crate) enum WriteType<T> {
    Fixed(FixedWriteFn<T>),
    Bits(ToBit<T>),
    DoubleBits(ToDoubleBit<T>),
}

#[derive(Copy, Clone)]
pub(crate) struct WriteInfo<T> {
    pub(crate) variation: Variation,
    pub(crate) write_type: WriteType<T>,
}

pub(crate) trait StaticVariation<T>: Copy + PartialEq {
    // most of the time this just returns itself
    // but g1v1, g4v1, g10v1 will return a different variation
    // if the flags are not ONLINE
    fn promote(&self, _value: &T) -> Self {
        *self
    }

    fn get_write_info(&self) -> WriteInfo<T>;
}

impl StaticVariation<Binary> for StaticBinaryVariation {
    fn promote(&self, value: &Binary) -> Self {
        if let StaticBinaryVariation::Group1Var1 = self {
            if value.flags.without(BIT_7) == Flags::ONLINE {
                *self
            } else {
                StaticBinaryVariation::Group1Var2
            }
        } else {
            *self
        }
    }

    fn get_write_info(&self) -> WriteInfo<Binary> {
        match self {
            Self::Group1Var1 => bit_type(Variation::Group1Var1, |v| v.value),
            Self::Group1Var2 => fixed_type::<Binary, Group1Var2>(),
        }
    }
}

impl StaticVariation<DoubleBitBinary> for StaticDoubleBitBinaryVariation {
    fn promote(&self, value: &DoubleBitBinary) -> Self {
        if let StaticDoubleBitBinaryVariation::Group3Var1 = self {
            if value.flags.without(BIT_6 | BIT_7) == Flags::ONLINE {
                *self
            } else {
                StaticDoubleBitBinaryVariation::Group3Var2
            }
        } else {
            *self
        }
    }

    fn get_write_info(&self) -> WriteInfo<DoubleBitBinary> {
        match self {
            Self::Group3Var1 => double_bit_type(Variation::Group3Var1, |v| v.value),
            Self::Group3Var2 => fixed_type::<DoubleBitBinary, Group3Var2>(),
        }
    }
}

impl StaticVariation<BinaryOutputStatus> for StaticBinaryOutputStatusVariation {
    fn promote(&self, value: &BinaryOutputStatus) -> Self {
        if let StaticBinaryOutputStatusVariation::Group10Var1 = self {
            if value.flags.without(BIT_7) == Flags::ONLINE {
                *self
            } else {
                StaticBinaryOutputStatusVariation::Group10Var2
            }
        } else {
            *self
        }
    }

    fn get_write_info(&self) -> WriteInfo<BinaryOutputStatus> {
        match self {
            Self::Group10Var1 => bit_type(Variation::Group10Var1, |v| v.value),
            Self::Group10Var2 => fixed_type::<BinaryOutputStatus, Group10Var2>(),
        }
    }
}

impl StaticVariation<Counter> for StaticCounterVariation {
    fn get_write_info(&self) -> WriteInfo<Counter> {
        match self {
            StaticCounterVariation::Group20Var1 => fixed_type::<Counter, Group20Var1>(),
            StaticCounterVariation::Group20Var2 => fixed_type::<Counter, Group20Var2>(),
            StaticCounterVariation::Group20Var5 => fixed_type::<Counter, Group20Var5>(),
            StaticCounterVariation::Group20Var6 => fixed_type::<Counter, Group20Var6>(),
        }
    }
}

impl StaticVariation<FrozenCounter> for StaticFrozenCounterVariation {
    fn get_write_info(&self) -> WriteInfo<FrozenCounter> {
        match self {
            StaticFrozenCounterVariation::Group21Var1 => fixed_type::<FrozenCounter, Group21Var1>(),
            StaticFrozenCounterVariation::Group21Var2 => fixed_type::<FrozenCounter, Group21Var2>(),
            StaticFrozenCounterVariation::Group21Var5 => fixed_type::<FrozenCounter, Group21Var5>(),
            StaticFrozenCounterVariation::Group21Var6 => fixed_type::<FrozenCounter, Group21Var6>(),
            StaticFrozenCounterVariation::Group21Var9 => fixed_type::<FrozenCounter, Group21Var9>(),
            StaticFrozenCounterVariation::Group21Var10 => {
                fixed_type::<FrozenCounter, Group21Var10>()
            }
        }
    }
}

impl StaticVariation<Analog> for StaticAnalogVariation {
    fn get_write_info(&self) -> WriteInfo<Analog> {
        match self {
            StaticAnalogVariation::Group30Var1 => fixed_type::<Analog, Group30Var1>(),
            StaticAnalogVariation::Group30Var2 => fixed_type::<Analog, Group30Var2>(),
            StaticAnalogVariation::Group30Var3 => fixed_type::<Analog, Group30Var3>(),
            StaticAnalogVariation::Group30Var4 => fixed_type::<Analog, Group30Var4>(),
            StaticAnalogVariation::Group30Var5 => fixed_type::<Analog, Group30Var5>(),
            StaticAnalogVariation::Group30Var6 => fixed_type::<Analog, Group30Var6>(),
        }
    }
}

impl StaticVariation<AnalogOutputStatus> for StaticAnalogOutputStatusVariation {
    fn get_write_info(&self) -> WriteInfo<AnalogOutputStatus> {
        match self {
            StaticAnalogOutputStatusVariation::Group40Var1 => {
                fixed_type::<AnalogOutputStatus, Group40Var1>()
            }
            StaticAnalogOutputStatusVariation::Group40Var2 => {
                fixed_type::<AnalogOutputStatus, Group40Var2>()
            }
            StaticAnalogOutputStatusVariation::Group40Var3 => {
                fixed_type::<AnalogOutputStatus, Group40Var3>()
            }
            StaticAnalogOutputStatusVariation::Group40Var4 => {
                fixed_type::<AnalogOutputStatus, Group40Var4>()
            }
        }
    }
}
