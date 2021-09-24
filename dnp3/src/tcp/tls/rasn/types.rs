use super::oid::get_oid;
use super::reader;
use chrono::{DateTime, FixedOffset};
use std::fmt::Display;

#[derive(Debug, PartialEq)]
pub(crate) struct ASNInteger<'a> {
    pub(crate) bytes: &'a [u8],
}

#[derive(Debug, PartialEq)]
pub(crate) enum TagClass {
    Universal,
    Application,
    ContextSpecific,
    Private,
}

#[derive(Debug, PartialEq)]
pub(crate) enum PC {
    Primitive,
    Constructed,
}

#[derive(Debug, PartialEq)]
pub(crate) struct Identifier {
    pub(crate) class: TagClass,
    pub(crate) pc: PC,
    pub(crate) tag: u8,
}

impl Identifier {
    pub(crate) fn new(class: TagClass, pc: PC, tag: u8) -> Identifier {
        Identifier { class, pc, tag }
    }

    pub(crate) fn from(byte: u8) -> Identifier {
        let class = match byte & 0b1100_0000 {
            0b0000_0000 => TagClass::Universal,
            0b0100_0000 => TagClass::Application,
            0b1000_0000 => TagClass::ContextSpecific,
            _ => TagClass::Private,
        };

        let pc = if (byte & 0b0010_0000) != 0 {
            PC::Constructed
        } else {
            PC::Primitive
        };

        let tag = byte & 0b0001_1111;

        Identifier::new(class, pc, tag)
    }
}

impl<'a> ASNInteger<'a> {
    const VALID_I32_LENGTHS: std::ops::Range<usize> = 1usize..4usize;

    pub(crate) fn new(bytes: &'a [u8]) -> ASNInteger {
        ASNInteger { bytes }
    }

    pub(crate) fn as_i32(&self) -> Option<i32> {
        // can only parse values with length in [1,4] bytes
        if !ASNInteger::VALID_I32_LENGTHS.contains(&self.bytes.len()) {
            return None;
        }

        let mut acc: i32 = 0;
        for byte in self.bytes {
            acc <<= 8;
            acc |= *byte as i32;
        }
        Some(acc)
    }
}

impl<'a> Display for ASNInteger<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.as_i32() {
            Some(x) => write!(f, "{}", x),
            None => {
                if let Some((tail, head)) = self.bytes.split_last() {
                    for byte in head {
                        write!(f, "{:02X}:", byte)?;
                    }
                    write!(f, "{:02X}", tail)
                } else {
                    write!(f, "[]")
                }
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct ASNBitString<'a> {
    // the number of unused bits in last octet [0, 7]
    unused_bits: u8,
    // the octets, the last one only has (8 - unused_bits) bits
    bytes: &'a [u8],
}

impl<'a> ASNBitString<'a> {
    pub(crate) fn new(unused_bits: u8, bytes: &'a [u8]) -> ASNBitString<'a> {
        ASNBitString { unused_bits, bytes }
    }

    // convertible to octets if it's all full bytes
    pub(crate) fn octets(&self) -> Option<&[u8]> {
        if self.unused_bits == 0 {
            Some(self.bytes)
        } else {
            None
        }
    }

    pub(crate) fn size(&self) -> usize {
        self.bytes.len() * 8 - (self.unused_bits as usize)
    }

    pub(crate) fn iter(&'a self) -> ASNBitStringIterator<'a> {
        ASNBitStringIterator::new(self)
    }
}

pub(crate) struct ASNBitStringIterator<'a> {
    bit_string: &'a ASNBitString<'a>,
    current_bit: usize,
}

impl<'a> ASNBitStringIterator<'a> {
    fn new(bit_string: &'a ASNBitString<'a>) -> ASNBitStringIterator<'a> {
        ASNBitStringIterator {
            bit_string,
            current_bit: 0,
        }
    }
}

