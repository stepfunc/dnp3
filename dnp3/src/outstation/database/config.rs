/// Enum representing all possible `BinaryInput` event variations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum EventBinaryInputVariation {
    /// Binary Input Event - without time
    Group2Var1,
    /// Binary Input Event - with Absolute time
    Group2Var2,
    /// Binary Input Event - with Relative time
    Group2Var3,
}

/// Enum representing all possible `BinaryOutputStatus` event variations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum EventBinaryOutputStatusVariation {
    /// Binary Output Event - Output Status without time
    Group11Var1,
    /// Binary Output Event - Output Status with time
    Group11Var2,
}

/// Enum representing all possible `DoubleBitBinaryInput` event variations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum EventDoubleBitBinaryInputVariation {
    /// Double-bit Binary Input Event - without time
    Group4Var1,
    /// Double-bit Binary Input Event - with Absolute time
    Group4Var2,
    /// Double-bit Binary Input Event - with Relative time
    Group4Var3,
}

/// Enum representing all possible `Counter` event variations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum EventCounterVariation {
    /// Counter Event - 32-bit with flag
    Group22Var1,
    /// Counter Event - 16-bit with flag
    Group22Var2,
    /// Counter Event - 32-bit with flag and time
    Group22Var5,
    /// Counter Event - 16-bit with flag and time
    Group22Var6,
}

/// Enum representing all possible `FrozenCounter` event variations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum EventFrozenCounterVariation {
    /// Frozen Counter Event - 32-bit with flag
    Group23Var1,
    /// Frozen Counter Event - 16-bit with flag
    Group23Var2,
    /// Frozen Counter Event - 32-bit with flag and time
    Group23Var5,
    /// Frozen Counter Event - 16-bit with flag and time
    Group23Var6,
}

/// Enum representing all possible `AnalogInput` event variations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum EventAnalogInputVariation {
    /// Analog Input Event - 32-bit with flag
    Group32Var1,
    /// Analog Input Event - 16-bit with flag
    Group32Var2,
    /// Analog Input Event - 32-bit with flag and time
    Group32Var3,
    /// Analog Input Event - 16-bit with flag and time
    Group32Var4,
    /// Analog Input Event - Single-precision with flag
    Group32Var5,
    /// Analog Input Event - Double-precision with flag
    Group32Var6,
    /// Analog Input Event - Single-precision with flag and time
    Group32Var7,
    /// Analog Input Event - Double-precision with flag and time
    Group32Var8,
}

/// Enum representing all possible `FrozenAnalogInput` event variations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum EventFrozenAnalogInputVariation {
    /// Frozen Analog Input Event - 32-bit with flag
    Group33Var1,
    /// Frozen Analog Input Event - 16-bit with flag
    Group33Var2,
    /// Frozen Analog Input Event - 32-bit with flag and time
    Group33Var3,
    /// Frozen Analog Input Event - 16-bit with flag and time
    Group33Var4,
    /// Frozen Analog Input Event - Single-precision with flag
    Group33Var5,
    /// Frozen Analog Input Event - Double-precision with flag
    Group33Var6,
    /// Frozen Analog Input Event - Single-precision with flag and time
    Group33Var7,
    /// Frozen Analog Input Event - Double-precision with flag and time
    Group33Var8,
}

/// Enum representing all possible `AnalogOutputStatus` event variations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum EventAnalogOutputStatusVariation {
    /// Analog Output Event - 32-bit with flag
    Group42Var1,
    /// Analog Output Event - 16-bit with flag
    Group42Var2,
    /// Analog Output Event - 32-bit with flag and time
    Group42Var3,
    /// Analog Output Event - 16-bit with flag and time
    Group42Var4,
    /// Analog Output Event - Single-precision with flag
    Group42Var5,
    /// Analog Output Event - Double-precision with flag
    Group42Var6,
    /// Analog Output Event - Single-precision with flag and time
    Group42Var7,
    /// Analog Output Event - Double-precision with flag and time
    Group42Var8,
}

// This is always g111vX
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub(crate) struct EventOctetStringVariation;

/// Enum representing all possible `BinaryInput` static variations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum StaticBinaryInputVariation {
    /// Binary Input - Packed Format
    Group1Var1,
    /// Binary Input - with flags
    Group1Var2,
}

/// Enum representing all possible `BinaryOutputStatus` static variations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum StaticBinaryOutputStatusVariation {
    /// Binary Output - Packed Format
    Group10Var1,
    /// Binary Output - Output Status with flags
    Group10Var2,
}

