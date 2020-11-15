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
