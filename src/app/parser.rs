use crate::app::gen::enums::QualifierCode;
use crate::app::gen::variations::allobjs::AllObjectsVariation;
use crate::app::gen::variations::gv::Variation;
use crate::app::gen::variations::ranged::RangedVariation;
use crate::app::header::Header;
use crate::app::range::{InvalidRange, Range};
use crate::util::cursor::{ReadCursor, ReadError};

#[derive(Copy, Clone)]
pub enum ParseType {
    Read,
    NonRead,
}

pub struct Parser<'a> {
    errored: bool,
    parse_type: ParseType,
    cursor: ReadCursor<'a>,
}

impl Variation {
    pub fn parse(cursor: &mut ReadCursor) -> Result<Variation, ParseError> {
        let group = cursor.read_u8()?;
        let var = cursor.read_u8()?;
        match Self::lookup(group, var) {
            Some(gv) => Ok(gv),
            None => Err(ParseError::UnknownGroupVariation(group, var)),
        }
    }
}

impl<'a> RangedVariation<'a> {
    pub fn parse(
        parse_type: ParseType,
        v: Variation,
        range: Range,
        cursor: &mut ReadCursor<'a>,
    ) -> Result<RangedVariation<'a>, ParseError> {
        match parse_type {
            ParseType::Read => Self::parse_read(v),
            ParseType::NonRead => Self::parse_non_read(v, range, cursor),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    UnknownGroupVariation(u8, u8),
    UnknownQualifier(u8),
    InsufficientBytes,
    InvalidRange,
    InvalidQualifierForVariation(Variation),
    UnsupportedQualifierCode(QualifierCode),
}

impl std::convert::From<ReadError> for ParseError {
    fn from(_: ReadError) -> Self {
        ParseError::InsufficientBytes
    }
}

impl std::convert::From<InvalidRange> for ParseError {
    fn from(_: InvalidRange) -> Self {
        ParseError::InvalidRange
    }
}

impl QualifierCode {
    pub fn parse(cursor: &mut ReadCursor) -> Result<QualifierCode, ParseError> {
        let x = cursor.read_u8()?;
        match Self::from(x) {
            Some(qc) => Ok(qc),
            None => Err(ParseError::UnknownQualifier(x)),
        }
    }
}

impl<'a> Parser<'a> {
    pub fn one_pass(
        parse_type: ParseType,
        data: &'a [u8],
    ) -> impl Iterator<Item = Result<Header<'a>, ParseError>> {
        Parser {
            cursor: ReadCursor::new(data),
            parse_type,
            errored: false,
        }
    }

    pub fn two_pass(
        parse_type: ParseType,
        data: &'a [u8],
    ) -> Result<impl Iterator<Item = Header<'a>>, ParseError> {
        for x in Parser::one_pass(parse_type, data) {
            if let Err(e) = x {
                return Err(e);
            }
        }

        // we can unwrap on the 2nd pass b/c it can't possibly panic
        Ok(Parser::one_pass(parse_type, data).map(|h| h.unwrap()))
    }

    fn parse_one(&mut self) -> Option<Result<Header<'a>, ParseError>> {
        if self.errored || self.cursor.is_empty() {
            return None;
        }

        let result = self.parse_one_inner();

        if result.is_err() {
            self.errored = true;
        }

        Some(result)
    }

    fn parse_one_inner(&mut self) -> Result<Header<'a>, ParseError> {
        let gv = Variation::parse(&mut self.cursor)?;
        let qualifier = QualifierCode::parse(&mut self.cursor)?;
        match qualifier {
            QualifierCode::AllObjects => self.parse_all_objects(gv),
            QualifierCode::Range8 => self.parse_start_stop_u8(gv),
            QualifierCode::Range16 => self.parse_start_stop_u16(gv),
            _ => Err(ParseError::UnsupportedQualifierCode(qualifier)),
        }
    }

    fn parse_all_objects(&mut self, gv: Variation) -> Result<Header<'a>, ParseError> {
        match AllObjectsVariation::get(gv) {
            Some(v) => Ok(Header::AllObjects(v)),
            None => Err(ParseError::InvalidQualifierForVariation(gv)),
        }
    }

    fn parse_start_stop_u8(&mut self, gv: Variation) -> Result<Header<'a>, ParseError> {
        let start = self.cursor.read_u8()?;
        let stop = self.cursor.read_u8()?;
        let range = Range::from(start as u16, stop as u16)?;
        let data = RangedVariation::parse(self.parse_type, gv, range, &mut self.cursor)?;
        Ok(Header::OneByteStartStop(start, stop, data))
    }

    fn parse_start_stop_u16(&mut self, gv: Variation) -> Result<Header<'a>, ParseError> {
        let start = self.cursor.read_u16_le()?;
        let stop = self.cursor.read_u16_le()?;
        let range = Range::from(start, stop)?;
        let data = RangedVariation::parse(self.parse_type, gv, range, &mut self.cursor)?;
        Ok(Header::TwoByteStartStop(start, stop, data))
    }
}

impl<'a> Iterator for Parser<'a> {
    type Item = Result<Header<'a>, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.parse_one()
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::app::gen::variations::fixed::*;
    use crate::app::header::*;

    #[test]
    fn parses_integrity_scan() {
        let vec: Vec<Header> = Parser::two_pass(
            ParseType::NonRead,
            &[
                0x3C, 0x02, 0x06, 0x3C, 0x03, 0x06, 0x3C, 0x04, 0x06, 0x3C, 0x01, 0x06,
            ],
        )
        .unwrap()
        .collect();

        assert_eq!(
            vec,
            vec![
                Header::AllObjects(AllObjectsVariation::Group60Var2),
                Header::AllObjects(AllObjectsVariation::Group60Var3),
                Header::AllObjects(AllObjectsVariation::Group60Var4),
                Header::AllObjects(AllObjectsVariation::Group60Var1),
            ]
        )
    }

    #[test]
    fn parses_range_of_g1v2_as_non_read() {
        let input = [0x01, 0x02, 0x00, 0x02, 0x03, 0xAA, 0xBB];

        let mut parser = Parser::one_pass(ParseType::NonRead, &input);

        let items: Vec<(Group1Var2, u16)> = match parser.next().unwrap().unwrap() {
            Header::OneByteStartStop(02, 03, RangedVariation::Group1Var2(seq)) => {
                seq.iter().collect()
            }
            x => panic!("got: {:?}", x),
        };

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

        let mut parser = Parser::two_pass(ParseType::Read, &input).unwrap();

        match parser.next().unwrap() {
            Header::OneByteStartStop(02, 03, RangedVariation::Group1Var2(seq)) => {
                assert!(seq.is_empty())
            }
            x => panic!("got: {:?}", x),
        };

        match parser.next().unwrap() {
            Header::OneByteStartStop(07, 09, RangedVariation::Group1Var2(seq)) => {
                assert!(seq.is_empty())
            }
            x => panic!("got: {:?}", x),
        };

        assert_eq!(parser.next(), None);
    }
}
