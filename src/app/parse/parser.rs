use crate::app::gen::enums::{FunctionCode, QualifierCode};
use crate::app::gen::variations::all::AllObjectsVariation;
use crate::app::gen::variations::count::CountVariation;
use crate::app::gen::variations::prefixed::PrefixedVariation;
use crate::app::gen::variations::ranged::RangedVariation;
use crate::app::gen::variations::variation::Variation;
use crate::app::header::{HeaderParseError, RequestHeader, ResponseHeader};
use crate::app::parse::prefix::Prefix;
use crate::app::parse::range::{InvalidRange, Range};
use crate::app::parse::traits::FixedSize;
use crate::util::cursor::{ReadCursor, ReadError};

/// Controls how parsed ASDUs are logged
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ParseLogLevel {
    /// Log nothing
    Nothing,
    /// Log the header-only
    Header,
    /// Log the header and the object headers
    ObjectHeaders,
    /// Log the header, the object headers, and the object values
    ObjectValues,
}

pub(crate) fn log_count_of_items<T, V>(level: log::Level, iter: T)
where
    T: Iterator<Item = V>,
    V: std::fmt::Display,
{
    for x in iter {
        log::log!(level, "{}", x);
    }
}

pub(crate) fn format_count_of_items<T, V>(f: &mut std::fmt::Formatter, iter: T) -> std::fmt::Result
where
    T: Iterator<Item = V>,
    V: std::fmt::Display,
{
    for x in iter {
        write!(f, "\n{}", x)?;
    }
    Ok(())
}

pub(crate) fn log_indexed_items<T, V, I>(level: log::Level, iter: T)
where
    T: Iterator<Item = (V, I)>,
    V: std::fmt::Display,
    I: std::fmt::Display,
{
    for (v, i) in iter {
        log::log!(level, "index: {} {}", i, v);
    }
}

pub(crate) fn format_indexed_items<T, V, I>(
    f: &mut std::fmt::Formatter,
    iter: T,
) -> std::fmt::Result
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

pub(crate) fn log_prefixed_items<T, V, I>(level: log::Level, iter: T)
where
    T: Iterator<Item = Prefix<I, V>>,
    V: FixedSize + std::fmt::Display,
    I: FixedSize + std::fmt::Display,
{
    for x in iter {
        log::log!(level, "index: {} {}", x.index, x.value);
    }
}

pub(crate) fn format_prefixed_items<T, V, I>(
    f: &mut std::fmt::Formatter,
    iter: T,
) -> std::fmt::Result
where
    T: Iterator<Item = Prefix<I, V>>,
    V: FixedSize + std::fmt::Display,
    I: FixedSize + std::fmt::Display,
{
    for x in iter {
        write!(f, "\nindex: {} {}", x.index, x.value)?;
    }
    Ok(())
}

impl ParseLogLevel {
    pub(crate) fn log_header(self) -> bool {
        match self {
            ParseLogLevel::Nothing => false,
            ParseLogLevel::Header => true,
            ParseLogLevel::ObjectHeaders => true,
            ParseLogLevel::ObjectValues => true,
        }
    }

    pub(crate) fn log_object_headers(self) -> bool {
        match self {
            ParseLogLevel::Nothing => false,
            ParseLogLevel::Header => false,
            ParseLogLevel::ObjectHeaders => true,
            ParseLogLevel::ObjectValues => true,
        }
    }

