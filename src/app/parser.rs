use crate::app::header::{Header, RangedVariation};
use crate::app::range::{InvalidRange, Range, RangedSequence};
use crate::util::cursor::{ReadCursor, ReadError};

pub struct Parser<'a> {
    errored: bool,
    cursor: ReadCursor<'a>,
}

enum GroupVar {
    Group2Var0,
    Group2Var1,
    Group2Var2,
    Group2Var3,
}

impl GroupVar {
    pub fn lookup(group: u8, var: u8) -> Result<GroupVar, ParseError> {
        match group {
            2 => match var {
                0 => Ok(GroupVar::Group2Var0),
                1 => Ok(GroupVar::Group2Var1),
                2 => Ok(GroupVar::Group2Var2),
                3 => Ok(GroupVar::Group2Var3),
                _ => Err(ParseError::UnknownGroupVariation(group, var)),
            },
            _ => Err(ParseError::UnknownGroupVariation(group, var)),
        }
    }

    pub fn parse(cursor: &mut ReadCursor) -> Result<GroupVar, ParseError> {
        Self::lookup(cursor.read_u8()?, cursor.read_u8()?)
    }
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    UnknownGroupVariation(u8, u8),
    UnknownQualifier(u8),
    InsufficientBytes,
    InvalidRange,
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

enum Qualifier {
    OneByteStartStop,
    TwoByteStartStop,
}

impl Qualifier {
    pub fn from(value: u8) -> Result<Qualifier, ParseError> {
        match value {
            0 => Ok(Qualifier::OneByteStartStop),
            1 => Ok(Qualifier::TwoByteStartStop),
            _ => Err(ParseError::UnknownQualifier(value)),
        }
    }

    pub fn parse(cursor: &mut ReadCursor) -> Result<Qualifier, ParseError> {
        Self::from(cursor.read_u8()?)
    }
}

impl<'a> Parser<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Parser {
            cursor: ReadCursor::new(data),
            errored: false,
        }
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
        let gv = GroupVar::parse(&mut self.cursor)?;
        let qualifier = Qualifier::parse(&mut self.cursor)?;
        match qualifier {
            Qualifier::OneByteStartStop => self.parse_start_stop_u8(gv),
            Qualifier::TwoByteStartStop => self.parse_start_stop_u16(gv),
        }
    }

    fn parse_start_stop_u8(&mut self, gv: GroupVar) -> Result<Header<'a>, ParseError> {
        let start = self.cursor.read_u8()?;
        let stop = self.cursor.read_u8()?;
        let range = Range::from(start as u16, stop as u16)?;
        Ok(Header::OneByteStartStop(
            start,
            stop,
            self.parse_ranged_variation(gv, range)?,
        ))
    }

    fn parse_start_stop_u16(&mut self, gv: GroupVar) -> Result<Header<'a>, ParseError> {
        let start = self.cursor.read_u16_le()?;
        let stop = self.cursor.read_u16_le()?;
        let range = Range::from(start, stop)?;
        Ok(Header::TwoByteStartStop(
            start,
            stop,
            self.parse_ranged_variation(gv, range)?,
        ))
    }

    fn parse_ranged_variation(
        &mut self,
        gv: GroupVar,
        range: Range,
    ) -> Result<RangedVariation<'a>, ParseError> {
        let variation = match gv {
            GroupVar::Group2Var0 => RangedVariation::Group2Var0,
            GroupVar::Group2Var1 => {
                RangedVariation::Group2Var1(RangedSequence::parse(range, &mut self.cursor)?)
            }
            GroupVar::Group2Var2 => {
                RangedVariation::Group2Var2(RangedSequence::parse(range, &mut self.cursor)?)
            }
            GroupVar::Group2Var3 => {
                RangedVariation::Group2Var3(RangedSequence::parse(range, &mut self.cursor)?)
            }
        };
        Ok(variation)
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
    use crate::app::header::*;
    use crate::app::variations::Group2Var1;

    #[test]
    fn parses_range_of_g2v1() {
        let input = [0x02, 0x01, 0x00, 0x02, 0x03, 0xAA, 0xBB];

        let mut parser = Parser::new(&input);

        let items: Vec<(Group2Var1, u16)> = match parser.next().unwrap().unwrap() {
            Header::OneByteStartStop(02, 03, RangedVariation::Group2Var1(seq)) => {
                seq.iter().collect()
            }
            x => panic!("got: {:?}", x),
        };

        assert_eq!(
            items,
            vec![
                (Group2Var1 { flags: 0xAA }, 2),
                (Group2Var1 { flags: 0xBB }, 3)
            ]
        );

        assert_eq!(parser.next(), None);
    }
}
