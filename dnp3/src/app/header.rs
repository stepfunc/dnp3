use crate::app::enums::FunctionCode;
use crate::app::sequence::Sequence;
use crate::util::bit::bits::*;
use crate::util::bit::{format_bitfield, Bitfield};
use crate::util::cursor::{ReadCursor, ReadError, WriteCursor, WriteError};
use std::fmt::Formatter;
use std::ops::{Add, BitOr, BitOrAssign};

/// Control field in the application-layer header
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Control {
    /// FIR bit - set if the first fragment in a multi-fragmented response
    pub fir: bool,
    /// FIN bit - set if the final fragment in a multi-fragmented response
    pub fin: bool,
    /// FIN bit - set if the fragment is requesting confirmation
    pub con: bool,
    /// UNS bit - set if sequence number is interpreted for unsolicited responses
    pub uns: bool,
    /// sequence number
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

    pub(crate) fn response(seq: Sequence, fir: bool, fin: bool, con: bool) -> Self {
        Self {
            fir,
            fin,
            con,
            uns: false,
            seq,
        }
    }

    pub(crate) fn request(seq: Sequence) -> Self {
        Self {
            fir: true,
            fin: true,
            con: false,
            uns: false,
            seq,
        }
    }

    pub(crate) fn response_with_confirmation(seq: Sequence) -> Self {
        Self {
            fir: true,
            fin: true,
            con: true,
            uns: false,
            seq,
        }
    }

    pub(crate) fn unsolicited(seq: Sequence) -> Self {
        Self {
            fir: true,
            fin: true,
            con: false,
            uns: true,
            seq,
        }
    }

    pub(crate) fn from(x: u8) -> Self {
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

    pub(crate) fn to_u8(self) -> u8 {
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

    pub(crate) fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(Self::from(cursor.read_u8()?))
    }

    pub(crate) fn write(self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        Ok(cursor.write_u8(self.to_u8())?)
    }
}

/// Internal Indications Byte #1
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct IIN1 {
    /// underlying value for IIN1
    pub value: u8,
}

/// Internal Indications Byte #2
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct IIN2 {
    /// underlying value for IIN2
    pub value: u8,
}

/// Internal Indications (2 bytes)
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct IIN {
    /// IIN byte #1
    pub iin1: IIN1,
    /// IIN byte #2
    pub iin2: IIN2,
}

impl IIN1 {
    pub const BROADCAST: IIN1 = IIN1::new(BIT_0.value);
    pub const CLASS_1_EVENTS: IIN1 = IIN1::new(BIT_1.value);
    pub const CLASS_2_EVENTS: IIN1 = IIN1::new(BIT_2.value);
    pub const CLASS_3_EVENTS: IIN1 = IIN1::new(BIT_3.value);
    pub const NEED_TIME: IIN1 = IIN1::new(BIT_5.value);
    pub const LOCAL_CONTROL: IIN1 = IIN1::new(BIT_5.value);
    pub const DEVICE_TROUBLE: IIN1 = IIN1::new(BIT_6.value);
    pub const RESTART: IIN1 = IIN1::new(BIT_7.value);

    /// Construct IIN1 from its underlying value
    pub const fn new(value: u8) -> Self {
        Self { value }
    }

    /// test IIN.1 to see if the BROADCAST bit is set
    pub fn get_broadcast(&self) -> bool {
        self.value.bit_0()
    }

    /// test IIN.1 to see if the CLASS_1_EVENTS bit is set
    pub fn get_class_1_events(self) -> bool {
        self.value.bit_1()
    }

    /// test IIN.1 to see if the CLASS_2_EVENTS bit is set
    pub fn get_class_2_events(self) -> bool {
        self.value.bit_2()
    }

    /// test IIN.1 to see if the CLASS_3_EVENTS bit is set
    pub fn get_class_3_events(self) -> bool {
        self.value.bit_3()
    }

    /// test IIN.1 to see if the NEED_TIME bit is set
    pub fn get_need_time(self) -> bool {
        self.value.bit_4()
    }

    /// test IIN.1 to see if the LOCAL_CONTROL bit is set
    pub fn get_local_control(self) -> bool {
        self.value.bit_5()
    }

    /// test IIN.1 to see if the DEVICE_TROUBLE bit is set
    pub fn get_device_trouble(self) -> bool {
        self.value.bit_6()
    }

    /// test IIN.1 to see if the DEVICE_RESTART bit is set
    pub fn get_device_restart(self) -> bool {
        self.value.bit_7()
    }
}

impl Default for IIN1 {
    fn default() -> Self {
        Self { value: 0 }
    }
}

impl Add<IIN2> for IIN1 {
    type Output = IIN;

    fn add(self, rhs: IIN2) -> Self::Output {
        IIN::new(self, rhs)
    }
}

impl BitOrAssign<IIN1> for IIN1 {
    fn bitor_assign(&mut self, rhs: IIN1) {
        self.value |= rhs.value
    }
}

impl IIN2 {
    pub(crate) const NO_FUNC_CODE_SUPPORT: IIN2 = IIN2::new(BIT_0.value);
    pub(crate) const OBJECT_UNKNOWN: IIN2 = IIN2::new(BIT_1.value);
    pub(crate) const PARAMETER_ERROR: IIN2 = IIN2::new(BIT_2.value);
    pub(crate) const EVENT_BUFFER_OVERFLOW: IIN2 = IIN2::new(BIT_3.value);
    pub(crate) const ALREADY_EXECUTING: IIN2 = IIN2::new(BIT_4.value);
    pub(crate) const CONFIG_CORRUPT: IIN2 = IIN2::new(BIT_5.value);

    /// Construct IIN2 from its underlying value
    pub const fn new(value: u8) -> Self {
        Self { value }
    }

    /// test IIN.2 to see if the NO_FUNC_CODE_SUPPORT bit is set
    pub fn get_no_func_code_support(self) -> bool {
        self.value.bit_0()
    }

    /// test IIN.2 to see if the OBJECT_UNKNOWN bit is set
    pub fn get_object_unknown(self) -> bool {
        self.value.bit_1()
    }

    /// test IIN.2 to see if the GET_PARAMETER_ERROR bit is set
    pub fn get_parameter_error(self) -> bool {
        self.value.bit_2()
    }

    /// test IIN.2 to see if the EVENT_BUFFER_OVERFLOW bit is set
    pub fn get_event_buffer_overflow(self) -> bool {
        self.value.bit_3()
    }

    /// test IIN.2 to see if the ALREADY_EXECUTING bit is set
    pub fn get_already_executing(self) -> bool {
        self.value.bit_4()
    }

    /// test IIN.2 to see if the CONFIG_CORRUPT bit is set
    pub fn get_config_corrupt(self) -> bool {
        self.value.bit_5()
    }

    /// test IIN.2 to see if the RESERVED_2 bit is set
    pub fn get_reserved_2(self) -> bool {
        self.value.bit_6()
    }

    /// test IIN.2 to see if the RESERVED_1 bit is set
    pub fn get_reserved_1(self) -> bool {
        self.value.bit_7()
    }
}

impl Default for IIN2 {
    fn default() -> Self {
        Self { value: 0 }
    }
}

impl BitOr for IIN1 {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            value: self.value | rhs.value,
        }
    }
}

