use crate::app::measurement::*;
use crate::outstation::event::variations::*;
use crate::outstation::event::write_fn::Continue;
use crate::outstation::event::writer::HeaderType;
use crate::util::cursor::{WriteCursor, WriteError};

pub(crate) trait EventVariation<T> {
    fn write(
        &self,
        cursor: &mut WriteCursor,
        event: &T,
        index: u16,
        cto: Time,
    ) -> Result<Continue, WriteError>;
    fn wrap(&self) -> HeaderType;
    fn get_group_var(&self) -> (u8, u8);
    fn uses_cto(&self) -> bool {
        false
    }
}

pub(crate) trait BaseEvent: Sized {
    type Variation: Copy + PartialEq + EventVariation<Self>;
}

impl BaseEvent for Binary {
    type Variation = BinaryEventVariation;
}

impl BaseEvent for DoubleBitBinary {
    type Variation = DoubleBitBinaryEventVariation;
}

impl BaseEvent for BinaryOutputStatus {
    type Variation = BinaryOutputStatusEventVariation;
}

impl BaseEvent for Counter {
    type Variation = CounterEventVariation;
}

impl BaseEvent for FrozenCounter {
    type Variation = FrozenCounterEventVariation;
}

impl BaseEvent for Analog {
    type Variation = AnalogEventVariation;
}

impl BaseEvent for AnalogOutputStatus {
    type Variation = AnalogOutputStatusEventVariation;
}
