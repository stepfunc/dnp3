use std::str;

use super::calendar;
use super::reader::Reader;
use super::types::*;

type ASNResult<'a> = Result<ASNType<'a>, ASNError>;

fn parse_seq(contents: &[u8]) -> ASNResult {
    Ok(Sequence::asn(contents))
}

fn parse_set(contents: &[u8]) -> ASNResult {
    Ok(Set::asn(contents))
}

fn parse_null(contents: &[u8]) -> ASNResult {
    if contents.is_empty() {
        Ok(ASNType::Null)
    } else {
        Err(ASNError::NullWithNonEmptyContents(contents.len()))
    }
}

fn parse_boolean(contents: &[u8]) -> ASNResult {
    match contents {
        [0xFF] => Ok(Boolean::asn(true)),
        [0x00] => Ok(Boolean::asn(false)),
        [x] => Err(ASNError::BadBooleanValue(*x)),
        _ => Err(ASNError::BadBooleanLength(contents.len())),
    }
}

fn parse_integer(contents: &[u8]) -> ASNResult {
    if contents.is_empty() {
        Err(ASNError::ZeroLengthInteger)
    } else {
        Ok(Integer::asn(ASNInteger::new(contents)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
enum TimeType {
    Utc,
    Generalized,
}

fn parse_utc_time(contents: &[u8]) -> ASNResult {
    parse_time(contents, TimeType::Utc)
}

fn parse_generalized_time(contents: &[u8]) -> ASNResult {
    parse_time(contents, TimeType::Generalized)
}

fn parse_time(contents: &[u8], time_type: TimeType) -> ASNResult {
    // This code is highly inspired from webpki available here:
    // https://github.com/briansmith/webpki/blob/18cda8a5e32dfc2723930018853a984bd634e667/src/der.rs#L113-L166

    // The original file is licensed under this:

    // Except as otherwise noted, this project is licensed under the following
    // (ISC-style) terms:
    //
    // Copyright 2015 Brian Smith.
    //
    // Permission to use, copy, modify, and/or distribute this software for any
    // purpose with or without fee is hereby granted, provided that the above
    // copyright notice and this permission notice appear in all copies.
    //
    // THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHORS DISCLAIM ALL WARRANTIES
    // WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
    // MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHORS BE LIABLE FOR
    // ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
    // WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
    // ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
    // OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.

    // The files under third-party/chromium are licensed as described in
    // third-party/chromium/LICENSE.

    let mut reader = Reader::new(contents);

    fn read_digit(inner: &mut Reader) -> Result<u64, ASNError> {
        const DIGIT: core::ops::RangeInclusive<u8> = b'0'..=b'9';
        let b = inner.read_byte().map_err(|_| ASNError::BadUTCTime)?;
        if DIGIT.contains(&b) {
            return Ok(u64::from(b - DIGIT.start()));
        }
        Err(ASNError::BadUTCTime)
    }

    fn read_two_digits(inner: &mut Reader, min: u64, max: u64) -> Result<u64, ASNError> {
        let hi = read_digit(inner)?;
        let lo = read_digit(inner)?;
        let value = (hi * 10) + lo;
        if value < min || value > max {
            return Err(ASNError::BadUTCTime);
        }
        Ok(value)
    }

    let (year_hi, year_lo) = match time_type {
        TimeType::Utc => {
            let lo = read_two_digits(&mut reader, 0, 99)?;
            let hi = if lo >= 50 { 19 } else { 20 };
            (hi, lo)
        }
        TimeType::Generalized => {
            let hi = read_two_digits(&mut reader, 0, 99)?;
            let lo = read_two_digits(&mut reader, 0, 99)?;
            (hi, lo)
        }
    };

    let year = (year_hi * 100) + year_lo;
    let month = read_two_digits(&mut reader, 1, 12)?;
    let days_in_month = calendar::days_in_month(year, month);
    let day_of_month = read_two_digits(&mut reader, 1, days_in_month)?;
    let hours = read_two_digits(&mut reader, 0, 23)?;
    let minutes = read_two_digits(&mut reader, 0, 59)?;
    let seconds = read_two_digits(&mut reader, 0, 59)?;

    let time_zone = reader.read_byte().map_err(|_| ASNError::BadUTCTime)?;
    if time_zone != b'Z' {
        return Err(ASNError::BadUTCTime);
    }

    calendar::time_from_ymdhms_utc(year, month, day_of_month, hours, minutes, seconds)
        .map(ASNType::UTCTime)
}

fn parse_string<T: Fn(&str) -> ASNType>(contents: &[u8], create: T) -> ASNResult {
    match str::from_utf8(contents) {
        Ok(x) => Ok(create(x)),
        Err(x) => Err(ASNError::BadUTF8(x)),
    }
}

fn parse_bit_string(contents: &[u8]) -> ASNResult {
    if contents.is_empty() {
        return Err(ASNError::EndOfStream);
    }

    let unused_bits = contents[0];
    if unused_bits > 7 {
        return Err(ASNError::BitStringUnusedBitsTooLarge(unused_bits));
    }

    Ok(BitString::asn(ASNBitString::new(
        unused_bits,
        &contents[1..],
    )))
}

fn parse_object_identifier(contents: &[u8]) -> ASNResult {
    fn parse_one(reader: &mut Reader) -> Result<u32, ASNError> {
        let mut sum: u32 = 0;
        let mut count: u32 = 0;
        loop {
            // only allow 4*7 = 28 bits so that we don't overflow u32
            if count > 3 {
                return Err(ASNError::BadOidLength);
            };

            let next_byte = reader.read_byte()?;
            let has_next: bool = (next_byte & 0b1000_0000) != 0;
            let value: u32 = (next_byte & 0b0111_1111) as u32;

            sum <<= 7;
            sum += value;

            count += 1;

            if !has_next {
                return Ok(sum);
            }
        }
    }

    let mut reader = Reader::new(contents);

    let mut items: Vec<u32> = Vec::new();

    let first_byte = reader.read_byte()?;

    items.push((first_byte / 40) as u32);
    items.push((first_byte % 40) as u32);

    while !reader.is_empty() {
        items.push(parse_one(&mut reader)?);
    }

    Ok(ObjectIdentifier::asn(ASNObjectIdentifier::new(items)))
}

fn parse_length(reader: &mut Reader) -> Result<usize, ASNError> {
    let first_byte = reader.read_byte()?;

    let top_bit = first_byte & 0b1000_0000;
    let count_of_bytes = first_byte & 0b0111_1111;

    if top_bit == 0 {
        return Ok(count_of_bytes as usize);
    }

    // DER only allows a single encoding for any particular
    // value. For a given count of bytes, the encoded
    // length must fit into the minimum length representation
    let min_value_for_count: u64 = match count_of_bytes {
        0 => return Err(ASNError::UnsupportedIndefiniteLength),
        127 => return Err(ASNError::ReservedLengthValue),
        // anything < these numbers indicate the value should have been encoded with fewer bytes
        1 => 128,
        2 => 256,
        3 => 65536,
        4 => 16777216,
        _ => return Err(ASNError::UnsupportedLengthByteCount(count_of_bytes)),
    };

    let mut value: usize = 0;

    for _ in 0..count_of_bytes {
        value <<= 8;
        value |= reader.read_byte()? as usize;
    }

    if (value as u64) < min_value_for_count {
        return Err(ASNError::BadLengthEncoding(count_of_bytes, value));
    }

    Ok(value)
}

fn parse_one_type<'a>(reader: &mut Reader<'a>) -> ASNResult<'a> {
    let id = Identifier::from(reader.read_byte()?);

    match read_type(&id) {
        Some((asn_type, tag)) => {
            let contents = get_contents(reader)?;
            parse_content(&asn_type, tag, contents)
        }
        None => Err(ASNError::UnsupportedId(id)),
    }
}

fn read_type(id: &Identifier) -> Option<(ASNTypeId, u8)> {
    match id {
        Identifier {
            class: TagClass::Universal,
            pc: PC::Primitive,
            tag,
        } => match tag {
            0x01 => Some((ASNTypeId::Boolean, *tag)),
            0x02 => Some((ASNTypeId::Integer, *tag)),
            0x03 => Some((ASNTypeId::BitString, *tag)),
            0x04 => Some((ASNTypeId::OctetString, *tag)),
            0x05 => Some((ASNTypeId::Null, *tag)),
            0x06 => Some((ASNTypeId::ObjectIdentifier, *tag)),
            0x0C => Some((ASNTypeId::UTF8String, *tag)),
            0x13 => Some((ASNTypeId::PrintableString, *tag)),
            0x16 => Some((ASNTypeId::IA5String, *tag)),
            0x17 => Some((ASNTypeId::UTCTime, *tag)),
            0x18 => Some((ASNTypeId::GeneralizedTime, *tag)),

            _ => None,
        },
        Identifier {
            class: TagClass::Universal,
            pc: PC::Constructed,
            tag,
        } => match tag {
            0x10 => Some((ASNTypeId::Sequence, *tag)),
            0x11 => Some((ASNTypeId::Set, *tag)),

            _ => None,
        },

        Identifier {
            class: TagClass::ContextSpecific,
            tag,
            ..
        } => Some((ASNTypeId::ExplicitTag, *tag)),

        _ => None,
    }
}

fn get_contents<'a>(reader: &mut Reader<'a>) -> Result<&'a [u8], ASNError> {
    let length = parse_length(reader)?;
    Ok(reader.take(length)?)
}

