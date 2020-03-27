use crate::app::gen::enums::FunctionCode;
use crate::app::sequence::Sequence;
use crate::util::cursor::{ReadCursor, ReadError, WriteCursor, WriteError};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Control {
    pub fir: bool,
    pub fin: bool,
    pub con: bool,
    pub uns: bool,
    pub seq: Sequence,
}

impl Control {
    const FIR_MASK: u8 = 0b1000_0000;
    const FIN_MASK: u8 = 0b0100_0000;
    const CON_MASK: u8 = 0b0010_0000;
    const UNS_MASK: u8 = 0b0001_0000;

    pub fn from(x: u8) -> Self {
        Self {
            fir: x & Self::FIR_MASK != 0,
            fin: x & Self::FIN_MASK != 0,
            con: x & Self::CON_MASK != 0,
            uns: x & Self::UNS_MASK != 0,
            seq: Sequence::new(x),
        }
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
    BadFirAndFin(Control),
    BadFunction(FunctionCode),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct RequestHeader {
    pub control: Control,
    pub function: FunctionCode,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ResponseFunction {
    Solicited,
    Unsolicited,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ResponseHeader {
    pub control: Control,
    pub function: ResponseFunction,
    pub iin: IIN,
}

impl RequestHeader {
    pub fn parse(cursor: &mut ReadCursor) -> Result<Self, HeaderParseError> {
        let control = Control::from(cursor.read_u8()?);
        let raw_func = cursor.read_u8()?;
        let function = match FunctionCode::from(raw_func) {
            None => return Err(HeaderParseError::UnknownFunction(raw_func)),
            Some(x) => x,
        };
        Ok(Self { control, function })
    }

    pub fn write(self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        self.control.write(cursor)?;
        cursor.write_u8(self.function.as_u8())?;
        Ok(())
    }
}

impl ResponseFunction {
    pub fn to_function(self) -> FunctionCode {
        match self {
            ResponseFunction::Solicited => FunctionCode::Response,
            ResponseFunction::Unsolicited => FunctionCode::UnsolicitedResponse,
        }
    }
}

impl ResponseHeader {
    pub fn parse(cursor: &mut ReadCursor) -> Result<Self, HeaderParseError> {
        let header = RequestHeader::parse(cursor)?;
        let iin = IIN::parse(cursor)?;
        let function = match header.function {
            FunctionCode::Response => ResponseFunction::Solicited,
            FunctionCode::UnsolicitedResponse => ResponseFunction::Unsolicited,
            _ => return Err(HeaderParseError::BadFunction(header.function)),
        };
        Ok(Self {
            control: header.control,
            function,
            iin,
        })
    }

    pub fn write(self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        self.control.write(cursor)?;
        self.function.to_function().write(cursor)?;
        self.iin.write(cursor)?;
        Ok(())
    }
}
