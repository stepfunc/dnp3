use crate::app::measurement::*;
use crate::app::parse::traits::{FixedSize, FixedSizeVariation};
use crate::app::variations::*;
use crate::outstation::database::config::*;
use crate::util::bit::bits::{BIT_6, BIT_7};

use scursor::{WriteCursor, WriteError};

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

fn octet_string(value: &OctetString) -> WriteInfo<OctetString> {
    fn write(cursor: &mut WriteCursor, value: &OctetString) -> Result<(), WriteError> {
        cursor.write_bytes(value.value())
    }
    WriteInfo {
        variation: Variation::Group110(value.len()),
        write_type: WriteType::Fixed(write),
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

    fn get_write_info(&self, value: &T) -> WriteInfo<T>;
}

impl StaticVariation<BinaryInput> for StaticBinaryInputVariation {
    fn promote(&self, value: &BinaryInput) -> Self {
        if let StaticBinaryInputVariation::Group1Var1 = self {
            if value.flags.without(BIT_7) == Flags::ONLINE {
                *self
            } else {
                StaticBinaryInputVariation::Group1Var2
            }
        } else {
            *self
        }
    }

    fn get_write_info(&self, _value: &BinaryInput) -> WriteInfo<BinaryInput> {
        match self {
            Self::Group1Var1 => bit_type(Variation::Group1Var1, |v| v.value),
            Self::Group1Var2 => fixed_type::<BinaryInput, Group1Var2>(),
        }
    }
}

impl StaticVariation<DoubleBitBinaryInput> for StaticDoubleBitBinaryInputVariation {
    fn promote(&self, value: &DoubleBitBinaryInput) -> Self {
        if let StaticDoubleBitBinaryInputVariation::Group3Var1 = self {
            if value.flags.without(BIT_6 | BIT_7) == Flags::ONLINE {
                *self
            } else {
                StaticDoubleBitBinaryInputVariation::Group3Var2
            }
        } else {
            *self
        }
    }

    fn get_write_info(&self, _value: &DoubleBitBinaryInput) -> WriteInfo<DoubleBitBinaryInput> {
        match self {
            Self::Group3Var1 => double_bit_type(Variation::Group3Var1, |v| v.value),
            Self::Group3Var2 => fixed_type::<DoubleBitBinaryInput, Group3Var2>(),
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

    fn get_write_info(&self, _value: &BinaryOutputStatus) -> WriteInfo<BinaryOutputStatus> {
        match self {
            Self::Group10Var1 => bit_type(Variation::Group10Var1, |v| v.value),
            Self::Group10Var2 => fixed_type::<BinaryOutputStatus, Group10Var2>(),
        }
    }
}

impl StaticVariation<Counter> for StaticCounterVariation {
    fn get_write_info(&self, _value: &Counter) -> WriteInfo<Counter> {
        match self {
            StaticCounterVariation::Group20Var1 => fixed_type::<Counter, Group20Var1>(),
            StaticCounterVariation::Group20Var2 => fixed_type::<Counter, Group20Var2>(),
            StaticCounterVariation::Group20Var5 => fixed_type::<Counter, Group20Var5>(),
            StaticCounterVariation::Group20Var6 => fixed_type::<Counter, Group20Var6>(),
        }
    }
}

impl AnalogInputDeadBandVariation {
    pub(crate) fn get_write_info(&self) -> WriteInfo<f64> {
        match self {
            AnalogInputDeadBandVariation::Group34Var1 => fixed_type::<f64, Group34Var1>(),
            AnalogInputDeadBandVariation::Group34Var2 => fixed_type::<f64, Group34Var2>(),
            AnalogInputDeadBandVariation::Group34Var3 => fixed_type::<f64, Group34Var3>(),
        }
    }
}

impl ToVariation<Group34Var1> for f64 {
    fn to_variation(&self) -> Group34Var1 {
        let num = self.round() as i64;
        let value = {
            if num > u16::MAX as i64 {
                u16::MAX
            } else if num < u16::MIN as i64 {
                u16::MIN
            } else {
                num as u16
            }
        };

        Group34Var1 { value }
    }
}

impl ToVariation<Group34Var2> for f64 {
    fn to_variation(&self) -> Group34Var2 {
        let num = self.round() as i64;
        let value = {
            if num > u32::MAX as i64 {
                u32::MAX
            } else if num < u32::MIN as i64 {
                u32::MIN
            } else {
                num as u32
            }
        };

        Group34Var2 { value }
    }
}

impl ToVariation<Group34Var3> for f64 {
    fn to_variation(&self) -> Group34Var3 {
        let num = *self;
        let value = {
            if num > f32::MAX as f64 {
                f32::MAX
            } else if num < f32::MIN as f64 {
                f32::MIN
            } else {
                num as f32
            }
        };

        Group34Var3 { value }
    }
}

impl StaticVariation<FrozenCounter> for StaticFrozenCounterVariation {
    fn get_write_info(&self, _value: &FrozenCounter) -> WriteInfo<FrozenCounter> {
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

impl StaticVariation<AnalogInput> for StaticAnalogInputVariation {
    fn get_write_info(&self, _value: &AnalogInput) -> WriteInfo<AnalogInput> {
        match self {
            StaticAnalogInputVariation::Group30Var1 => fixed_type::<AnalogInput, Group30Var1>(),
            StaticAnalogInputVariation::Group30Var2 => fixed_type::<AnalogInput, Group30Var2>(),
            StaticAnalogInputVariation::Group30Var3 => fixed_type::<AnalogInput, Group30Var3>(),
            StaticAnalogInputVariation::Group30Var4 => fixed_type::<AnalogInput, Group30Var4>(),
            StaticAnalogInputVariation::Group30Var5 => fixed_type::<AnalogInput, Group30Var5>(),
            StaticAnalogInputVariation::Group30Var6 => fixed_type::<AnalogInput, Group30Var6>(),
        }
    }
}

impl StaticVariation<AnalogOutputStatus> for StaticAnalogOutputStatusVariation {
    fn get_write_info(&self, _value: &AnalogOutputStatus) -> WriteInfo<AnalogOutputStatus> {
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

impl StaticVariation<OctetString> for StaticOctetStringVariation {
    fn get_write_info(&self, value: &OctetString) -> WriteInfo<OctetString> {
        octet_string(value)
    }
}