impl BitOrAssign<IIN2> for IIN2 {
    fn bitor_assign(&mut self, rhs: IIN2) {
        self.value |= rhs.value;
    }
}

impl BitOr for IIN2 {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            value: self.value | rhs.value,
        }
    }
}

impl BitOr<IIN2> for IIN {
    type Output = Self;

    fn bitor(self, rhs: IIN2) -> Self::Output {
        Self {
            iin1: self.iin1,
            iin2: self.iin2 | rhs,
        }
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
    /// construct an IIN from `IIN1` and `IIN2`
    pub fn new(iin1: IIN1, iin2: IIN2) -> Self {
        Self { iin1, iin2 }
    }

    pub(crate) fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(Self {
            iin1: IIN1::new(cursor.read_u8()?),
            iin2: IIN2::new(cursor.read_u8()?),
        })
    }

    pub(crate) fn has_request_error(self) -> bool {
        self.iin2.get_no_func_code_support()
            || self.iin2.get_object_unknown()
            || self.iin2.get_parameter_error()
    }

    pub(crate) fn write(self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        cursor.write_u8(self.iin1.value)?;
        cursor.write_u8(self.iin2.value)?;
        Ok(())
    }
}

impl Default for IIN {
    fn default() -> Self {
        IIN::new(IIN1::new(0), IIN2::new(0))
    }
}

impl BitOr for IIN {
    type Output = IIN;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            iin1: IIN1 {
                value: self.iin1.value | rhs.iin1.value,
            },
            iin2: IIN2 {
                value: self.iin2.value | rhs.iin2.value,
            },
        }
    }
}

/// application-layer header for requests
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct RequestHeader {
    /// control field
    pub control: Control,
    /// function code
    pub function: FunctionCode,
}

/// Only 2 function codes allowed in responses
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ResponseFunction {
    Response,
    UnsolicitedResponse,
}

/// application-layer header for responses
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ResponseHeader {
    /// control field
    pub control: Control,
    /// Function code limited to Response or UnsolicitedResponse
    pub function: ResponseFunction,
    /// internal indications field
    pub iin: IIN,
}

impl ResponseFunction {
    pub fn is_unsolicited(self) -> bool {
        match self {
            ResponseFunction::Response => false,
            ResponseFunction::UnsolicitedResponse => true,
        }
    }

    pub(crate) fn function(&self) -> FunctionCode {
        match self {
            ResponseFunction::Response => FunctionCode::Response,
            ResponseFunction::UnsolicitedResponse => FunctionCode::UnsolicitedResponse,
        }
    }
}

impl From<ResponseFunction> for FunctionCode {
    fn from(from: ResponseFunction) -> Self {
        match from {
            ResponseFunction::Response => FunctionCode::Response,
            ResponseFunction::UnsolicitedResponse => FunctionCode::UnsolicitedResponse,
        }
    }
}

impl RequestHeader {
    pub(crate) fn new(control: Control, function: FunctionCode) -> Self {
        Self { control, function }
    }

    pub(crate) fn write(self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        self.control.write(cursor)?;
        self.function.write(cursor)?;
        Ok(())
    }
}

impl ResponseHeader {
    pub(crate) const LENGTH: usize = 4;

    pub(crate) fn new(control: Control, function: ResponseFunction, iin: IIN) -> Self {
        Self {
            control,
            function,
            iin,
        }
    }

    pub(crate) fn function(self) -> FunctionCode {
        self.function.into()
    }

    pub(crate) fn write(self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        self.control.write(cursor)?;
        self.function.function().write(cursor)?;
        self.iin.write(cursor)?;
        Ok(())
    }
}
