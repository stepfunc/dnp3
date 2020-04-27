use crate::app::gen::enums::{FunctionCode, QualifierCode};
use crate::app::gen::variations::all::AllObjectsVariation;
use crate::app::gen::variations::count::CountVariation;
use crate::app::gen::variations::prefixed::PrefixedVariation;
use crate::app::gen::variations::ranged::RangedVariation;
use crate::app::gen::variations::variation::Variation;
use crate::app::header::{Control, RequestHeader, ResponseHeader, IIN};
use crate::app::parse::error::*;
use crate::app::parse::prefix::Prefix;
use crate::app::parse::range::Range;
use crate::app::parse::traits::{FixedSizeVariation, Index};
use crate::util::cursor::ReadCursor;
use std::fmt::{Debug, Formatter};

/// Controls how transmitted and received ASDUs are logged
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DecodeLogLevel {
    /// Log nothing
    Nothing,
    /// Log the header-only
    Header,
    /// Log the header and the object headers
    ObjectHeaders,
    /// Log the header, the object headers, and the object values
    ObjectValues,
}

#[derive(Copy, Clone)]
pub struct DecodeSettings {
    is_transmit: bool,
    level: DecodeLogLevel,
}

impl DecodeLogLevel {
    pub fn transmit(self) -> DecodeSettings {
        DecodeSettings {
            is_transmit: true,
            level: self,
        }
    }

    pub fn receive(self) -> DecodeSettings {
        DecodeSettings {
            is_transmit: false,
            level: self,
        }
    }
}

pub(crate) fn format_count_of_items<T, V>(f: &mut Formatter, iter: T) -> std::fmt::Result
where
    T: Iterator<Item = V>,
    V: std::fmt::Display,
{
    for x in iter {
        write!(f, "\n{}", x)?;
    }
    Ok(())
}

pub(crate) fn format_indexed_items<T, V, I>(f: &mut Formatter, iter: T) -> std::fmt::Result
where
    T: Iterator<Item = (V, I)>,
    V: std::fmt::Display,
    I: std::fmt::Display,
{
    for (v, i) in iter {
        write!(f, "\nindex: {} {}", i, v)?;
    }
    Ok(())
}

pub(crate) fn format_prefixed_items<T, V, I>(f: &mut Formatter, iter: T) -> std::fmt::Result
where
    T: Iterator<Item = Prefix<I, V>>,
    V: FixedSizeVariation,
    I: Index,
{
    for x in iter {
        write!(f, "\nindex: {} {}", x.index, x.value)?;
    }
    Ok(())
}

pub struct ParsedFragment<'a> {
    pub control: Control,
    pub function: FunctionCode,
    pub iin: Option<IIN>,
    pub objects: Result<HeaderCollection<'a>, ObjectParseError>,
    pub raw_objects: &'a [u8],
}

