use std::fmt::{Debug, Formatter};

use xxhash_rust::xxh64::xxh64;

use crate::app::gen::all::AllObjectsVariation;
use crate::app::gen::count::CountVariation;
use crate::app::gen::prefixed::PrefixedVariation;
use crate::app::gen::ranged::RangedVariation;
use crate::app::header::{ControlField, Iin, RequestHeader, ResponseFunction, ResponseHeader};
use crate::app::parse::prefix::Prefix;
use crate::app::parse::range::Range;
use crate::app::parse::traits::{FixedSizeVariation, Index};
use crate::app::parse_error::*;
use crate::app::variations::Variation;
use crate::app::{FunctionCode, QualifierCode};
use crate::decode::AppDecodeLevel;

use crate::app::attr::Attribute;
use crate::app::parse::free_format::FreeFormatVariation;
use scursor::ReadCursor;

pub(crate) fn format_count_of_items<T, V>(f: &mut Formatter, iter: T) -> std::fmt::Result
where
    T: Iterator<Item = V>,
    V: std::fmt::Display,
{
    for x in iter {
        write!(f, "\n{x}")?;
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
        write!(f, "\nindex: {i} {v}")?;
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

pub(crate) fn format_optional_attribute(
    f: &mut Formatter,
    attr: &Option<Attribute>,
) -> std::fmt::Result {
    if let Some(attr) = attr {
        attr.format(f)?;
    }
    Ok(())
}

#[derive(Copy, Clone)]
pub(crate) struct ParsedFragment<'a> {
    pub(crate) control: ControlField,
    pub(crate) function: FunctionCode,
    pub(crate) iin: Option<Iin>,
    pub(crate) objects: Result<HeaderCollection<'a>, ObjectParseError>,
    pub(crate) raw_fragment: &'a [u8],
    pub(crate) raw_objects: &'a [u8],
}

impl<'a> ParsedFragment<'a> {
    pub(crate) fn display(&'a self, level: AppDecodeLevel) -> FragmentDisplay<'a> {
        FragmentDisplay {
            level,
            fragment: *self,
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

    pub(crate) fn to_request(self) -> Result<Request<'a>, RequestValidationError> {
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
            raw_fragment: self.raw_fragment,
            raw_objects: self.raw_objects,
            objects: self.objects,
        })
    }

    pub(crate) fn to_response(self) -> Result<Response<'a>, ResponseValidationError> {
        let (function, iin) = match (self.function, self.iin) {
            (FunctionCode::Response, Some(x)) => (ResponseFunction::Response, x),
            (FunctionCode::UnsolicitedResponse, Some(x)) => {
                (ResponseFunction::UnsolicitedResponse, x)
            }
            _ => return Err(ResponseValidationError::UnexpectedFunction(self.function)),
        };

        if !function.is_unsolicited() && self.control.uns {
            return Err(ResponseValidationError::SolicitedResponseWithUnsBit);
        }

        if function.is_unsolicited() && !self.control.uns {
            return Err(ResponseValidationError::UnsolicitedResponseWithoutUnsBit);
        }

        if function.is_unsolicited() && !self.control.is_fir_and_fin() {
            return Err(ResponseValidationError::UnsolicitedResponseWithoutFirAndFin);
        }

        Ok(Response {
            header: ResponseHeader::new(self.control, function, iin),
            raw_objects: self.raw_objects,
            objects: self.objects,
        })
    }

    fn parse_no_logging(fragment: &'a [u8]) -> Result<Self, HeaderParseError> {
        let mut cursor = ReadCursor::new(fragment);

        let control = ControlField::parse(&mut cursor)?;
        let raw_func = cursor.read_u8()?;
        let function = match FunctionCode::from(raw_func) {
            None => return Err(HeaderParseError::UnknownFunction(control.seq, raw_func)),
            Some(x) => x,
        };
        let iin = match function {
            FunctionCode::Response => Some(Iin::parse(&mut cursor)?),
            FunctionCode::UnsolicitedResponse => Some(Iin::parse(&mut cursor)?),
            _ => None,
        };

        let objects = cursor.read_all();
        let fragment = Self {
            control,
            function,
            iin,
            objects: HeaderCollection::parse(function, objects),
            raw_fragment: fragment,
            raw_objects: objects,
        };

        Ok(fragment)
    }

    pub(crate) fn parse(fragment: &'a [u8]) -> Result<Self, HeaderParseError> {
        Self::parse_no_logging(fragment)
    }
}

