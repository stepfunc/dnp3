use oo_bindgen::{LibraryBuilder, BindingError};
use oo_bindgen::constants::{ConstantValue, Representation};

mod bits {
    pub(crate) const BIT_0: u8 = 0b0000_0001;
    pub(crate) const BIT_1: u8 = 0b0000_0010;
    pub(crate) const BIT_2: u8 = 0b0000_0100;
    pub(crate) const BIT_3: u8 = 0b0000_1000;
    pub(crate) const BIT_4: u8 = 0b0001_0000;
    pub(crate) const BIT_5: u8 = 0b0010_0000;
    pub(crate) const BIT_6: u8 = 0b0100_0000;
}

pub(crate) fn define(lib: &mut LibraryBuilder) -> Result<(), BindingError> {

    use bits::*;

    lib.define_constants("Flag")?
        .add("Online", ConstantValue::U8(BIT_0,Representation::Hex), "Object value is 'good' / 'valid' / 'nominal'")?
        .add("Restart", ConstantValue::U8(BIT_1,Representation::Hex), "Object value has not been updated since device restart")?
        .add("CommLost", ConstantValue::U8(BIT_2,Representation::Hex), "Object value represents the last value available before a communication failure occurred. Should never be set by originating devices")?
        .add("RemoteForced", ConstantValue::U8(BIT_3,Representation::Hex), "Object value is overridden in a downstream reporting device")?
        .add("LocalForced", ConstantValue::U8(BIT_4,Representation::Hex), "Object value is overridden by the device reporting this flag")?
        .add("ChatterFilter", ConstantValue::U8(BIT_5,Representation::Hex), "Object value is changing state rapidly (device dependent meaning)")?
        .add("OverRange", ConstantValue::U8(BIT_5,Representation::Hex), "Object's true exceeds the measurement range of the reported variation")?
        .add("Discontinuity", ConstantValue::U8(BIT_6,Representation::Hex), "Reported counter value cannot be compared against a prior value to obtain the correct count difference")?
        .add("ReferenceErr", ConstantValue::U8(BIT_6,Representation::Hex), "Object's value might not have the expected level of accuracy")?
        .doc("Individual flag constants that may be combined using bitwise-OR operator")?
        .build()
}