impl<'a> ParsedFragment<'a> {
    pub(crate) fn display_view(
        &'a self,
        settings: DecodeSettings,
    ) -> Option<ParsedFragmentDisplay<'a>> {
        match settings.level {
            DecodeLogLevel::Nothing => None,
            DecodeLogLevel::Header => Some(ParsedFragmentDisplay {
                is_transmit: settings.is_transmit,
                format_objects_headers: false,
                format_object_values: false,
                fragment: self,
            }),
            DecodeLogLevel::ObjectHeaders => Some(ParsedFragmentDisplay {
                is_transmit: settings.is_transmit,
                format_objects_headers: true,
                format_object_values: false,
                fragment: self,
            }),
            DecodeLogLevel::ObjectValues => Some(ParsedFragmentDisplay {
                is_transmit: settings.is_transmit,
                format_objects_headers: true,
                format_object_values: true,
                fragment: self,
            }),
        }
    }

    fn format_header(&self, f: &mut Formatter) -> std::fmt::Result {
        match self.iin {
            Some(iin) => write!(
                f,
                "ctrl: {} func: {:?} {} {} ... (len = {})",
                self.control,
                self.function,
                iin.iin1,
                iin.iin2,
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

    pub fn to_request(&self) -> Result<Request<'a>, RequestValidationError> {
        if self.iin.is_some() {
            return Err(RequestValidationError::UnexpectedFunction(self.function));
        }

        if !(self.control.is_fir_and_fin()) {
            return Err(RequestValidationError::NonFirFin);
        }

        if self.control.uns && self.function != FunctionCode::Confirm {
            return Err(RequestValidationError::UnexpectedUnsBit(self.function));
        }

        Ok(Request {
            header: RequestHeader::new(self.control, self.function),
            raw_objects: self.raw_objects,
            objects: self.objects,
        })
    }

    pub(crate) fn to_response(&self) -> Result<Response<'a>, ResponseValidationError> {
        let (unsolicited, iin) = match (self.function, self.iin) {
            (FunctionCode::Response, Some(x)) => (false, x),
            (FunctionCode::UnsolicitedResponse, Some(x)) => (true, x),
            _ => return Err(ResponseValidationError::UnexpectedFunction(self.function)),
        };

        if !unsolicited && self.control.uns {
            return Err(ResponseValidationError::SolicitedResponseWithUnsBit);
        }

        if unsolicited && !self.control.uns {
            return Err(ResponseValidationError::UnsolicitedResponseWithoutUnsBit);
        }

        if unsolicited && !self.control.is_fir_and_fin() {
            return Err(ResponseValidationError::UnsolicitedResponseWithoutFirAndFin);
        }

        Ok(Response {
            header: ResponseHeader::new(self.control, unsolicited, iin),
            raw_objects: self.raw_objects,
            objects: self.objects,
        })
    }

    fn parse_no_logging(fragment: &'a [u8]) -> Result<Self, HeaderParseError> {
        let mut cursor = ReadCursor::new(fragment);

        let control = Control::parse(&mut cursor)?;
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
            objects: HeaderCollection::parse(function, objects),
            raw_objects: objects,
        };

        Ok(fragment)
    }

    pub fn parse(settings: DecodeSettings, fragment: &'a [u8]) -> Result<Self, HeaderParseError> {
        let result = Self::parse_no_logging(fragment);
        match &result {
            Ok(fragment) => {
                if let Some(view) = fragment.display_view(settings) {
                    log::info!("{}", view);
                }
            }
            Err(err) => {
                log::warn!("error parsing fragment header: {}", err);
            }
        };
        result
    }
}

#[derive(Debug, PartialEq)]
pub struct ObjectHeader<'a> {
    pub variation: Variation,
    pub details: HeaderDetails<'a>,
}

impl<'a> ObjectHeader<'a> {
    pub(crate) fn new(variation: Variation, details: HeaderDetails<'a>) -> Self {
        Self { variation, details }
    }