    pub(crate) fn log_object_values(self) -> bool {
        match self {
            ParseLogLevel::Nothing => false,
            ParseLogLevel::Header => false,
            ParseLogLevel::ObjectHeaders => false,
            ParseLogLevel::ObjectValues => true,
        }
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

    pub(crate) fn display_header_only(&'a self) -> ObjectHeaderDisplay<'a> {
        ObjectHeaderDisplay {
            objects: false,
            header: self,
        }
    }

    pub(crate) fn display_header_and_objects(&'a self) -> ObjectHeaderDisplay<'a> {
        ObjectHeaderDisplay {
            objects: true,
            header: self,
        }
    }
}

pub(crate) struct ObjectHeaderDisplay<'a> {
    objects: bool,
    header: &'a ObjectHeader<'a>,
}

impl<'a> std::fmt::Display for ObjectHeaderDisplay<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self.header.details {
            HeaderDetails::AllObjects(_) => write!(
                f,
                "{} {}",
                self.header.variation,
                self.header.details.qualifier().description()
            ),
            HeaderDetails::OneByteStartStop(s1, s2, seq) => {
                write!(
                    f,
                    "{} {} [{}, {}]",
                    self.header.variation,
                    self.header.details.qualifier().description(),
                    s1,
                    s2
                )?;
                if self.objects {
                    seq.format_objects(f)?;
                }
                Ok(())
            }
            HeaderDetails::TwoByteStartStop(s1, s2, seq) => {
                write!(
                    f,
                    "{} {} [{}, {}]",
                    self.header.variation,
                    self.header.details.qualifier().description(),
                    s1,
                    s2
                )?;
                if self.objects {
                    seq.format_objects(f)?;
                }
                Ok(())
            }
            HeaderDetails::OneByteCount(c, seq) => {
                write!(
                    f,
                    "{} {} [{}]",
                    self.header.variation,
                    self.header.details.qualifier().description(),
                    c
                )?;
                if self.objects {
                    seq.format_objects(f)?;
                }
                Ok(())
            }
            HeaderDetails::TwoByteCount(c, seq) => {
                write!(
                    f,
                    "{} {} [{}]",
                    self.header.variation,
                    self.header.details.qualifier().description(),
                    c
                )?;
                if self.objects {
                    seq.format_objects(f)?;
                }
                Ok(())
            }
            HeaderDetails::OneByteCountAndPrefix(c, seq) => {
                write!(
                    f,
                    "{} {} [{}]",
                    self.header.variation,
                    self.header.details.qualifier().description(),
                    c
                )?;
                if self.objects {
                    seq.format_objects(f)?;
                }
                Ok(())
            }
            HeaderDetails::TwoByteCountAndPrefix(c, seq) => {
                write!(
                    f,
                    "{} {} [{}]",
                    self.header.variation,
                    self.header.details.qualifier().description(),
                    c
                )?;
                if self.objects {
                    seq.format_objects(f)?;
                }
                Ok(())
            }
        }
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

impl<'a> HeaderDetails<'a> {
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

    pub(crate) fn log_object_values(&self, level: log::Level) {
        match self {
            HeaderDetails::AllObjects(_) => {}
            HeaderDetails::OneByteStartStop(_, _, var) => var.log_objects(level),
            HeaderDetails::TwoByteStartStop(_, _, var) => var.log_objects(level),
            HeaderDetails::OneByteCount(_, var) => var.log_objects(level),
            HeaderDetails::TwoByteCount(_, var) => var.log_objects(level),
            HeaderDetails::OneByteCountAndPrefix(_, var) => var.log_objects(level),
            HeaderDetails::TwoByteCountAndPrefix(_, var) => var.log_objects(level),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ObjectParseError {
    UnknownGroupVariation(u8, u8),
    UnknownQualifier(u8),
    InsufficientBytes,
    InvalidRange,
    InvalidQualifierForVariation(Variation),
    UnsupportedQualifierCode(QualifierCode),
    ZeroLengthOctetData,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Request<'a> {
    level: ParseLogLevel,
    pub header: RequestHeader,
    pub objects: &'a [u8],
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Response<'a> {
    level: ParseLogLevel,
    pub header: ResponseHeader,
    pub objects: &'a [u8],
}

pub(crate) fn log_tx_fragment(level: ParseLogLevel, is_master: bool, data: &[u8]) {
    if !level.log_header() {
        return;
    }

    if is_master {
        match Request::parse(level, data) {
            Ok(request) => {
                if level.log_object_headers() {
                    if let Err(err) = request.parse_objects() {
                        log::error!("error parsing tx request objects: {:?}", err);
                    }
                }
            }
            Err(err) => {
                log::error!("error parsing tx request header: {:?}", err);
            }
        }
    } else {
        match Response::parse(level, data) {
            Ok(request) => {
                if level.log_object_headers() {
                    if let Err(err) = request.parse_objects() {
                        log::error!("error parsing tx response objects: {:?}", err);
                    }
                }
            }
            Err(err) => {
                log::error!("error parsing tx response header: {:?}", err);
            }
        }
    }
}

impl<'a> Request<'a> {
    pub fn parse_objects(&self) -> Result<HeaderCollection<'a>, ObjectParseError> {
        Ok(HeaderCollection::parse(
            self.level,
            self.header.function,
            self.objects,
        )?)
    }

    pub fn parse(level: ParseLogLevel, bytes: &'a [u8]) -> Result<Self, HeaderParseError> {
        let mut cursor = ReadCursor::new(bytes);
        let header = RequestHeader::parse(&mut cursor)?;
        let objects = cursor.read_all();

        if level.log_header() {
            log::info!("{:?} control: {}", header.function, header.control,);
        }

        if header.control.uns {
            return Err(HeaderParseError::UnsolicitedBitNotAllowed(header.function));
        }

        if !(header.control.fir && header.control.fin) {
            return Err(HeaderParseError::BadFirAndFin(header.control));
        }

        match header.function {
            FunctionCode::Response => Err(HeaderParseError::BadFunction(header.function)),
            FunctionCode::UnsolicitedResponse => {
                Err(HeaderParseError::BadFunction(header.function))
            }
            _ => Ok(Self {
                level,
                header,
                objects,
            }),
        }
    }
}

impl<'a> Response<'a> {
    pub fn parse_objects(&self) -> Result<HeaderCollection<'a>, ObjectParseError> {
        Ok(HeaderCollection::parse(
            self.level,
            self.header.function(),
            self.objects,
        )?)
    }

    pub fn parse(level: ParseLogLevel, bytes: &'a [u8]) -> Result<Self, HeaderParseError> {
        let mut cursor = ReadCursor::new(bytes);
        let header = ResponseHeader::parse(&mut cursor)?;
        if level.log_header() {
            log::info!(
                "{:?} control: {} iin: {}",
                header.function(),
                header.control,
                header.iin
            );
        }
        Ok(Self {
            level,
            header,
            objects: cursor.read_all(),
        })
    }
}

struct ObjectParser<'a> {
    errored: bool,
    function: FunctionCode,
    cursor: ReadCursor<'a>,
}

/// An abstract collection of pre-validated object headers
/// that can provide an iterator of the headers.
pub struct HeaderCollection<'a> {
    function: FunctionCode,
    data: &'a [u8],
}

