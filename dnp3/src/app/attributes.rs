use crate::app::parse::range::Range;
use crate::app::parse::traits::{FixedSize, Index};
use crate::app::{ObjectParseError, Timestamp};
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
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum AnyAttribute<'a> {
    /// Either an attribute from a private set or an unknown attribute in the default set
    Other(Attribute<'a>),
    /// An attribute defined in the default set
    Known(KnownAttribute<'a>),
}

/// Enumeration of all the known string attributes
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum StringAttr {
    /// Variation 252 - Device manufacturer's name
    DeviceManufacturersName,
    /// Variation 250 - Device manufacturer's product name and model
    ProductNameAndModel,
    /// Variation 249 - DNP3 subset and conformance
    SubsetAndConformance,
    /// Variation 248 - DNP3 subset and conformance
    DeviceSerialNumber,
    /// Variation 247 - User assigned device name
    UserAssignedDeviceName,
    /// Variation 246 - User assigned ID code/number
    UserAssignedId,
    /// Variation 245 - User assigned location/name
    UserAssignedLocation,
    /// Variation 243 - Device manufacturer hardware version
    DeviceManufacturerHardwareVersion,
    /// Variation 242 - Device manufacturer software version
    DeviceManufacturerSoftwareVersion,
    /// Variation 211 - Identification of user-specific attributes
    UserSpecificAttributes,
}

/// Enumeration of all the known unsigned integer
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UIntAttr {
    /// Variation 241 - Maximum receive fragment size
    MaximumReceiveFragmentSize,
    /// Variation 240 - Maximum transmit fragment size
    MaximumTransmitFragmentSize,
    /// Variation 239 - Number of binary input points
    NumBinaryInput,
    /// Variation 238 - Maximum binary input point index
    MaxBinaryInputIndex,
    /// Variation 236 - Number of double-bit binary input points
    NumDoubleBitBinaryInput,
    /// Variation 235 - Maximum double-bit binary input point index
    MaxDoubleBitBinaryInputIndex,
    /// Variation 233 - Number of analog input points
    NumAnalogInput,
    /// Variation 232 - Maximum analog input point index
    MaxAnalogInputIndex,
    /// Variation 229 - Number of counter points
    NumCounter,
    /// Variation 228 - Maximum counter point index
    MaxCounterIndex,
    /// Variation 224 - Number of binary outputs
    NumBinaryOutputs,
    /// Variation 223 - Maximum binary output index
    MaxBinaryOutputIndex,
    /// Variation 221 - Number of analog outputs
    NumAnalogOutputs,
    /// Variation 220 - Maximum analog output index
    MaxAnalogOutputIndex,
    /// Variation 218 - Duration of time accuracy (seconds)
    DurationOfTimeAccuracy,
    /// Variation 217 - Local timing accuracy (microseconds)
    LocalTimingAccuracy,
    /// Variation 216 - Maximum number of binary outputs per request
    MaxBinaryOutputPerRequest,
    /// Variation 215 - Number of outstation defined data-sets
    NumOutstationDefinedDataSets,
    /// Variation 214 - Number of master defined data-sets
    NumMasterDefinedDataSets,
    /// Variation 213 - Number of outstation defined data-set prototypes
    NumOutstationDefinedDataSetProto,
    /// Variation 212 - Number of master defined data-set prototypes
    NumMasterDefinedDataSetProto,
    /// Number of security statistics per association
    NumSecurityStatsPerAssoc,
    /// Variation 209 - Secure authentication version
    SecureAuthenticationVersion,
}

/// Enumeration of all the known boolean attributes
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BoolAttr {
    /// Variation 237 - Support binary input events
    SupportsBinaryInputEvents,
    /// Variation 234 - Support double-bit binary input events
    SupportsDoubleBitBinaryInputEvents,
    /// Variation 231 - Support analog input events
    SupportsAnalogInputEvents,
    /// Variation 230 - Support frozen analog input events
    SupportsFrozenAnalogInputs,
    /// Variation 227 - Support counter events
    SupportsCounterEvents,
    /// Variation 226 - Support frozen counters
    SupportsFrozenCounters,
    /// Variation 225 - Support frozen counter events
    SupportsFrozenCounterEvents,
    /// Variation 222 - Supports binary output events
    SupportsBinaryOutputEvents,
    /// Variation 219 - Supports analog output events
    SupportsAnalogOutputEvents,
}

/// Variants for all the pre-defined attributes in the standard
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum KnownAttribute<'a> {
    /// Variation 255 - List of attribute variations
    AttributeList(AttrList<'a>),
    /// VStr attributes
    String(StringAttr, &'a str),
    /// UInt attributes
    UInt(UIntAttr, u32),
    /// Bool attributes,
    Bool(BoolAttr, bool),
}

impl<'a> AnyAttribute<'a> {
    pub(crate) fn try_from(attr: &Attribute<'a>) -> Result<Self, TypeError> {
        if let AttrSet::Private(_) = attr.set {
            return Ok(AnyAttribute::Other(*attr));
        }

        let known = match attr.variation {
            255 => KnownAttribute::AttributeList(attr.value.expect_attr_list()?),
            252 => KnownAttribute::String(
                StringAttr::DeviceManufacturersName,
                attr.value.expect_vstr()?,
            ),
            250 => {
                KnownAttribute::String(StringAttr::ProductNameAndModel, attr.value.expect_vstr()?)
            }
            249 => {
                KnownAttribute::String(StringAttr::SubsetAndConformance, attr.value.expect_vstr()?)
            }
            248 => {
                KnownAttribute::String(StringAttr::DeviceSerialNumber, attr.value.expect_vstr()?)
            }
            247 => KnownAttribute::String(
                StringAttr::UserAssignedDeviceName,
                attr.value.expect_vstr()?,
            ),
            246 => KnownAttribute::String(StringAttr::UserAssignedId, attr.value.expect_vstr()?),
            245 => {
                KnownAttribute::String(StringAttr::UserAssignedLocation, attr.value.expect_vstr()?)
            }
            243 => KnownAttribute::String(
                StringAttr::DeviceManufacturerHardwareVersion,
                attr.value.expect_vstr()?,
            ),
            242 => KnownAttribute::String(
                StringAttr::DeviceManufacturerSoftwareVersion,
                attr.value.expect_vstr()?,
            ),
            241 => KnownAttribute::UInt(
                UIntAttr::MaximumReceiveFragmentSize,
                attr.value.expect_uint()?,
            ),
            240 => KnownAttribute::UInt(
                UIntAttr::MaximumTransmitFragmentSize,
                attr.value.expect_uint()?,
            ),
            239 => KnownAttribute::UInt(UIntAttr::NumBinaryInput, attr.value.expect_uint()?),
            238 => KnownAttribute::UInt(UIntAttr::MaxBinaryInputIndex, attr.value.expect_uint()?),
            237 => KnownAttribute::Bool(
                BoolAttr::SupportsBinaryInputEvents,
                attr.value.expect_bool()?,
            ),
            236 => {
                KnownAttribute::UInt(UIntAttr::NumDoubleBitBinaryInput, attr.value.expect_uint()?)
            }
            235 => KnownAttribute::UInt(
                UIntAttr::MaxDoubleBitBinaryInputIndex,
                attr.value.expect_uint()?,
            ),
            234 => KnownAttribute::Bool(
                BoolAttr::SupportsDoubleBitBinaryInputEvents,
                attr.value.expect_bool()?,
            ),
            233 => KnownAttribute::UInt(UIntAttr::NumAnalogInput, attr.value.expect_uint()?),
            232 => KnownAttribute::UInt(UIntAttr::MaxAnalogInputIndex, attr.value.expect_uint()?),
            231 => KnownAttribute::Bool(
                BoolAttr::SupportsAnalogInputEvents,
                attr.value.expect_bool()?,
            ),
            230 => KnownAttribute::Bool(
                BoolAttr::SupportsFrozenAnalogInputs,
                attr.value.expect_bool()?,
            ),
            229 => KnownAttribute::UInt(UIntAttr::NumCounter, attr.value.expect_uint()?),
            228 => KnownAttribute::UInt(UIntAttr::MaxCounterIndex, attr.value.expect_uint()?),
            227 => KnownAttribute::Bool(BoolAttr::SupportsCounterEvents, attr.value.expect_bool()?),
            226 => {
                KnownAttribute::Bool(BoolAttr::SupportsFrozenCounters, attr.value.expect_bool()?)
            }
            225 => KnownAttribute::Bool(
                BoolAttr::SupportsFrozenCounterEvents,
                attr.value.expect_bool()?,
            ),
            224 => KnownAttribute::UInt(UIntAttr::NumBinaryOutputs, attr.value.expect_uint()?),
            223 => KnownAttribute::UInt(UIntAttr::MaxBinaryInputIndex, attr.value.expect_uint()?),
            222 => KnownAttribute::Bool(
                BoolAttr::SupportsBinaryOutputEvents,
                attr.value.expect_bool()?,
            ),
            221 => KnownAttribute::UInt(UIntAttr::NumAnalogOutputs, attr.value.expect_uint()?),
            220 => KnownAttribute::UInt(UIntAttr::MaxAnalogOutputIndex, attr.value.expect_uint()?),
            219 => KnownAttribute::Bool(
                BoolAttr::SupportsAnalogOutputEvents,
                attr.value.expect_bool()?,
            ),
            218 => {
                KnownAttribute::UInt(UIntAttr::DurationOfTimeAccuracy, attr.value.expect_uint()?)
            }
            217 => KnownAttribute::UInt(UIntAttr::LocalTimingAccuracy, attr.value.expect_uint()?),
            216 => KnownAttribute::UInt(
                UIntAttr::MaxBinaryOutputPerRequest,
                attr.value.expect_uint()?,
            ),
            215 => KnownAttribute::UInt(
                UIntAttr::NumOutstationDefinedDataSets,
                attr.value.expect_uint()?,
            ),
            214 => KnownAttribute::UInt(
                UIntAttr::NumMasterDefinedDataSets,
                attr.value.expect_uint()?,
            ),
            213 => KnownAttribute::UInt(
                UIntAttr::NumOutstationDefinedDataSetProto,
                attr.value.expect_uint()?,
            ),
            212 => KnownAttribute::UInt(
                UIntAttr::NumMasterDefinedDataSetProto,
                attr.value.expect_uint()?,
            ),
            211 => KnownAttribute::String(
                StringAttr::UserSpecificAttributes,
                attr.value.expect_vstr()?,
            ),
            209 => KnownAttribute::UInt(
                UIntAttr::SecureAuthenticationVersion,
                attr.value.expect_uint()?,
            ),

            _ => return Ok(AnyAttribute::Other(*attr)),
        };

        Ok(AnyAttribute::Known(known))
    }
}
const VISIBLE_STRING: u8 = 1;
const UNSIGNED_INT: u8 = 2;
const SIGNED_INT: u8 = 3;
const FLOATING_POINT: u8 = 4;
const OCTET_STRING: u8 = 5;
const BIT_STRING: u8 = 6;
const DNP3_TIME: u8 = 7;
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
    /// DNP3 Time
    Timestamp,
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
            DNP3_TIME => Some(Self::Timestamp),
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

impl<'a> AttrList<'a> {
    /// Create an iterator of the list
    pub fn iter(&self) -> AttrIter<'a> {
        AttrIter { data: self.data }
    }
}

/// An iterator over an `AttrList`
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AttrIter<'a> {
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
}