fn parse_content<'a>(type_id: &ASNTypeId, tag: u8, contents: &'a [u8]) -> ASNResult<'a> {
    match type_id {
        ASNTypeId::Boolean => parse_boolean(contents),
        ASNTypeId::Integer => parse_integer(contents),
        ASNTypeId::BitString => parse_bit_string(contents),
        ASNTypeId::OctetString => Ok(OctetString::asn(contents)),
        ASNTypeId::Null => parse_null(contents),
        ASNTypeId::ObjectIdentifier => parse_object_identifier(contents),
        ASNTypeId::UTF8String => parse_string(contents, |s| UTF8String::asn(s)),
        ASNTypeId::PrintableString => parse_string(contents, |s| PrintableString::asn(s)),
        ASNTypeId::IA5String => parse_string(contents, |s| IA5String::asn(s)),
        ASNTypeId::UTCTime => parse_utc_time(contents),
        ASNTypeId::GeneralizedTime => parse_generalized_time(contents),

        ASNTypeId::Sequence => parse_seq(contents),
        ASNTypeId::Set => parse_set(contents),

        ASNTypeId::ExplicitTag => Ok(ExplicitTag::asn(ASNExplicitTag::new(tag, contents))),
    }
}

pub(crate) struct Parser<'a> {
    reader: Reader<'a>,
}

