// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//  _   _         ______    _ _ _   _             _ _ _
// | \ | |       |  ____|  | (_) | (_)           | | | |
// |  \| | ___   | |__   __| |_| |_ _ _ __   __ _| | | |
// | . ` |/ _ \  |  __| / _` | | __| | '_ \ / _` | | | |
// | |\  | (_) | | |___| (_| | | |_| | | | | (_| |_|_|_|
// |_| \_|\___/  |______\__,_|_|\__|_|_| |_|\__, (_|_|_)
//                                           __/ |
//                                          |___/
//
// This file is auto-generated. Do not edit manually
//

#[cfg(test)]
use crate::app::Variation;

/// Enum representing all possible `BinaryInput` static variations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum StaticBinaryInputVariation {
    /// Binary Input - Packed format
    Group1Var1,
    /// Binary Input - With flags
    Group1Var2,
}

/// Enum representing all possible `BinaryInput` event variations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum EventBinaryInputVariation {
    /// Binary Input Event - Without time
    Group2Var1,
    /// Binary Input Event - With absolute time
    ///
    /// Events recorded with unsynchronized time are reported as `Group2Var3` after an
    /// unsynchronized common time of occurrence (g51v2), and events recorded without time as
    /// `Group2Var1`
    Group2Var2,
    /// Binary Input Event - With relative time
    ///
    /// Events recorded without time are reported as `Group2Var1`
    Group2Var3,
}

/// Enum representing all possible `DoubleBitBinaryInput` static variations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum StaticDoubleBitBinaryInputVariation {
    /// Double-bit Binary Input - Packed format
    Group3Var1,
    /// Double-bit Binary Input - With flags
    Group3Var2,
}

/// Enum representing all possible `DoubleBitBinaryInput` event variations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum EventDoubleBitBinaryInputVariation {
    /// Double-bit Binary Input Event - Without time
    Group4Var1,
    /// Double-bit Binary Input Event - With absolute time
    ///
    /// Events recorded with unsynchronized time are reported as `Group4Var3` after an
    /// unsynchronized common time of occurrence (g51v2), and events recorded without time as
    /// `Group4Var1`
    Group4Var2,
    /// Double-bit Binary Input Event - With relative time
    ///
    /// Events recorded without time are reported as `Group4Var1`
    Group4Var3,
}

/// Enum representing all possible `BinaryOutputStatus` static variations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum StaticBinaryOutputStatusVariation {
    /// Binary Output - Packed format
    Group10Var1,
    /// Binary Output - With flags
    Group10Var2,
}

/// Enum representing all possible `BinaryOutputStatus` event variations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum EventBinaryOutputStatusVariation {
    /// Binary Output Event - Without time
    Group11Var1,
    /// Binary Output Event - With time
    ///
    /// Events recorded without time are reported as `Group11Var1`
    Group11Var2,
}

/// Enum representing all possible `Counter` static variations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum StaticCounterVariation {
    /// Counter - 32-bit with flags
    Group20Var1,
    /// Counter - 16-bit with flags
    Group20Var2,
    /// Counter - 32-bit without flags
    Group20Var5,
    /// Counter - 16-bit without flags
    Group20Var6,
}

/// Enum representing all possible `Counter` event variations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum EventCounterVariation {
    /// Counter Event - 32-bit without time
    Group22Var1,
    /// Counter Event - 16-bit without time
    Group22Var2,
    /// Counter Event - 32-bit with time
    ///
    /// Events recorded without time are reported as `Group22Var1`
    Group22Var5,
    /// Counter Event - 16-bit with time
    ///
    /// Events recorded without time are reported as `Group22Var2`
    Group22Var6,
}

/// Enum representing all possible `FrozenCounter` static variations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum StaticFrozenCounterVariation {
    /// Frozen Counter - 32-bit with flags
    Group21Var1,
    /// Frozen Counter - 16-bit with flags
    Group21Var2,
    /// Frozen Counter - 32-bit with flags and time
    Group21Var5,
    /// Frozen Counter - 16-bit with flags and time
    Group21Var6,
    /// Frozen Counter - 32-bit without flags
    Group21Var9,
    /// Frozen Counter - 16-bit without flags
    Group21Var10,
}

/// Enum representing all possible `FrozenCounter` event variations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum EventFrozenCounterVariation {
    /// Frozen Counter Event - 32-bit without time
    Group23Var1,
    /// Frozen Counter Event - 16-bit without time
    Group23Var2,
    /// Frozen Counter Event - 32-bit with time
    ///
    /// Events recorded without time are reported as `Group23Var1`
    Group23Var5,
    /// Frozen Counter Event - 16-bit with time
    ///
    /// Events recorded without time are reported as `Group23Var2`
    Group23Var6,
}