/// Enum representing all possible `DoubleBitBinaryInput` static variations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum StaticDoubleBitBinaryInputVariation {
    /// Double-bit Binary Input - Packed Format
    Group3Var1,
    /// Double-bit Binary Input - with flags
    Group3Var2,
}

/// Enum representing all possible `Counter` static variations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum StaticCounterVariation {
    /// Counter - 32-bit with flag
    Group20Var1,
    /// Counter - 16-bit with flag
    Group20Var2,
    /// Counter - 32-bit without flag
    Group20Var5,
    /// Counter - 16-bit without flag
    Group20Var6,
}

/// Enum representing all possible `FrozenCounter` static variations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum StaticFrozenCounterVariation {
    /// Frozen Counter - 32-bit with flag
    Group21Var1,
    /// Frozen Counter - 16-bit with flag
    Group21Var2,
    /// Frozen Counter - 32-bit with flag and time
    Group21Var5,
    /// Frozen Counter - 16-bit with flag and time
    Group21Var6,
    /// Frozen Counter - 32-bit without flag
    Group21Var9,
    /// Frozen Counter - 16-bit without flag
    Group21Var10,
}

/// Enum representing all possible `AnalogInput` static variations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum StaticAnalogInputVariation {
    /// Analog Input - 32-bit with flag
    Group30Var1,
    /// Analog Input - 16-bit with flag
    Group30Var2,
    /// Analog Input - 32-bit without flag
    Group30Var3,
    /// Analog Input - 16-bit without flag
    Group30Var4,
    /// Analog Input - Single-precision with flag
    Group30Var5,
    /// Analog Input - Double-precision with flag
    Group30Var6,
}

/// Enum representing all possible `FrozenAnalogInput` static variations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum StaticFrozenAnalogInputVariation {
    /// Frozen Analog Input - 32-bit with flag
    Group31Var1,
    /// Frozen Analog Input - 16-bit with flag
    Group31Var2,
    /// Frozen Analog Input - 32-bit with flag and time-of-freeze
    Group31Var3,
    /// Frozen Analog Input - 32-bit with flag and time-of-freeze
    Group31Var4,
    /// Frozen Analog Input - 32-bit without flag
    Group31Var5,
    /// Frozen Analog Input - 16-bit without flag
    Group31Var6,
    /// Frozen Analog Input - Single-precision with flag
    Group31Var7,
    /// Frozen Analog Input - Double-precision with flag
    Group31Var8,
}

/// Enum representing all possible `AnalogOutputStatus` static variations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum StaticAnalogOutputStatusVariation {
    /// Analog Output Status - 32-bit with flag
    Group40Var1,
    /// Analog Output Status - 16-bit with flag
    Group40Var2,
    /// Analog Output Status - Single-precision with flag
    Group40Var3,
    /// Analog Output Status - Double-precision with flag
    Group40Var4,
}

/// this is not currently public, but it could be a future release
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub(crate) enum AnalogInputDeadBandVariation {
    /// Analog Input Dead-band - 16-bit
    Group34Var1,
    /// Analog Input Dead-band - 32-bit
    Group34Var2,
    /// Analog Input Dead-band - Single-precision floating-point
    Group34Var3,
}

// This is always g110vX
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub(crate) struct StaticOctetStringVariation;

/// configuration for a `BinaryInput` point
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct BinaryInputConfig {
    /// default static variation
    pub s_var: StaticBinaryInputVariation,
    /// default event variation
    pub e_var: EventBinaryInputVariation,
}

/// configuration for a `DoubleBitBinaryInput` point
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct DoubleBitBinaryInputConfig {
    /// default static variation
    pub s_var: StaticDoubleBitBinaryInputVariation,
    /// default event variation
    pub e_var: EventDoubleBitBinaryInputVariation,
}

/// configuration for a `BinaryOutputStatus` point
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct BinaryOutputStatusConfig {
    /// default static variation
    pub s_var: StaticBinaryOutputStatusVariation,
    /// default event variation
    pub e_var: EventBinaryOutputStatusVariation,
}

/// configuration for a `Counter` point
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct CounterConfig {
    /// default static variation
    pub s_var: StaticCounterVariation,
    /// default event variation
    pub e_var: EventCounterVariation,
    /// deadband - value of 0 means that any change will trigger an event
    pub deadband: u32,
}

/// configuration for a `FrozenCounter` point
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct FrozenCounterConfig {
    /// default static variation
    pub s_var: StaticFrozenCounterVariation,
    /// default event variation
    pub e_var: EventFrozenCounterVariation,
    /// deadband - value of 0 means that any change will trigger an event
    pub deadband: u32,
}

