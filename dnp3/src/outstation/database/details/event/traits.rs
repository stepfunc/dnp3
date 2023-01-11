use crate::app::measurement::*;
use crate::app::variations::*;
use crate::outstation::database::config::*;
use crate::outstation::database::details::event::write_fn::{
    write_cto, write_fixed_size, Continue,
};
use crate::outstation::database::details::event::writer::HeaderType;

use scursor::{WriteCursor, WriteError};

use super::write_fn::write_octet_string;

#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) struct OctetStringLength(pub(crate) usize);

pub(crate) trait EventVariation<T> {
    fn write(
        &self,
        cursor: &mut WriteCursor,
        event: &T,
        index: u16,
        cto: Time,
    ) -> Result<Continue, WriteError>;
    fn wrap(&self) -> HeaderType;
    fn get_group_var(&self, event: &T) -> (u8, u8);
    fn uses_cto(&self) -> bool {
        false
    }
}

impl EventVariation<BinaryInput> for EventBinaryInputVariation {
    fn write(
        &self,
        cursor: &mut WriteCursor,
        event: &BinaryInput,
        index: u16,
        cto: Time,
    ) -> Result<Continue, WriteError> {
        match self {
            Self::Group2Var1 => {
                write_fixed_size::<Group2Var1, BinaryInput>(cursor, event, index, cto)
            }
            Self::Group2Var2 => {
                write_fixed_size::<Group2Var2, BinaryInput>(cursor, event, index, cto)
            }
            Self::Group2Var3 => write_cto::<Group2Var3, BinaryInput>(cursor, event, index, cto),
        }
    }

    fn wrap(&self) -> HeaderType {
        HeaderType::Binary(*self)
    }

    fn get_group_var(&self, _event: &BinaryInput) -> (u8, u8) {
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

impl EventVariation<BinaryOutputStatus> for EventBinaryOutputStatusVariation {
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

    fn get_group_var(&self, _event: &BinaryOutputStatus) -> (u8, u8) {
        match self {
            Self::Group11Var1 => (11, 1),
            Self::Group11Var2 => (11, 2),
        }
    }
}

impl EventVariation<DoubleBitBinaryInput> for EventDoubleBitBinaryInputVariation {
    fn write(
        &self,
        cursor: &mut WriteCursor,
        event: &DoubleBitBinaryInput,
        index: u16,
        cto: Time,
    ) -> Result<Continue, WriteError> {
        match self {
            Self::Group4Var1 => {
                write_fixed_size::<Group4Var1, DoubleBitBinaryInput>(cursor, event, index, cto)
            }
            Self::Group4Var2 => {
                write_fixed_size::<Group4Var2, DoubleBitBinaryInput>(cursor, event, index, cto)
            }
            Self::Group4Var3 => {
                write_cto::<Group4Var3, DoubleBitBinaryInput>(cursor, event, index, cto)
            }
        }
    }

    fn wrap(&self) -> HeaderType {
        HeaderType::DoubleBitBinary(*self)
    }

    fn get_group_var(&self, _event: &DoubleBitBinaryInput) -> (u8, u8) {
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

impl EventVariation<Counter> for EventCounterVariation {
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

    fn get_group_var(&self, _event: &Counter) -> (u8, u8) {
        match self {
            Self::Group22Var1 => (22, 1),
            Self::Group22Var2 => (22, 2),
            Self::Group22Var5 => (22, 5),
            Self::Group22Var6 => (22, 6),
        }
    }
}

impl EventVariation<FrozenCounter> for EventFrozenCounterVariation {
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

    fn get_group_var(&self, _event: &FrozenCounter) -> (u8, u8) {
        match self {
            Self::Group23Var1 => (23, 1),
            Self::Group23Var2 => (23, 2),
            Self::Group23Var5 => (23, 5),
            Self::Group23Var6 => (23, 6),
        }
    }
}

impl EventVariation<AnalogInput> for EventAnalogInputVariation {
    fn write(
        &self,
        cursor: &mut WriteCursor,
        event: &AnalogInput,
        index: u16,
        cto: Time,
    ) -> Result<Continue, WriteError> {
        match self {
            Self::Group32Var1 => {
                write_fixed_size::<Group32Var1, AnalogInput>(cursor, event, index, cto)
            }
            Self::Group32Var2 => {
                write_fixed_size::<Group32Var2, AnalogInput>(cursor, event, index, cto)
            }
            Self::Group32Var3 => {
                write_fixed_size::<Group32Var3, AnalogInput>(cursor, event, index, cto)
            }
            Self::Group32Var4 => {
                write_fixed_size::<Group32Var4, AnalogInput>(cursor, event, index, cto)
            }
            Self::Group32Var5 => {
                write_fixed_size::<Group32Var5, AnalogInput>(cursor, event, index, cto)
            }
            Self::Group32Var6 => {
                write_fixed_size::<Group32Var6, AnalogInput>(cursor, event, index, cto)
            }
            Self::Group32Var7 => {
                write_fixed_size::<Group32Var7, AnalogInput>(cursor, event, index, cto)
            }
            Self::Group32Var8 => {
                write_fixed_size::<Group32Var8, AnalogInput>(cursor, event, index, cto)
            }
        }
    }

    fn wrap(&self) -> HeaderType {
        HeaderType::Analog(*self)
    }

    fn get_group_var(&self, _event: &AnalogInput) -> (u8, u8) {
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

impl EventVariation<AnalogOutputStatus> for EventAnalogOutputStatusVariation {
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

    fn get_group_var(&self, _event: &AnalogOutputStatus) -> (u8, u8) {
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

impl EventVariation<Box<[u8]>> for OctetStringLength {
    fn write(
        &self,
        cursor: &mut WriteCursor,
        event: &Box<[u8]>,
        index: u16,
        _cto: Time,
    ) -> Result<Continue, WriteError> {
        write_octet_string(cursor, event, index)
    }

    fn wrap(&self) -> HeaderType {
        HeaderType::OctetString(*self)
    }

    fn get_group_var(&self, event: &Box<[u8]>) -> (u8, u8) {
        (111, event.len() as u8)
    }
}