/// Enum representing all possible `AnalogInput` static variations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum StaticAnalogInputVariation {
    /// Analog Input - 32-bit with flags
    Group30Var1,
    /// Analog Input - 16-bit with flags
    Group30Var2,
    /// Analog Input - 32-bit without flags
    Group30Var3,
    /// Analog Input - 16-bit without flags
    Group30Var4,
    /// Analog Input - Single-precision floating point with flags
    Group30Var5,
    /// Analog Input - Double-precision floating point with flags
    Group30Var6,
}

/// Enum representing all possible `AnalogInput` event variations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum EventAnalogInputVariation {
    /// Analog Input Event - 32-bit without time
    Group32Var1,
    /// Analog Input Event - 16-bit without time
    Group32Var2,
    /// Analog Input Event - 32-bit with time
    ///
    /// Events recorded without time are reported as `Group32Var1`
    Group32Var3,
    /// Analog Input Event - 16-bit with time
    ///
    /// Events recorded without time are reported as `Group32Var2`
    Group32Var4,
    /// Analog Input Event - Single-precision floating point without time
    Group32Var5,
    /// Analog Input Event - Double-precision floating point without time
    Group32Var6,
    /// Analog Input Event - Single-precision floating point with time
    ///
    /// Events recorded without time are reported as `Group32Var5`
    Group32Var7,
    /// Analog Input Event - Double-precision floating point with time
    ///
    /// Events recorded without time are reported as `Group32Var6`
    Group32Var8,
}

/// Enum representing all possible `FrozenAnalogInput` static variations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum StaticFrozenAnalogInputVariation {
    /// Frozen Analog Input - 32-bit with flags
    Group31Var1,
    /// Frozen Analog Input - 16-bit with flags
    Group31Var2,
    /// Frozen Analog Input - 32-bit with flags and time of freeze
    Group31Var3,
    /// Frozen Analog Input - 16-bit with flags and time of freeze
    Group31Var4,
    /// Frozen Analog Input - 32-bit without flags
    Group31Var5,
    /// Frozen Analog Input - 16-bit without flags
    Group31Var6,
    /// Frozen Analog Input - Single-precision floating point with flags
    Group31Var7,
    /// Frozen Analog Input - Double-precision floating point with flags
    Group31Var8,
}

/// Enum representing all possible `FrozenAnalogInput` event variations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum EventFrozenAnalogInputVariation {
    /// Frozen Analog Input Event - 32-bit without time
    Group33Var1,
    /// Frozen Analog Input Event - 16-bit without time
    Group33Var2,
    /// Frozen Analog Input Event - 32-bit with time of freeze
    Group33Var3,
    /// Frozen Analog Input Event - 16-bit with time of freeze
    Group33Var4,
    /// Frozen Analog Input Event - Single-precision floating point without time
    Group33Var5,
    /// Frozen Analog Input Event - Double-precision floating point without time
    Group33Var6,
    /// Frozen Analog Input Event - Single-precision floating point with time of freeze
    Group33Var7,
    /// Frozen Analog Input Event - Double-precision floating point with time of freeze
    Group33Var8,
}

/// Enum representing all possible `AnalogOutputStatus` static variations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum StaticAnalogOutputStatusVariation {
    /// Analog Output Status - 32-bit with flags
    Group40Var1,
    /// Analog Output Status - 16-bit with flags
    Group40Var2,
    /// Analog Output Status - Single-precision floating point with flags
    Group40Var3,
    /// Analog Output Status - Double-precision floating point with flags
    Group40Var4,
}

/// Enum representing all possible `AnalogOutputStatus` event variations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum EventAnalogOutputStatusVariation {
    /// Analog Output Event - 32-bit without time
    Group42Var1,
    /// Analog Output Event - 16-bit without time
    Group42Var2,
    /// Analog Output Event - 32-bit with time
    ///
    /// Events recorded without time are reported as `Group42Var1`
    Group42Var3,
    /// Analog Output Event - 16-bit with time
    ///
    /// Events recorded without time are reported as `Group42Var2`
    Group42Var4,
    /// Analog Output Event - Single-precision floating point without time
    Group42Var5,
    /// Analog Output Event - Double-precision floating point without time
    Group42Var6,
    /// Analog Output Event - Single-precision floating point with time
    ///
    /// Events recorded without time are reported as `Group42Var5`
    Group42Var7,
    /// Analog Output Event - Double-precision floating point with time
    ///
    /// Events recorded without time are reported as `Group42Var6`
    Group42Var8,
}

/// Enum representing all possible `AnalogInputDeadBand` static variations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub(crate) enum AnalogInputDeadBandVariation {
    /// Analog Input Reporting Deadband - 16-bit
    Group34Var1,
    /// Analog Input Reporting Deadband - 32-bit
    Group34Var2,
    /// Analog Input Reporting Deadband - Single-precision floating point
    Group34Var3,
}