impl<'a> Iterator for ASNBitStringIterator<'a> {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_bit < self.bit_string.size() {
            let result = Some(
                self.bit_string.bytes[self.current_bit / 8] << ((self.current_bit % 8) as u8)
                    & 0x80
                    != 0,
            );
            self.current_bit += 1;
            result
        } else {
            None
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct ASNExplicitTag<'a> {
    pub(crate) value: u8,
    pub(crate) contents: &'a [u8],
}

impl<'a> ASNExplicitTag<'a> {
    pub(crate) fn new(value: u8, contents: &'a [u8]) -> ASNExplicitTag<'a> {
        ASNExplicitTag { value, contents }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct ASNObjectIdentifier {
    items: Vec<u32>,
}

impl ASNObjectIdentifier {
    pub(crate) fn new(items: Vec<u32>) -> ASNObjectIdentifier {
        ASNObjectIdentifier { items }
    }

    pub(crate) fn values(&self) -> &[u32] {
        self.items.as_slice()
    }
}

impl Display for ASNObjectIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match get_oid(self.values()) {
            Some(oid) => f.write_str(oid.to_str()),
            None => {
                if let Some((last, first)) = self.values().split_last() {
                    for value in first {
                        write!(f, "{}.", value)?;
                    }
                    write!(f, "{}", last)?;
                }
                Ok(())
            }
        }
    }
}

pub(crate) trait ASNWrapperType<'a> {
    type Item;

    //fn new<'b>(value: Self::Item) -> ASNType<'b>;
    fn get_id() -> ASNTypeId;
    fn get_value(asn_type: ASNType<'a>) -> Option<Self::Item>;
}

#[derive(Debug, PartialEq)]
pub(crate) struct Boolean {
    pub(crate) value: bool,
}
impl Boolean {
    pub(crate) fn asn<'a>(value: bool) -> ASNType<'a> {
        ASNType::Boolean(Boolean { value })
    }
}
impl<'a> ASNWrapperType<'a> for Boolean {
    type Item = bool;

    fn get_id() -> ASNTypeId {
        ASNTypeId::Boolean
    }

