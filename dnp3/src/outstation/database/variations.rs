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