#[cfg(test)]
impl StaticBinaryInputVariation {
    /// Every variation, in order
    pub(crate) const ALL: &[Self] = &[Self::Group1Var1, Self::Group1Var2];
}

#[cfg(test)]
impl EventBinaryInputVariation {
    /// Every variation, in order
    pub(crate) const ALL: &[Self] = &[Self::Group2Var1, Self::Group2Var2, Self::Group2Var3];
}

#[cfg(test)]
impl StaticDoubleBitBinaryInputVariation {
    /// Every variation, in order
    pub(crate) const ALL: &[Self] = &[Self::Group3Var1, Self::Group3Var2];
}

#[cfg(test)]
impl EventDoubleBitBinaryInputVariation {
    /// Every variation, in order
    pub(crate) const ALL: &[Self] = &[Self::Group4Var1, Self::Group4Var2, Self::Group4Var3];
}

#[cfg(test)]
impl StaticBinaryOutputStatusVariation {
    /// Every variation, in order
    pub(crate) const ALL: &[Self] = &[Self::Group10Var1, Self::Group10Var2];
}

#[cfg(test)]
impl EventBinaryOutputStatusVariation {
    /// Every variation, in order
    pub(crate) const ALL: &[Self] = &[Self::Group11Var1, Self::Group11Var2];
}

#[cfg(test)]
impl StaticCounterVariation {
    /// Every variation, in order
    pub(crate) const ALL: &[Self] = &[
        Self::Group20Var1,
        Self::Group20Var2,
        Self::Group20Var5,
        Self::Group20Var6,
    ];
}

#[cfg(test)]
impl EventCounterVariation {
    /// Every variation, in order
    pub(crate) const ALL: &[Self] = &[
        Self::Group22Var1,
        Self::Group22Var2,
        Self::Group22Var5,
        Self::Group22Var6,
    ];
}

#[cfg(test)]
impl StaticFrozenCounterVariation {
    /// Every variation, in order
    pub(crate) const ALL: &[Self] = &[
        Self::Group21Var1,
        Self::Group21Var2,
        Self::Group21Var5,
        Self::Group21Var6,
        Self::Group21Var9,
        Self::Group21Var10,
    ];
}

#[cfg(test)]
impl EventFrozenCounterVariation {
    /// Every variation, in order
    pub(crate) const ALL: &[Self] = &[
        Self::Group23Var1,
        Self::Group23Var2,
        Self::Group23Var5,
        Self::Group23Var6,
    ];
}

#[cfg(test)]
impl StaticAnalogInputVariation {
    /// Every variation, in order
    pub(crate) const ALL: &[Self] = &[
        Self::Group30Var1,
        Self::Group30Var2,
        Self::Group30Var3,
        Self::Group30Var4,
        Self::Group30Var5,
        Self::Group30Var6,
    ];
}

#[cfg(test)]
impl EventAnalogInputVariation {
    /// Every variation, in order
    pub(crate) const ALL: &[Self] = &[
        Self::Group32Var1,
        Self::Group32Var2,
        Self::Group32Var3,
        Self::Group32Var4,
        Self::Group32Var5,
        Self::Group32Var6,
        Self::Group32Var7,
        Self::Group32Var8,
    ];
}

#[cfg(test)]
impl StaticAnalogOutputStatusVariation {
    /// Every variation, in order
    pub(crate) const ALL: &[Self] = &[
        Self::Group40Var1,
        Self::Group40Var2,
        Self::Group40Var3,
        Self::Group40Var4,
    ];
}

#[cfg(test)]
impl EventAnalogOutputStatusVariation {
    /// Every variation, in order
    pub(crate) const ALL: &[Self] = &[
        Self::Group42Var1,
        Self::Group42Var2,
        Self::Group42Var3,
        Self::Group42Var4,
        Self::Group42Var5,
        Self::Group42Var6,
        Self::Group42Var7,
        Self::Group42Var8,
    ];
}

/// The variations a READ of the static binary input points can name, variation 0 first
#[cfg(test)]
pub(crate) const BINARY_INPUT_STATIC_READS: &[Variation] = &[
    Variation::Group1Var0,
    Variation::Group1Var1,
    Variation::Group1Var2,
];

/// The variations a READ of the binary input events can name, variation 0 first
#[cfg(test)]
pub(crate) const BINARY_INPUT_EVENT_READS: &[Variation] = &[
    Variation::Group2Var0,
    Variation::Group2Var1,
    Variation::Group2Var2,
    Variation::Group2Var3,
];

/// The variations a READ of the static double-bit binary input points can name, variation 0 first
#[cfg(test)]
pub(crate) const DOUBLE_BIT_BINARY_INPUT_STATIC_READS: &[Variation] = &[
    Variation::Group3Var0,
    Variation::Group3Var1,
    Variation::Group3Var2,
];