/// configuration for an `AnalogInput` point
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct AnalogInputConfig {
    /// default static variation
    pub s_var: StaticAnalogInputVariation,
    /// default event variation
    pub e_var: EventAnalogInputVariation,
    /// deadband - value of 0 means that any change will trigger an event
    pub deadband: f64,
}

/// configuration for an `AnalogOutputStatus` point
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct AnalogOutputStatusConfig {
    /// default static variation
    pub s_var: StaticAnalogOutputStatusVariation,
    /// default event variation
    pub e_var: EventAnalogOutputStatusVariation,
    /// deadband - value of 0 means that any change will trigger an event
    pub deadband: f64,
}

/// Octet strings don't actually need any configuration b/c the transmitted variation is determined
/// by the size. This struct is more of a placeholder required by a couple of internal traits.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct OctetStringConfig;

impl BinaryInputConfig {
    /// construct a `BinaryConfig` from its fields
    pub fn new(s_var: StaticBinaryInputVariation, e_var: EventBinaryInputVariation) -> Self {
        Self { s_var, e_var }
    }
}

impl DoubleBitBinaryInputConfig {
    /// construct a `DoubleBitBinaryConfig` from its fields
    pub fn new(
        s_var: StaticDoubleBitBinaryInputVariation,
        e_var: EventDoubleBitBinaryInputVariation,
    ) -> Self {
        Self { s_var, e_var }
    }
}

impl BinaryOutputStatusConfig {
    /// construct a `BinaryOutputStatusConfig` from its fields
    pub fn new(
        s_var: StaticBinaryOutputStatusVariation,
        e_var: EventBinaryOutputStatusVariation,
    ) -> Self {
        Self { s_var, e_var }
    }
}

impl CounterConfig {
    /// construct a `CounterConfig` from its fields
    pub fn new(s_var: StaticCounterVariation, e_var: EventCounterVariation, deadband: u32) -> Self {
        Self {
            s_var,
            e_var,
            deadband,
        }
    }
}

impl FrozenCounterConfig {
    /// construct a `FrozenCounterConfig` from its fields
    pub fn new(
        s_var: StaticFrozenCounterVariation,
        e_var: EventFrozenCounterVariation,
        deadband: u32,
    ) -> Self {
        Self {
            s_var,
            e_var,
            deadband,
        }
    }
}

impl AnalogInputConfig {
    /// construct an `AnalogConfig` from its fields
    pub fn new(
        s_var: StaticAnalogInputVariation,
        e_var: EventAnalogInputVariation,
        deadband: f64,
    ) -> Self {
        Self {
            s_var,
            e_var,
            deadband,
        }
    }
}

impl AnalogOutputStatusConfig {
    /// construct an `AnalogOutputStatusConfig` from its fields
    pub fn new(
        s_var: StaticAnalogOutputStatusVariation,
        e_var: EventAnalogOutputStatusVariation,
        deadband: f64,
    ) -> Self {
        Self {
            s_var,
            e_var,
            deadband,
        }
    }
}

impl Default for BinaryInputConfig {
    fn default() -> Self {
        Self::new(
            StaticBinaryInputVariation::Group1Var1,
            EventBinaryInputVariation::Group2Var1,
        )
    }
}

impl Default for DoubleBitBinaryInputConfig {
    fn default() -> Self {
        Self::new(
            StaticDoubleBitBinaryInputVariation::Group3Var1,
            EventDoubleBitBinaryInputVariation::Group4Var1,
        )
    }
}

impl Default for BinaryOutputStatusConfig {
    fn default() -> Self {
        Self::new(
            StaticBinaryOutputStatusVariation::Group10Var1,
            EventBinaryOutputStatusVariation::Group11Var2,
        )
    }
}

impl Default for CounterConfig {
    fn default() -> Self {
        Self::new(
            StaticCounterVariation::Group20Var1,
            EventCounterVariation::Group22Var1,
            0,
        )
    }
}

impl Default for FrozenCounterConfig {
    fn default() -> Self {
        Self::new(
            StaticFrozenCounterVariation::Group21Var1,
            EventFrozenCounterVariation::Group23Var1,
            0,
        )
    }
}

impl Default for AnalogInputConfig {
    fn default() -> Self {
        Self::new(
            StaticAnalogInputVariation::Group30Var1,
            EventAnalogInputVariation::Group32Var1,
            0.0,
        )
    }
}

impl Default for AnalogOutputStatusConfig {
    fn default() -> Self {
        Self::new(
            StaticAnalogOutputStatusVariation::Group40Var1,
            EventAnalogOutputStatusVariation::Group42Var1,
            0.0,
        )
    }
}
