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
use crate::app::parse::options::ParseOptions;
use scursor::{ReadCursor, ReadError};

#[derive(Copy, Clone)]
pub(crate) struct ParsedFragment<'a> {
    pub(crate) control: ControlField,
    pub(crate) function: FunctionCode,
    pub(crate) options: ParseOptions,
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

    #[cfg_attr(all(test, dnp3_no_panic), no_panic::no_panic)]
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
            objects: self.objects,
        })
    }

    #[cfg_attr(all(test, dnp3_no_panic), no_panic::no_panic)]
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

    #[cfg_attr(all(test, dnp3_no_panic), no_panic::no_panic)]
    pub(crate) fn parse_no_logging(
        options: ParseOptions,
        fragment: &'a [u8],
    ) -> Result<Self, HeaderParseError> {
        let mut cursor = ReadCursor::new(fragment);

        let control = ControlField::parse(&mut cursor)?;
        let raw_func = cursor.read_u8()?;
        let function = match FunctionCode::from(raw_func) {
            None => return Err(HeaderParseError::UnknownFunction(control.seq, raw_func)),
            Some(x) => x,
        };
        let iin = if function.get_function_info().has_iin {
            Some(Iin::parse(&mut cursor)?)
        } else {
            None
        };

        let objects = cursor.read_all();
        let fragment = Self {
            control,
            function,
            options,
            iin,
            objects: HeaderCollection::parse(options, function, objects),
            raw_fragment: fragment,
            raw_objects: objects,
        };

        Ok(fragment)
    }

    pub(crate) fn parse(
        options: ParseOptions,
        fragment: &'a [u8],
    ) -> Result<Self, HeaderParseError> {
        Self::parse_no_logging(options, fragment)
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
        write!(
            f,
            "{} : {} - {}",
            self.variation,
            self.variation.description(),
            self.details.qualifier().description()
        )?;

        match &self.details {
            HeaderDetails::AllObjects(_) => return Ok(()),
            HeaderDetails::Range(_, range, _) => {
                write!(f, " - [{}, {}]", range.get_start(), range.get_stop())?
            }
            HeaderDetails::Count(_, count, _) => write!(f, " - [{count}]")?,
            HeaderDetails::CountAndPrefix8(count, _) => write!(f, " - [{count}]")?,
            HeaderDetails::CountAndPrefix16(count, _) => write!(f, " - [{count}]")?,
            HeaderDetails::FreeFormat(_) => f.write_str(" - [1]")?,
        }

        if !format_values {
            return Ok(());
        }

        match &self.details {
            HeaderDetails::AllObjects(_) => Ok(()),
            HeaderDetails::Range(_, _, seq) => seq.format_objects(f),
            HeaderDetails::Count(_, _, seq) => seq.format_objects(f),
            HeaderDetails::CountAndPrefix8(_, seq) => seq.format_objects(f),
            HeaderDetails::CountAndPrefix16(_, seq) => seq.format_objects(f),
            HeaderDetails::FreeFormat(var) => var.format_objects(f),
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
                for header in ObjectParser::one_pass(
                    self.fragment.options,
                    self.fragment.function,
                    self.fragment.raw_objects,
                )
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

/// Width of the start/stop indices or the count in a range or count qualifier
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum Width {
    One,
    Two,
}

impl Width {
    fn range_qualifier(self) -> QualifierCode {
        match self {
            Width::One => QualifierCode::Range8,
            Width::Two => QualifierCode::Range16,
        }
    }

    fn count_qualifier(self) -> QualifierCode {
        match self {
            Width::One => QualifierCode::Count8,
            Width::Two => QualifierCode::Count16,
        }
    }

    fn read(self, cursor: &mut ReadCursor) -> Result<u16, ReadError> {
        match self {
            Width::One => Ok(cursor.read_u8()?.into()),
            Width::Two => cursor.read_u16_le(),
        }
    }
}

#[derive(Debug)]
pub(crate) enum HeaderDetails<'a> {
    AllObjects(AllObjectsVariation),
    Range(Width, Range, RangedVariation<'a>),
    Count(Width, u16, CountVariation<'a>),
    CountAndPrefix8(u8, PrefixedVariation<'a, u8>),
    CountAndPrefix16(u16, PrefixedVariation<'a, u16>),
    /// the count of a free-format header is always 1
    FreeFormat(FreeFormatVariation<'a>),
}

impl HeaderDetails<'_> {
    pub(crate) fn qualifier(&self) -> QualifierCode {
        match self {
            HeaderDetails::AllObjects(_) => QualifierCode::AllObjects,
            HeaderDetails::Range(width, _, _) => width.range_qualifier(),
            HeaderDetails::Count(width, _, _) => width.count_qualifier(),
            HeaderDetails::CountAndPrefix8(_, _) => QualifierCode::CountAndPrefix8,
            HeaderDetails::CountAndPrefix16(_, _) => QualifierCode::CountAndPrefix16,
            HeaderDetails::FreeFormat(_) => QualifierCode::FreeFormat16,
        }
    }

    pub(crate) fn count(&self) -> Option<&CountVariation<'_>> {
        match self {
            HeaderDetails::Count(_, _, objects) => Some(objects),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct Request<'a> {
    pub(crate) header: RequestHeader,
    pub(crate) raw_fragment: &'a [u8],
    pub(crate) objects: Result<HeaderCollection<'a>, ObjectParseError>,
}

#[derive(Copy, Clone, Debug)]
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
    options: ParseOptions,
    cursor: ReadCursor<'a>,
}

/// An abstract collection of pre-validated object headers
/// that can provide an iterator of the headers.
#[derive(Copy, Clone, Debug)]
pub(crate) struct HeaderCollection<'a> {
    options: ParseOptions,
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
    /// parse the raw header data in accordance with the provided function code
    pub(crate) fn parse(
        options: ParseOptions,
        function: FunctionCode,
        data: &'a [u8],
    ) -> Result<Self, ObjectParseError> {
        ObjectParser::parse(options, function, data)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// return and iterator of the headers that lazily parses them
    pub(crate) fn iter(&self) -> HeaderIterator<'a> {
        HeaderIterator {
            parser: ObjectParser::one_pass(self.options, self.function, self.data),
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

    #[cfg_attr(all(test, dnp3_no_panic), no_panic::no_panic)]
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
        options: ParseOptions,
        function: FunctionCode,
        data: &'a [u8],
    ) -> Result<HeaderCollection<'a>, ObjectParseError> {
        // we first do a single pass to ensure the ASDU is well-formed, returning an error if it occurs
        for result in ObjectParser::one_pass(options, function, data) {
            result?;
        }

        // now we know that we know the headers are well-formed, our 2nd pass
        // can use the HeaderCollection iterator implementation to read them
        Ok(HeaderCollection {
            options,
            function,
            data,
        })
    }

    fn one_pass(options: ParseOptions, function: FunctionCode, data: &'a [u8]) -> Self {
        ObjectParser {
            options,
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
            QualifierCode::Range8 => self.parse_range(gv, Width::One),
            QualifierCode::Range16 => self.parse_range(gv, Width::Two),
            QualifierCode::Count8 => self.parse_count(gv, Width::One),
            QualifierCode::Count16 => self.parse_count(gv, Width::Two),
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

    fn parse_count(
        &mut self,
        v: Variation,
        width: Width,
    ) -> Result<ObjectHeader<'a>, ObjectParseError> {
        let count = width.read(&mut self.cursor)?;
        let data = CountVariation::parse(
            self.function,
            width.count_qualifier(),
            v,
            count,
            &mut self.cursor,
        )?;
        Ok(ObjectHeader::new(
            v,
            HeaderDetails::Count(width, count, data),
        ))
    }

    fn parse_range(
        &mut self,
        v: Variation,
        width: Width,
    ) -> Result<ObjectHeader<'a>, ObjectParseError> {
        let start = width.read(&mut self.cursor)?;
        let stop = width.read(&mut self.cursor)?;
        let range = Range::from(start, stop)?;
        let data = RangedVariation::parse(
            self.options,
            self.function,
            width.range_qualifier(),
            v,
            range,
            &mut self.cursor,
        )?;
        Ok(ObjectHeader::new(
            v,
            HeaderDetails::Range(width, range, data),
        ))
    }

    fn parse_count_and_prefix_u8(
        &mut self,
        v: Variation,
    ) -> Result<ObjectHeader<'a>, ObjectParseError> {
        let count = self.cursor.read_u8()?;
        let data = PrefixedVariation::<u8>::parse(v, count.into(), self.options, &mut self.cursor)?;
        Ok(ObjectHeader::new(
            v,
            HeaderDetails::CountAndPrefix8(count, data),
        ))
    }

    fn parse_count_and_prefix_u16(
        &mut self,
        v: Variation,
    ) -> Result<ObjectHeader<'a>, ObjectParseError> {
        let count = self.cursor.read_u16_le()?;
        let data = PrefixedVariation::<u16>::parse(v, count, self.options, &mut self.cursor)?;
        Ok(ObjectHeader::new(
            v,
            HeaderDetails::CountAndPrefix16(count, data),
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
        let bytes = self.cursor.read_bytes(length.into())?;

        // create a sub-cursor over only these bytes
        let mut cursor = ReadCursor::new(bytes);

        let variation = FreeFormatVariation::parse(v, &mut cursor)?;

        // all of the designated data must be consumed otherwise the encoding is invalid
        cursor.expect_empty()?;

        Ok(ObjectHeader::new(v, HeaderDetails::FreeFormat(variation)))
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
        options: ParseOptions,
        function: FunctionCode,
        qualifier: QualifierCode,
        v: Variation,
        range: Range,
        cursor: &mut ReadCursor<'a>,
    ) -> Result<RangedVariation<'a>, ObjectParseError> {
        if function == FunctionCode::Read {
            Self::parse_read(v, qualifier)
        } else {
            Self::parse_non_read(v, qualifier, range, options, cursor)
        }
    }
}

impl<'a> CountVariation<'a> {
    pub(crate) fn parse(
        function: FunctionCode,
        qualifier: QualifierCode,
        v: Variation,
        count: u16,
        cursor: &mut ReadCursor<'a>,
    ) -> Result<CountVariation<'a>, ObjectParseError> {
        if function == FunctionCode::Read {
            Self::parse_read(v, qualifier)
        } else {
            Self::parse_non_read(v, qualifier, count, cursor)
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
        assert_eq!(
            ObjectParser::parse(ParseOptions::default(), func, input)
                .err()
                .unwrap(),
            err
        );
    }

    fn test_request_validation_error(input: &[u8], err: RequestValidationError) {
        assert_eq!(
            ParsedFragment::parse(ParseOptions::default(), input)
                .unwrap()
                .to_request()
                .err()
                .unwrap(),
            err
        );
    }

    fn test_response_validation_error(input: &[u8], err: ResponseValidationError) {
        assert_eq!(
            ParsedFragment::parse(ParseOptions::default(), input)
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
        let request = ParsedFragment::parse(ParseOptions::default(), fragment)
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
        assert_eq!(
            request.objects.err().unwrap(),
            ObjectParseError::InsufficientBytes
        )
    }

    #[test]
    fn parses_valid_unsolicited_response() {
        let fragment = &[0b11010010, 0x82, 0xFF, 0xAA, 0x01, 0x02];
        let response = ParsedFragment::parse(ParseOptions::default(), fragment)
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
            let request = ParsedFragment::parse(ParseOptions::default(), &[0b11010000, 0x00])
                .unwrap()
                .to_request()
                .unwrap();
            assert_eq!(request.header.function, FunctionCode::Confirm);
            assert!(request.header.control.uns);
        }
        {
            let request = ParsedFragment::parse(ParseOptions::default(), &[0b11000000, 0x00])
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
            ParseOptions::default(),
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
        let mut headers =
            ObjectParser::parse(ParseOptions::default(), FunctionCode::Operate, header)
                .unwrap()
                .iter();

        let items: Vec<Prefix<u8, Group41Var1>> = assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::CountAndPrefix8(1, PrefixedVariation::<u8>::Group41Var1(seq)) => seq.iter().collect()
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
        let mut headers =
            ObjectParser::parse(ParseOptions::default(), FunctionCode::Response, header)
                .unwrap()
                .iter();

        let items: Vec<(DoubleBit, u16)> = assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::Range(Width::One, range, RangedVariation::Group3Var1(seq)) if range == Range::from(1, 4).unwrap() => seq.iter().collect()
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
        let mut headers =
            HeaderCollection::parse(ParseOptions::default(), FunctionCode::Write, header)
                .unwrap()
                .iter();

        let items: Vec<Group50Var1> = assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::Count(Width::One, 1, CountVariation::Group50Var1(seq)) => seq.iter().collect()
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

        let mut headers =
            HeaderCollection::parse(ParseOptions::default(), FunctionCode::Response, &input)
                .unwrap()
                .iter();

        let items: Vec<(Group1Var2, u16)> = assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::Range(Width::One, range, RangedVariation::Group1Var2(seq)) if range == Range::from(2, 3).unwrap() => seq.iter().collect()
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

        let mut headers =
            HeaderCollection::parse(ParseOptions::default(), FunctionCode::Read, &input)
                .unwrap()
                .iter();

        assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::Range(Width::One, range, RangedVariation::Group1Var2(seq)) if range == Range::from(2, 3).unwrap() => {
                assert!(seq.iter().next().is_none())
            }
        );

        assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::Range(Width::One, range, RangedVariation::Group1Var2(seq)) if range == Range::from(7, 9).unwrap() => {
                assert!(seq.iter().next().is_none())
            }
        );

        assert_matches!(headers.next(), None);
    }

    #[test]
    fn parses_g34_var1_with_count_and_prefix() {
        let input = [0x22, 0x01, 0x17, 0x01, 0x03, 0xCA, 0xFE];

        let mut headers =
            HeaderCollection::parse(ParseOptions::default(), FunctionCode::Write, &input)
                .unwrap()
                .iter();

        assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::CountAndPrefix8(1, PrefixedVariation::Group34Var1(seq)) => {
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
        let mut headers =
            HeaderCollection::parse(ParseOptions::default(), FunctionCode::Write, &input)
                .unwrap()
                .iter();

        let vec: Vec<(bool, u16)> = assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::Range(Width::One, range, RangedVariation::Group80Var1(seq)) if range == Range::from(7, 7).unwrap() => {
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
        let mut headers =
            HeaderCollection::parse(ParseOptions::default(), FunctionCode::Write, &input)
                .unwrap()
                .iter();

        let received = match headers.next().unwrap().details {
            HeaderDetails::Count(Width::One, 1, CountVariation::Group50Var2(seq)) => {
                seq.single().unwrap()
            }
            _ => panic!("unexpected header details"),
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
        let mut headers =
            HeaderCollection::parse(ParseOptions::default(), FunctionCode::Read, &input)
                .unwrap()
                .iter();
        assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::Range(Width::One, range, RangedVariation::Group110Var0) if range == Range::from(2, 3).unwrap()
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
        let mut headers =
            ObjectParser::parse(ParseOptions::default(), FunctionCode::Response, &input)
                .unwrap()
                .iter();

        let bytes: Vec<(&[u8], u16)> = assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::Range(Width::One, range, RangedVariation::Group110VarX(0x01, seq)) if range == Range::from(1, 2).unwrap() => {
                seq.iter().collect()
            }
        );

        assert_eq!(bytes, vec![([0xAA].as_slice(), 1), ([0xBB].as_slice(), 2)]);
        assert_matches!(headers.next(), None);
    }

    #[test]
    fn parses_group110var1_at_max_index() {
        let input = [0x6E, 0x01, 0x01, 0xFF, 0xFF, 0xFF, 0xFF, 0xAA];
        let mut headers =
            ObjectParser::parse(ParseOptions::default(), FunctionCode::Response, &input)
                .unwrap()
                .iter();

        let bytes: Vec<(&[u8], u16)> = assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::Range(Width::Two, range, RangedVariation::Group110VarX(0x01, seq)) if range == Range::from(0xFFFF, 0xFFFF).unwrap() => {
                seq.iter().collect()
            }
        );

        assert_eq!(bytes, vec![([0xAA].as_slice(), 0xFFFF)]);
        assert_matches!(headers.next(), None);
    }

    #[test]
    fn parses_group111var1_as_non_read() {
        let input = [
            0x6F, 0x01, 0x28, 0x02, 0x00, 0x01, 0x00, 0xAA, 0x02, 0x00, 0xBB,
        ];
        let mut headers =
            ObjectParser::parse(ParseOptions::default(), FunctionCode::Response, &input)
                .unwrap()
                .iter();

        let bytes: Vec<(&[u8], u16)> = assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::CountAndPrefix16(0x02, PrefixedVariation::Group111VarX(0x01, seq)) => {
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
    fn g110v0_can_be_parsed_if_enabled() {
        let options = ParseOptions {
            parse_zero_length_strings: true,
        };
        let mut headers = ObjectParser::parse(
            options,
            FunctionCode::Response,
            &[0x6E, 0x00, 0x00, 0x07, 0x07],
        )
        .unwrap()
        .iter();
        let first = headers.next().unwrap();
        assert!(headers.next().is_none());
        match first.details {
            HeaderDetails::Range(Width::One, range, RangedVariation::Group110VarX(0, seq))
                if range == Range::from(7, 7).unwrap() =>
            {
                let mut iter = seq.iter();
                let first: (&[u8], u16) = iter.next().unwrap();
                let empty: &[u8] = &[];
                assert_eq!(first, (empty, 7));
                assert_eq!(iter.next(), None);
            }
            _ => panic!("unexpected header details"),
        }
    }

    #[test]
    fn parses_specific_attribute_in_range() {
        let input: &[u8] = &[0x00, 0xCA, 0x00, 0x07, 0x07, 0x02, 0x01, 42];

        let mut headers =
            ObjectParser::parse(ParseOptions::default(), FunctionCode::Response, input)
                .unwrap()
                .iter();

        let first = headers.next().unwrap();
        assert!(headers.next().is_none());

        assert_eq!(first.variation, Variation::Group0(0xCA));
        let set = match first.details {
            HeaderDetails::Range(Width::One, range, RangedVariation::Group0(0xCA, Some(set)))
                if range == Range::from(0x07, 0x07).unwrap() =>
            {
                set
            }
            _ => panic!("unexpected header details"),
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

        let mut headers = ObjectParser::parse(ParseOptions::default(), FunctionCode::Read, input)
            .unwrap()
            .iter();

        let first = headers.next().unwrap();
        assert!(headers.next().is_none());

        assert_eq!(first.variation, Variation::Group0(0xCA));
        assert!(std::matches!(
            first.details,
            HeaderDetails::Range(Width::One, range, RangedVariation::Group0(0xCA, None)) if range == Range::from(0x07, 0x07).unwrap()
        ));
    }

    #[test]
    fn range_parsing_fails_for_specific_attribute_with_count_equal_two() {
        let input: &[u8] = &[0x00, 0xCA, 0x00, 0x07, 0x08, 0x02, 0x01, 42];
        let err = ObjectParser::parse(ParseOptions::default(), FunctionCode::Response, input)
            .unwrap_err();
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
        let headers =
            ObjectParser::parse(ParseOptions::default(), FunctionCode::Response, input).unwrap();
        let header = headers.iter().next().unwrap();

        assert_eq!(header.variation, Variation::Group70Var5);
        let obj = match header.details {
            HeaderDetails::FreeFormat(FreeFormatVariation::Group70Var5(obj)) => obj,
            _ => panic!("unexpected header details"),
        };
        assert_eq!(
            obj,
            crate::app::file::Group70Var5 {
                file_handle: 0x04030201,
                block_number: 0xDDCCBBAA,
                file_data: b"data",
            }
        );
    }

    #[test]
    fn count_of_time_in_read_carries_no_data() {
        let mut headers = ObjectParser::parse(
            ParseOptions::default(),
            FunctionCode::Read,
            &[0x32, 0x01, 0x07, 0x01],
        )
        .unwrap()
        .iter();
        assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::Count(Width::One, 1, CountVariation::Group50Var1(seq)) => assert_eq!(seq.iter().count(), 0)
        );
        assert_matches!(headers.next(), None);

        let mut headers = ObjectParser::parse(
            ParseOptions::default(),
            FunctionCode::Read,
            &[0x32, 0x01, 0x08, 0x01, 0x00],
        )
        .unwrap()
        .iter();
        assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::Count(Width::Two, 1, CountVariation::Group50Var1(seq)) => assert_eq!(seq.iter().count(), 0)
        );
        assert_matches!(headers.next(), None);
    }

    #[test]
    fn headers_after_count_of_time_in_read_are_parsed() {
        let input = [0x32, 0x01, 0x07, 0x01, 0x3C, 0x02, 0x06, 0x3C, 0x03, 0x06];
        let mut headers = ObjectParser::parse(ParseOptions::default(), FunctionCode::Read, &input)
            .unwrap()
            .iter();
        assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::Count(Width::One, 1, CountVariation::Group50Var1(seq)) => assert_eq!(seq.iter().count(), 0)
        );
        assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::AllObjects(AllObjectsVariation::Group60Var2)
        );
        assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::AllObjects(AllObjectsVariation::Group60Var3)
        );
        assert_matches!(headers.next(), None);
    }

    #[test]
    fn count_of_sized_events_is_rejected_outside_read() {
        // g2v1, count 6, followed by bytes that would parse as g1v2 index 0
        let input = [0x02, 0x01, 0x07, 0x06, 0x01, 0x02, 0x00, 0x00, 0x00, 0x01];
        for function in [FunctionCode::Response, FunctionCode::UnsolicitedResponse] {
            test_parse_error(
                &input,
                function,
                ObjectParseError::InvalidQualifierForVariation(
                    Variation::Group2Var1,
                    QualifierCode::Count8,
                ),
            );
        }

        // g32v1, two-byte count of 1, with its object data, in a WRITE
        test_parse_error(
            &[0x20, 0x01, 0x08, 0x01, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05],
            FunctionCode::Write,
            ObjectParseError::InvalidQualifierForVariation(
                Variation::Group32Var1,
                QualifierCode::Count16,
            ),
        );

        // g111v1, count 1, with one octet
        test_parse_error(
            &[0x6F, 0x01, 0x07, 0x01, 0xAA],
            FunctionCode::Response,
            ObjectParseError::InvalidQualifierForVariation(
                Variation::Group111(1),
                QualifierCode::Count8,
            ),
        );
    }

    #[test]
    fn count_of_sized_events_in_read_carries_no_data() {
        let input = [
            0x02, 0x01, 0x07, 0x05, 0x20, 0x02, 0x08, 0x0A, 0x00, 0x6F, 0x01, 0x07, 0x01,
        ];
        let mut headers = ObjectParser::parse(ParseOptions::default(), FunctionCode::Read, &input)
            .unwrap()
            .iter();
        assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::Count(Width::One, 5, CountVariation::Group2Var1)
        );
        assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::Count(Width::Two, 10, CountVariation::Group32Var2)
        );
        assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::Count(Width::One, 1, CountVariation::Group111VarX(1))
        );
        assert_matches!(headers.next(), None);
    }

    #[test]
    fn count_of_any_variation_class_and_g111v0_is_accepted_outside_read() {
        let input = [
            0x02, 0x00, 0x07, 0x01, 0x3C, 0x02, 0x08, 0x01, 0x00, 0x6F, 0x00, 0x07, 0x01,
        ];
        let mut headers =
            ObjectParser::parse(ParseOptions::default(), FunctionCode::Response, &input)
                .unwrap()
                .iter();
        assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::Count(Width::One, 1, CountVariation::Group2Var0)
        );
        assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::Count(Width::Two, 1, CountVariation::Group60Var2)
        );
        assert_matches!(
            headers.next().unwrap().details,
            HeaderDetails::Count(Width::One, 1, CountVariation::Group111Var0)
        );
        assert_matches!(headers.next(), None);
    }

    /// one response header with each qualifier, the 1-byte and 2-byte forms of each family next
    /// to each other
    const RESPONSE_WITH_EVERY_QUALIFIER: &[u8] = &[
        // g1v0, all objects
        0x01, 0x00, 0x06, //
        // g1v2, 1-byte start/stop [7, 8]
        0x01, 0x02, 0x00, 0x07, 0x08, 0x81, 0x01, //
        // g1v2, 2-byte start/stop [256, 257]
        0x01, 0x02, 0x01, 0x00, 0x01, 0x01, 0x01, 0x01, 0x81, //
        // g50v1, 1-byte count of 1
        0x32, 0x01, 0x07, 0x01, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, //
        // g50v1, 2-byte count of 1
        0x32, 0x01, 0x08, 0x01, 0x00, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, //
        // g2v1, 1-byte count of 1 and 1-byte prefix, index 5
        0x02, 0x01, 0x17, 0x01, 0x05, 0x81, //
        // g2v1, 2-byte count of 1 and 2-byte prefix, index 261
        0x02, 0x01, 0x28, 0x01, 0x00, 0x05, 0x01, 0x01, //
        // g70v5, free-format, count of 1, 12 bytes
        70, 5, 0x5B, 0x01, 12, 0x00, 0x01, 0x02, 0x03, 0x04, 0xAA, 0xBB, 0xCC, 0xDD, b'd', b'a',
        b't', b'a',
    ];

    /// the variation, qualifier and whether `count()` is available for each header
    fn qualifier_transcript(
        function: FunctionCode,
        input: &[u8],
    ) -> Vec<(Variation, QualifierCode, bool)> {
        ObjectParser::parse(ParseOptions::default(), function, input)
            .unwrap()
            .iter()
            .map(|x| {
                (
                    x.variation,
                    x.details.qualifier(),
                    x.details.count().is_some(),
                )
            })
            .collect()
    }

    #[test]
    fn response_headers_keep_their_qualifier_at_both_widths() {
        assert_eq!(
            qualifier_transcript(FunctionCode::Response, RESPONSE_WITH_EVERY_QUALIFIER),
            vec![
                (Variation::Group1Var0, QualifierCode::AllObjects, false),
                (Variation::Group1Var2, QualifierCode::Range8, false),
                (Variation::Group1Var2, QualifierCode::Range16, false),
                (Variation::Group50Var1, QualifierCode::Count8, true),
                (Variation::Group50Var1, QualifierCode::Count16, true),
                (Variation::Group2Var1, QualifierCode::CountAndPrefix8, false),
                (
                    Variation::Group2Var1,
                    QualifierCode::CountAndPrefix16,
                    false
                ),
                (Variation::Group70Var5, QualifierCode::FreeFormat16, false),
            ]
        );
    }

    #[test]
    fn read_headers_keep_their_qualifier_at_both_widths() {
        let input = [
            0x01, 0x00, 0x06, // g1v0, all objects
            0x01, 0x02, 0x00, 0x07, 0x08, // g1v2, 1-byte start/stop
            0x01, 0x02, 0x01, 0x00, 0x01, 0x01, 0x01, // g1v2, 2-byte start/stop
            0x02, 0x00, 0x07, 0x03, // g2v0, 1-byte count
            0x02, 0x00, 0x08, 0x03, 0x01, // g2v0, 2-byte count
        ];
        assert_eq!(
            qualifier_transcript(FunctionCode::Read, &input),
            vec![
                (Variation::Group1Var0, QualifierCode::AllObjects, false),
                (Variation::Group1Var2, QualifierCode::Range8, false),
                (Variation::Group1Var2, QualifierCode::Range16, false),
                (Variation::Group2Var0, QualifierCode::Count8, true),
                (Variation::Group2Var0, QualifierCode::Count16, true),
            ]
        );
    }

    #[test]
    fn control_headers_keep_their_qualifier_at_both_widths() {
        let input = [
            // g12v1, 1-byte count and prefix, index 3
            0x0C, 0x01, 0x17, 0x01, 0x03, 0x01, 0x01, 0x64, 0x00, 0x00, 0x00, 0xC8, 0x00, 0x00,
            0x00, 0x00, //
            // g12v1, 2-byte count and prefix, index 259
            0x0C, 0x01, 0x28, 0x01, 0x00, 0x03, 0x01, 0x01, 0x01, 0x64, 0x00, 0x00, 0x00, 0xC8,
            0x00, 0x00, 0x00, 0x00,
        ];
        assert_eq!(
            qualifier_transcript(FunctionCode::Operate, &input),
            vec![
                (
                    Variation::Group12Var1,
                    QualifierCode::CountAndPrefix8,
                    false
                ),
                (
                    Variation::Group12Var1,
                    QualifierCode::CountAndPrefix16,
                    false
                ),
            ]
        );
    }

    #[test]
    fn free_format_count_other_than_one_is_rejected() {
        for count in [0, 2] {
            test_parse_error(
                &[70, 5, 0x5B, count, 0x00, 0x00],
                FunctionCode::Response,
                ObjectParseError::UnsupportedFreeFormatCount(count),
            );
        }
    }

    #[test]
    fn object_header_log_format_for_every_qualifier() {
        let mut fragment = vec![0xC0, 0x81, 0x00, 0x00];
        fragment.extend_from_slice(RESPONSE_WITH_EVERY_QUALIFIER);
        let fragment = ParsedFragment::parse(ParseOptions::default(), &fragment).unwrap();
        let text = fragment.display(AppDecodeLevel::ObjectValues).to_string();
        let expected = [
            "ctrl: [fir: true fin: true con: false uns: false seq: 0] func: Response iin1: [] iin2: [] ... (len = 72)",
            "g1v0 : Binary Input - Any Variation - all objects",
            "g1v2 : Binary Input - With Flags - 1-byte start/stop - [7, 8]",
            "index: 7 flags: 0x81 [ONLINE, STATE]",
            "index: 8 flags: 0x01 [ONLINE]",
            "g1v2 : Binary Input - With Flags - 2-byte start/stop - [256, 257]",
            "index: 256 flags: 0x01 [ONLINE]",
            "index: 257 flags: 0x81 [ONLINE, STATE]",
            "g50v1 : Time and Date - Absolute Time - 1-byte count of objects - [1]",
            "time: 2179-09-26T07:18:29.121Z",
            "g50v1 : Time and Date - Absolute Time - 2-byte count of objects - [1]",
            "time: 2739-05-26T01:15:18.417Z",
            "g2v1 : Binary Input Event - Without Time - 1-byte count of objects - [1]",
            "index: 5 flags: 0x81 [ONLINE, STATE]",
            "g2v1 : Binary Input Event - Without Time - 2-byte count of objects - [1]",
            "index: 261 flags: 0x01 [ONLINE]",
            "g70v5 : File-control - file transport - 2-byte free format - [1]",
            "file handle: 67305985",
            "block number: 3721182122",
            "file data length: 4",
        ]
        .join("\n");
        assert_eq!(text, expected);
    }

    fn parse_header(input: &[u8]) -> Result<ParsedFragment<'_>, HeaderParseError> {
        ParsedFragment::parse(ParseOptions::default(), input)
    }

    fn control(fir: bool, fin: bool, con: bool, uns: bool, seq: u8) -> ControlField {
        ControlField {
            fir,
            fin,
            con,
            uns,
            seq: Sequence::new(seq),
        }
    }

    #[test]
    fn every_function_code_parses_as_a_request_or_a_response() {
        for raw in 0..=u8::MAX {
            let Some(function) = FunctionCode::from(raw) else {
                let fragment = [0xC5, raw, 0xAA, 0x55];
                assert_eq!(
                    parse_header(&fragment).err(),
                    Some(HeaderParseError::UnknownFunction(Sequence::new(5), raw))
                );
                continue;
            };

            let response_function = match function {
                FunctionCode::Response => Some(ResponseFunction::Response),
                FunctionCode::UnsolicitedResponse => Some(ResponseFunction::UnsolicitedResponse),
                _ => None,
            };
            let uns = function == FunctionCode::UnsolicitedResponse;
            let fragment = [if uns { 0xD5 } else { 0xC5 }, raw, 0xAA, 0x55];
            let expected_control = control(true, true, false, uns, 5);
            let parsed = parse_header(&fragment).unwrap();

            match response_function {
                Some(response_function) => {
                    let response = parsed.to_response().unwrap();
                    assert_eq!(
                        response.header,
                        ResponseHeader {
                            control: expected_control,
                            function: response_function,
                            iin: Iin {
                                iin1: Iin1::new(0xAA),
                                iin2: Iin2::new(0x55),
                            },
                        }
                    );
                    assert!(response.raw_objects.is_empty());
                    assert!(response.objects.unwrap().is_empty());
                    assert_eq!(
                        parsed.to_request().err(),
                        Some(RequestValidationError::UnexpectedFunction(function))
                    );
                }
                None => {
                    let request = parsed.to_request().unwrap();
                    assert_eq!(
                        request.header,
                        RequestHeader {
                            control: expected_control,
                            function,
                        }
                    );
                    assert_eq!(request.raw_fragment, &fragment);
                    // a request has no IIN, so the two bytes after the function code are objects
                    assert_eq!(
                        request.objects.err(),
                        Some(ObjectParseError::UnknownGroupVariation(0xAA, 0x55))
                    );
                    assert_eq!(
                        parsed.to_response().err(),
                        Some(ResponseValidationError::UnexpectedFunction(function))
                    );
                }
            }
        }
    }

    #[test]
    fn header_shorter_than_its_function_needs_is_insufficient() {
        for fragment in [
            &[][..],
            &[0xC0],
            &[0xC0, 0x81],
            &[0xC0, 0x81, 0x00],
            &[0xD0, 0x82],
            &[0xD0, 0x82, 0x00],
        ] {
            assert_eq!(
                parse_header(fragment).err(),
                Some(HeaderParseError::InsufficientBytes)
            );
        }

        assert_eq!(
            parse_header(&[0xC3, 0x70]).err(),
            Some(HeaderParseError::UnknownFunction(Sequence::new(3), 0x70))
        );

        for (fragment, function) in [
            (&[0xC0, 0x00], FunctionCode::Confirm),
            (&[0xC0, 0x01], FunctionCode::Read),
            (&[0xC0, 0x0D], FunctionCode::ColdRestart),
        ] {
            let request = parse_header(fragment).unwrap().to_request().unwrap();
            assert_eq!(request.header.function, function);
            assert_eq!(request.raw_fragment, fragment);
            assert!(request.objects.unwrap().is_empty());
        }
    }

    #[test]
    fn request_validation_checks_the_function_then_fir_fin_then_uns() {
        for (fragment, err) in [
            (
                &[0x40, 0x81, 0x00, 0x00][..],
                RequestValidationError::UnexpectedFunction(FunctionCode::Response),
            ),
            (
                &[0x10, 0x82, 0x00, 0x00],
                RequestValidationError::UnexpectedFunction(FunctionCode::UnsolicitedResponse),
            ),
            (&[0x40, 0x01], RequestValidationError::NonFirFin),
            (&[0x80, 0x01], RequestValidationError::NonFirFin),
            (&[0x00, 0x01], RequestValidationError::NonFirFin),
            (&[0x50, 0x01], RequestValidationError::NonFirFin),
            (&[0x40, 0x00], RequestValidationError::NonFirFin),
            (
                &[0xF0, 0x01],
                RequestValidationError::UnexpectedUnsBit(FunctionCode::Read),
            ),
        ] {
            test_request_validation_error(fragment, err);
        }

        let request = parse_header(&[0xE7, 0x01]).unwrap().to_request().unwrap();
        assert_eq!(request.header.control, control(true, true, true, false, 7));
    }

    #[test]
    fn response_validation_checks_the_function_then_uns_then_fir_fin() {
        for (fragment, err) in [
            (
                &[0xD0, 0x01][..],
                ResponseValidationError::UnexpectedFunction(FunctionCode::Read),
            ),
            (
                &[0x00, 0x00],
                ResponseValidationError::UnexpectedFunction(FunctionCode::Confirm),
            ),
            (
                &[0x80, 0x82, 0x00, 0x00],
                ResponseValidationError::UnsolicitedResponseWithoutUnsBit,
            ),
            (
                &[0x90, 0x82, 0x00, 0x00],
                ResponseValidationError::UnsolicitedResponseWithoutFirAndFin,
            ),
            (
                &[0x50, 0x82, 0x00, 0x00],
                ResponseValidationError::UnsolicitedResponseWithoutFirAndFin,
            ),
            (
                &[0x10, 0x82, 0x00, 0x00],
                ResponseValidationError::UnsolicitedResponseWithoutFirAndFin,
            ),
            (
                &[0x50, 0x81, 0x00, 0x00],
                ResponseValidationError::SolicitedResponseWithUnsBit,
            ),
        ] {
            test_response_validation_error(fragment, err);
        }

        for (fragment, expected) in [
            (
                &[0xA1, 0x81, 0x12, 0x34],
                control(true, false, true, false, 1),
            ),
            (
                &[0x02, 0x81, 0x12, 0x34],
                control(false, false, false, false, 2),
            ),
            (
                &[0x43, 0x81, 0x12, 0x34],
                control(false, true, false, false, 3),
            ),
            (
                &[0xF4, 0x82, 0x12, 0x34],
                control(true, true, true, true, 4),
            ),
        ] {
            let response = parse_header(fragment).unwrap().to_response().unwrap();
            assert_eq!(response.header.control, expected);
            assert_eq!(
                response.header.iin,
                Iin {
                    iin1: Iin1::new(0x12),
                    iin2: Iin2::new(0x34),
                }
            );
        }
    }

    #[test]
    fn objects_follow_the_function_code_in_requests_and_the_iin_in_responses() {
        let fragment = &[0xC0, 0x01, 60, 1, 0x06];
        let request = parse_header(fragment).unwrap().to_request().unwrap();
        let variations: Vec<Variation> = request
            .objects
            .unwrap()
            .iter()
            .map(|x| x.variation)
            .collect();
        assert_eq!(variations, [Variation::Group60Var1]);

        let fragment = &[0xC0, 0x81, 0x00, 0x00, 0x01, 0x02, 0x00, 0x03, 0x03, 0x81];
        let response = parse_header(fragment).unwrap().to_response().unwrap();
        assert_eq!(response.raw_objects, &fragment[4..]);
        let variations: Vec<Variation> = response
            .objects
            .unwrap()
            .iter()
            .map(|x| x.variation)
            .collect();
        assert_eq!(variations, [Variation::Group1Var2]);
    }

    #[test]
    fn request_header_log_format() {
        let fragment = [0xC3, 0x01, 60, 1, 0x06, 60];
        let text = parse_header(&fragment)
            .unwrap()
            .display(AppDecodeLevel::ObjectHeaders)
            .to_string();
        let expected = [
            "ctrl: [fir: true fin: true con: false uns: false seq: 3] func: Read ... (len = 4)",
            "g60v1 : Class Data - Class 0 - all objects",
            "decoding err: insufficient bytes for object header",
        ]
        .join("\n");
        assert_eq!(text, expected);
    }
}