    pub(crate) fn format(&self, format_values: bool, f: &mut Formatter) -> std::fmt::Result {
        match &self.details {
            HeaderDetails::AllObjects(_) => write!(
                f,
                "{} : {} - {}",
                self.variation,
                self.variation.description(),
                self.details.qualifier().description()
            ),
            HeaderDetails::OneByteStartStop(s1, s2, seq) => {
                write!(
                    f,
                    "{} : {} - {} - [{}, {}]",
                    self.variation,
                    self.variation.description(),
                    self.details.qualifier().description(),
                    s1,
                    s2
                )?;
                if format_values {
                    seq.format_objects(f)?;
                }
                Ok(())
            }
            HeaderDetails::TwoByteStartStop(s1, s2, seq) => {
                write!(
                    f,
                    "{} : {} - {} - [{}, {}]",
                    self.variation,
                    self.variation.description(),
                    self.details.qualifier().description(),
                    s1,
                    s2
                )?;
                if format_values {
                    seq.format_objects(f)?;
                }
                Ok(())
            }
            HeaderDetails::OneByteCount(c, seq) => {
                write!(
                    f,
                    "{} : {} - {} - [{}]",
                    self.variation,
                    self.variation.description(),
                    self.details.qualifier().description(),
                    c
                )?;
                if format_values {
                    seq.format_objects(f)?;
                }
                Ok(())
            }
            HeaderDetails::TwoByteCount(c, seq) => {
                write!(
                    f,
                    "{} : {} - {} - [{}]",
                    self.variation,
                    self.variation.description(),
                    self.details.qualifier().description(),
                    c
                )?;
                if format_values {
                    seq.format_objects(f)?;
                }
                Ok(())
            }
            HeaderDetails::OneByteCountAndPrefix(c, seq) => {
                write!(
                    f,
                    "{} : {} - {} - [{}]",
                    self.variation,
                    self.variation.description(),
                    self.details.qualifier().description(),
                    c
                )?;
                if format_values {
                    seq.format_objects(f)?;
                }
                Ok(())
            }
            HeaderDetails::TwoByteCountAndPrefix(c, seq) => {
                write!(
                    f,
                    "{} : {} - {} - [{}]",
                    self.variation,
                    self.variation.description(),
                    self.details.qualifier().description(),
                    c
                )?;
                if format_values {
                    seq.format_objects(f)?;
                }
                Ok(())
            }
        }
    }
}

pub(crate) struct ParsedFragmentDisplay<'a> {
    is_transmit: bool,
    format_objects_headers: bool,
    format_object_values: bool,
    fragment: &'a ParsedFragment<'a>,
}

