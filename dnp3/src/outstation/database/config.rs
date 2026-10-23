// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::outstation::database::variations::*;

// This is always g111vX
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub(crate) struct EventOctetStringVariation;

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
    ///
    /// Must be zero or positive, otherwise the point is not added
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
    ///
    /// Must be zero or positive, otherwise the point is not added
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