/// The variations a READ of the double-bit binary input events can name, variation 0 first
#[cfg(test)]
pub(crate) const DOUBLE_BIT_BINARY_INPUT_EVENT_READS: &[Variation] = &[
    Variation::Group4Var0,
    Variation::Group4Var1,
    Variation::Group4Var2,
    Variation::Group4Var3,
];

/// The variations a READ of the static binary output status points can name, variation 0 first
#[cfg(test)]
pub(crate) const BINARY_OUTPUT_STATUS_STATIC_READS: &[Variation] = &[
    Variation::Group10Var0,
    Variation::Group10Var1,
    Variation::Group10Var2,
];

/// The variations a READ of the binary output status events can name, variation 0 first
#[cfg(test)]
pub(crate) const BINARY_OUTPUT_STATUS_EVENT_READS: &[Variation] = &[
    Variation::Group11Var0,
    Variation::Group11Var1,
    Variation::Group11Var2,
];

/// The variations a READ of the static counter points can name, variation 0 first
#[cfg(test)]
pub(crate) const COUNTER_STATIC_READS: &[Variation] = &[
    Variation::Group20Var0,
    Variation::Group20Var1,
    Variation::Group20Var2,
    Variation::Group20Var5,
    Variation::Group20Var6,
];

/// The variations a READ of the counter events can name, variation 0 first
#[cfg(test)]
pub(crate) const COUNTER_EVENT_READS: &[Variation] = &[
    Variation::Group22Var0,
    Variation::Group22Var1,
    Variation::Group22Var2,
    Variation::Group22Var5,
    Variation::Group22Var6,
];

/// The variations a READ of the static frozen counter points can name, variation 0 first
#[cfg(test)]
pub(crate) const FROZEN_COUNTER_STATIC_READS: &[Variation] = &[
    Variation::Group21Var0,
    Variation::Group21Var1,
    Variation::Group21Var2,
    Variation::Group21Var5,
    Variation::Group21Var6,
    Variation::Group21Var9,
    Variation::Group21Var10,
];

/// The variations a READ of the frozen counter events can name, variation 0 first
#[cfg(test)]
pub(crate) const FROZEN_COUNTER_EVENT_READS: &[Variation] = &[
    Variation::Group23Var0,
    Variation::Group23Var1,
    Variation::Group23Var2,
    Variation::Group23Var5,
    Variation::Group23Var6,
];

/// The variations a READ of the static analog input points can name, variation 0 first
#[cfg(test)]
pub(crate) const ANALOG_INPUT_STATIC_READS: &[Variation] = &[
    Variation::Group30Var0,
    Variation::Group30Var1,
    Variation::Group30Var2,
    Variation::Group30Var3,
    Variation::Group30Var4,
    Variation::Group30Var5,
    Variation::Group30Var6,
];

/// The variations a READ of the analog input events can name, variation 0 first
#[cfg(test)]
pub(crate) const ANALOG_INPUT_EVENT_READS: &[Variation] = &[
    Variation::Group32Var0,
    Variation::Group32Var1,
    Variation::Group32Var2,
    Variation::Group32Var3,
    Variation::Group32Var4,
    Variation::Group32Var5,
    Variation::Group32Var6,
    Variation::Group32Var7,
    Variation::Group32Var8,
];

/// The variations a READ of the static analog output status points can name, variation 0 first
#[cfg(test)]
pub(crate) const ANALOG_OUTPUT_STATUS_STATIC_READS: &[Variation] = &[
    Variation::Group40Var0,
    Variation::Group40Var1,
    Variation::Group40Var2,
    Variation::Group40Var3,
    Variation::Group40Var4,
];

/// The variations a READ of the analog output status events can name, variation 0 first
#[cfg(test)]
pub(crate) const ANALOG_OUTPUT_STATUS_EVENT_READS: &[Variation] = &[
    Variation::Group42Var0,
    Variation::Group42Var1,
    Variation::Group42Var2,
    Variation::Group42Var3,
    Variation::Group42Var4,
    Variation::Group42Var5,
    Variation::Group42Var6,
    Variation::Group42Var7,
    Variation::Group42Var8,
];

/// The variations a READ of the static analog input deadband points can name, variation 0 first
#[cfg(test)]
pub(crate) const ANALOG_INPUT_DEAD_BAND_STATIC_READS: &[Variation] = &[
    Variation::Group34Var0,
    Variation::Group34Var1,
    Variation::Group34Var2,
    Variation::Group34Var3,
];

/// The variations a READ of the static octet string points can name, variation 0 first
#[cfg(test)]
pub(crate) const OCTET_STRING_STATIC_READS: &[Variation] = &[Variation::Group110(0)];

/// The variations a READ of the octet string events can name, variation 0 first
#[cfg(test)]
pub(crate) const OCTET_STRING_EVENT_READS: &[Variation] = &[Variation::Group111(0)];
