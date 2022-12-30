use crate::app::parse::range::Range;
use crate::app::parse::traits::{FixedSize, Index};
use crate::app::ObjectParseError;
use scursor::{ReadCursor, ReadError};
use std::fmt::{Display, Formatter};
use std::str::Utf8Error;

/// Set to which a device attribute belongs
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AttrSet {
    /// The default attribute set defined by DNP3.org
    Default,
    /// Non-zero privately defined attribute set
    Private(u8),
}

impl AttrSet {
    /// Initialize based on raw value
    pub fn new(value: u8) -> Self {
        match value {
            0 => Self::Default,
            _ => Self::Private(value),
        }
    }

    /// Initialize based on raw value
    pub fn get(self) -> u8 {
        match self {
            AttrSet::Default => 0,
            AttrSet::Private(x) => x,
        }
    }

    pub(crate) fn from_range(range: Range) -> Result<Self, AttrParseError> {
        let value: u8 = match range.get_start().try_into() {
            Err(_) => {
                return Err(AttrParseError::SetIdNotU8(range.get_start()));
            }
            Ok(x) => x,
        };

        if range.get_count() != 1 {
            return Err(AttrParseError::CountNotOne(range.get_count()));
        }

        Ok(Self::new(value))
    }
}

impl Default for AttrSet {
    fn default() -> Self {
        Self::Default
    }
}