impl std::fmt::Display for ParsedFragmentDisplay<'_> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        // always display the header
        f.write_str(if self.is_transmit {
            "APP TX - "
        } else {
            "APP RX - "
        })?;
        self.fragment.format_header(f)?;

        match self.fragment.objects {
            Ok(headers) => {
                if !self.format_objects_headers {
                    return Ok(());
                }
                for header in headers.iter() {
                    f.write_str("\n")?;
                    header.format(self.format_object_values, f)?
                }
            }
            Err(err) => {
                // if an error occurred, we re-parse the object headers so we can log any headers before the error
                for result in
                    ObjectParser::one_pass(self.fragment.function, self.fragment.raw_objects)
                {
                    if let Ok(header) = result {
                        f.write_str("\n")?;
                        header.format(self.format_object_values, f)?;
                    }
                }
                // log the original error after any valid headers that preceded it
                log::warn!("{}", err);
            }
        }

        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub enum HeaderDetails<'a> {
    AllObjects(AllObjectsVariation),
    OneByteStartStop(u8, u8, RangedVariation<'a>),
    TwoByteStartStop(u16, u16, RangedVariation<'a>),
    OneByteCount(u8, CountVariation<'a>),
    TwoByteCount(u16, CountVariation<'a>),
    OneByteCountAndPrefix(u8, PrefixedVariation<'a, u8>),
    TwoByteCountAndPrefix(u16, PrefixedVariation<'a, u16>),
}

impl HeaderDetails<'_> {
    pub fn qualifier(&self) -> QualifierCode {
        match self {
            HeaderDetails::AllObjects(_) => QualifierCode::AllObjects,
            HeaderDetails::OneByteStartStop(_, _, _) => QualifierCode::Range8,
            HeaderDetails::TwoByteStartStop(_, _, _) => QualifierCode::Range16,
            HeaderDetails::OneByteCount(_, _) => QualifierCode::Count8,
            HeaderDetails::TwoByteCount(_, _) => QualifierCode::Count16,
            HeaderDetails::OneByteCountAndPrefix(_, _) => QualifierCode::CountAndPrefix8,
            HeaderDetails::TwoByteCountAndPrefix(_, _) => QualifierCode::CountAndPrefix16,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Request<'a> {
    pub header: RequestHeader,
    pub raw_objects: &'a [u8],
    pub objects: Result<HeaderCollection<'a>, ObjectParseError>,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Response<'a> {
    pub header: ResponseHeader,
    pub raw_objects: &'a [u8],
    pub objects: Result<HeaderCollection<'a>, ObjectParseError>,
}

struct ObjectParser<'a> {
    errored: bool,
    function: FunctionCode,
    cursor: ReadCursor<'a>,
}

/// An abstract collection of pre-validated object headers
/// that can provide an iterator of the headers.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct HeaderCollection<'a> {
    function: FunctionCode,
    data: &'a [u8],
}

impl<'a> HeaderCollection<'a> {
    /// parse the the raw header data in accordance with the provided function code
    pub fn parse(function: FunctionCode, data: &'a [u8]) -> Result<Self, ObjectParseError> {
        ObjectParser::parse(function, data)
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// return and iterator of the headers that lazily parses them
    pub fn iter(&self) -> HeaderIterator<'a> {
        HeaderIterator {
            parser: ObjectParser::one_pass(self.function, self.data),
        }
    }
}

pub struct HeaderIterator<'a> {
    parser: ObjectParser<'a>,
}

impl<'a> Iterator for HeaderIterator<'a> {
    type Item = ObjectHeader<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.parser.next() {
            None => None,
            Some(Ok(x)) => Some(x),
            // this should never happen, but this is better than blindly unwrapping
            Some(Err(_)) => None,
        }
    }
}

impl<'a> ObjectParser<'a> {
    pub(crate) fn parse(
        function: FunctionCode,
        data: &'a [u8],
    ) -> Result<HeaderCollection<'a>, ObjectParseError> {
        // we first do a single pass to ensure the ASDU is well-formed, returning an error if it occurs
        for result in ObjectParser::one_pass(function, data) {
            if let Err(err) = result {
                return Err(err);
            }
        }

        // now we know that we know the headers are well-formed, our 2nd pass
        // can use the HeaderCollection iterator implementation to read them
        Ok(HeaderCollection { function, data })
    }

    fn one_pass(function: FunctionCode, data: &'a [u8]) -> Self {
        ObjectParser {
            cursor: ReadCursor::new(data),
            function,
            errored: false,
        }
    }

    fn parse_one(&mut self) -> Option<Result<ObjectHeader<'a>, ObjectParseError>> {
        if self.errored || self.cursor.is_empty() {
            return None;
        }

        let result = self.parse_one_inner();

        if result.is_err() {
            self.errored = true;
        }

        Some(result)
    }

    fn parse_one_inner(&mut self) -> Result<ObjectHeader<'a>, ObjectParseError> {
        let gv = Variation::parse(&mut self.cursor)?;
        let qualifier = QualifierCode::parse(&mut self.cursor)?;
        match qualifier {
            QualifierCode::AllObjects => self.parse_all_objects(gv),
            QualifierCode::Range8 => self.parse_start_stop_u8(gv),
            QualifierCode::Range16 => self.parse_start_stop_u16(gv),
            QualifierCode::Count8 => self.parse_count_u8(gv),
            QualifierCode::Count16 => self.parse_count_u16(gv),
            QualifierCode::CountAndPrefix8 => self.parse_count_and_prefix_u8(gv),
            QualifierCode::CountAndPrefix16 => self.parse_count_and_prefix_u16(gv),
            _ => Err(ObjectParseError::UnsupportedQualifierCode(qualifier)),
        }
    }

    fn parse_all_objects(&mut self, v: Variation) -> Result<ObjectHeader<'a>, ObjectParseError> {
        match AllObjectsVariation::get(v) {
            Some(av) => Ok(ObjectHeader::new(v, HeaderDetails::AllObjects(av))),
            None => Err(ObjectParseError::InvalidQualifierForVariation(
                v,
                QualifierCode::AllObjects,
            )),
        }
    }

    fn parse_count_u8(&mut self, v: Variation) -> Result<ObjectHeader<'a>, ObjectParseError> {
        let count = self.cursor.read_u8()?;
        let data = CountVariation::parse(v, QualifierCode::Count8, count as u16, &mut self.cursor)?;
        Ok(ObjectHeader::new(
            v,
            HeaderDetails::OneByteCount(count, data),
        ))
    }

    fn parse_count_u16(&mut self, v: Variation) -> Result<ObjectHeader<'a>, ObjectParseError> {
        let count = self.cursor.read_u16_le()?;
        let data = CountVariation::parse(v, QualifierCode::Count16, count, &mut self.cursor)?;
        Ok(ObjectHeader::new(
            v,
            HeaderDetails::TwoByteCount(count, data),
        ))
    }

    fn parse_start_stop_u8(&mut self, v: Variation) -> Result<ObjectHeader<'a>, ObjectParseError> {
        let start = self.cursor.read_u8()?;
        let stop = self.cursor.read_u8()?;
        let range = Range::from(start as u16, stop as u16)?;
        let data = RangedVariation::parse(
            self.function,
            QualifierCode::Range8,
            v,
            range,
            &mut self.cursor,
        )?;
        Ok(ObjectHeader::new(
            v,
            HeaderDetails::OneByteStartStop(start, stop, data),
        ))
    }

    fn parse_start_stop_u16(&mut self, v: Variation) -> Result<ObjectHeader<'a>, ObjectParseError> {
        let start = self.cursor.read_u16_le()?;
        let stop = self.cursor.read_u16_le()?;
        let range = Range::from(start, stop)?;
        let data = RangedVariation::parse(
            self.function,
            QualifierCode::Range16,
            v,
            range,
            &mut self.cursor,
        )?;
        Ok(ObjectHeader::new(
            v,
            HeaderDetails::TwoByteStartStop(start, stop, data),
        ))
    }

    fn parse_count_and_prefix_u8(
        &mut self,
        v: Variation,
    ) -> Result<ObjectHeader<'a>, ObjectParseError> {
        let count = self.cursor.read_u8()?;
        let data = PrefixedVariation::<u8>::parse(v, count as u16, &mut self.cursor)?;
        Ok(ObjectHeader::new(
            v,
            HeaderDetails::OneByteCountAndPrefix(count, data),
        ))
    }

    fn parse_count_and_prefix_u16(
        &mut self,
        v: Variation,
    ) -> Result<ObjectHeader<'a>, ObjectParseError> {
        let count = self.cursor.read_u16_le()?;
        let data = PrefixedVariation::<u16>::parse(v, count, &mut self.cursor)?;
        Ok(ObjectHeader::new(
            v,
            HeaderDetails::TwoByteCountAndPrefix(count, data),
        ))
    }
}

