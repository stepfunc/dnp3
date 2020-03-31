use crate::app::gen::enums::{FunctionCode, QualifierCode};
use crate::app::gen::variations::all::AllObjectsVariation;
use crate::app::gen::variations::count::CountVariation;
use crate::app::gen::variations::gv::Variation;
use crate::app::gen::variations::prefixed::PrefixedVariation;
use crate::app::gen::variations::ranged::RangedVariation;
use crate::app::header::{HeaderParseError, RequestHeader, ResponseHeader};
use crate::app::parse::range::{InvalidRange, Range};
use crate::util::cursor::{ReadCursor, ReadError};

#[derive(Debug, PartialEq)]
pub enum ObjectHeader<'a> {
    AllObjects(AllObjectsVariation),
    OneByteStartStop(u8, u8, RangedVariation<'a>),
    TwoByteStartStop(u16, u16, RangedVariation<'a>),
    OneByteCount(u8, CountVariation<'a>),
    TwoByteCount(u16, CountVariation<'a>),
    OneByteCountAndPrefix(u8, PrefixedVariation<'a, u8>),
    TwoByteCountAndPrefix(u16, PrefixedVariation<'a, u16>),
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
    pub header: RequestHeader,
    pub objects: &'a [u8],
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Response<'a> {
    pub header: ResponseHeader,
    pub objects: &'a [u8],
}

impl<'a> Request<'a> {
    pub fn parse_objects(
        &self,
    ) -> Result<impl Iterator<Item = ObjectHeader<'a>>, ObjectParseError> {
        Ok(ObjectParser::parse(self.header.function, self.objects)?)
    }

    pub fn parse(bytes: &'a [u8]) -> Result<Self, HeaderParseError> {
        let mut cursor = ReadCursor::new(bytes);
        let header = RequestHeader::parse(&mut cursor)?;
        let objects = cursor.read_all();

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
            _ => Ok(Self { header, objects }),
        }
    }
}

impl<'a> Response<'a> {
    pub fn parse_objects(
        &self,
    ) -> Result<impl Iterator<Item = ObjectHeader<'a>>, ObjectParseError> {
        Ok(ObjectParser::parse(
            self.header.function.to_function(),
            self.objects,
        )?)
    }

    pub fn parse(bytes: &'a [u8]) -> Result<Self, HeaderParseError> {
        let mut cursor = ReadCursor::new(bytes);
        let header = ResponseHeader::parse(&mut cursor)?;
        Ok(Self {
            header,
            objects: cursor.read_all(),
        })
    }
}

pub struct ObjectParser<'a> {
    errored: bool,
    function: FunctionCode,
    cursor: ReadCursor<'a>,
}

impl<'a> ObjectParser<'a> {
    pub fn parse(
        function: FunctionCode,
        data: &'a [u8],
    ) -> Result<impl Iterator<Item = ObjectHeader<'a>>, ObjectParseError> {
        // we first do a single pass to ensure the ASDU is well-formed, returning an error if it occurs
        for x in ObjectParser::one_pass(function, data) {
            if let Err(e) = x {
                return Err(e);
            }
        }

        // on the 2nd pass, we can unwrap b/c it can't possibly panic
        Ok(ObjectParser::one_pass(function, data).map(|h| h.unwrap()))
    }

    fn one_pass(
        function: FunctionCode,
        data: &'a [u8],
    ) -> impl Iterator<Item = Result<ObjectHeader<'a>, ObjectParseError>> {
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

    fn parse_all_objects(&mut self, gv: Variation) -> Result<ObjectHeader<'a>, ObjectParseError> {
        match AllObjectsVariation::get(gv) {
            Some(v) => Ok(ObjectHeader::AllObjects(v)),
            None => Err(ObjectParseError::InvalidQualifierForVariation(gv)),
        }
    }

    fn parse_count_u8(&mut self, gv: Variation) -> Result<ObjectHeader<'a>, ObjectParseError> {
        let count = self.cursor.read_u8()?;
        let data = CountVariation::parse(gv, count as u16, &mut self.cursor)?;
        Ok(ObjectHeader::OneByteCount(count, data))
    }

    fn parse_count_u16(&mut self, gv: Variation) -> Result<ObjectHeader<'a>, ObjectParseError> {
        let count = self.cursor.read_u16_le()?;
        let data = CountVariation::parse(gv, count, &mut self.cursor)?;
        Ok(ObjectHeader::TwoByteCount(count, data))
    }

    fn parse_start_stop_u8(&mut self, gv: Variation) -> Result<ObjectHeader<'a>, ObjectParseError> {
        let start = self.cursor.read_u8()?;
        let stop = self.cursor.read_u8()?;
        let range = Range::from(start as u16, stop as u16)?;
        let data = RangedVariation::parse(self.function, gv, range, &mut self.cursor)?;
        Ok(ObjectHeader::OneByteStartStop(start, stop, data))
    }

    fn parse_start_stop_u16(
        &mut self,
        gv: Variation,
    ) -> Result<ObjectHeader<'a>, ObjectParseError> {
        let start = self.cursor.read_u16_le()?;
        let stop = self.cursor.read_u16_le()?;
        let range = Range::from(start, stop)?;
        let data = RangedVariation::parse(self.function, gv, range, &mut self.cursor)?;
        Ok(ObjectHeader::TwoByteStartStop(start, stop, data))
    }

    fn parse_count_and_prefix_u8(
        &mut self,
        gv: Variation,
    ) -> Result<ObjectHeader<'a>, ObjectParseError> {
        let count = self.cursor.read_u8()?;
        let data = PrefixedVariation::<u8>::parse(gv, count as u16, &mut self.cursor)?;
        Ok(ObjectHeader::OneByteCountAndPrefix(count, data))
    }

    fn parse_count_and_prefix_u16(
        &mut self,
        gv: Variation,
    ) -> Result<ObjectHeader<'a>, ObjectParseError> {
        let count = self.cursor.read_u16_le()?;
        let data = PrefixedVariation::<u16>::parse(gv, count, &mut self.cursor)?;
        Ok(ObjectHeader::TwoByteCountAndPrefix(count, data))
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
    use crate::app::gen::variations::gv::Variation::Group110;
    use crate::app::header::{Control, ResponseFunction, IIN};
    use crate::app::parse::bytes::Bytes;
    use crate::app::parse::prefix::Prefix;
    use crate::app::sequence::Sequence;
    use crate::app::types::DoubleBit;

    fn test_parse_error(input: &[u8], func: FunctionCode, err: ObjectParseError) {
        assert_eq!(ObjectParser::parse(func, input).err().unwrap(), err);
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
        let request = Request::parse(fragment).unwrap();
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
        let response = Response::parse(fragment).unwrap();
        let expected = ResponseHeader {
            control: Control {
                fir: true,
                fin: true,
                con: false,
                uns: false,
                seq: Sequence::new(0x02),
            },
            function: ResponseFunction::Unsolicited,
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
        let vec: Vec<ObjectHeader> = ObjectParser::parse(
            FunctionCode::Read,
            &[
                0x3C, 0x02, 0x06, 0x3C, 0x03, 0x06, 0x3C, 0x04, 0x06, 0x3C, 0x01, 0x06,
            ],
        )
        .unwrap()
        .collect();

        assert_eq!(
            vec,
            vec![
                ObjectHeader::AllObjects(AllObjectsVariation::Group60Var2),
                ObjectHeader::AllObjects(AllObjectsVariation::Group60Var3),
                ObjectHeader::AllObjects(AllObjectsVariation::Group60Var4),
                ObjectHeader::AllObjects(AllObjectsVariation::Group60Var1),
            ]
        )
    }

    #[test]
    fn parses_analog_output() {
        let header = &[0x29, 0x01, 0x17, 0x01, 0xFF, 0x01, 0x02, 0x03, 0x04, 0x00];
        let mut parser = ObjectParser::parse(FunctionCode::Operate, header).unwrap();

        let items: Vec<Prefix<u8, Group41Var1>> = assert_matches!(
            parser.next().unwrap(),
            ObjectHeader::OneByteCountAndPrefix(01, PrefixedVariation::<u8>::Group41Var1(seq)) => seq.iter().collect()
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
        assert_eq!(parser.next(), None);
    }

    #[test]
    fn parses_range_of_g3v1() {
        let header = &[0x03, 0x01, 0x00, 0x01, 0x04, 0b11_10_01_00];
        let mut parser = ObjectParser::parse(FunctionCode::Response, header).unwrap();

        let items: Vec<(DoubleBit, u16)> = assert_matches!(
            parser.next().unwrap(),
            ObjectHeader::OneByteStartStop(01, 04, RangedVariation::Group3Var1(seq)) => seq.iter().collect()
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
        assert_eq!(parser.next(), None);
    }

    #[test]
    fn parses_count_of_time() {
        let header = &[0x32, 0x01, 0x07, 0x01, 0xFF, 0xFE, 0xFD, 0xFC, 0xFB, 0xFA];
        let mut parser = ObjectParser::parse(FunctionCode::Write, header).unwrap();

        let items: Vec<Group50Var1> = assert_matches!(
            parser.next().unwrap(),
            ObjectHeader::OneByteCount(01, CountVariation::Group50Var1(seq)) => seq.iter().collect()
        );

        assert_eq!(
            items,
            vec![Group50Var1 {
                time: 0x00_00_FA_FB_FC_FD_FE_FF
            }]
        );

        assert_eq!(parser.next(), None);
    }

    #[test]
    fn parses_range_of_g1v2_as_non_read() {
        let input = [0x01, 0x02, 0x00, 0x02, 0x03, 0xAA, 0xBB];

        let mut parser = ObjectParser::parse(FunctionCode::Response, &input).unwrap();

        let items: Vec<(Group1Var2, u16)> = assert_matches!(
            parser.next().unwrap(),
            ObjectHeader::OneByteStartStop(02, 03, RangedVariation::Group1Var2(seq)) => seq.iter().collect()
        );

        assert_eq!(
            items,
            vec![
                (Group1Var2 { flags: 0xAA }, 2),
                (Group1Var2 { flags: 0xBB }, 3)
            ]
        );
        assert_eq!(parser.next(), None);
    }

    #[test]
    fn parses_range_of_g1v2_as_read() {
        let input = [0x01, 0x02, 0x00, 0x02, 0x03, 0x01, 0x02, 0x00, 0x07, 0x09];

        let mut parser = ObjectParser::parse(FunctionCode::Read, &input).unwrap();

        assert_matches!(
            parser.next().unwrap(),
            ObjectHeader::OneByteStartStop(02, 03, RangedVariation::Group1Var2(seq)) => {
                assert!(seq.is_empty())
            }
        );

        assert_matches!(
            parser.next().unwrap(),
            ObjectHeader::OneByteStartStop(07, 09, RangedVariation::Group1Var2(seq)) => {
                assert!(seq.is_empty())
            }
        );

        assert_eq!(parser.next(), None);
    }

    #[test]
    fn parses_range_of_g80v1() {
        // this is what is typically sent to clear the restart IIN
        let input = [0x50, 0x01, 0x00, 0x07, 0x07, 0x00];
        let mut parser = ObjectParser::parse(FunctionCode::Write, &input).unwrap();

        let vec: Vec<(bool, u16)> = assert_matches!(
            parser.next().unwrap(),
            ObjectHeader::OneByteStartStop(07, 07, RangedVariation::Group80Var1(seq)) => {
                seq.iter().collect()
            }
        );

        assert_eq!(vec, vec![(false, 7)]);
        assert_eq!(parser.next(), None);
    }

    #[test]
    fn parses_group110var0_as_read() {
        let input = [0x6E, 0x00, 0x00, 0x02, 0x03];
        let mut parser = ObjectParser::parse(FunctionCode::Read, &input).unwrap();
        assert_eq!(
            parser.next().unwrap(),
            ObjectHeader::OneByteStartStop(02, 03, RangedVariation::Group110Var0)
        );
        assert_eq!(parser.next(), None);
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
        let mut parser = ObjectParser::parse(FunctionCode::Response, &input).unwrap();

        let bytes: Vec<(Bytes, u16)> = assert_matches!(
            parser.next().unwrap(),
            ObjectHeader::OneByteStartStop(01, 02, RangedVariation::Group110VarX(0x01, seq)) => {
                seq.iter().collect()
            }
        );

        assert_eq!(
            bytes,
            vec![(Bytes { value: &[0xAA] }, 1), (Bytes { value: &[0xBB] }, 2)]
        );
        assert_eq!(parser.next(), None);
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
