// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::measurement::*;
use crate::app::parse::traits::{FixedSize, FixedSizeVariation};
use crate::app::variations::*;
use crate::outstation::database::config::*;
use crate::util::bit::bits::{BIT_6, BIT_7};
use crate::util::float::{f64_to_f32, f64_to_u16, f64_to_u32};

use scursor::{WriteCursor, WriteError};

pub(crate) type FixedWriteFn<T> = fn(&mut WriteCursor, &T) -> Result<(), WriteError>;
pub(crate) type ToBit<T> = fn(&T) -> bool;
pub(crate) type ToDoubleBit<T> = fn(&T) -> DoubleBit;

#[derive(Copy, Clone)]
pub(crate) enum WriteType<T> {
    Fixed(FixedWriteFn<T>),
    Bits(ToBit<T>),
    DoubleBits(ToDoubleBit<T>),
}

/// How a static value is encoded: the variation in the header and how to write each value.
///
/// It's only built by the constructors below, which take the variation from the encoding, so
/// the two can't disagree.
#[derive(Copy, Clone)]
pub(crate) struct WriteInfo<T> {
    variation: Variation,
    write_type: WriteType<T>,
}

impl<T> WriteInfo<T> {
    /// a fixed-size variation
    fn fixed<V>() -> Self
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
        Self {
            variation: V::VARIATION,
            write_type: WriteType::Fixed(write::<T, V>),
        }
    }

    pub(crate) fn variation(&self) -> Variation {
        self.variation
    }

    pub(crate) fn write_type(&self) -> &WriteType<T> {
        &self.write_type
    }
}

impl WriteInfo<BinaryInput> {
    /// g1v1, the packed state of each input
    fn packed() -> Self {
        Self {
            variation: Variation::Group1Var1,
            write_type: WriteType::Bits(|v| v.value),
        }
    }
}

impl WriteInfo<DoubleBitBinaryInput> {
    /// g3v1, the packed state of each input
    fn packed() -> Self {
        Self {
            variation: Variation::Group3Var1,
            write_type: WriteType::DoubleBits(|v| v.value),
        }
    }
}

impl WriteInfo<BinaryOutputStatus> {
    /// g10v1, the packed state of each output
    fn packed() -> Self {
        Self {
            variation: Variation::Group10Var1,
            write_type: WriteType::Bits(|v| v.value),
        }
    }
}

impl WriteInfo<OctetString> {
    /// g110, whose variation is the length of the octet string
    fn octet_string(value: &OctetString) -> Self {
        fn write(cursor: &mut WriteCursor, value: &OctetString) -> Result<(), WriteError> {
            cursor.write_bytes(value.value())
        }
        Self {
            variation: Variation::Group110(value.len()),
            write_type: WriteType::Fixed(write),
        }
    }
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
            Self::Group1Var1 => WriteInfo::<BinaryInput>::packed(),
            Self::Group1Var2 => WriteInfo::fixed::<Group1Var2>(),
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
            Self::Group3Var1 => WriteInfo::<DoubleBitBinaryInput>::packed(),
            Self::Group3Var2 => WriteInfo::fixed::<Group3Var2>(),
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
            Self::Group10Var1 => WriteInfo::<BinaryOutputStatus>::packed(),
            Self::Group10Var2 => WriteInfo::fixed::<Group10Var2>(),
        }
    }
}

impl StaticVariation<Counter> for StaticCounterVariation {
    fn promote(&self, value: &Counter) -> Self {
        if value.flags == Flags::ONLINE {
            return *self;
        }

        match self {
            Self::Group20Var5 => Self::Group20Var1,
            Self::Group20Var6 => Self::Group20Var2,
            Self::Group20Var1 | Self::Group20Var2 => *self,
        }
    }

    fn get_write_info(&self, _value: &Counter) -> WriteInfo<Counter> {
        match self {
            StaticCounterVariation::Group20Var1 => WriteInfo::fixed::<Group20Var1>(),
            StaticCounterVariation::Group20Var2 => WriteInfo::fixed::<Group20Var2>(),
            StaticCounterVariation::Group20Var5 => WriteInfo::fixed::<Group20Var5>(),
            StaticCounterVariation::Group20Var6 => WriteInfo::fixed::<Group20Var6>(),
        }
    }
}

impl AnalogInputDeadBandVariation {
    pub(crate) fn get_write_info(&self) -> WriteInfo<f64> {
        match self {
            AnalogInputDeadBandVariation::Group34Var1 => WriteInfo::fixed::<Group34Var1>(),
            AnalogInputDeadBandVariation::Group34Var2 => WriteInfo::fixed::<Group34Var2>(),
            AnalogInputDeadBandVariation::Group34Var3 => WriteInfo::fixed::<Group34Var3>(),
        }
    }
}

impl ToVariation<Group34Var1> for f64 {
    fn to_variation(&self) -> Group34Var1 {
        Group34Var1 {
            value: f64_to_u16(self.round()),
        }
    }
}

impl ToVariation<Group34Var2> for f64 {
    fn to_variation(&self) -> Group34Var2 {
        Group34Var2 {
            value: f64_to_u32(self.round()),
        }
    }
}

impl ToVariation<Group34Var3> for f64 {
    fn to_variation(&self) -> Group34Var3 {
        let num = *self;
        // finite values past the f32 range clamp, and infinity is kept
        let value = if num.is_finite() && num > f32::MAX.into() {
            f32::MAX
        } else if num.is_finite() && num < f32::MIN.into() {
            f32::MIN
        } else {
            f64_to_f32(num)
        };

        Group34Var3 { value }
    }
}