impl<'a> HeaderCollection<'a> {
    /// parse the the raw header data in accordance with the provided function code
    pub fn parse(
        level: ParseLogLevel,
        function: FunctionCode,
        data: &'a [u8],
    ) -> Result<Self, ObjectParseError> {
        ObjectParser::parse(level, function, data)
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
        self.parser.next().map(|x| x.unwrap())
    }
}

impl<'a> ObjectParser<'a> {
    pub fn parse(
        level: ParseLogLevel,
        function: FunctionCode,
        data: &'a [u8],
    ) -> Result<HeaderCollection<'a>, ObjectParseError> {
        // we first do a single pass to ensure the ASDU is well-formed, returning an error if it occurs
        for result in ObjectParser::one_pass(function, data) {
            match result {
                Err(err) => {
                    log::warn!("error parsing object header: {:?}", err); // TODO implement std::fmt::Display
                    return Err(err);
                }
                Ok(header) => {
                    if level.log_object_headers() {
                        if level.log_object_values() {
                            log::info!("{}", header.display_header_and_objects());
                        } else {
                            log::info!("{}", header.display_header_only());
                        }
                    }
                }
            }
        }

        // now we know that the headers are well-formed and our 2nd pass
        // on the same data can just unwrap() the results w/o fear of panic
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
            None => Err(ObjectParseError::InvalidQualifierForVariation(v)),
        }
    }

    fn parse_count_u8(&mut self, v: Variation) -> Result<ObjectHeader<'a>, ObjectParseError> {
        let count = self.cursor.read_u8()?;
        let data = CountVariation::parse(v, count as u16, &mut self.cursor)?;
        Ok(ObjectHeader::new(
            v,
            HeaderDetails::OneByteCount(count, data),
        ))
    }

    fn parse_count_u16(&mut self, v: Variation) -> Result<ObjectHeader<'a>, ObjectParseError> {
        let count = self.cursor.read_u16_le()?;
        let data = CountVariation::parse(v, count, &mut self.cursor)?;
        Ok(ObjectHeader::new(
            v,
            HeaderDetails::TwoByteCount(count, data),
        ))
    }

    fn parse_start_stop_u8(&mut self, v: Variation) -> Result<ObjectHeader<'a>, ObjectParseError> {
        let start = self.cursor.read_u8()?;
        let stop = self.cursor.read_u8()?;
        let range = Range::from(start as u16, stop as u16)?;
        let data = RangedVariation::parse(self.function, v, range, &mut self.cursor)?;
        Ok(ObjectHeader::new(
            v,
            HeaderDetails::OneByteStartStop(start, stop, data),
        ))
    }

    fn parse_start_stop_u16(&mut self, v: Variation) -> Result<ObjectHeader<'a>, ObjectParseError> {
        let start = self.cursor.read_u16_le()?;
        let stop = self.cursor.read_u16_le()?;
        let range = Range::from(start, stop)?;
        let data = RangedVariation::parse(self.function, v, range, &mut self.cursor)?;
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
    pub fn parse(cursor: &mut ReadCursor) -> Result<Variation, ObjectParseError> {
        let group = cursor.read_u8()?;
        let var = cursor.read_u8()?;
        match Self::lookup(group, var) {
            Some(gv) => Ok(gv),
            None => Err(ObjectParseError::UnknownGroupVariation(group, var)),
        }
    }
}

