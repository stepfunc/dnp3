// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::measurement::*;
use crate::app::parse::traits::FixedSizeVariation;
use crate::app::variations::*;
use crate::outstation::database::config::*;
use crate::outstation::database::variations::*;
use crate::util::bit::bits::{BIT_6, BIT_7};
use crate::util::float::{f64_to_f32, f64_to_u16, f64_to_u32};

/// A measurement written as one bit of a packed variation
pub(crate) trait ToBit {
    fn to_bit(&self) -> bool;
}

/// A measurement written as two bits of a packed variation
pub(crate) trait ToDoubleBit {
    fn to_double_bit(&self) -> DoubleBit;
}

impl ToBit for BinaryInput {
    fn to_bit(&self) -> bool {
        self.value
    }
}

impl ToBit for BinaryOutputStatus {
    fn to_bit(&self) -> bool {
        self.value
    }
}

/// A measurement written as the bytes of an octet string
pub(crate) trait ToOctets {
    fn to_octets(&self) -> &[u8];
}

impl ToOctets for OctetString {
    fn to_octets(&self) -> &[u8] {
        self.value()
    }
}

impl ToDoubleBit for DoubleBitBinaryInput {
    fn to_double_bit(&self) -> DoubleBit {
        self.value
    }
}

/// Writes a static value in the encoding its variation chooses
pub(crate) trait StaticEncoder<T> {
    type Output;

    /// a fixed-size variation
    fn fixed<V>(self) -> Self::Output
    where
        V: FixedSizeVariation,
        T: ToVariation<V>;

    /// a packed variation of one bit per value
    fn bits(self, variation: Variation) -> Self::Output
    where
        T: ToBit;

    /// a packed variation of two bits per value
    fn double_bits(self, variation: Variation) -> Self::Output
    where
        T: ToDoubleBit;

    /// an octet string, whose variation is its length
    fn octet_string(self) -> Self::Output
    where
        T: ToOctets;
}

pub(crate) trait StaticVariation<T>: Copy + PartialEq {
    // most of the time this just returns itself
    // but g1v1, g4v1, g10v1 will return a different variation
    // if the flags are not ONLINE
    fn promote(&self, _value: &T) -> Self {
        *self
    }

    /// apply the encoder to the encoding of this variation
    ///
    /// Each variation names its encoding once: the type a fixed-size variation is written as,
    /// or the wire variation of a packed one.
    fn encode<E>(self, encoder: E) -> E::Output
    where
        E: StaticEncoder<T>;
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

    fn encode<E>(self, encoder: E) -> E::Output
    where
        E: StaticEncoder<BinaryInput>,
    {
        match self {
            Self::Group1Var1 => encoder.bits(Variation::Group1Var1),
            Self::Group1Var2 => encoder.fixed::<Group1Var2>(),
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

    fn encode<E>(self, encoder: E) -> E::Output
    where
        E: StaticEncoder<DoubleBitBinaryInput>,
    {
        match self {
            Self::Group3Var1 => encoder.double_bits(Variation::Group3Var1),
            Self::Group3Var2 => encoder.fixed::<Group3Var2>(),
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

    fn encode<E>(self, encoder: E) -> E::Output
    where
        E: StaticEncoder<BinaryOutputStatus>,
    {
        match self {
            Self::Group10Var1 => encoder.bits(Variation::Group10Var1),
            Self::Group10Var2 => encoder.fixed::<Group10Var2>(),
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

    fn encode<E>(self, encoder: E) -> E::Output
    where
        E: StaticEncoder<Counter>,
    {
        match self {
            StaticCounterVariation::Group20Var1 => encoder.fixed::<Group20Var1>(),
            StaticCounterVariation::Group20Var2 => encoder.fixed::<Group20Var2>(),
            StaticCounterVariation::Group20Var5 => encoder.fixed::<Group20Var5>(),
            StaticCounterVariation::Group20Var6 => encoder.fixed::<Group20Var6>(),
        }
    }
}

impl StaticVariation<f64> for AnalogInputDeadBandVariation {
    fn encode<E>(self, encoder: E) -> E::Output
    where
        E: StaticEncoder<f64>,
    {
        match self {
            AnalogInputDeadBandVariation::Group34Var1 => encoder.fixed::<Group34Var1>(),
            AnalogInputDeadBandVariation::Group34Var2 => encoder.fixed::<Group34Var2>(),
            AnalogInputDeadBandVariation::Group34Var3 => encoder.fixed::<Group34Var3>(),
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

    fn encode<E>(self, encoder: E) -> E::Output
    where
        E: StaticEncoder<FrozenCounter>,
    {
        match self {
            StaticFrozenCounterVariation::Group21Var1 => encoder.fixed::<Group21Var1>(),
            StaticFrozenCounterVariation::Group21Var2 => encoder.fixed::<Group21Var2>(),
            StaticFrozenCounterVariation::Group21Var5 => encoder.fixed::<Group21Var5>(),
            StaticFrozenCounterVariation::Group21Var6 => encoder.fixed::<Group21Var6>(),
            StaticFrozenCounterVariation::Group21Var9 => encoder.fixed::<Group21Var9>(),
            StaticFrozenCounterVariation::Group21Var10 => encoder.fixed::<Group21Var10>(),
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

    fn encode<E>(self, encoder: E) -> E::Output
    where
        E: StaticEncoder<AnalogInput>,
    {
        match self {
            StaticAnalogInputVariation::Group30Var1 => encoder.fixed::<Group30Var1>(),
            StaticAnalogInputVariation::Group30Var2 => encoder.fixed::<Group30Var2>(),
            StaticAnalogInputVariation::Group30Var3 => encoder.fixed::<Group30Var3>(),
            StaticAnalogInputVariation::Group30Var4 => encoder.fixed::<Group30Var4>(),
            StaticAnalogInputVariation::Group30Var5 => encoder.fixed::<Group30Var5>(),
            StaticAnalogInputVariation::Group30Var6 => encoder.fixed::<Group30Var6>(),
        }
    }
}

impl StaticVariation<AnalogOutputStatus> for StaticAnalogOutputStatusVariation {
    fn encode<E>(self, encoder: E) -> E::Output
    where
        E: StaticEncoder<AnalogOutputStatus>,
    {
        match self {
            StaticAnalogOutputStatusVariation::Group40Var1 => encoder.fixed::<Group40Var1>(),
            StaticAnalogOutputStatusVariation::Group40Var2 => encoder.fixed::<Group40Var2>(),
            StaticAnalogOutputStatusVariation::Group40Var3 => encoder.fixed::<Group40Var3>(),
            StaticAnalogOutputStatusVariation::Group40Var4 => encoder.fixed::<Group40Var4>(),
        }
    }
}

impl StaticVariation<OctetString> for StaticOctetStringVariation {
    fn encode<E>(self, encoder: E) -> E::Output
    where
        E: StaticEncoder<OctetString>,
    {
        encoder.octet_string()
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