impl<'a> Iterator for ObjectParser<'a> {
    type Item = Result<ObjectHeader<'a>, ObjectParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.parse_one()
    }
}

impl Variation {
    pub(crate) fn parse(cursor: &mut ReadCursor) -> Result<Variation, ObjectParseError> {
        let group = cursor.read_u8()?;
        let var = cursor.read_u8()?;
        match Self::lookup(group, var) {
            Some(gv) => Ok(gv),
            None => Err(ObjectParseError::UnknownGroupVariation(group, var)),
        }
    }
}

impl<'a> RangedVariation<'a> {
    pub(crate) fn parse(
        function: FunctionCode,
        qualifier: QualifierCode,
        v: Variation,
        range: Range,
        cursor: &mut ReadCursor<'a>,
    ) -> Result<RangedVariation<'a>, ObjectParseError> {
        match function {
            FunctionCode::Read => Self::parse_read(v, qualifier),
            _ => Self::parse_non_read(v, qualifier, range, cursor),
        }
    }
}

impl QualifierCode {
    pub(crate) fn parse(cursor: &mut ReadCursor) -> Result<QualifierCode, ObjectParseError> {
        let x = cursor.read_u8()?;
        match Self::from(x) {
            Some(qc) => Ok(qc),
            None => Err(ObjectParseError::UnknownQualifier(x)),
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::app::gen::enums::CommandStatus;
    use crate::app::gen::variations::fixed::*;
    use crate::app::gen::variations::variation::Variation::Group110;
    use crate::app::header::{Control, IIN, IIN1, IIN2};
    use crate::app::parse::bytes::Bytes;
    use crate::app::parse::error::{RequestValidationError, ResponseValidationError};
    use crate::app::parse::prefix::Prefix;
    use crate::app::sequence::Sequence;
    use crate::app::types::{DoubleBit, Timestamp};

    fn test_parse_error(input: &[u8], func: FunctionCode, err: ObjectParseError) {
        assert_eq!(ObjectParser::parse(func, input).err().unwrap(), err);
    }

    fn test_request_validation_error(input: &[u8], err: RequestValidationError) {
        assert_eq!(
            ParsedFragment::parse(DecodeLogLevel::Nothing.receive(), input)
                .unwrap()
                .to_request()
                .err()
                .unwrap(),
            err
        );
    }

    fn test_response_validation_error(input: &[u8], err: ResponseValidationError) {
        assert_eq!(
            ParsedFragment::parse(DecodeLogLevel::Nothing.receive(), input)
                .unwrap()
                .to_response()
                .err()
                .unwrap(),
            err
        );
    }

    #[test]
    fn catches_insufficient_data_for_header() {
        let bad_frames: Vec<&[u8]> = vec![
            &[0x01],
            &[0x01, 0x02],
            &[0x01, 0x02, 0x06, 0x01], // error on 2nd header
            &[0x01, 0x02, 0x00, 0x07],
            &[0x01, 0x02, 0x00, 0x07, 0x08],
            &[0x01, 0x02, 0x00, 0x07, 0x08, 0xFF],
            &[0x01, 0x02, 0x00, 0x07, 0x08, 0xFF],
        ];

        for frame in bad_frames {
            test_parse_error(
                frame,
                FunctionCode::Write,
                ObjectParseError::InsufficientBytes,
            );
        }
    }

    #[test]
    fn parses_valid_request() {
        let fragment = &[0xC2, 0x02, 0xAA];
        let request = ParsedFragment::parse(DecodeLogLevel::Nothing.receive(), fragment)
            .unwrap()
            .to_request()
            .unwrap();
        let expected = RequestHeader {
            control: Control {
                fir: true,
                fin: true,
                con: false,
                uns: false,
                seq: Sequence::new(0x02),
            },
            function: FunctionCode::Write,
        };

        assert_eq!(request.header, expected);
        assert_eq!(request.raw_objects, &[0xAA]);
        assert_eq!(
            request.objects.err().unwrap(),
            ObjectParseError::InsufficientBytes
        )
    }

    #[test]
    fn parses_valid_unsolicited_response() {
        let fragment = &[0b11010010, 0x82, 0xFF, 0xAA, 0x01, 0x02];
        let response = ParsedFragment::parse(DecodeLogLevel::Nothing.receive(), fragment)
            .unwrap()
            .to_response()
            .unwrap();
        let expected = ResponseHeader {
            control: Control {
                fir: true,
                fin: true,
                con: false,
                uns: true,
                seq: Sequence::new(0x02),
            },
            unsolicited: true,
            iin: IIN {
                iin1: IIN1::new(0xFF),
                iin2: IIN2::new(0xAA),
            },
        };

        assert_eq!(response.header, expected);
        assert_eq!(response.raw_objects, &[0x01, 0x02]);
        assert_eq!(
            response.objects.err().unwrap(),
            ObjectParseError::InsufficientBytes
        )
    }

    #[test]
    fn fails_unsolicited_response_without_uns_bit() {
        test_response_validation_error(
            &[0b11000000, 0x82, 0x00, 0x00],
            ResponseValidationError::UnsolicitedResponseWithoutUnsBit,
        );
    }

    #[test]
    fn fails_solicited_response_with_uns_bit() {
        test_response_validation_error(
            &[0b11010000, 0x81, 0x00, 0x00],
            ResponseValidationError::SolicitedResponseWithUnsBit,
        );
    }

    #[test]
    fn fails_bad_request_function_with_uns_bit() {
        test_request_validation_error(
            &[0b11010000, 0x02], // write with UNS
            RequestValidationError::UnexpectedUnsBit(FunctionCode::Write),
        );
    }

    #[test]
    fn confirms_may_or_may_not_have_uns_set() {
        {
            let request =
                ParsedFragment::parse(DecodeLogLevel::Nothing.receive(), &[0b11010000, 0x00])
                    .unwrap()
                    .to_request()
                    .unwrap();
            assert_eq!(request.header.function, FunctionCode::Confirm);
            assert!(request.header.control.uns);
        }
        {
            let request =
                ParsedFragment::parse(DecodeLogLevel::Nothing.receive(), &[0b11000000, 0x00])
                    .unwrap()
                    .to_request()
                    .unwrap();
            assert_eq!(request.header.function, FunctionCode::Confirm);
            assert!(!request.header.control.uns);
        }
    }

    #[test]
    fn parses_integrity_scan() {
        let vec: Vec<HeaderDetails> = ObjectParser::parse(
            FunctionCode::Read,
            &[
                0x3C, 0x02, 0x06, 0x3C, 0x03, 0x06, 0x3C, 0x04, 0x06, 0x3C, 0x01, 0x06,
            ],
        )
        .unwrap()
        .iter()
        .map(|x| x.details)
        .collect();

        assert_eq!(
            vec,
            vec![
                HeaderDetails::AllObjects(AllObjectsVariation::Group60Var2),
                HeaderDetails::AllObjects(AllObjectsVariation::Group60Var3),
                HeaderDetails::AllObjects(AllObjectsVariation::Group60Var4),
                HeaderDetails::AllObjects(AllObjectsVariation::Group60Var1),
            ]
        )
    }

    #[test]
    fn parses_analog_output() {
        let header = &[0x29, 0x01, 0x17, 0x01, 0xFF, 0x01, 0x02, 0x03, 0x04, 0x00];
        let mut headers = ObjectParser::parse(FunctionCode::Operate, header)
            .unwrap()
            .iter();

        let items: Vec<Prefix<u8, Group41Var1>> = assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::OneByteCountAndPrefix(01, PrefixedVariation::<u8>::Group41Var1(seq)) => seq.iter().collect()
        );

        assert_eq!(
            items,
            vec![Prefix::<u8, Group41Var1> {
                index: 0xFF,
                value: Group41Var1 {
                    value: 0x04030201,
                    status: CommandStatus::Success,
                },
            }]
        );
        assert_eq!(headers.next(), None);
    }

    #[test]
    fn parses_range_of_g3v1() {
        let header = &[0x03, 0x01, 0x00, 0x01, 0x04, 0b11_10_01_00];
        let mut headers = ObjectParser::parse(FunctionCode::Response, header)
            .unwrap()
            .iter();

        let items: Vec<(DoubleBit, u16)> = assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::OneByteStartStop(01, 04, RangedVariation::Group3Var1(seq)) => seq.iter().collect()
        );

        assert_eq!(
            items,
            vec![
                (DoubleBit::Intermediate, 1),
                (DoubleBit::DeterminedOff, 2),
                (DoubleBit::DeterminedOn, 3),
                (DoubleBit::Indeterminate, 4),
            ]
        );
        assert_eq!(headers.next(), None);
    }

