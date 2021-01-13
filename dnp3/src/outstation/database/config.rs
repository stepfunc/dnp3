#[derive(Copy, Clone, Debug, PartialEq)]
pub enum EventBinaryVariation {
    Group2Var1,
    Group2Var2,
    Group2Var3,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum EventBinaryOutputStatusVariation {
    Group11Var1,
    Group11Var2,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum EventDoubleBitBinaryVariation {
    Group4Var1,
    Group4Var2,
    Group4Var3,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum EventCounterVariation {
    Group22Var1,
    Group22Var2,
    Group22Var5,
    Group22Var6,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum EventFrozenCounterVariation {
    Group23Var1,
    Group23Var2,
    Group23Var5,
    Group23Var6,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum EventAnalogVariation {
    Group32Var1,
    Group32Var2,
    Group32Var3,
    Group32Var4,
    Group32Var5,
    Group32Var6,
    Group32Var7,
    Group32Var8,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum EventAnalogOutputStatusVariation {
    Group42Var1,
    Group42Var2,
    Group42Var3,
    Group42Var4,
    Group42Var5,
    Group42Var6,
    Group42Var7,
    Group42Var8,
}

// This is always g111vX
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct EventOctetStringVariation;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum StaticBinaryVariation {
    Group1Var1,
    Group1Var2,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum StaticBinaryOutputStatusVariation {
    Group10Var1,
    Group10Var2,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum StaticDoubleBitBinaryVariation {
    Group3Var1,
    Group3Var2,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum StaticCounterVariation {
    Group20Var1,
    Group20Var2,
    Group20Var5,
    Group20Var6,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum StaticFrozenCounterVariation {
    Group21Var1,
    Group21Var2,
    Group21Var5,
    Group21Var6,
    Group21Var9,
    Group21Var10,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum StaticAnalogVariation {
    Group30Var1,
    Group30Var2,
    Group30Var3,
    Group30Var4,
    Group30Var5,
    Group30Var6,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum StaticAnalogOutputStatusVariation {
    Group40Var1,
    Group40Var2,
    Group40Var3,
    Group40Var4,
}

// This is always g110vX
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct StaticOctetStringVariation;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct BinaryConfig {
    pub s_var: StaticBinaryVariation,
    pub e_var: EventBinaryVariation,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct DoubleBitBinaryConfig {
    pub s_var: StaticDoubleBitBinaryVariation,
    pub e_var: EventDoubleBitBinaryVariation,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct BinaryOutputStatusConfig {
    pub s_var: StaticBinaryOutputStatusVariation,
    pub e_var: EventBinaryOutputStatusVariation,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CounterConfig {
    pub s_var: StaticCounterVariation,
    pub e_var: EventCounterVariation,
    pub deadband: u32,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FrozenCounterConfig {
    pub s_var: StaticFrozenCounterVariation,
    pub e_var: EventFrozenCounterVariation,
    pub deadband: u32,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AnalogConfig {
    pub s_var: StaticAnalogVariation,
    pub e_var: EventAnalogVariation,
    pub deadband: f64,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AnalogOutputStatusConfig {
    pub s_var: StaticAnalogOutputStatusVariation,
    pub e_var: EventAnalogOutputStatusVariation,
    pub deadband: f64,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct OctetStringConfig;

impl BinaryConfig {
    pub fn new(s_var: StaticBinaryVariation, e_var: EventBinaryVariation) -> Self {
        Self { s_var, e_var }
    }
}

impl DoubleBitBinaryConfig {
    pub fn new(
        s_var: StaticDoubleBitBinaryVariation,
        e_var: EventDoubleBitBinaryVariation,
    ) -> Self {
        Self { s_var, e_var }
    }
}

impl BinaryOutputStatusConfig {
    pub fn new(
        s_var: StaticBinaryOutputStatusVariation,
        e_var: EventBinaryOutputStatusVariation,
    ) -> Self {
        Self { s_var, e_var }
    }
}

impl CounterConfig {
    pub fn new(s_var: StaticCounterVariation, e_var: EventCounterVariation, deadband: u32) -> Self {
        Self {
            s_var,
            e_var,
            deadband,
        }
    }
}

impl FrozenCounterConfig {
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

impl AnalogConfig {
    pub fn new(s_var: StaticAnalogVariation, e_var: EventAnalogVariation, deadband: f64) -> Self {
        Self {
            s_var,
            e_var,
            deadband,
        }
    }
}

impl AnalogOutputStatusConfig {
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

impl Default for BinaryConfig {
    fn default() -> Self {
        Self::new(
            StaticBinaryVariation::Group1Var1,
            EventBinaryVariation::Group2Var1,
        )
    }
}

impl Default for DoubleBitBinaryConfig {
    fn default() -> Self {
        Self::new(
            StaticDoubleBitBinaryVariation::Group3Var1,
            EventDoubleBitBinaryVariation::Group4Var1,
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

impl Default for AnalogConfig {
    fn default() -> Self {
        Self::new(
            StaticAnalogVariation::Group30Var1,
            EventAnalogVariation::Group32Var1,
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
