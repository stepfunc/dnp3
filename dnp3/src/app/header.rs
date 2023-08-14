use std::fmt::Formatter;
use std::ops::{Add, BitOr, BitOrAssign};

use crate::app::sequence::Sequence;
use crate::app::FunctionCode;
use crate::outstation::{ApplicationIin, RequestError};
use crate::util::bit::bits::*;
use crate::util::bit::{format_bitfield, Bitfield};

use scursor::*;

/// Control field in the application-layer header
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ControlField {
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

impl std::fmt::Display for ControlField {
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

impl ControlField {
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

    pub(crate) fn single_response(seq: Sequence) -> Self {
        Self {
            fir: true,
            fin: true,
            con: false,
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

    pub(crate) fn unsolicited(seq: Sequence) -> Self {
        Self {
            fir: true,
            fin: true,
            con: false,
            uns: true,
            seq,
        }
    }

    pub(crate) fn unsolicited_response(seq: Sequence) -> Self {
        Self {
            fir: true,
            fin: true,
            con: true,
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
        cursor.write_u8(self.to_u8())
    }
}

/// Internal Indications Byte #1
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Iin1 {
    /// underlying value for IIN1
    pub value: u8,
}

/// Internal Indications Byte #2
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Iin2 {
    /// underlying value for IIN2
    pub value: u8,
}

/// Internal Indications (2 bytes)
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Iin {
    /// IIN byte #1
    pub iin1: Iin1,
    /// IIN byte #2
    pub iin2: Iin2,
}

impl Iin1 {
    /// IIN1 struct with only the BROADCAST bit set
    pub const BROADCAST: Iin1 = Iin1::new(BIT_0.value);
    /// IIN1 struct with only the CLASS_1_EVENTS bit set
    pub const CLASS_1_EVENTS: Iin1 = Iin1::new(BIT_1.value);
    /// IIN1 struct with only the CLASS_2_EVENTS bit set
    pub const CLASS_2_EVENTS: Iin1 = Iin1::new(BIT_2.value);
    /// IIN1 struct with only the CLASS_3_EVENTS bit set
    pub const CLASS_3_EVENTS: Iin1 = Iin1::new(BIT_3.value);
    /// IIN1 struct with only the NEED_TIME bit set
    pub const NEED_TIME: Iin1 = Iin1::new(BIT_4.value);
    /// IIN1 struct with only the LOCAL_CONTROL bit set
    pub const LOCAL_CONTROL: Iin1 = Iin1::new(BIT_5.value);
    /// IIN1 struct with only the DEVICE_TROUBLE bit set
    pub const DEVICE_TROUBLE: Iin1 = Iin1::new(BIT_6.value);
    /// IIN1 struct with only the RESTART bit set
    pub const RESTART: Iin1 = Iin1::new(BIT_7.value);

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

impl Default for Iin1 {
    fn default() -> Self {
        Self::new(0)
    }
}

impl BitOr for Iin1 {
    type Output = Self;

    fn bitor(self, rhs: Iin1) -> Self::Output {
        Self::new(self.value | rhs.value)
    }
}

impl BitOrAssign<Iin1> for Iin1 {
    fn bitor_assign(&mut self, rhs: Iin1) {
        *self = *self | rhs
    }
}

impl Iin2 {
    /// IIN2 struct with only the NO_FUNC_CODE_SUPPORT bit set
    pub const NO_FUNC_CODE_SUPPORT: Iin2 = Iin2::new(BIT_0.value);
    /// IIN2 struct with only the OBJECT_UNKNOWN bit set
    pub const OBJECT_UNKNOWN: Iin2 = Iin2::new(BIT_1.value);
    /// IIN2 struct with only the PARAMETER_ERROR bit set
    pub const PARAMETER_ERROR: Iin2 = Iin2::new(BIT_2.value);
    /// IIN2 struct with only the EVENT_BUFFER_OVERFLOW bit set
    pub const EVENT_BUFFER_OVERFLOW: Iin2 = Iin2::new(BIT_3.value);
    /// IIN2 struct with only the ALREADY_EXECUTING bit set
    pub const ALREADY_EXECUTING: Iin2 = Iin2::new(BIT_4.value);
    /// IIN2 struct with only the CONFIG_CORRUPT bit set
    pub const CONFIG_CORRUPT: Iin2 = Iin2::new(BIT_5.value);

    /// Construct IIN2 from its underlying value
    pub const fn new(value: u8) -> Self {
        Self { value }
    }

    pub(crate) fn set(&mut self, iin2: Self) {
        self.value |= iin2.value;
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

impl Default for Iin2 {
    fn default() -> Self {
        Self::new(0)
    }
}

impl BitOr for Iin2 {
    type Output = Self;

    fn bitor(self, rhs: Iin2) -> Self::Output {
        Self::new(self.value | rhs.value)
    }
}

impl BitOrAssign<Iin2> for Iin2 {
    fn bitor_assign(&mut self, rhs: Iin2) {
        *self = *self | rhs
    }
}

impl Default for Iin {
    fn default() -> Self {
        Iin::new(Iin1::default(), Iin2::default())
    }
}

impl BitOr for Iin {
    type Output = Iin;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            iin1: self.iin1 | rhs.iin1,
            iin2: self.iin2 | rhs.iin2,
        }
    }
}

impl BitOrAssign for Iin {
    fn bitor_assign(&mut self, rhs: Iin) {
        *self = *self | rhs
    }
}

impl BitOr<Iin1> for Iin {
    type Output = Self;

    fn bitor(self, rhs: Iin1) -> Self::Output {
        Self {
            iin1: self.iin1 | rhs,
            iin2: self.iin2,
        }
    }
}

impl BitOrAssign<Iin1> for Iin {
    fn bitor_assign(&mut self, rhs: Iin1) {
        *self = *self | rhs;
    }
}

impl BitOr<Iin2> for Iin {
    type Output = Self;

    fn bitor(self, rhs: Iin2) -> Self::Output {
        Self {
            iin1: self.iin1,
            iin2: self.iin2 | rhs,
        }
    }
}

impl BitOrAssign<Iin2> for Iin {
    fn bitor_assign(&mut self, rhs: Iin2) {
        *self = *self | rhs;
    }
}

impl BitOr<ApplicationIin> for Iin {
    type Output = Self;

    fn bitor(mut self, rhs: ApplicationIin) -> Self::Output {
        if rhs.need_time {
            self |= Iin1::NEED_TIME;
        }

        if rhs.local_control {
            self |= Iin1::LOCAL_CONTROL;
        }

        if rhs.device_trouble {
            self |= Iin1::DEVICE_TROUBLE;
        }

        if rhs.config_corrupt {
            self |= Iin2::CONFIG_CORRUPT;
        }

        self
    }
}

impl BitOrAssign<ApplicationIin> for Iin {
    fn bitor_assign(&mut self, rhs: ApplicationIin) {
        *self = *self | rhs;
    }
}

impl From<RequestError> for Iin2 {
    fn from(from: RequestError) -> Self {
        match from {
            RequestError::ParameterError => Iin2::PARAMETER_ERROR,
            RequestError::NotSupported => Iin2::NO_FUNC_CODE_SUPPORT,
        }
    }
}

impl Add<Iin2> for Iin1 {
    type Output = Iin;

    fn add(self, rhs: Iin2) -> Self::Output {
        Iin::new(self, rhs)
    }
}

impl std::fmt::Display for Iin1 {
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

impl std::fmt::Display for Iin2 {
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

impl Iin {
    /// construct an IIN from `IIN1` and `IIN2`
    pub const fn new(iin1: Iin1, iin2: Iin2) -> Self {
        Self { iin1, iin2 }
    }

    pub(crate) fn parse(cursor: &mut ReadCursor) -> Result<Self, ReadError> {
        Ok(Self {
            iin1: Iin1::new(cursor.read_u8()?),
            iin2: Iin2::new(cursor.read_u8()?),
        })
    }

    pub(crate) fn has_bad_request_error(self) -> bool {
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

/// Application-layer header for requests
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RequestHeader {
    /// control field
    pub control: ControlField,
    /// function code
    pub function: FunctionCode,
}

/// Only 2 function codes allowed in responses
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ResponseFunction {
    /// (solicited) response (0x81)
    Response,
    /// unsolicited response (0x82)
    UnsolicitedResponse,
}

/// Application-layer header for responses
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ResponseHeader {
    /// control field
    pub control: ControlField,
    /// Function code limited to Response or UnsolicitedResponse
    pub function: ResponseFunction,
    /// internal indications field
    pub iin: Iin,
}

impl ResponseFunction {
    /// test if the response function is unsolicited
    pub fn is_unsolicited(self) -> bool {
        match self {
            ResponseFunction::Response => false,
            ResponseFunction::UnsolicitedResponse => true,
        }
    }

    /// map the response function to a `FunctionCode`
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
    pub(crate) fn new(control: ControlField, function: FunctionCode) -> Self {
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

    pub(crate) fn new(control: ControlField, function: ResponseFunction, iin: Iin) -> Self {
        Self {
            control,
            function,
            iin,
        }
    }

    pub(crate) fn write(self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        self.control.write(cursor)?;
        self.function.function().write(cursor)?;
        self.iin.write(cursor)?;
        Ok(())
    }
}
