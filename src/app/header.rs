use crate::app::gen::enums::FunctionCode;
use crate::app::parse::parser::{
    HeaderCollection, ObjectParseError, ParseLogLevel, Request, Response,
};
use crate::app::sequence::Sequence;
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

#[derive(Debug, PartialEq)]
pub enum HeaderParseError {
    UnknownFunction(u8),
    InsufficientBytes,
    UnsolicitedBitNotAllowed(FunctionCode),
    ExpectedFirAndFin(FunctionCode),
    UnexpectedResponseFunction(FunctionCode),
    UnexpectedRequestFunction(FunctionCode),
    UnsolicitedResponseWithoutUnsBit,
    ResponseWithUnsBit,
}

impl std::fmt::Display for HeaderParseError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            HeaderParseError::UnknownFunction(x) => write!(f, "unknown function: {:?}", x),
            HeaderParseError::InsufficientBytes => write!(f, "insufficient bytes"),
            HeaderParseError::UnsolicitedBitNotAllowed(x) => {
                write!(f, "UNS bit not allowed for function: {:?}", x)
            }
            HeaderParseError::ExpectedFirAndFin(x) => {
                write!(f, "function {:?} must have fir/fin both set to 1", x)
            }
            HeaderParseError::UnexpectedResponseFunction(x) => {
                write!(f, "expected response, but received {:?}", x)
            }
            HeaderParseError::UnexpectedRequestFunction(x) => {
                write!(f, "expected a request, but received {:?}", x)
            }
            HeaderParseError::UnsolicitedResponseWithoutUnsBit => {
                write!(f, "unsolicited responses must have the UNS bit set")
            }
            HeaderParseError::ResponseWithUnsBit => {
                write!(f, "solicited responses may not have the UNS bit set")
            }
        }
    }
}

pub struct ParsedFragment<'a> {
    pub control: Control,
    pub function: FunctionCode,
    pub iin: Option<IIN>,
    pub raw_objects: &'a [u8],
    pub objects: Result<HeaderCollection<'a>, ObjectParseError>,
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

impl<'a> ParsedFragment<'a> {
    pub fn to_request(&self) -> Result<Request<'a>, HeaderParseError> {
        if self.iin.is_some() {
            return Err(HeaderParseError::UnexpectedRequestFunction(self.function));
        }

        if !(self.control.is_fir_and_fin()) {
            return Err(HeaderParseError::ExpectedFirAndFin(self.function));
        }

        if self.control.uns && self.function != FunctionCode::Confirm {
            return Err(HeaderParseError::UnsolicitedBitNotAllowed(self.function));
        }

        Ok(Request {
            header: RequestHeader::new(self.control, self.function),
            raw_objects: self.raw_objects,
            objects: self.objects,
        })
    }

    pub fn to_response(&self) -> Result<Response<'a>, HeaderParseError> {
        let (unsolicited, iin) = match (self.function, self.iin) {
            (FunctionCode::Response, Some(x)) => (false, x),
            (FunctionCode::UnsolicitedResponse, Some(x)) => (true, x),
            _ => return Err(HeaderParseError::UnexpectedResponseFunction(self.function)),
        };

        if !unsolicited && self.control.uns {
            return Err(HeaderParseError::ResponseWithUnsBit);
        }

        if unsolicited && !self.control.uns {
            return Err(HeaderParseError::UnsolicitedResponseWithoutUnsBit);
        }

        Ok(Response {
            header: ResponseHeader::new(self.control, unsolicited, iin),
            raw_objects: self.raw_objects,
            objects: self.objects,
        })
    }

    pub fn parse(level: ParseLogLevel, fragment: &'a [u8]) -> Result<Self, HeaderParseError> {
        let mut cursor = ReadCursor::new(fragment);

        let control = Control::from(cursor.read_u8()?);
        let raw_func = cursor.read_u8()?;
        let function = match FunctionCode::from(raw_func) {
            None => return Err(HeaderParseError::UnknownFunction(raw_func)),
            Some(x) => x,
        };
        let iin = match function {
            FunctionCode::Response => Some(IIN::parse(&mut cursor)?),
            FunctionCode::UnsolicitedResponse => Some(IIN::parse(&mut cursor)?),
            _ => None,
        };

        let objects = cursor.read_all();
        let fragment = Self {
            control,
            function,
            iin,
            raw_objects: objects,
            objects: HeaderCollection::parse(level, function, objects),
        };
        if level.log_header() {
            log::info!("{}", fragment);
        }
        Ok(fragment)
    }
}

impl<'a> std::fmt::Display for ParsedFragment<'a> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self.iin {
            Some(iin) => write!(
                f,
                "ctrl: {} func: {:?} iin: {} ... (len = {})",
                self.control,
                self.function,
                iin,
                self.raw_objects.len()
            ),
            None => write!(
                f,
                "ctrl: {} func: {:?} ... (len = {})",
                self.control,
                self.function,
                self.raw_objects.len()
            ),
        }
    }
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
