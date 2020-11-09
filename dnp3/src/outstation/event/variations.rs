use crate::app::measurement::*;
use crate::app::variations::*;
use crate::outstation::event::traits::EventVariation;
use crate::outstation::event::write_fn::*;
use crate::outstation::event::writer::HeaderType;
use crate::util::cursor::{WriteCursor, WriteError};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum BinaryEventVariation {
    Group2Var1,
    Group2Var2,
    Group2Var3,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum BinaryOutputStatusEventVariation {
    Group11Var1,
    Group11Var2,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DoubleBitBinaryEventVariation {
    Group4Var1,
    Group4Var2,
    Group4Var3,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum CounterEventVariation {
    Group22Var1,
    Group22Var2,
    Group22Var5,
    Group22Var6,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FrozenCounterEventVariation {
    Group23Var1,
    Group23Var2,
    Group23Var5,
    Group23Var6,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum AnalogEventVariation {
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
pub enum AnalogOutputStatusEventVariation {
    Group42Var1,
    Group42Var2,
    Group42Var3,
    Group42Var4,
    Group42Var5,
    Group42Var6,
    Group42Var7,
    Group42Var8,
}

impl EventVariation<Binary> for BinaryEventVariation {
    fn write(
        &self,
        cursor: &mut WriteCursor,
        event: &Binary,
        index: u16,
        cto: Time,
    ) -> Result<Continue, WriteError> {
        match self {
            Self::Group2Var1 => write_fixed_size::<Group2Var1, Binary>(cursor, event, index, cto),
            Self::Group2Var2 => write_fixed_size::<Group2Var2, Binary>(cursor, event, index, cto),
            Self::Group2Var3 => write_cto::<Group2Var3, Binary>(cursor, event, index, cto),
        }
    }

    fn wrap(&self) -> HeaderType {
        HeaderType::Binary(*self)
    }

    fn get_group_var(&self) -> (u8, u8) {
        match self {
            Self::Group2Var1 => (2, 1),
            Self::Group2Var2 => (2, 2),
            Self::Group2Var3 => (2, 3),
        }
    }

    fn uses_cto(&self) -> bool {
        std::matches!(self, Self::Group2Var3)
    }
}

impl EventVariation<BinaryOutputStatus> for BinaryOutputStatusEventVariation {
    fn write(
        &self,
        cursor: &mut WriteCursor,
        event: &BinaryOutputStatus,
        index: u16,
        cto: Time,
    ) -> Result<Continue, WriteError> {
        match self {
            Self::Group11Var1 => {
                write_fixed_size::<Group11Var1, BinaryOutputStatus>(cursor, event, index, cto)
            }
            Self::Group11Var2 => {
                write_fixed_size::<Group11Var2, BinaryOutputStatus>(cursor, event, index, cto)
            }
        }
    }

    fn wrap(&self) -> HeaderType {
        HeaderType::BinaryOutputStatus(*self)
    }

    fn get_group_var(&self) -> (u8, u8) {
        match self {
            Self::Group11Var1 => (11, 1),
            Self::Group11Var2 => (11, 2),
        }
    }
}

impl EventVariation<DoubleBitBinary> for DoubleBitBinaryEventVariation {
    fn write(
        &self,
        cursor: &mut WriteCursor,
        event: &DoubleBitBinary,
        index: u16,
        cto: Time,
    ) -> Result<Continue, WriteError> {
        match self {
            Self::Group4Var1 => {
                write_fixed_size::<Group4Var1, DoubleBitBinary>(cursor, event, index, cto)
            }
            Self::Group4Var2 => {
                write_fixed_size::<Group4Var2, DoubleBitBinary>(cursor, event, index, cto)
            }
            Self::Group4Var3 => write_cto::<Group4Var3, DoubleBitBinary>(cursor, event, index, cto),
        }
    }

    fn wrap(&self) -> HeaderType {
        HeaderType::DoubleBitBinary(*self)
    }

    fn get_group_var(&self) -> (u8, u8) {
        match self {
            Self::Group4Var1 => (4, 1),
            Self::Group4Var2 => (4, 2),
            Self::Group4Var3 => (4, 3),
        }
    }

    fn uses_cto(&self) -> bool {
        std::matches!(self, Self::Group4Var3)
    }
}

impl EventVariation<Counter> for CounterEventVariation {
    fn write(
        &self,
        cursor: &mut WriteCursor,
        event: &Counter,
        index: u16,
        cto: Time,
    ) -> Result<Continue, WriteError> {
        match self {
            Self::Group22Var1 => {
                write_fixed_size::<Group22Var1, Counter>(cursor, event, index, cto)
            }
            Self::Group22Var2 => {
                write_fixed_size::<Group22Var2, Counter>(cursor, event, index, cto)
            }
            Self::Group22Var5 => {
                write_fixed_size::<Group22Var5, Counter>(cursor, event, index, cto)
            }
            Self::Group22Var6 => {
                write_fixed_size::<Group22Var6, Counter>(cursor, event, index, cto)
            }
        }
    }

    fn wrap(&self) -> HeaderType {
        HeaderType::Counter(*self)
    }

    fn get_group_var(&self) -> (u8, u8) {
        match self {
            Self::Group22Var1 => (22, 1),
            Self::Group22Var2 => (22, 2),
            Self::Group22Var5 => (22, 5),
            Self::Group22Var6 => (22, 6),
        }
    }
}

impl EventVariation<FrozenCounter> for FrozenCounterEventVariation {
    fn write(
        &self,
        cursor: &mut WriteCursor,
        event: &FrozenCounter,
        index: u16,
        cto: Time,
    ) -> Result<Continue, WriteError> {
        match self {
            Self::Group23Var1 => {
                write_fixed_size::<Group23Var1, FrozenCounter>(cursor, event, index, cto)
            }
            Self::Group23Var2 => {
                write_fixed_size::<Group23Var2, FrozenCounter>(cursor, event, index, cto)
            }
            Self::Group23Var5 => {
                write_fixed_size::<Group23Var5, FrozenCounter>(cursor, event, index, cto)
            }
            Self::Group23Var6 => {
                write_fixed_size::<Group23Var6, FrozenCounter>(cursor, event, index, cto)
            }
        }
    }

    fn wrap(&self) -> HeaderType {
        HeaderType::FrozenCounter(*self)
    }

    fn get_group_var(&self) -> (u8, u8) {
        match self {
            Self::Group23Var1 => (23, 1),
            Self::Group23Var2 => (23, 2),
            Self::Group23Var5 => (23, 5),
            Self::Group23Var6 => (23, 6),
        }
    }
}

impl EventVariation<Analog> for AnalogEventVariation {
    fn write(
        &self,
        cursor: &mut WriteCursor,
        event: &Analog,
        index: u16,
        cto: Time,
    ) -> Result<Continue, WriteError> {
        match self {
            Self::Group32Var1 => write_fixed_size::<Group32Var1, Analog>(cursor, event, index, cto),
            Self::Group32Var2 => write_fixed_size::<Group32Var2, Analog>(cursor, event, index, cto),
            Self::Group32Var3 => write_fixed_size::<Group32Var3, Analog>(cursor, event, index, cto),
            Self::Group32Var4 => write_fixed_size::<Group32Var4, Analog>(cursor, event, index, cto),
            Self::Group32Var5 => write_fixed_size::<Group32Var5, Analog>(cursor, event, index, cto),
            Self::Group32Var6 => write_fixed_size::<Group32Var6, Analog>(cursor, event, index, cto),
            Self::Group32Var7 => write_fixed_size::<Group32Var7, Analog>(cursor, event, index, cto),
            Self::Group32Var8 => write_fixed_size::<Group32Var8, Analog>(cursor, event, index, cto),
        }
    }

    fn wrap(&self) -> HeaderType {
        HeaderType::Analog(*self)
    }

    fn get_group_var(&self) -> (u8, u8) {
        match self {
            Self::Group32Var1 => (32, 1),
            Self::Group32Var2 => (32, 2),
            Self::Group32Var3 => (32, 3),
            Self::Group32Var4 => (32, 4),
            Self::Group32Var5 => (32, 5),
            Self::Group32Var6 => (32, 6),
            Self::Group32Var7 => (32, 7),
            Self::Group32Var8 => (32, 8),
        }
    }
}

impl EventVariation<AnalogOutputStatus> for AnalogOutputStatusEventVariation {
    fn write(
        &self,
        cursor: &mut WriteCursor,
        event: &AnalogOutputStatus,
        index: u16,
        cto: Time,
    ) -> Result<Continue, WriteError> {
        match self {
            Self::Group42Var1 => {
                write_fixed_size::<Group42Var1, AnalogOutputStatus>(cursor, event, index, cto)
            }
            Self::Group42Var2 => {
                write_fixed_size::<Group42Var2, AnalogOutputStatus>(cursor, event, index, cto)
            }
            Self::Group42Var3 => {
                write_fixed_size::<Group42Var3, AnalogOutputStatus>(cursor, event, index, cto)
            }
            Self::Group42Var4 => {
                write_fixed_size::<Group42Var4, AnalogOutputStatus>(cursor, event, index, cto)
            }
            Self::Group42Var5 => {
                write_fixed_size::<Group42Var5, AnalogOutputStatus>(cursor, event, index, cto)
            }
            Self::Group42Var6 => {
                write_fixed_size::<Group42Var6, AnalogOutputStatus>(cursor, event, index, cto)
            }
            Self::Group42Var7 => {
                write_fixed_size::<Group42Var7, AnalogOutputStatus>(cursor, event, index, cto)
            }
            Self::Group42Var8 => {
                write_fixed_size::<Group42Var8, AnalogOutputStatus>(cursor, event, index, cto)
            }
        }
    }

    fn wrap(&self) -> HeaderType {
        HeaderType::AnalogOutputStatus(*self)
    }

    fn get_group_var(&self) -> (u8, u8) {
        match self {
            Self::Group42Var1 => (42, 1),
            Self::Group42Var2 => (42, 2),
            Self::Group42Var3 => (42, 3),
            Self::Group42Var4 => (42, 4),
            Self::Group42Var5 => (42, 5),
            Self::Group42Var6 => (42, 6),
            Self::Group42Var7 => (42, 7),
            Self::Group42Var8 => (42, 8),
        }
    }
}