#[allow(unused)]
impl<'a> Parser<'a> {
    pub(crate) fn parse_all<'b, T: 'b>(
        input: &'b [u8],
        parse: fn(&mut Parser<'b>) -> Result<T, ASNError>,
    ) -> Result<T, ASNError> {
        let mut parser = Parser::new(input);
        let value = parse(&mut parser)?;
        parser.expect_end()?;
        Ok(value)
    }

    pub(crate) fn new(input: &'a [u8]) -> Parser {
        Parser {
            reader: Reader::new(input),
        }
    }

    pub(crate) fn unwrap_outer_sequence(input: &'a [u8]) -> Result<Parser, ASNError> {
        let mut parser = Parser::new(input);
        let bytes = parser.expect::<Sequence>()?;
        parser.expect_end()?;
        Ok(Parser::new(bytes))
    }

    pub(crate) fn unwrap_outer_set(input: &'a [u8]) -> Result<Parser, ASNError> {
        let mut parser = Parser::new(input);
        let bytes = parser.expect::<Set>()?;
        parser.expect_end()?;
        Ok(Parser::new(bytes))
    }

    pub(crate) fn get_explicitly_tagged_value_or_default<T: ASNWrapperType<'a>>(
        &mut self,
        tag: u8,
        default: T::Item,
    ) -> Result<T::Item, ASNError> {
        match self.get_optional_explicit_tag_value::<T>(tag)? {
            Some(item) => Ok(item),
            None => Ok(default),
        }
    }