impl<'a> Iterator for AttrIter<'a> {
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
    /// DNP3 Time
    Timestamp(Timestamp),
    /// BSTR - Bit string
    BitString(&'a [u8]),
    /// List of UINT8-BSTR8
    AttrList(AttrList<'a>),
}

/// Expected type X but received type Y
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct TypeError {
    pub(crate) expected: AttrDataType,
    pub(crate) actual: AttrDataType,
}

impl TypeError {
    fn new(expected: AttrDataType, actual: AttrDataType) -> Self {
        Self { expected, actual }
    }
}

impl<'a> AttrValue<'a> {
    pub(crate) fn write(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            AttrValue::VisibleString(x) => write!(f, "visible string: {x}"),
            AttrValue::UnsignedInt(x) => write!(f, "unsigned int: {x}"),
            AttrValue::SignedInt(x) => write!(f, "signed int: {x}"),
            AttrValue::FloatingPoint(x) => match x {
                FloatType::F32(x) => write!(f, "float32: {x}"),
                FloatType::F64(x) => write!(f, "float64: {x}"),
            },
            AttrValue::OctetString(x) => write!(f, "octet string len == {}", x.len()),
            AttrValue::BitString(x) => write!(f, "bit string len == {}", x.len()),
            AttrValue::Timestamp(x) => write!(f, "{}", x),
            AttrValue::AttrList(list) => {
                for x in list.iter() {
                    write!(
                        f,
                        "\n variation: {} writeable: {}",
                        x.variation, x.properties.is_writable
                    )?;
                }
                Ok(())
            }
        }
    }

    pub(crate) fn expect_vstr(&self) -> Result<&'a str, TypeError> {
        match self {
            AttrValue::VisibleString(x) => Ok(x),
            _ => Err(TypeError::new(AttrDataType::VisibleString, self.get_type())),
        }
    }