    #[test]
    fn parses_count_of_time() {
        let header = &[0x32, 0x01, 0x07, 0x01, 0xFF, 0xFE, 0xFD, 0xFC, 0xFB, 0xFA];
        let mut headers = HeaderCollection::parse(FunctionCode::Write, header)
            .unwrap()
            .iter();

        let items: Vec<Group50Var1> = assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::OneByteCount(01, CountVariation::Group50Var1(seq)) => seq.iter().collect()
        );

        assert_eq!(
            items,
            vec![Group50Var1 {
                time: Timestamp::new(0x00_00_FA_FB_FC_FD_FE_FF)
            }]
        );

        assert_eq!(headers.next(), None);
    }

    #[test]
    fn parses_range_of_g1v2_as_non_read() {
        let input = [0x01, 0x02, 0x00, 0x02, 0x03, 0xAA, 0xBB];

        let mut headers = HeaderCollection::parse(FunctionCode::Response, &input)
            .unwrap()
            .iter();

        let items: Vec<(Group1Var2, u16)> = assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::OneByteStartStop(02, 03, RangedVariation::Group1Var2(seq)) => seq.iter().collect()
        );

        assert_eq!(
            items,
            vec![
                (Group1Var2 { flags: 0xAA }, 2),
                (Group1Var2 { flags: 0xBB }, 3)
            ]
        );
        assert_eq!(headers.next(), None);
    }

    #[test]
    fn parses_range_of_g1v2_as_read() {
        let input = [0x01, 0x02, 0x00, 0x02, 0x03, 0x01, 0x02, 0x00, 0x07, 0x09];

        let mut headers = HeaderCollection::parse(FunctionCode::Read, &input)
            .unwrap()
            .iter();

        assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::OneByteStartStop(02, 03, RangedVariation::Group1Var2(seq)) => {
                assert!(seq.is_empty())
            }
        );

        assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::OneByteStartStop(07, 09, RangedVariation::Group1Var2(seq)) => {
                assert!(seq.is_empty())
            }
        );

        assert_eq!(headers.next(), None);
    }

    #[test]
    fn parses_range_of_g80v1() {
        // this is what is typically sent to clear the restart IIN
        let input = [0x50, 0x01, 0x00, 0x07, 0x07, 0x00];
        let mut headers = HeaderCollection::parse(FunctionCode::Write, &input)
            .unwrap()
            .iter();

        let vec: Vec<(bool, u16)> = assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::OneByteStartStop(07, 07, RangedVariation::Group80Var1(seq)) => {
                seq.iter().collect()
            }
        );

        assert_eq!(vec, vec![(false, 7)]);
        assert_eq!(headers.next(), None);
    }

    #[test]
    fn parses_group110var0_as_read() {
        let input = [0x6E, 0x00, 0x00, 0x02, 0x03];
        let mut headers = HeaderCollection::parse(FunctionCode::Read, &input)
            .unwrap()
            .iter();
        assert_eq!(
            headers.next().unwrap().details,
            HeaderDetails::OneByteStartStop(02, 03, RangedVariation::Group110Var0)
        );
        assert_eq!(headers.next(), None);
    }

    #[test]
    fn g110_variations_other_than_0_cannot_be_used_in_read() {
        test_parse_error(
            &[0x6E, 0x01, 0x00, 0x01, 0x02],
            FunctionCode::Read,
            ObjectParseError::InvalidQualifierForVariation(Group110(1), QualifierCode::Range8),
        );
    }

    #[test]
    fn parses_group110var1_as_non_read() {
        let input = [0x6E, 0x01, 0x00, 0x01, 0x02, 0xAA, 0xBB];
        let mut headers = ObjectParser::parse(FunctionCode::Response, &input)
            .unwrap()
            .iter();

        let bytes: Vec<(Bytes, u16)> = assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::OneByteStartStop(01, 02, RangedVariation::Group110VarX(0x01, seq)) => {
                seq.iter().collect()
            }
        );

        assert_eq!(
            bytes,
            vec![(Bytes { value: &[0xAA] }, 1), (Bytes { value: &[0xBB] }, 2)]
        );
        assert_eq!(headers.next(), None);
    }

    #[test]
    fn g110v0_cannot_be_used_in_non_read() {
        test_parse_error(
            &[0x6E, 0x00, 0x00, 0x01, 0x02],
            FunctionCode::Response,
            ObjectParseError::ZeroLengthOctetData,
        );
    }
}