impl StaticVariation<FrozenCounter> for StaticFrozenCounterVariation {
    fn promote(&self, value: &FrozenCounter) -> Self {
        if value.flags == Flags::ONLINE {
            return *self;
        }

        match self {
            Self::Group21Var9 => Self::Group21Var1,
            Self::Group21Var10 => Self::Group21Var2,
            Self::Group21Var1 | Self::Group21Var2 | Self::Group21Var5 | Self::Group21Var6 => *self,
        }
    }

    fn get_write_info(&self, _value: &FrozenCounter) -> WriteInfo<FrozenCounter> {
        match self {
            StaticFrozenCounterVariation::Group21Var1 => WriteInfo::fixed::<Group21Var1>(),
            StaticFrozenCounterVariation::Group21Var2 => WriteInfo::fixed::<Group21Var2>(),
            StaticFrozenCounterVariation::Group21Var5 => WriteInfo::fixed::<Group21Var5>(),
            StaticFrozenCounterVariation::Group21Var6 => WriteInfo::fixed::<Group21Var6>(),
            StaticFrozenCounterVariation::Group21Var9 => WriteInfo::fixed::<Group21Var9>(),
            StaticFrozenCounterVariation::Group21Var10 => WriteInfo::fixed::<Group21Var10>(),
        }
    }
}

impl StaticVariation<AnalogInput> for StaticAnalogInputVariation {
    // the flags checked are those the variation with flags would report,
    // which include OVER_RANGE if the value doesn't fit
    fn promote(&self, value: &AnalogInput) -> Self {
        match self {
            Self::Group30Var3 if value.to_i32().0 != Flags::ONLINE => Self::Group30Var1,
            Self::Group30Var4 if value.to_i16().0 != Flags::ONLINE => Self::Group30Var2,
            Self::Group30Var1
            | Self::Group30Var2
            | Self::Group30Var3
            | Self::Group30Var4
            | Self::Group30Var5
            | Self::Group30Var6 => *self,
        }
    }

    fn get_write_info(&self, _value: &AnalogInput) -> WriteInfo<AnalogInput> {
        match self {
            StaticAnalogInputVariation::Group30Var1 => WriteInfo::fixed::<Group30Var1>(),
            StaticAnalogInputVariation::Group30Var2 => WriteInfo::fixed::<Group30Var2>(),
            StaticAnalogInputVariation::Group30Var3 => WriteInfo::fixed::<Group30Var3>(),
            StaticAnalogInputVariation::Group30Var4 => WriteInfo::fixed::<Group30Var4>(),
            StaticAnalogInputVariation::Group30Var5 => WriteInfo::fixed::<Group30Var5>(),
            StaticAnalogInputVariation::Group30Var6 => WriteInfo::fixed::<Group30Var6>(),
        }
    }
}

impl StaticVariation<AnalogOutputStatus> for StaticAnalogOutputStatusVariation {
    fn get_write_info(&self, _value: &AnalogOutputStatus) -> WriteInfo<AnalogOutputStatus> {
        match self {
            StaticAnalogOutputStatusVariation::Group40Var1 => WriteInfo::fixed::<Group40Var1>(),
            StaticAnalogOutputStatusVariation::Group40Var2 => WriteInfo::fixed::<Group40Var2>(),
            StaticAnalogOutputStatusVariation::Group40Var3 => WriteInfo::fixed::<Group40Var3>(),
            StaticAnalogOutputStatusVariation::Group40Var4 => WriteInfo::fixed::<Group40Var4>(),
        }
    }
}

impl StaticVariation<OctetString> for StaticOctetStringVariation {
    fn get_write_info(&self, value: &OctetString) -> WriteInfo<OctetString> {
        WriteInfo::octet_string(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g34v1(value: f64) -> u16 {
        ToVariation::<Group34Var1>::to_variation(&value).value
    }

    fn g34v2(value: f64) -> u32 {
        ToVariation::<Group34Var2>::to_variation(&value).value
    }

    fn g34v3(value: f64) -> f32 {
        ToVariation::<Group34Var3>::to_variation(&value).value
    }

    #[test]
    fn integer_deadbands_round_and_saturate() {
        assert_eq!(g34v1(0.5), 1);
        assert_eq!(g34v1(-0.4), 0);
        assert_eq!(g34v1(65534.6), u16::MAX);
        assert_eq!(g34v1(65535.5), u16::MAX);
        assert_eq!(g34v1(f64::INFINITY), u16::MAX);
        assert_eq!(g34v1(f64::NEG_INFINITY), 0);
        assert_eq!(g34v1(f64::NAN), 0);

        assert_eq!(g34v2(2.5), 3);
        assert_eq!(g34v2(4_294_967_295.5), u32::MAX);
        assert_eq!(g34v2(f64::INFINITY), u32::MAX);
        assert_eq!(g34v2(f64::NEG_INFINITY), 0);
        assert_eq!(g34v2(f64::NAN), 0);
    }

    #[test]
    fn float_deadband_clamps_to_the_f32_range() {
        // these values are exact, so compare their bits
        assert_eq!(g34v3(1.5).to_bits(), 1.5f32.to_bits());
        assert_eq!(g34v3(f64::MAX).to_bits(), f32::MAX.to_bits());
        assert_eq!(g34v3(f64::MIN).to_bits(), f32::MIN.to_bits());
        assert_eq!(g34v3(f64::INFINITY).to_bits(), f32::INFINITY.to_bits());
        assert!(g34v3(f64::NAN).is_nan());
    }
}