    pub(crate) fn expect_bool(&self) -> Result<bool, TypeError> {
        Ok(self.expect_int()? == 1)
    }

    pub(crate) fn expect_uint(&self) -> Result<u32, TypeError> {
        match self {
            AttrValue::UnsignedInt(x) => Ok(*x),
            _ => Err(TypeError::new(AttrDataType::UnsignedInt, self.get_type())),
        }
    }

    pub(crate) fn expect_int(&self) -> Result<i32, TypeError> {
        match self {
            AttrValue::SignedInt(x) => Ok(*x),
            _ => Err(TypeError::new(AttrDataType::SignedInt, self.get_type())),
        }
    }

    pub(crate) fn expect_attr_list(&self) -> Result<AttrList<'a>, TypeError> {
        match self {
            AttrValue::AttrList(x) => Ok(*x),
            _ => Err(TypeError::new(AttrDataType::AttrList, self.get_type())),
        }
    }
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

fn get_default_desc(var: u8) -> &'static str {
    match var {
        209 => "Secure authentication version",
        210 => "Number of security statistics per association",
        211 => "Identification of support for user-specific attributes",
        212 => "Number of master-defined data set prototypes",
        213 => "Number of outstation-defined data set prototypes",
        214 => "Number of master-defined data sets",
        215 => "Number of outstation-defined data sets",
        216 => "Maximum number of binary output objects per request",
        217 => "Local timing accuracy",
        218 => "Duration of time accuracy",
        219 => "Support for analog output events",
        220 => "Maximum analog output index",
        221 => "Number of analog outputs",
        222 => "Support for binary output events",
        223 => "Maximum binary output index",
        224 => "Number of binary outputs",
        225 => "Support for frozen counter events",
        226 => "Support for frozen counters",
        227 => "Support for counter events",
        228 => "Maximum counter index",
        229 => "Number of counter points",
        230 => "Support for frozen analog inputs",
        231 => "Support for analog input events",
        232 => "Maximum analog input index",
        233 => "Number of analog input points",
        234 => "Support for double-bit binary input events",
        235 => "Maximum double-bit binary input index",
        236 => "Number of double-bit binary input points",
        237 => "Support for binary input events",
        238 => "Maximum binary input index",
        239 => "Number of binary input points",
        240 => "Maximum transmit fragment size",
        241 => "Maximum receive fragment size",
        242 => "Device manufacturer's software version",
        243 => "Device manufacturer's hardware version",
        245 => "User-assigned location name",
        246 => "User-assigned ID code/number",
        247 => "User-assigned device name",
        248 => "Device serial number",
        249 => "DNP3 subset and conformance",
        250 => "Device manufacturer's product name and model",
        252 => "Device manufacturer's name",
        255 => "List of attribute variations",
        _ => "Unknown",
    }
}

