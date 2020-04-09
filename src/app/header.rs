use crate::app::gen::enums::FunctionCode;
use crate::app::sequence::Sequence;
use crate::util::bit::{format_bitfield, Bitfield};
use crate::util::cursor::{ReadCursor, ReadError, WriteCursor, WriteError};
use std::fmt::Formatter;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Control {
    pub fir: bool,
    pub fin: bool,
    pub con: bool,
    pub uns: bool,
    pub seq: Sequence,
}

impl std::fmt::Display for Control {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "[fir: {} fin: {} con: {} uns: {} seq: {}]",
            self.fir,
            self.fin,
            self.con,
            self.uns,
            self.seq.value()
        )
    }
}

impl Control {
    const FIR_MASK: u8 = 0b1000_0000;
    const FIN_MASK: u8 = 0b0100_0000;
    const CON_MASK: u8 = 0b0010_0000;
    const UNS_MASK: u8 = 0b0001_0000;

    pub fn request(seq: Sequence) -> Self {
        Self {
            fir: true,
            fin: true,
            con: false,
            uns: false,
            seq,
        }
    }

    pub fn unsolicited(seq: Sequence) -> Self {
        Self {
            fir: true,
            fin: true,
            con: false,
            uns: true,
            seq,
        }
    }

    pub fn from(x: u8) -> Self {
        Self {
            fir: x & Self::FIR_MASK != 0,
            fin: x & Self::FIN_MASK != 0,
            con: x & Self::CON_MASK != 0,
            uns: x & Self::UNS_MASK != 0,
            seq: Sequence::new(x),
        }
    }

    pub(crate) fn is_fir_and_fin(self) -> bool {
        self.fir && self.fin
    }

    pub fn to_u8(self) -> u8 {
        let mut x: u8 = 0;
        if self.fir {
            x |= Self::FIR_MASK;
        }
        if self.fin {
            x |= Self::FIN_MASK;
        }
        if self.con {
            x |= Self::CON_MASK;
        }
        if self.uns {
            x |= Self::UNS_MASK;
        }
        x |= self.seq.value();
        x
    }

    pub fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(Self::from(cursor.read_u8()?))
    }

    pub fn write(self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        Ok(cursor.write_u8(self.to_u8())?)
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct IIN1 {
    pub value: u8,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct IIN2 {
    pub value: u8,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct IIN {
    pub iin1: IIN1,
    pub iin2: IIN2,
}

impl IIN1 {
    pub fn new(value: u8) -> Self {
        Self { value }
    }

    pub fn get_broadcast(self) -> bool {
        self.value.bit_0()
    }

    pub fn get_class_1_events(self) -> bool {
        self.value.bit_1()
    }

    pub fn get_class_2_events(self) -> bool {
        self.value.bit_2()
    }

    pub fn get_class_3_events(self) -> bool {
        self.value.bit_3()
    }

    pub fn get_need_time(self) -> bool {
        self.value.bit_4()
    }

    pub fn get_local_control(self) -> bool {
        self.value.bit_5()
    }

    pub fn get_device_trouble(self) -> bool {
        self.value.bit_6()
    }

    pub fn get_device_restart(self) -> bool {
        self.value.bit_7()
    }
}

impl IIN2 {
    pub fn new(value: u8) -> Self {
        Self { value }
    }

    pub fn get_no_func_code_support(self) -> bool {
        self.value.bit_0()
    }

    pub fn get_object_unknown(self) -> bool {
        self.value.bit_1()
    }

    pub fn get_parameter_error(self) -> bool {
        self.value.bit_2()
    }

    pub fn get_event_buffer_overflow(self) -> bool {
        self.value.bit_3()
    }

    pub fn get_already_executing(self) -> bool {
        self.value.bit_4()
    }

    pub fn get_config_corrupt(self) -> bool {
        self.value.bit_5()
    }

    pub fn get_reserved_2(self) -> bool {
        self.value.bit_6()
    }

    pub fn get_reserved_1(self) -> bool {
        self.value.bit_7()
    }
}

impl std::fmt::Display for IIN1 {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        format_bitfield(
            f,
            self.value,
            "iin1",
            [
                "BROADCAST",
                "CLASS_1_EVENTS",
                "CLASS_2_EVENTS",
                "CLASS_3_EVENTS",
                "NEED_TIME",
                "LOCAL_CONTROL",
                "DEVICE_TROUBLE",
                "DEVICE_RESTART",
            ],
        )
    }
}

impl std::fmt::Display for IIN2 {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        format_bitfield(
            f,
            self.value,
            "iin2",
            [
                "NO_FUNC_CODE_SUPPORT",
                "OBJECT_UNKNOWN",
                "PARAMETER_ERROR",
                "EVENT_BUFFER_OVERFLOW",
                "ALREADY_EXECUTING",
                "CONFIG_CORRUPT",
                "RESERVED_2",
                "RESERVED_1",
            ],
        )
    }
}

impl IIN {
    pub fn new(iin1: IIN1, iin2: IIN2) -> Self {
        Self { iin1, iin2 }
    }

    pub fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(Self {
            iin1: IIN1::new(cursor.read_u8()?),
            iin2: IIN2::new(cursor.read_u8()?),
        })
    }

    pub fn write(self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.iin1.value)?;
        cursor.write_u8(self.iin2.value)?;
        Ok(())
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct RequestHeader {
    pub control: Control,
    pub function: FunctionCode,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ResponseHeader {
    pub control: Control,
    pub unsolicited: bool,
    pub iin: IIN,
}

impl RequestHeader {
    pub fn new(control: Control, function: FunctionCode) -> Self {
        Self { control, function }
    }

    pub fn write(self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        self.control.write(cursor)?;
        self.function.write(cursor)?;
        Ok(())
    }
}

impl ResponseHeader {
    pub fn new(control: Control, unsolicited: bool, iin: IIN) -> Self {
        Self {
            control,
            unsolicited,
            iin,
        }
    }

    pub fn function(self) -> FunctionCode {
        if self.unsolicited {
            FunctionCode::UnsolicitedResponse
        } else {
            FunctionCode::Response
        }
    }

    pub fn write(self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        self.control.write(cursor)?;
        self.function().write(cursor)?;
        self.iin.write(cursor)?;
        Ok(())
    }
}