/// Variants for all the pre-defined attributes in the standard
pub enum KnownAttribute<'a> {
    /// Variation 252 - Device manufacturer's name
    DeviceManufacturersName(&'a str),
    /// Variation 250 - Device manufacturer's product name and model
    ProductNameAndModel(&'a str),
    /// Variation 249 - DNP3 subset and conformance
    SubsetAndConformance(&'a str),
    /// Variation 248 - DNP3 subset and conformance
    DeviceSerialNumber(&'a str),
    /// Variation 247 - User assigned device name
    UserAssignedDeviceName(&'a str),
    /// Variation 246 - User assigned ID code/number
    UserAssignedId(&'a str),
    /// Variation 245 - User assigned location/name
    UserAssignedLocation(&'a str),
    /// Variation 243 - Device manufacturer hardware version
    DeviceManufacturerHardwareVersion(&'a str),
    /// Variation 242 - Device manufacturer software version
    DeviceManufacturerSoftwareVersion(&'a str),
    /// Variation 241 - Maximum receive fragment size
    MaximumReceiveFragmentSize(u32),
    /// Variation 240 - Maximum transmit fragment size
    MaximumTransmitFragmentSize(u32),
    /// Variation 239 - Number of binary input points
    NumBinaryInput(u32),
    /// Variation 238 - Maximum binary input point index
    MaxBinaryInputIndex(u32),
    /// Variation 237 - Support binary input events
    SupportsBinaryInputEvents(bool),
    /// Variation 236 - Number of double-bit binary input points
    NumDoubleBitBinaryInput(u32),
    /// Variation 235 - Maximum double-bit binary input point index
    MaxDoubleBitBinaryInputIndex(u32),
    /// Variation 234 - Support double-bit binary input events
    SupportsDoubleBitBinaryInputEvents(bool),
    /// Variation 233 - Number of analog input points
    NumAnalogInput(u32),
    /// Variation 232 - Maximum analog input point index
    MaAnalogInputIndex(u32),
    /// Variation 231 - Support analog input events
    SupportsAnalogInputEvents(bool),
    /// Variation 230 - Support frozen analog input events
    SupportsFrozenAnalogInputs(bool),
    /// Variation 229 - Number of counter points
    NumCounter(u32),
    /// Variation 228 - Maximum counter point index
    MaxCounterIndex(u32),
    /// Variation 227 - Support counter events
    SupportsCounterEvents(bool),
    /// Variation 226 - Support frozen counters
    SupportsFrozenCounters(bool),
    /// Variation 225 - Support frozen counter events
    SupportsFrozenCounterEvents(bool),
    /// Variation 224 - Number of binary outputs
    NumBinaryOutputs(u32),
    /// Variation 223 - Maximum binary output index
    MaxBinaryOutputIndex(u32),
    /// Variation 222 - Supports binary output events
    SupportsBinaryOutputEvents(bool),
    /// Variation 221 - Number of analog outputs
    NumAnalogOutputs(u32),
    /// Variation 220 - Maximum analog output index
    MaxAnalogOutputIndex(u32),
    /// Variation 219 - Supports analog output events
    SupportsAnalogOutputEvents(bool),
    /// Variation 218 - Duration of time accuracy (seconds)
    DurationOfTimeAccuracy(u32),
    /// Variation 217 - Local timing accuracy (microseconds)
    LocalTimingAccuracy(u32),
    /// Variation 216 - Maximum number of binary outputs per request
    MaxBinaryOutputPerRequest(u32),
    /// Variation 215 - Number of outstation defined data-sets
    NumOutstationDefinedDataSets(u32),
    /// Variation 214 - Number of master defined data-sets
    NumMasterDefinedDataSets(u32),
    /// Variation 213 - Number of outstation defined data-set prototypes
    NumOutstationDefinedDataSetProto(u32),
    /// Variation 212 - Number of master defined data-set prototypes
    NumMasterDefinedDataSetProto(u32),
    /// Variation 211 - Identification of user-specific attributes
    UserSpecificAttributes(&'a str),
    /// Variation 209 - Secure authentication version
    SecureAuthenticationVersion(u32),
}

const VISIBLE_STRING: u8 = 1;
const UNSIGNED_INT: u8 = 2;
const SIGNED_INT: u8 = 3;
const FLOATING_POINT: u8 = 4;
const OCTET_STRING: u8 = 5;
const BIT_STRING: u8 = 6;
const ATTR_LIST: u8 = 254;
const EXT_ATTR_LIST: u8 = 255;

/// Attribute data type code
///
/// IEEE 1815-2012 pg 150
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AttrDataType {
    /// VSTR - Visible character suitable for print and display
    VisibleString,
    /// UINT - Unsigned integer
    UnsignedInt,
    /// Int - Signed integer
    SignedInt,
    /// FLT - Floating-point
    FloatingPoint,
    /// OSTR - Octet string
    OctetString,
    /// BSTR - Bit string
    BitString,
    /// List of UINT8-BSTR8 pairs
    AttrList,
    /// Extended list of UINT8-BSTR8 pairs
    ExtAttrList,
}

impl AttrDataType {
    pub(crate) fn get(value: u8) -> Option<AttrDataType> {
        match value {
            VISIBLE_STRING => Some(Self::VisibleString),
            UNSIGNED_INT => Some(Self::UnsignedInt),
            SIGNED_INT => Some(Self::SignedInt),
            FLOATING_POINT => Some(Self::FloatingPoint),
            OCTET_STRING => Some(Self::OctetString),
            BIT_STRING => Some(Self::BitString),
            ATTR_LIST => Some(Self::AttrList),
            EXT_ATTR_LIST => Some(Self::ExtAttrList),
            _ => None,
        }
    }

    /*
    pub(crate) fn to_u8(self) -> u8 {
        match self {
            Self::VisibleString => VISIBLE_STRING,
            Self::UnsignedInt => UNSIGNED_INT,
            Self::SignedInt => SIGNED_INT,
            Self::FloatingPoint => FLOATING_POINT,
            Self::OctetString => OCTET_STRING,
            Self::BitString => BIT_STRING,
            Self::AttrList => ATTR_LIST,
            Self::ExtAttrList => EXT_ATTR_LIST,
        }
    }
    */
}

/// A list of attributes returned from the outstation. This type is
/// the payload of g0v255. It implements an iterator over [`AttrItem`] values.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AttrList<'a> {
    data: &'a [u8],
}

/// Single entry in the attribute list
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AttrItem {
    /// Variation of the attribute
    pub variation: u8,
    /// Associated properties
    pub properties: AttrProp,
}

/// Attribute properties encoded in an attribute list
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct AttrProp {
    /// property indicating if the attribute can be written by the master
    is_writable: bool,
}

impl AttrProp {
    const READ_BIT: u8 = 0x01;

    /// Construct `AttrProp` with the write-able bit set
    pub fn writable(self) -> Self {
        Self { is_writable: true }
    }

    /// Returns true if the attribute is writeable
    pub fn is_writable(&self) -> bool {
        self.is_writable
    }

    pub(crate) fn new(props: u8) -> Self {
        Self {
            is_writable: props & Self::READ_BIT != 0,
        }
    }

    /*
    pub(crate) fn value(self) -> u8 {
        let mut value = 0;
        if self.is_writable {
            value |= Self::READ_BIT;
        }
        value
    }
     */
}

impl<'a> Iterator for AttrList<'a> {
    type Item = AttrItem;

    fn next(&mut self) -> Option<Self::Item> {
        let variation = *self.data.first()?;
        let prop = *self.data.get(1)?;

        self.data = match self.data.get(2..) {
            Some(x) => x,
            None => &[],
        };

        Some(AttrItem {
            variation,
            properties: AttrProp::new(prop),
        })
    }
}

/// Floating point attribute can be F32 or F64
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FloatType {
    /// Single-precision
    F32(f32),
    /// Double-precision
    F64(f64),
}

/// Represents the value of a device attribute
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum AttrValue<'a> {
    /// VSTR - Visible character suitable for print and display
    VisibleString(&'a str),
    /// UINT - Unsigned integer
    UnsignedInt(u32),
    /// Signed integer
    SignedInt(i32),
    /// Int - Signed integer
    FloatingPoint(FloatType),
    /// OSTR - Octet string
    OctetString(&'a [u8]),
    /// BSTR - Bit string
    BitString(&'a [u8]),
    /// List of UINT8-BSTR8
    AttrList(AttrList<'a>),
}

/// Attribute value and the set to which it belongs
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Attribute<'a> {
    /// Set to which the attribute belongs
    pub set: AttrSet,
    /// The variation of the attribute
    pub variation: u8,
    /// Value of the attribute
    pub value: AttrValue<'a>,
}

impl<'a> Attribute<'a> {
    pub(crate) fn parse_prefixed<I>(
        variation: u8,
        count: u16,
        cursor: &mut ReadCursor<'a>,
    ) -> Result<Self, ObjectParseError>
    where
        I: FixedSize + Index + Display,
    {
        if count != 1 {
            return Err(ObjectParseError::BadAttribute(AttrParseError::CountNotOne(
                count as usize,
            )));
        }
        let index = I::read(cursor)?.widen_to_u16();
        let set: u8 = index
            .try_into()
            .map_err(|_| AttrParseError::SetIdNotU8(index))?;
        let value = AttrValue::parse(cursor)?;
        Ok(Self {
            set: AttrSet::new(set),
            variation,
            value,
        })
    }

    pub(crate) fn parse_from_range(
        variation: u8,
        range: Range,
        cursor: &mut ReadCursor<'a>,
    ) -> Result<Self, AttrParseError> {
        let set = AttrSet::from_range(range)?;
        let value = AttrValue::parse(cursor)?;
        Ok(Self {
            set,
            variation,
            value,
        })
    }
}

/// Possible errors when parsing device attributes
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AttrParseError {
    /// End of buffer
    ReadError,
    /// Unknown data type code
    UnknownDataType(u8),
    /// Only 1, 2, and 4 byte lengths are supported
    BadIntegerLength(u8),
    /// Only 4 and 8 byte floats are supported
    BadFloatLength(u8),
    /// Attribute lists must be even in length (2*N) where N is number of pairs
    BadAttrListLength(u16),
    /// Visible string is not UTF-8. The DNP3 standard doesn't really define what "visible" means
    /// but this handles ASCII and is more flexible for non-english users.
    BadVisibleString(Utf8Error),
    /// Set identifier is not u8
    SetIdNotU8(u16),
    /// Count != 1
    CountNotOne(usize),
    /// Expected type X but received type Y
    UnexpectedType(AttrDataType, AttrDataType),
}

impl Display for AttrParseError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            AttrParseError::ReadError => f.write_str("attr read error"),
            AttrParseError::UnknownDataType(x) => write!(f, "Unknown attribute data type: {x}"),
            AttrParseError::BadIntegerLength(x) => {
                write!(f, "Unsupported attribute integer length: {x}")
            }
            AttrParseError::BadFloatLength(x) => {
                write!(f, "Bad attribute floating point length: {x}")
            }
            AttrParseError::BadAttrListLength(x) => {
                write!(f, "Attribute list has non-even length: {x}")
            }
            AttrParseError::BadVisibleString(x) => {
                write!(f, "Attribute visible string is not UTF8: {x}")
            }
            AttrParseError::SetIdNotU8(id) => {
                write!(f, "Attribute range or prefix is not [0,255] - value: {id}")
            }
            AttrParseError::CountNotOne(count) => write!(f, "Attribute count {count} != 1"),
            AttrParseError::UnexpectedType(x, y) => {
                write!(f, "Expected attribute type {x:?} but received {y:?}")
            }
        }
    }
}

impl std::error::Error for AttrParseError {}

impl From<ReadError> for AttrParseError {
    fn from(_: ReadError) -> Self {
        Self::ReadError
    }
}

impl From<Utf8Error> for AttrParseError {
    fn from(value: Utf8Error) -> Self {
        Self::BadVisibleString(value)
    }
}

impl<'a> AttrValue<'a> {
    /// Parse a device attribute (code, length, and payload)
    pub fn parse(cursor: &mut ReadCursor<'a>) -> Result<Self, AttrParseError> {
        let data_type = {
            let data_type = cursor.read_u8()?;
            match AttrDataType::get(data_type) {
                Some(x) => x,
                None => return Err(AttrParseError::UnknownDataType(data_type)),
            }
        };

        // the raw length
        let len = cursor.read_u8()?;

        let attr = match data_type {
            AttrDataType::VisibleString => {
                Self::VisibleString(Self::parse_visible_string(cursor, len)?)
            }
            AttrDataType::UnsignedInt => Self::UnsignedInt(Self::parse_unsigned_int(cursor, len)?),
            AttrDataType::SignedInt => Self::SignedInt(Self::parse_signed_int(cursor, len)?),
            AttrDataType::FloatingPoint => {
                Self::FloatingPoint(Self::parse_floating_point(cursor, len)?)
            }
            AttrDataType::OctetString => Self::OctetString(cursor.read_bytes(len as usize)?),
            AttrDataType::BitString => Self::BitString(cursor.read_bytes(len as usize)?),
            AttrDataType::AttrList => Self::AttrList(Self::parse_attr_list(cursor, len as u16)?),
            AttrDataType::ExtAttrList => {
                // with extended attribute lists, the len is really len + 256
                let len = len as u16 + 256;
                Self::AttrList(Self::parse_attr_list(cursor, len)?)
            }
        };

        Ok(attr)
    }

    /// underlying type
    pub fn get_type(&self) -> AttrDataType {
        match self {
            AttrValue::VisibleString(_) => AttrDataType::VisibleString,
            AttrValue::UnsignedInt(_) => AttrDataType::UnsignedInt,
            AttrValue::SignedInt(_) => AttrDataType::SignedInt,
            AttrValue::FloatingPoint(_) => AttrDataType::FloatingPoint,
            AttrValue::OctetString(_) => AttrDataType::OctetString,
            AttrValue::BitString(_) => AttrDataType::BitString,
            AttrValue::AttrList(_) => AttrDataType::AttrList,
        }
    }

    fn parse_visible_string(
        cursor: &mut ReadCursor<'a>,
        len: u8,
    ) -> Result<&'a str, AttrParseError> {
        let data = cursor.read_bytes(len as usize)?;
        let value = std::str::from_utf8(data)?;
        Ok(value)
    }

    fn parse_unsigned_int(cursor: &mut ReadCursor<'a>, len: u8) -> Result<u32, AttrParseError> {
        match len {
            1 => Ok(cursor.read_u8()? as u32),
            2 => Ok(cursor.read_u16_le()? as u32),
            4 => Ok(cursor.read_u32_le()?),
            _ => Err(AttrParseError::BadIntegerLength(len)),
        }
    }

    fn parse_signed_int(cursor: &mut ReadCursor<'a>, len: u8) -> Result<i32, AttrParseError> {
        match len {
            1 => Ok(cursor.read_u8()? as i32),
            2 => Ok(cursor.read_i16_le()? as i32),
            4 => Ok(cursor.read_i32_le()?),
            _ => Err(AttrParseError::BadIntegerLength(len)),
        }
    }

    fn parse_floating_point(
        cursor: &mut ReadCursor<'a>,
        len: u8,
    ) -> Result<FloatType, AttrParseError> {
        match len {
            4 => Ok(FloatType::F32(cursor.read_f32_le()?)),
            8 => Ok(FloatType::F64(cursor.read_f64_le()?)),
            _ => Err(AttrParseError::BadFloatLength(len)),
        }
    }

    fn parse_attr_list(
        cursor: &mut ReadCursor<'a>,
        len: u16,
    ) -> Result<AttrList<'a>, AttrParseError> {
        if len % 2 != 0 {
            return Err(AttrParseError::BadAttrListLength(len));
        }

        let data = cursor.read_bytes(len as usize)?;

        Ok(AttrList { data })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parses_visible_string() {
        let vis_str: &[u8] = &[VISIBLE_STRING, 0x05, b'H', b'E', b'L', b'L', b'O'];
        let mut cursor = ReadCursor::new(vis_str);
        let attr = AttrValue::parse(&mut cursor).unwrap();
        assert_eq!(attr, AttrValue::VisibleString("HELLO"));
        assert!(cursor.is_empty());
    }

    #[test]
    fn parses_one_byte_unsigned_int() {
        let mut cursor = ReadCursor::new(&[UNSIGNED_INT, 0x01, 42]);
        assert_eq!(
            AttrValue::parse(&mut cursor).unwrap(),
            AttrValue::UnsignedInt(42)
        );
        assert!(cursor.is_empty());
    }

    #[test]
    fn parses_two_byte_unsigned_int() {
        let mut cursor = ReadCursor::new(&[UNSIGNED_INT, 0x02, 42, 00]);
        assert_eq!(
            AttrValue::parse(&mut cursor).unwrap(),
            AttrValue::UnsignedInt(42)
        );
        assert!(cursor.is_empty());
    }

    #[test]
    fn parses_four_byte_unsigned_int() {
        let mut cursor = ReadCursor::new(&[UNSIGNED_INT, 0x04, 42, 00, 00, 00]);
        assert_eq!(
            AttrValue::parse(&mut cursor).unwrap(),
            AttrValue::UnsignedInt(42)
        );
        assert!(cursor.is_empty());
    }

    #[test]
    fn rejects_three_byte_unsigned_int() {
        let mut cursor = ReadCursor::new(&[UNSIGNED_INT, 0x03, 42, 00, 00]);
        assert_eq!(
            AttrValue::parse(&mut cursor),
            Err(AttrParseError::BadIntegerLength(3))
        );
    }

    #[test]
    fn parses_attr_list() {
        let mut cursor = ReadCursor::new(&[ATTR_LIST, 0x06, 20, 00, 21, 01, 22, 02]);
        let parsed_list: Vec<AttrItem> = match AttrValue::parse(&mut cursor).unwrap() {
            AttrValue::AttrList(x) => x.collect(),
            _ => unreachable!(),
        };
        assert_eq!(
            &parsed_list,
            &[
                AttrItem {
                    variation: 20,
                    properties: AttrProp { is_writable: false },
                },
                AttrItem {
                    variation: 21,
                    properties: AttrProp { is_writable: true },
                },
                AttrItem {
                    variation: 22,
                    properties: AttrProp { is_writable: false },
                },
            ]
        );
    }

    #[test]
    fn parses_f32() {
        let bytes = [0x01, 0x02, 0x03, 0x04];
        let expected = f32::from_le_bytes(bytes);
        let input = &[FLOATING_POINT, 0x04, 0x01, 0x02, 0x03, 0x04];
        let mut cursor = ReadCursor::new(input.as_slice());
        assert_eq!(
            AttrValue::parse(&mut cursor),
            Ok(AttrValue::FloatingPoint(FloatType::F32(expected)))
        );
    }

    #[test]
    fn parses_f64() {
        let input = &[
            FLOATING_POINT,
            0x08,
            0x01,
            0x02,
            0x03,
            0x04,
            0x05,
            0x06,
            0x07,
            0x08,
        ];
        let expected = f64::from_le_bytes([0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
        let mut cursor = ReadCursor::new(input.as_slice());
        assert_eq!(
            AttrValue::parse(&mut cursor),
            Ok(AttrValue::FloatingPoint(FloatType::F64(expected)))
        );
    }

    #[test]
    fn rejects_bad_float_length() {
        let input = &[FLOATING_POINT, 0x07, 0x01, 0x02];
        let mut cursor = ReadCursor::new(input.as_slice());
        assert_eq!(
            AttrValue::parse(&mut cursor),
            Err(AttrParseError::BadFloatLength(7))
        );
    }
}