impl<'a> Attribute<'a> {
    pub(crate) fn write(&self, f: &mut Formatter) -> std::fmt::Result {
        match self.set {
            AttrSet::Default => {
                // lookup description
                let desc = get_default_desc(self.variation);
                writeln!(f, "\nDefault set - variation {} - {desc}", self.variation)?;
            }
            AttrSet::Private(x) => {
                writeln!(f, "Private set ({x})")?;
            }
        }
        self.value.write(f)
    }

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
    /// Time length must be 6
    BadTimeLength(u8),
    /// Attribute lists must be even in length (2*N) where N is number of pairs
    BadAttrListLength(u16),
    /// Visible string is not UTF-8. The DNP3 standard doesn't really define what "visible" means
    /// but this handles ASCII and is more flexible for non-english users.
    BadVisibleString(Utf8Error),
    /// Set identifier is not u8
    SetIdNotU8(u16),
    /// Count != 1
    CountNotOne(usize),
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
            AttrParseError::BadTimeLength(x) => {
                write!(f, "Time attribute length {x} != 6")
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
            AttrDataType::Timestamp => {
                if len != 6 {
                    return Err(AttrParseError::BadTimeLength(len));
                }
                Self::Timestamp(Timestamp::new(cursor.read_u48_le()?))
            }
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
            AttrValue::Timestamp(_) => AttrDataType::Timestamp,
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
            AttrValue::AttrList(x) => x.iter().collect(),
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