    fn get_value(asn_type: ASNType<'a>) -> Option<Self::Item> {
        match asn_type {
            ASNType::Boolean(wrapper) => Some(wrapper.value),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct Integer<'a> {
    pub(crate) value: ASNInteger<'a>,
}
impl<'a> Integer<'a> {
    pub(crate) fn asn(value: ASNInteger<'a>) -> ASNType<'a> {
        ASNType::Integer(Integer { value })
    }
}
impl<'a> ASNWrapperType<'a> for Integer<'a> {
    type Item = ASNInteger<'a>;

    fn get_id() -> ASNTypeId {
        ASNTypeId::Integer
    }

    fn get_value(asn_type: ASNType<'a>) -> Option<Self::Item> {
        match asn_type {
            ASNType::Integer(wrapper) => Some(wrapper.value),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct PrintableString<'a> {
    pub(crate) value: &'a str,
}
impl<'a> PrintableString<'a> {
    pub(crate) fn asn(value: &'a str) -> ASNType<'a> {
        ASNType::PrintableString(PrintableString { value })
    }
}
impl<'a> ASNWrapperType<'a> for PrintableString<'a> {
    type Item = &'a str;

    fn get_id() -> ASNTypeId {
        ASNTypeId::PrintableString
    }

    fn get_value(asn_type: ASNType<'a>) -> Option<Self::Item> {
        match asn_type {
            ASNType::PrintableString(wrapper) => Some(wrapper.value),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct IA5String<'a> {
    pub(crate) value: &'a str,
}
impl<'a> IA5String<'a> {
    pub(crate) fn asn(value: &'a str) -> ASNType<'a> {
        ASNType::IA5String(IA5String { value })
    }
}
impl<'a> ASNWrapperType<'a> for IA5String<'a> {
    type Item = &'a str;

    fn get_id() -> ASNTypeId {
        ASNTypeId::IA5String
    }

    fn get_value(asn_type: ASNType<'a>) -> Option<Self::Item> {
        match asn_type {
            ASNType::IA5String(wrapper) => Some(wrapper.value),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct UTF8String<'a> {
    pub(crate) value: &'a str,
}
impl<'a> UTF8String<'a> {
    pub(crate) fn asn(value: &'a str) -> ASNType<'a> {
        ASNType::UTF8String(UTF8String { value })
    }
}
impl<'a> ASNWrapperType<'a> for UTF8String<'a> {
    type Item = &'a str;

    fn get_id() -> ASNTypeId {
        ASNTypeId::UTF8String
    }

    fn get_value(asn_type: ASNType<'a>) -> Option<Self::Item> {
        match asn_type {
            ASNType::UTF8String(wrapper) => Some(wrapper.value),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct Sequence<'a> {
    pub(crate) value: &'a [u8],
}
impl<'a> Sequence<'a> {
    pub(crate) fn asn(value: &'a [u8]) -> ASNType<'a> {
        ASNType::Sequence(Sequence { value })
    }
}
impl<'a> ASNWrapperType<'a> for Sequence<'a> {
    type Item = &'a [u8];

    fn get_id() -> ASNTypeId {
        ASNTypeId::Sequence
    }

    fn get_value(asn_type: ASNType<'a>) -> Option<Self::Item> {
        match asn_type {
            ASNType::Sequence(wrapper) => Some(wrapper.value),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct Set<'a> {
    pub(crate) value: &'a [u8],
}
impl<'a> Set<'a> {
    pub(crate) fn asn(value: &'a [u8]) -> ASNType<'a> {
        ASNType::Set(Set { value })
    }
}
impl<'a> ASNWrapperType<'a> for Set<'a> {
    type Item = &'a [u8];

    fn get_id() -> ASNTypeId {
        ASNTypeId::Set
    }

    fn get_value(asn_type: ASNType<'a>) -> Option<Self::Item> {
        match asn_type {
            ASNType::Set(wrapper) => Some(wrapper.value),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct ObjectIdentifier {
    pub(crate) value: ASNObjectIdentifier,
}
impl ObjectIdentifier {
    pub(crate) fn asn<'a>(value: ASNObjectIdentifier) -> ASNType<'a> {
        ASNType::ObjectIdentifier(ObjectIdentifier { value })
    }
}
impl<'a> ASNWrapperType<'a> for ObjectIdentifier {
    type Item = ASNObjectIdentifier;

    fn get_id() -> ASNTypeId {
        ASNTypeId::ObjectIdentifier
    }

    fn get_value(asn_type: ASNType<'a>) -> Option<Self::Item> {
        match asn_type {
            ASNType::ObjectIdentifier(wrapper) => Some(wrapper.value),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct OctetString<'a> {
    pub(crate) value: &'a [u8],
}
impl<'a> OctetString<'a> {
    pub(crate) fn asn(value: &'a [u8]) -> ASNType<'a> {
        ASNType::OctetString(OctetString { value })
    }
}
impl<'a> ASNWrapperType<'a> for OctetString<'a> {
    type Item = &'a [u8];

    fn get_id() -> ASNTypeId {
        ASNTypeId::OctetString
    }

    fn get_value(asn_type: ASNType<'a>) -> Option<Self::Item> {
        match asn_type {
            ASNType::OctetString(wrapper) => Some(wrapper.value),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct BitString<'a> {
    pub(crate) value: ASNBitString<'a>,
}
impl<'a> BitString<'a> {
    pub(crate) fn asn(value: ASNBitString<'a>) -> ASNType<'a> {
        ASNType::BitString(BitString { value })
    }
}
impl<'a> ASNWrapperType<'a> for BitString<'a> {
    type Item = ASNBitString<'a>;

    fn get_id() -> ASNTypeId {
        ASNTypeId::BitString
    }

    fn get_value(asn_type: ASNType<'a>) -> Option<Self::Item> {
        match asn_type {
            ASNType::BitString(wrapper) => Some(wrapper.value),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct UtcTime {
    pub(crate) value: DateTime<FixedOffset>,
}
impl UtcTime {
    pub(crate) fn asn<'a>(value: DateTime<FixedOffset>) -> ASNType<'a> {
        ASNType::UTCTime(UtcTime { value })
    }
}
impl<'a> ASNWrapperType<'a> for UtcTime {
    type Item = DateTime<FixedOffset>;

    fn get_id() -> ASNTypeId {
        ASNTypeId::UTCTime
    }

    fn get_value(asn_type: ASNType<'a>) -> Option<Self::Item> {
        match asn_type {
            ASNType::UTCTime(wrapper) => Some(wrapper.value),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct ExplicitTag<'a> {
    pub(crate) value: ASNExplicitTag<'a>,
}
impl<'a> ExplicitTag<'a> {
    pub(crate) fn asn(value: ASNExplicitTag<'a>) -> ASNType<'a> {
        ASNType::ExplicitTag(ExplicitTag { value })
    }
}
impl<'a> ASNWrapperType<'a> for ExplicitTag<'a> {
    type Item = ASNExplicitTag<'a>;

    fn get_id() -> ASNTypeId {
        ASNTypeId::ExplicitTag
    }

    fn get_value(asn_type: ASNType<'a>) -> Option<Self::Item> {
        match asn_type {
            ASNType::ExplicitTag(wrapper) => Some(wrapper.value),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum ASNType<'a> {
    Boolean(Boolean),
    Sequence(Sequence<'a>),
    Set(Set<'a>),
    Integer(Integer<'a>),
    PrintableString(PrintableString<'a>),
    IA5String(IA5String<'a>),
    UTF8String(UTF8String<'a>),
    Null,
    UTCTime(UtcTime),
    BitString(BitString<'a>),
    OctetString(OctetString<'a>),
    ObjectIdentifier(ObjectIdentifier),
    ExplicitTag(ExplicitTag<'a>),
}

// An identifier for the type that carries no data
// used for error purposes
#[derive(Debug, PartialEq)]
pub(crate) enum ASNTypeId {
    Boolean,
    Sequence,
    Set,
    Integer,
    PrintableString,
    IA5String,
    UTF8String,
    Null,
    UTCTime,
    BitString,
    OctetString,
    ObjectIdentifier,
    ExplicitTag,
}

impl<'a> ASNType<'a> {
    pub(crate) fn get_id(&self) -> ASNTypeId {
        match self {
            ASNType::Boolean(_) => ASNTypeId::Boolean,
            ASNType::Sequence(_) => ASNTypeId::Sequence,
            ASNType::Set(_) => ASNTypeId::Set,
            ASNType::Integer(_) => ASNTypeId::Integer,
            ASNType::PrintableString(_) => ASNTypeId::PrintableString,
            ASNType::IA5String(_) => ASNTypeId::IA5String,
            ASNType::UTF8String(_) => ASNTypeId::UTF8String,
            ASNType::Null => ASNTypeId::Null,
            ASNType::UTCTime(_) => ASNTypeId::UTCTime,
            ASNType::BitString(_) => ASNTypeId::BitString,
            ASNType::OctetString(_) => ASNTypeId::OctetString,
            ASNType::ObjectIdentifier(_) => ASNTypeId::ObjectIdentifier,
            ASNType::ExplicitTag(_) => ASNTypeId::ExplicitTag,
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum ASNError {
    // these errors relate to core DER parsing
    BadBooleanLength(usize),
    BadBooleanValue(u8),
    EndOfStream,
    ZeroLengthInteger,
    NullWithNonEmptyContents(usize),
    UnsupportedId(Identifier),
    UnsupportedIndefiniteLength,
    ReservedLengthValue,
    UnsupportedLengthByteCount(u8),
    BadLengthEncoding(u8, usize), // count of bytes followed by the value
    BadOidLength,
    BadUTF8(std::str::Utf8Error),
    BadUTCTime(chrono::format::ParseError),
    BitStringUnusedBitsTooLarge(u8),
    // these errors relate to schemas
    UnexpectedType(ASNTypeId, ASNTypeId), // the expected type followed by the actual type
    ExpectedEnd(ASNTypeId),               // type present instead of end
    IntegerTooLarge(usize),               // count of bytes
    BadEnumValue(&'static str, i32),      // name of the enum and the bad integer value
    UnexpectedOid(ASNObjectIdentifier),   // unexpected object identifier
    UnexpectedTag(u8),                    // unexpected tag
}

impl From<reader::EndOfStream> for ASNError {
    fn from(_: reader::EndOfStream) -> Self {
        ASNError::EndOfStream
    }
}

impl From<std::str::Utf8Error> for ASNError {
    fn from(err: std::str::Utf8Error) -> Self {
        ASNError::BadUTF8(err)
    }
}