    pub(crate) fn get_optional_explicit_tag_value<T: ASNWrapperType<'a>>(
        &mut self,
        tag: u8,
    ) -> Result<Option<T::Item>, ASNError> {
        match self.get_optional_explicit_tag(tag)? {
            Some(tag) => {
                let mut parser = Parser::new(tag.contents);
                Ok(Some(parser.expect::<T>()?))
            }
            None => Ok(None),
        }
    }

    pub(crate) fn get_optional_explicit_tag(
        &mut self,
        tag: u8,
    ) -> Result<Option<ASNExplicitTag<'a>>, ASNError> {
        if self.reader.is_empty() {
            return Ok(None);
        }

        let id = Identifier::from(self.reader.peek_or_fail()?);

        match read_type(&id) {
            Some((ASNTypeId::ExplicitTag, actual_tag)) if tag == actual_tag => {
                Ok(Some(self.expect::<ExplicitTag>()?))
            }
            Some(_) => Ok(None),
            None => Err(ASNError::UnsupportedId(id)),
        }
    }

    pub(crate) fn get_optional_or_default<T: ASNWrapperType<'a>>(
        &mut self,
        default: T::Item,
    ) -> Result<T::Item, ASNError> {
        match self.get_optional::<T>()? {
            Some(value) => Ok(value),
            None => Ok(default),
        }
    }

    pub(crate) fn get_optional<T: ASNWrapperType<'a>>(
        &mut self,
    ) -> Result<Option<T::Item>, ASNError> {
        if self.reader.is_empty() {
            return Ok(None);
        }

        let id = Identifier::from(self.reader.peek_or_fail()?);

        match read_type(&id) {
            Some((ref id, _)) if *id == T::get_id() => Ok(Some(self.expect::<T>()?)),
            Some(_) => Ok(None),
            None => Err(ASNError::UnsupportedId(id)),
        }
    }

    pub(crate) fn parse_implicit<T: ASNWrapperType<'a>>(&mut self) -> Result<T::Item, ASNError> {
        let result = match T::get_value(parse_content(&T::get_id(), 0, self.reader.remainder())?) {
            Some(value) => Ok(value),
            None => panic!("Wrapper should have returned a {:?}!", T::get_id()),
        };
        self.reader.clear();
        result
    }

    pub(crate) fn expect<T: ASNWrapperType<'a>>(&mut self) -> Result<T::Item, ASNError> {
        match self.expect_any() {
            Ok(asn_type) => {
                let id = asn_type.get_id();
                match T::get_value(asn_type) {
                    Some(value) => Ok(value),
                    None => Err(ASNError::UnexpectedType(T::get_id(), id)),
                }
            }
            Err(err) => Err(err),
        }
    }

    pub(crate) fn expect_or_end<T: ASNWrapperType<'a>>(
        &mut self,
    ) -> Result<Option<T::Item>, ASNError> {
        match self.expect::<T>() {
            Ok(value) => Ok(Some(value)),
            Err(ASNError::EndOfStream) => Ok(None),
            Err(err) => Err(err),
        }
    }

    pub(crate) fn expect_any(&mut self) -> Result<ASNType<'a>, ASNError> {
        match self.next() {
            Some(Ok(asn)) => Ok(asn),
            Some(Err(err)) => Err(err),
            None => Err(ASNError::EndOfStream),
        }
    }

    pub(crate) fn expect_any_or_end(&mut self) -> Result<Option<ASNType<'a>>, ASNError> {
        match self.next() {
            Some(Ok(asn)) => Ok(Some(asn)),
            Some(Err(err)) => Err(err),
            None => Ok(None),
        }
    }

    pub(crate) fn expect_end(&mut self) -> Result<(), ASNError> {
        match self.next() {
            None => Ok(()),
            Some(Err(err)) => Err(err),
            Some(Ok(asn)) => Err(ASNError::ExpectedEnd(asn.get_id())),
        }
    }
}