#[derive(Debug)]
pub(crate) struct ObjectHeader<'a> {
    pub(crate) variation: Variation,
    pub(crate) details: HeaderDetails<'a>,
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
            HeaderDetails::TwoByteFreeFormat(count, var) => {
                write!(
                    f,
                    "{} : {} - {} - [{}]",
                    self.variation,
                    self.variation.description(),
                    self.details.qualifier().description(),
                    count
                )?;
                if format_values {
                    var.format_objects(f)?;
                }
                Ok(())
            }
        }
    }
}

pub(crate) struct FragmentDisplay<'a> {
    level: AppDecodeLevel,
    fragment: ParsedFragment<'a>,
}

impl std::fmt::Display for FragmentDisplay<'_> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fragment.format_header(f)?;

        match self.fragment.objects {
            Ok(headers) => {
                if !self.level.object_headers() {
                    return Ok(());
                }
                for header in headers.iter() {
                    f.write_str("\n")?;
                    header.format(self.level.object_values(), f)?
                }
            }
            Err(err) => {
                // if an error occurred, we re-parse the object headers so we can log any headers before the error
                for header in
                    ObjectParser::one_pass(self.fragment.function, self.fragment.raw_objects)
                        .flatten()
                {
                    f.write_str("\n")?;
                    header.format(self.level.object_values(), f)?;
                }
                write!(f, "\ndecoding err: {err}")?;
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub(crate) enum HeaderDetails<'a> {
    AllObjects(AllObjectsVariation),
    OneByteStartStop(u8, u8, RangedVariation<'a>),
    TwoByteStartStop(u16, u16, RangedVariation<'a>),
    OneByteCount(u8, CountVariation<'a>),
    TwoByteCount(u16, CountVariation<'a>),
    OneByteCountAndPrefix(u8, PrefixedVariation<'a, u8>),
    TwoByteCountAndPrefix(u16, PrefixedVariation<'a, u16>),
    TwoByteFreeFormat(u8, FreeFormatVariation<'a>),
}

impl HeaderDetails<'_> {
    pub(crate) fn qualifier(&self) -> QualifierCode {
        match self {
            HeaderDetails::AllObjects(_) => QualifierCode::AllObjects,
            HeaderDetails::OneByteStartStop(_, _, _) => QualifierCode::Range8,
            HeaderDetails::TwoByteStartStop(_, _, _) => QualifierCode::Range16,
            HeaderDetails::OneByteCount(_, _) => QualifierCode::Count8,
            HeaderDetails::TwoByteCount(_, _) => QualifierCode::Count16,
            HeaderDetails::OneByteCountAndPrefix(_, _) => QualifierCode::CountAndPrefix8,
            HeaderDetails::TwoByteCountAndPrefix(_, _) => QualifierCode::CountAndPrefix16,
            HeaderDetails::TwoByteFreeFormat(_, _) => QualifierCode::FreeFormat16,
        }
    }

    pub(crate) fn count(&self) -> Option<&CountVariation> {
        match self {
            HeaderDetails::OneByteCount(_, objects) => Some(objects),
            HeaderDetails::TwoByteCount(_, objects) => Some(objects),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Request<'a> {
    pub(crate) header: RequestHeader,
    pub(crate) raw_fragment: &'a [u8],
    pub(crate) raw_objects: &'a [u8],
    pub(crate) objects: Result<HeaderCollection<'a>, ObjectParseError>,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Response<'a> {
    pub(crate) header: ResponseHeader,
    pub(crate) raw_objects: &'a [u8],
    pub(crate) objects: Result<HeaderCollection<'a>, ObjectParseError>,
}

impl<'a> Response<'a> {
    pub(crate) fn get_only_object_header(&self) -> Result<ObjectHeader<'a>, SingleHeaderError> {
        self.objects?.get_only_header()
    }
}

#[derive(Copy, Clone)]
struct ObjectParser<'a> {
    errored: bool,
    function: FunctionCode,
    cursor: ReadCursor<'a>,
}

/// An abstract collection of pre-validated object headers
/// that can provide an iterator of the headers.
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct HeaderCollection<'a> {
    function: FunctionCode,
    data: &'a [u8],
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum SingleHeaderError {
    BadParse,
    NoHeaders,
    MoreThanOneHeader,
}

impl From<ObjectParseError> for SingleHeaderError {
    fn from(_: ObjectParseError) -> Self {
        Self::BadParse
    }
}

impl<'a> HeaderCollection<'a> {
    /// parse the the raw header data in accordance with the provided function code
    pub(crate) fn parse(function: FunctionCode, data: &'a [u8]) -> Result<Self, ObjectParseError> {
        ObjectParser::parse(function, data)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// return and iterator of the headers that lazily parses them
    pub(crate) fn iter(&self) -> HeaderIterator<'a> {
        HeaderIterator {
            parser: ObjectParser::one_pass(self.function, self.data),
        }
    }

    pub(crate) fn get_only_header(&self) -> Result<ObjectHeader<'a>, SingleHeaderError> {
        let mut iter = self.iter();
        match iter.next() {
            None => Err(SingleHeaderError::NoHeaders),
            Some(x) => match iter.next() {
                Some(_) => Err(SingleHeaderError::MoreThanOneHeader),
                None => Ok(x),
            },
        }
    }

    pub(crate) fn hash(&self) -> u64 {
        xxh64(self.data, 0)
    }
}

#[derive(Copy, Clone)]
pub(crate) struct HeaderIterator<'a> {
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
            result?;
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
            QualifierCode::FreeFormat16 => self.parse_free_format_u16(gv),
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

    fn parse_free_format_u16(
        &mut self,
        v: Variation,
    ) -> Result<ObjectHeader<'a>, ObjectParseError> {
        // read the count of objects
        let count = self.cursor.read_u8()?;

        if count != 1 {
            return Err(ObjectParseError::UnsupportedFreeFormatCount(count));
        }

        // read the length of free-format data
        let length = self.cursor.read_u16_le()?;
        // the bytes inside the header
        let bytes = self.cursor.read_bytes(length as usize)?;

        // create a sub-cursor over only these bytes
        let mut cursor = ReadCursor::new(bytes);

        let variation = FreeFormatVariation::parse(v, &mut cursor)?;

        // all of the designated data must be consumed otherwise the encoding is invalid
        cursor.expect_empty()?;

        Ok(ObjectHeader::new(
            v,
            HeaderDetails::TwoByteFreeFormat(count, variation),
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
    use crate::app::attr::{AttrParseError, AttrSet, AttrValue, Attribute};
    use crate::app::control::CommandStatus;
    use crate::app::header::{ControlField, Iin, Iin1, Iin2};
    use crate::app::measurement::DoubleBit;
    use crate::app::parse::prefix::Prefix;
    use crate::app::parse_error::ResponseValidationError;
    use crate::app::sequence::Sequence;
    use crate::app::types::Timestamp;
    use crate::app::variations::*;

    use super::*;

    fn test_parse_error(input: &[u8], func: FunctionCode, err: ObjectParseError) {
        assert_eq!(ObjectParser::parse(func, input).err().unwrap(), err);
    }

    fn test_request_validation_error(input: &[u8], err: RequestValidationError) {
        assert_eq!(
            ParsedFragment::parse(input)
                .unwrap()
                .to_request()
                .err()
                .unwrap(),
            err
        );
    }

    fn test_response_validation_error(input: &[u8], err: ResponseValidationError) {
        assert_eq!(
            ParsedFragment::parse(input)
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
        let request = ParsedFragment::parse(fragment)
            .unwrap()
            .to_request()
            .unwrap();
        let expected = RequestHeader {
            control: ControlField {
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
        let response = ParsedFragment::parse(fragment)
            .unwrap()
            .to_response()
            .unwrap();
        let expected = ResponseHeader {
            control: ControlField {
                fir: true,
                fin: true,
                con: false,
                uns: true,
                seq: Sequence::new(0x02),
            },
            function: ResponseFunction::UnsolicitedResponse,
            iin: Iin {
                iin1: Iin1::new(0xFF),
                iin2: Iin2::new(0xAA),
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
            let request = ParsedFragment::parse(&[0b11010000, 0x00])
                .unwrap()
                .to_request()
                .unwrap();
            assert_eq!(request.header.function, FunctionCode::Confirm);
            assert!(request.header.control.uns);
        }
        {
            let request = ParsedFragment::parse(&[0b11000000, 0x00])
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

        assert_eq!(vec.len(), 4);
        assert_matches!(
            vec[0],
            HeaderDetails::AllObjects(AllObjectsVariation::Group60Var2)
        );
        assert_matches!(
            vec[1],
            HeaderDetails::AllObjects(AllObjectsVariation::Group60Var3)
        );
        assert_matches!(
            vec[2],
            HeaderDetails::AllObjects(AllObjectsVariation::Group60Var4)
        );
        assert_matches!(
            vec[3],
            HeaderDetails::AllObjects(AllObjectsVariation::Group60Var1)
        );
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
        assert_matches!(headers.next(), None);
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
        assert_matches!(headers.next(), None);
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

        assert_matches!(headers.next(), None);
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
        assert_matches!(headers.next(), None);
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
                assert!(seq.iter().next().is_none())
            }
        );

        assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::OneByteStartStop(07, 09, RangedVariation::Group1Var2(seq)) => {
                assert!(seq.iter().next().is_none())
            }
        );

        assert_matches!(headers.next(), None);
    }

    #[test]
    fn parses_g34_var1_with_count_and_prefix() {
        let input = [0x22, 0x01, 0x17, 0x01, 0x03, 0xCA, 0xFE];

        let mut headers = HeaderCollection::parse(FunctionCode::Write, &input)
            .unwrap()
            .iter();

        assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::OneByteCountAndPrefix(01, PrefixedVariation::Group34Var1(seq)) => {
                let prefix = seq.single().unwrap();
                assert_eq!(prefix, Prefix { index: 0x03, value: Group34Var1 { value: 0xFECA }});
            }
        );

        assert_matches!(headers.next(), None);
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
        assert_matches!(headers.next(), None);
    }

    #[test]
    fn parses_count_of_g50v2() {
        let input = [
            0x32, 0x02, 0x07, 0x01, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0xAA, 0xBB, 0xCC, 0xDD,
        ];
        let mut headers = HeaderCollection::parse(FunctionCode::Write, &input)
            .unwrap()
            .iter();

        let received = match headers.next().unwrap().details {
            HeaderDetails::OneByteCount(01, CountVariation::Group50Var2(seq)) => {
                seq.single().unwrap()
            }
            _ => unreachable!(),
        };

        assert!(headers.next().is_none());

        let expected = Group50Var2 {
            time: Timestamp::new(0xFF),
            interval: 0xDDCCBBAA,
        };

        assert_eq!(received, expected);
    }

    #[test]
    fn parses_group110var0_as_read() {
        let input = [0x6E, 0x00, 0x00, 0x02, 0x03];
        let mut headers = HeaderCollection::parse(FunctionCode::Read, &input)
            .unwrap()
            .iter();
        assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::OneByteStartStop(02, 03, RangedVariation::Group110Var0)
        );
        assert_matches!(headers.next(), None);
    }

    #[test]
    fn g110_variations_other_than_0_cannot_be_used_in_read() {
        test_parse_error(
            &[0x6E, 0x01, 0x00, 0x01, 0x02],
            FunctionCode::Read,
            ObjectParseError::InvalidQualifierForVariation(
                Variation::Group110(1),
                QualifierCode::Range8,
            ),
        );
    }

    #[test]
    fn parses_group110var1_as_non_read() {
        let input = [0x6E, 0x01, 0x00, 0x01, 0x02, 0xAA, 0xBB];
        let mut headers = ObjectParser::parse(FunctionCode::Response, &input)
            .unwrap()
            .iter();

        let bytes: Vec<(&[u8], u16)> = assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::OneByteStartStop(01, 02, RangedVariation::Group110VarX(0x01, seq)) => {
                seq.iter().collect()
            }
        );

        assert_eq!(bytes, vec![([0xAA].as_slice(), 1), ([0xBB].as_slice(), 2)]);
        assert_matches!(headers.next(), None);
    }

    #[test]
    fn parses_group111var1_as_non_read() {
        let input = [
            0x6F, 0x01, 0x28, 0x02, 0x00, 0x01, 0x00, 0xAA, 0x02, 0x00, 0xBB,
        ];
        let mut headers = ObjectParser::parse(FunctionCode::Response, &input)
            .unwrap()
            .iter();

        let bytes: Vec<(&[u8], u16)> = assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::TwoByteCountAndPrefix(0x02, PrefixedVariation::Group111VarX(0x01, seq)) => {
                seq.iter().collect()
            }
        );

        assert_eq!(bytes, vec![([0xAA].as_slice(), 1), ([0xBB].as_slice(), 2)]);
        assert_matches!(headers.next(), None);
    }

    #[test]
    fn g110v0_cannot_be_used_in_non_read() {
        test_parse_error(
            &[0x6E, 0x00, 0x00, 0x01, 0x02],
            FunctionCode::Response,
            ObjectParseError::ZeroLengthOctetData,
        );
    }

    #[test]
    fn parses_specific_attribute_in_range() {
        let input: &[u8] = &[0x00, 0xCA, 0x00, 0x07, 0x07, 0x02, 0x01, 42];

        let mut headers = ObjectParser::parse(FunctionCode::Response, input)
            .unwrap()
            .iter();

        let first = headers.next().unwrap();
        assert!(headers.next().is_none());

        assert_eq!(first.variation, Variation::Group0(0xCA));
        let set = match first.details {
            HeaderDetails::OneByteStartStop(
                0x07,
                0x07,
                RangedVariation::Group0(0xCA, Some(set)),
            ) => set,
            _ => unreachable!(),
        };

        assert_eq!(
            set,
            Attribute {
                set: AttrSet::Private(0x07),
                variation: 0xCA,
                value: AttrValue::UnsignedInt(42)
            }
        );
    }

    #[test]
    fn parses_specific_attribute_in_read_request() {
        let input: &[u8] = &[0x00, 0xCA, 0x00, 0x07, 0x07];

        let mut headers = ObjectParser::parse(FunctionCode::Read, input)
            .unwrap()
            .iter();

        let first = headers.next().unwrap();
        assert!(headers.next().is_none());

        assert_eq!(first.variation, Variation::Group0(0xCA));
        assert!(std::matches!(
            first.details,
            HeaderDetails::OneByteStartStop(0x07, 0x07, RangedVariation::Group0(0xCA, None))
        ));
    }

    #[test]
    fn range_parsing_fails_for_specific_attribute_with_count_equal_two() {
        let input: &[u8] = &[0x00, 0xCA, 0x00, 0x07, 0x08, 0x02, 0x01, 42];
        let err = ObjectParser::parse(FunctionCode::Response, input).unwrap_err();
        assert_eq!(
            err,
            ObjectParseError::BadAttribute(AttrParseError::CountNotOne(2))
        );
    }

    #[test]
    fn parses_free_format() {
        let input: &[u8] = &[
            70, 5, 0x5B, 0x01, 12, 0x00, 0x01, 0x02, 0x03, 0x04, 0xAA, 0xBB, 0xCC, 0xDD, b'd',
            b'a', b't', b'a',
        ];
        let headers = ObjectParser::parse(FunctionCode::Response, input).unwrap();
        let header = headers.iter().next().unwrap();

        assert_eq!(header.variation, Variation::Group70Var5);
        let obj = match header.details {
            HeaderDetails::TwoByteFreeFormat(1, FreeFormatVariation::Group70Var5(obj)) => obj,
            _ => unreachable!(),
        };
        assert_eq!(
            obj,
            crate::app::file::Group70Var5 {
                file_handle: 0x04030201,
                block_number: 0xDDCCBBAA,
                file_data: &[b'd', b'a', b't', b'a'],
            }
        );
    }
}
