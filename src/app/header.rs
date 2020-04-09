use crate::app::gen::enums::FunctionCode;
use crate::app::sequence::Sequence;
use crate::util::bit::BitTest;
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
pub struct IIN {
    pub iin1: u8,
    pub iin2: u8,
}

impl std::fmt::Display for IIN {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "[iin1: 0x{:02X}, iin2: 0x{:02X}]", self.iin1, self.iin2)
    }
}

impl IIN {
    pub fn get_broadcast(self) -> bool {
        self.iin1.bit_0()
    }

    pub fn get_class_1_events(self) -> bool {
        self.iin1.bit_1()
    }

    pub fn get_class_2_events(self) -> bool {
        self.iin1.bit_2()
    }

    pub fn get_class_3_events(self) -> bool {
        self.iin1.bit_3()
    }

    pub fn get_need_time(self) -> bool {
        self.iin1.bit_4()
    }

    pub fn get_local_control(self) -> bool {
        self.iin1.bit_5()
    }

    pub fn get_device_trouble(self) -> bool {
        self.iin1.bit_6()
    }

    pub fn get_device_restart(self) -> bool {
        self.iin1.bit_7()
    }

    pub fn get_no_func_code_support(self) -> bool {
        self.iin2.bit_0()
    }

    pub fn get_object_unknown(self) -> bool {
        self.iin2.bit_1()
    }

    pub fn get_parameter_error(self) -> bool {
        self.iin2.bit_2()
    }

    pub fn get_event_buffer_overflow(self) -> bool {
        self.iin2.bit_3()
    }

    pub fn get_already_executing(self) -> bool {
        self.iin2.bit_4()
    }

    pub fn get_config_corrupt(self) -> bool {
        self.iin2.bit_5()
    }

    pub fn get_reserved_2(self) -> bool {
        self.iin2.bit_6()
    }

    pub fn get_reserved_1(self) -> bool {
        self.iin2.bit_7()
    }

    pub fn new(iin1: u8, iin2: u8) -> Self {
        Self { iin1, iin2 }
    }

    pub fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(Self {
            iin1: cursor.read_u8()?,
            iin2: cursor.read_u8()?,
        })
    }

    pub fn write(self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.iin1)?;
        cursor.write_u8(self.iin2)?;
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