impl<'a> RangedVariation<'a> {
    pub fn parse(
        function: FunctionCode,
        v: Variation,
        range: Range,
        cursor: &mut ReadCursor<'a>,
    ) -> Result<RangedVariation<'a>, ObjectParseError> {
        match function {
            FunctionCode::Read => Self::parse_read(v),
            _ => Self::parse_non_read(v, range, cursor),
        }
    }
}

impl std::convert::From<ReadError> for ObjectParseError {
    fn from(_: ReadError) -> Self {
        ObjectParseError::InsufficientBytes
    }
}

impl std::convert::From<ReadError> for HeaderParseError {
    fn from(_: ReadError) -> Self {
        HeaderParseError::InsufficientBytes
    }
}

impl std::convert::From<InvalidRange> for ObjectParseError {
    fn from(_: InvalidRange) -> Self {
        ObjectParseError::InvalidRange
    }
}

impl QualifierCode {
    pub fn parse(cursor: &mut ReadCursor) -> Result<QualifierCode, ObjectParseError> {
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
    use crate::app::header::{Control, IIN};
    use crate::app::parse::bytes::Bytes;
    use crate::app::parse::prefix::Prefix;
    use crate::app::sequence::Sequence;
    use crate::app::types::{DoubleBit, Timestamp};

    fn test_parse_error(input: &[u8], func: FunctionCode, err: ObjectParseError) {
        assert_eq!(
            ObjectParser::parse(ParseLogLevel::Nothing, func, input)
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
        let request = Request::parse(ParseLogLevel::Nothing, fragment).unwrap();
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
        assert_eq!(request.objects, &[0xAA]);
        assert_eq!(
            request.parse_objects().err().unwrap(),
            ObjectParseError::InsufficientBytes
        )
    }

    #[test]
    fn parses_valid_response() {
        let fragment = &[0xC2, 0x82, 0xFF, 0xAA, 0x01, 0x02];
        let response = Response::parse(ParseLogLevel::Nothing, fragment).unwrap();
        let expected = ResponseHeader {
            control: Control {
                fir: true,
                fin: true,
                con: false,
                uns: false,
                seq: Sequence::new(0x02),
            },
            unsolicited: true,
            iin: IIN {
                iin1: 0xFF,
                iin2: 0xAA,
            },
        };

        assert_eq!(response.header, expected);
        assert_eq!(response.objects, &[0x01, 0x02]);
        assert_eq!(
            response.parse_objects().err().unwrap(),
            ObjectParseError::InsufficientBytes
        )
    }

    #[test]
    fn parses_integrity_scan() {
        let vec: Vec<HeaderDetails> = ObjectParser::parse(
            ParseLogLevel::Nothing,
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
        let mut headers =
            ObjectParser::parse(ParseLogLevel::Nothing, FunctionCode::Operate, header)
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
        let mut headers =
            ObjectParser::parse(ParseLogLevel::Nothing, FunctionCode::Response, header)
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
        let mut headers =
            HeaderCollection::parse(ParseLogLevel::Nothing, FunctionCode::Write, header)
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

        let mut headers =
            HeaderCollection::parse(ParseLogLevel::Nothing, FunctionCode::Response, &input)
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

        let mut headers =
            HeaderCollection::parse(ParseLogLevel::Nothing, FunctionCode::Read, &input)
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
        let mut headers =
            HeaderCollection::parse(ParseLogLevel::Nothing, FunctionCode::Write, &input)
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
        let mut headers =
            HeaderCollection::parse(ParseLogLevel::Nothing, FunctionCode::Read, &input)
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
            ObjectParseError::InvalidQualifierForVariation(Group110(1)),
        );
    }

    #[test]
    fn parses_group110var1_as_non_read() {
        let input = [0x6E, 0x01, 0x00, 0x01, 0x02, 0xAA, 0xBB];
        let mut headers =
            ObjectParser::parse(ParseLogLevel::Nothing, FunctionCode::Response, &input)
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