impl<'a> Iterator for Parser<'a> {
    type Item = Result<ASNType<'a>, ASNError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.reader.is_empty() {
            return None;
        }

        match parse_one_type(&mut self.reader) {
            Err(e) => {
                self.reader.clear();
                Some(Err(e))
            }
            Ok(token) => Some(Ok(token)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOP_BIT: u8 = 1 << 7;

    fn test_parse_length(bytes: &[u8]) -> Result<usize, ASNError> {
        let mut reader = Reader::new(bytes);
        parse_length(&mut reader)
    }

    #[test]
    fn decode_length_on_empty_bytes_fails() {
        let mut reader = Reader::new(&[]);
        assert_eq!(parse_length(&mut reader), Err(ASNError::EndOfStream));
    }

    #[test]
    fn detects_indefinite_length() {
        let mut reader = Reader::new(&[0x80]);
        assert_eq!(
            parse_length(&mut reader),
            Err(ASNError::UnsupportedIndefiniteLength)
        )
    }

    #[test]
    fn detects_reserved_length_of_127() {
        let mut reader = Reader::new(&[0xFF]);
        assert_eq!(
            parse_length(&mut reader),
            Err(ASNError::ReservedLengthValue)
        )
    }

    #[test]
    fn decode_length_on_single_byte_returns_valid_result() {
        let mut reader = Reader::new(&[127, 0xDE, 0xAD]);
        assert_eq!(parse_length(&mut reader), Ok(127));
        assert_eq!(reader.remainder(), &[0xDE, 0xAD]);
    }

    #[test]
    fn detects_one_byte_bad_length_encoding() {
        assert_eq!(test_parse_length(&[TOP_BIT | 1, 128]), Ok(128));
        assert_eq!(
            test_parse_length(&[TOP_BIT | 1, 127]),
            Err(ASNError::BadLengthEncoding(1, 127))
        );
    }

    #[test]
    fn detects_two_byte_bad_length_encoding() {
        assert_eq!(test_parse_length(&[TOP_BIT | 2, 0x01, 0x00]), Ok(256));
        assert_eq!(
            test_parse_length(&[TOP_BIT | 2, 0x00, 0xFF]),
            Err(ASNError::BadLengthEncoding(2, 255))
        );
    }

    #[test]
    fn detects_three_byte_bad_length_encoding() {
        assert_eq!(
            test_parse_length(&[TOP_BIT | 3, 0x01, 0x00, 0x00]),
            Ok(65536)
        );
        assert_eq!(
            test_parse_length(&[TOP_BIT | 3, 0x00, 0xFF, 0xFF]),
            Err(ASNError::BadLengthEncoding(3, 65535))
        );
    }

    #[test]
    fn detects_four_byte_bad_length_encoding() {
        assert_eq!(
            test_parse_length(&[TOP_BIT | 4, 0x01, 0x00, 0x00, 0x00]),
            Ok(16777216)
        );
        assert_eq!(
            test_parse_length(&[TOP_BIT | 4, 0x00, 0xFF, 0xFF, 0xFF]),
            Err(ASNError::BadLengthEncoding(4, 16777215))
        );
    }

    #[test]
    fn decode_length_on_count_of_one_succeeds_if_value_greater_than_127() {
        let mut reader = Reader::new(&[TOP_BIT | 1, 128]);
        assert_eq!(parse_length(&mut reader), Ok(128));
        assert!(reader.is_empty());
    }

    #[test]
    fn decode_length_on_count_of_two_succeeds() {
        let mut reader = Reader::new(&[TOP_BIT | 2, 0x01, 0x02, 0x03]);
        assert_eq!(parse_length(&mut reader), Ok(0x0102));
        assert_eq!(reader.remainder(), &[0x03]);
    }

    #[test]
    fn decode_length_on_count_of_three_succeeds() {
        let mut reader = Reader::new(&[TOP_BIT | 3, 0x01, 0x02, 0x03, 0x04]);
        assert_eq!(parse_length(&mut reader), Ok(0x010203));
        assert_eq!(reader.remainder(), &[0x04]);
    }

    #[test]
    fn decode_length_on_count_of_four_succeeds() {
        let mut reader = Reader::new(&[TOP_BIT | 4, 0x01, 0x02, 0x03, 0x04, 0x05]);
        assert_eq!(parse_length(&mut reader), Ok(0x01020304));
        assert_eq!(reader.remainder(), &[0x05]);
    }

    #[test]
    fn decode_length_on_count_of_five_fails() {
        let mut reader = Reader::new(&[TOP_BIT | 5, 0x01, 0x02, 0x03, 0x04, 0x05]);
        assert_eq!(
            parse_length(&mut reader),
            Err(ASNError::UnsupportedLengthByteCount(5))
        )
    }

    #[test]
    fn parse_one_fails_for_non_universal_type() {
        let mut reader = Reader::new(&[0xFF]);
        assert_eq!(
            parse_one_type(&mut reader),
            Err(ASNError::UnsupportedId(Identifier::new(
                TagClass::Private,
                PC::Constructed,
                0x1F
            )))
        )
    }

    #[test]
    fn parse_one_fails_for_unknown_universal_type() {
        let mut reader = Reader::new(&[0x1F, 0x00]);
        assert_eq!(
            parse_one_type(&mut reader),
            Err(ASNError::UnsupportedId(Identifier::new(
                TagClass::Universal,
                PC::Primitive,
                0x1F
            )))
        )
    }

    #[test]
    fn parses_sequence_correctly() {
        let mut reader = Reader::new(&[0x30, 0x03, 0x02, 0x03, 0x04, 0x05, 0x06]);
        assert_eq!(
            parse_one_type(&mut reader),
            Ok(Sequence::asn(&[0x02, 0x03, 0x04]))
        );
        assert_eq!(reader.remainder(), &[0x05, 0x06]);
    }

    #[test]
    fn parse_sequence_fails_if_insufficient_bytes() {
        let mut reader = Reader::new(&[0x30, 0x0F, 0xDE, 0xAD]);
        assert_eq!(parse_one_type(&mut reader), Err(ASNError::EndOfStream));
    }

    #[test]
    fn parses_explicit_tag() {
        let mut reader = Reader::new(&[0xA1, 0x02, 0xCA, 0xFE]);
        assert_eq!(
            parse_one_type(&mut reader),
            Ok(ExplicitTag::asn(ASNExplicitTag::new(1, &[0xCA, 0xFE])))
        );
    }

    #[test]
    fn parses_utc_time() {
        // UTC time in the 20th century
        assert_eq!(
            parse_utc_time("990102052345Z".as_bytes()),
            Ok(UtcTime::asn(915254625))
        );

        // UTC time in the 21th century.
        // On October 9th 2001, Leonard Cohen and Sharon Robinson released "Ten New Songs".
        //
        // "The ponies run, the girls are young,
        // The odds are there to beat.
        // You win a while, and then it's done
        // Your little winning streak.
        // And summoned now to deal
        // With your invincible defeat,
        // You live your life as if it's real,
        // A Thousand Kisses Deep."
        assert_eq!(
            parse_utc_time("011009010203Z".as_bytes()),
            Ok(UtcTime::asn(1002589323))
        );
    }

    #[test]
    fn parses_generalized_time() {
        // UTC time in the 20th century
        assert_eq!(
            parse_generalized_time("19990102052345Z".as_bytes()),
            Ok(UtcTime::asn(915254625))
        );
    }

    #[test]
    fn parses_known_object_identifiers() {
        // Microsoft: szOID_REQUEST_CLIENT_INFO
        assert_eq!(
            parse_object_identifier(&[0x2b, 0x06, 0x01, 0x04, 0x01, 0x82, 0x37, 0x15, 0x14]),
            Ok(ObjectIdentifier::asn(ASNObjectIdentifier::new(
                [1, 3, 6, 1, 4, 1, 311, 21, 20].to_vec()
            )))
        );

        // sha1WithRSAEncryption
        assert_eq!(
            parse_object_identifier(&[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x05]),
            Ok(ObjectIdentifier::asn(ASNObjectIdentifier::new(
                [1, 2, 840, 113549, 1, 1, 5].to_vec()
            )))
        );
    }
}
