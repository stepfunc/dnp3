use crate::app::parse::range::Range;
use crate::app::parse::traits::{FixedSize, Index};
use crate::app::{ObjectParseError, Timestamp};
use crate::master::{BadEncoding, TaskError};
use scursor::{ReadCursor, ReadError, WriteCursor, WriteError};
use std::fmt::{Display, Formatter};
use std::str::Utf8Error;

/// Constants defining attribute variations from set 0
pub mod var {
    /// Variation 196
    pub const CONFIG_ID: u8 = 196;
    /// Variation 197
    pub const CONFIG_VERSION: u8 = 197;
    /// Variation 198
    pub const CONFIG_BUILD_DATE: u8 = 198;
    /// Variation 199
    pub const CONFIG_LAST_CHANGE_DATE: u8 = 199;
    /// Variation 200
    pub const CONFIG_DIGEST: u8 = 200;
    /// Variation 201
    pub const CONFIG_DIGEST_ALGORITHM: u8 = 201;
    /// Variation 202
    pub const MASTER_RESOURCE_ID: u8 = 202;
    /// Variation 203
    pub const DEVICE_LOCATION_ALTITUDE: u8 = 203;
    /// Variation 204
    pub const DEVICE_LOCATION_LONGITUDE: u8 = 204;
    /// Variation 205
    pub const DEVICE_LOCATION_LATITUDE: u8 = 205;
    /// Variation 206
    pub const USER_ASSIGNED_SECONDARY_OPERATOR_NAME: u8 = 206;
    /// Variation 207
    pub const USER_ASSIGNED_PRIMARY_OPERATOR_NAME: u8 = 207;
    /// Variation 208
    pub const USER_ASSIGNED_SYSTEM_NAME: u8 = 208;
    /// Variation 209
    pub const SECURE_AUTH_VERSION: u8 = 209;
    /// Variation 210
    pub const NUM_SECURITY_STAT_PER_ASSOC: u8 = 210;
    /// Variation 211
    pub const USER_SPECIFIC_ATTRIBUTES: u8 = 211;
    /// Variation 212
    pub const NUM_MASTER_DEFINED_DATA_SET_PROTO: u8 = 212;
    /// Variation 213
    pub const NUM_OUTSTATION_DEFINED_DATA_SET_PROTO: u8 = 213;
    /// Variation 214
    pub const NUM_MASTER_DEFINED_DATA_SETS: u8 = 214;
    /// Variation 215
    pub const NUM_OUTSTATION_DEFINED_DATA_SETS: u8 = 215;
    /// Variation 216
    pub const MAX_BINARY_OUTPUT_PER_REQUEST: u8 = 216;
    /// Variation 217
    pub const LOCAL_TIMING_ACCURACY: u8 = 217;
    /// Variation 218
    pub const DURATION_OF_TIME_ACCURACY: u8 = 218;
    /// Variation 219
    pub const SUPPORTS_ANALOG_OUTPUT_EVENTS: u8 = 219;
    /// Variation 220
    pub const MAX_ANALOG_OUTPUT_INDEX: u8 = 220;
    /// Variation 221
    pub const NUM_ANALOG_OUTPUT: u8 = 221;
    /// Variation 222
    pub const SUPPORTS_BINARY_OUTPUT_EVENTS: u8 = 222;
    /// Variation 223
    pub const MAX_BINARY_OUTPUT_INDEX: u8 = 223;
    /// Variation 224
    pub const NUM_BINARY_OUTPUT: u8 = 224;
    /// Variation 225
    pub const SUPPORTS_FROZEN_COUNTER_EVENTS: u8 = 225;
    /// Variation 226
    pub const SUPPORTS_FROZEN_COUNTERS: u8 = 226;
    /// Variation 227
    pub const SUPPORTS_COUNTER_EVENTS: u8 = 227;
    /// Variation 228
    pub const MAX_COUNTER_INDEX: u8 = 228;
    /// Variation 229
    pub const NUM_COUNTER: u8 = 229;
    /// Variation 230
    pub const SUPPORTS_FROZEN_ANALOG_INPUTS: u8 = 230;
    /// Variation 231
    pub const SUPPORTS_ANALOG_INPUT_EVENTS: u8 = 231;
    /// Variation 232
    pub const MAX_ANALOG_INPUT_INDEX: u8 = 232;
    /// Variation 233
    pub const NUM_ANALOG_INPUT: u8 = 233;
    /// Variation 234
    pub const SUPPORTS_DOUBLE_BIT_BINARY_INPUT_EVENTS: u8 = 234;
    /// Variation 235
    pub const MAX_DOUBLE_BIT_BINARY_INPUT_INDEX: u8 = 235;
    /// Variation 236
    pub const NUM_DOUBLE_BIT_BINARY_INPUT: u8 = 236;
    /// Variation 237
    pub const SUPPORTS_BINARY_INPUT_EVENTS: u8 = 237;
    /// Variation 238
    pub const MAX_BINARY_INPUT_INDEX: u8 = 238;
    /// Variation 239
    pub const NUM_BINARY_INPUT: u8 = 239;
    /// Variation 240
    pub const MAX_TX_FRAGMENT_SIZE: u8 = 240;
    /// Variation 241
    pub const MAX_RX_FRAGMENT_SIZE: u8 = 241;
    /// Variation 242
    pub const DEVICE_MANUFACTURER_SOFTWARE_VERSION: u8 = 242;
    /// Variation 243
    pub const DEVICE_MANUFACTURER_HARDWARE_VERSION: u8 = 243;
    /// Variation 243
    pub const USER_ASSIGNED_OWNER_NAME: u8 = 244;
    /// Variation 245
    pub const USER_ASSIGNED_LOCATION: u8 = 245;
    /// Variation 246
    pub const USER_ASSIGNED_ID: u8 = 246;
    /// Variation 247
    pub const USER_ASSIGNED_DEVICE_NAME: u8 = 247;
    /// Variation 248
    pub const DEVICE_SERIAL_NUMBER: u8 = 248;
    /// Variation 249
    pub const DEVICE_SUBSET_AND_CONFORMANCE: u8 = 249;
    /// Variation 250
    pub const PRODUCT_NAME_AND_MODEL: u8 = 250;
    /// Variation 252
    pub const DEVICE_MANUFACTURER_NAME: u8 = 252;
    /// Variation 254
    pub const ALL_ATTRIBUTES_REQUEST: u8 = 254;
    /// Variation 255
    pub const LIST_OF_ATTRIBUTE_VARIATIONS: u8 = 255;
}

/// Set to which a device attribute belongs
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
    pub fn value(self) -> u8 {
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

/// Enum that represent default or private set attributes
#[derive(Clone, Debug, PartialEq)]
pub enum AnyAttribute<'a> {
    /// Either an attribute from a private set or an unknown attribute in the default set
    Other(Attribute<'a>),
    /// An attribute defined in the default set
    Known(KnownAttribute<'a>),
}

/// Unit value used to specify a g0v254 request
#[derive(Copy, Clone, Debug)]
pub struct AllAttributes;

impl From<AllAttributes> for u8 {
    fn from(_: AllAttributes) -> Self {
        254
    }
}

/// Enumeration of all the variation list attributes
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum VariationListAttr {
    /// Variation 255 - List of attribute variations
    ListOfVariations,
}

impl VariationListAttr {
    fn extract(self, value: AttrValue) -> Result<KnownAttribute, TypeError> {
        Ok(KnownAttribute::AttributeList(
            self,
            value.expect_attr_list()?,
        ))
    }

    /// The variation associated with this string attribute
    pub fn variation(self) -> u8 {
        match self {
            VariationListAttr::ListOfVariations => 255,
        }
    }
}

impl From<VariationListAttr> for u8 {
    fn from(value: VariationListAttr) -> Self {
        value.variation()
    }
}

/// Enumeration of all the known string attributes
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum StringAttr {
    /// Variation 196 - Configuration id
    ConfigId,
    /// Variation 197 - Configuration version
    ConfigVersion,
    /// Variation 201 - Configuration digest algorithm
    ConfigDigestAlgorithm,
    /// Variation 202 - Master resource id (mRID)
    MasterResourceId,
    /// Variation 206 - User-assigned secondary operator name
    UserAssignedSecondaryOperatorName,
    /// Variation 207 - User-assigned primary operator name
    UserAssignedPrimaryOperatorName,
    /// Variation 208 - User-assigned system name
    UserAssignedSystemName,
    /// Variation 211 - Identification of user-specific attributes
    UserSpecificAttributes,
    /// Variation 242 - Device manufacturer software version
    DeviceManufacturerSoftwareVersion,
    /// Variation 243 - Device manufacturer hardware version
    DeviceManufacturerHardwareVersion,
    /// Variation 244 - User-assigned owner name
    UserAssignedOwnerName,
    /// Variation 245 - User assigned location/name
    UserAssignedLocation,
    /// Variation 246 - User assigned ID code/number
    UserAssignedId,
    /// Variation 247 - User assigned device name
    UserAssignedDeviceName,
    /// Variation 248 - Device serial number
    DeviceSerialNumber,
    /// Variation 249 - DNP3 subset and conformance
    DeviceSubsetAndConformance,
    /// Variation 250 - Device manufacturer's product name and model
    ProductNameAndModel,
    /// Variation 252 - Device manufacturer's name
    DeviceManufacturersName,
}

impl StringAttr {
    fn extract(self, value: AttrValue) -> Result<KnownAttribute, TypeError> {
        Ok(KnownAttribute::String(self, value.expect_vstr()?))
    }

    /// The variation associated with this string attribute
    pub fn variation(self) -> u8 {
        match self {
            StringAttr::ConfigId => var::CONFIG_ID,
            StringAttr::ConfigVersion => var::CONFIG_VERSION,
            StringAttr::ConfigDigestAlgorithm => var::CONFIG_DIGEST_ALGORITHM,
            StringAttr::MasterResourceId => var::MASTER_RESOURCE_ID,
            StringAttr::UserAssignedSecondaryOperatorName => {
                var::USER_ASSIGNED_SECONDARY_OPERATOR_NAME
            }
            StringAttr::UserAssignedPrimaryOperatorName => var::USER_ASSIGNED_PRIMARY_OPERATOR_NAME,
            StringAttr::UserAssignedSystemName => var::USER_ASSIGNED_SYSTEM_NAME,
            StringAttr::UserSpecificAttributes => var::USER_SPECIFIC_ATTRIBUTES,
            StringAttr::DeviceManufacturerSoftwareVersion => {
                var::DEVICE_MANUFACTURER_SOFTWARE_VERSION
            }
            StringAttr::DeviceManufacturerHardwareVersion => {
                var::DEVICE_MANUFACTURER_HARDWARE_VERSION
            }
            StringAttr::UserAssignedOwnerName => var::USER_ASSIGNED_OWNER_NAME,
            StringAttr::UserAssignedLocation => var::USER_ASSIGNED_LOCATION,
            StringAttr::UserAssignedId => var::USER_ASSIGNED_ID,
            StringAttr::UserAssignedDeviceName => var::USER_ASSIGNED_DEVICE_NAME,
            StringAttr::DeviceSerialNumber => var::DEVICE_SERIAL_NUMBER,
            StringAttr::DeviceSubsetAndConformance => var::DEVICE_SUBSET_AND_CONFORMANCE,
            StringAttr::ProductNameAndModel => var::PRODUCT_NAME_AND_MODEL,
            StringAttr::DeviceManufacturersName => var::DEVICE_MANUFACTURER_NAME,
        }
    }

    /// Construct an ['OwnedAttribute'] given a value
    pub fn with_value<S: Into<String>>(self, value: S) -> OwnedAttribute {
        OwnedAttribute::new(
            AttrSet::Default,
            self.variation(),
            OwnedAttrValue::VisibleString(value.into()),
        )
    }
}

impl From<StringAttr> for u8 {
    fn from(value: StringAttr) -> Self {
        value.variation()
    }
}

/// Enumeration of all the known unsigned integer attributes
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UIntAttr {
    /// Variation 209 - Secure authentication version
    SecureAuthVersion,
    /// Variation 210 - Number of security statistics per association
    NumSecurityStatsPerAssoc,
    /// Variation 212 - Number of master defined data-set prototypes
    NumMasterDefinedDataSetProto,
    /// Variation 213 - Number of outstation defined data-set prototypes
    NumOutstationDefinedDataSetProto,
    /// Variation 214 - Number of master defined data-sets
    NumMasterDefinedDataSets,
    /// Variation 215 - Number of outstation defined data-sets
    NumOutstationDefinedDataSets,
    /// Variation 216 - Maximum number of binary outputs per request
    MaxBinaryOutputPerRequest,
    /// Variation 217 - Local timing accuracy (microseconds)
    LocalTimingAccuracy,
    /// Variation 218 - Duration of time accuracy (seconds)
    DurationOfTimeAccuracy,
    /// Variation 220 - Maximum analog output index
    MaxAnalogOutputIndex,
    /// Variation 221 - Number of analog outputs
    NumAnalogOutputs,
    /// Variation 223 - Maximum binary output index
    MaxBinaryOutputIndex,
    /// Variation 224 - Number of binary outputs
    NumBinaryOutputs,
    /// Variation 228 - Maximum counter point index
    MaxCounterIndex,
    /// Variation 229 - Number of counter points
    NumCounter,
    /// Variation 232 - Maximum analog input point index
    MaxAnalogInputIndex,
    /// Variation 233 - Number of analog input points
    NumAnalogInput,
    /// Variation 235 - Maximum double-bit binary input point index
    MaxDoubleBitBinaryInputIndex,
    /// Variation 236 - Number of double-bit binary input points
    NumDoubleBitBinaryInput,
    /// Variation 238 - Maximum binary input point index
    MaxBinaryInputIndex,
    /// Variation 239 - Number of binary input points
    NumBinaryInput,
    /// Variation 240 - Maximum transmit fragment size
    MaxTxFragmentSize,
    /// Variation 241 - Maximum receive fragment size
    MaxRxFragmentSize,
}

impl UIntAttr {
    fn extract(self, value: AttrValue) -> Result<KnownAttribute, TypeError> {
        Ok(KnownAttribute::UInt(self, value.expect_uint()?))
    }

    /// The variation associated with this string attribute
    pub fn variation(self) -> u8 {
        match self {
            UIntAttr::SecureAuthVersion => var::SECURE_AUTH_VERSION,
            UIntAttr::NumSecurityStatsPerAssoc => var::NUM_SECURITY_STAT_PER_ASSOC,
            UIntAttr::NumMasterDefinedDataSetProto => var::NUM_MASTER_DEFINED_DATA_SET_PROTO,
            UIntAttr::NumOutstationDefinedDataSetProto => {
                var::NUM_OUTSTATION_DEFINED_DATA_SET_PROTO
            }
            UIntAttr::NumMasterDefinedDataSets => var::NUM_MASTER_DEFINED_DATA_SETS,
            UIntAttr::NumOutstationDefinedDataSets => var::NUM_OUTSTATION_DEFINED_DATA_SETS,
            UIntAttr::MaxBinaryOutputPerRequest => var::MAX_BINARY_OUTPUT_PER_REQUEST,
            UIntAttr::LocalTimingAccuracy => var::LOCAL_TIMING_ACCURACY,
            UIntAttr::DurationOfTimeAccuracy => var::DURATION_OF_TIME_ACCURACY,
            UIntAttr::MaxAnalogOutputIndex => var::MAX_ANALOG_OUTPUT_INDEX,
            UIntAttr::NumAnalogOutputs => var::NUM_ANALOG_OUTPUT,
            UIntAttr::MaxBinaryOutputIndex => var::MAX_BINARY_OUTPUT_INDEX,
            UIntAttr::NumBinaryOutputs => var::NUM_BINARY_OUTPUT,
            UIntAttr::MaxCounterIndex => var::MAX_COUNTER_INDEX,
            UIntAttr::NumCounter => var::NUM_COUNTER,
            UIntAttr::MaxAnalogInputIndex => var::MAX_ANALOG_INPUT_INDEX,
            UIntAttr::NumAnalogInput => var::NUM_ANALOG_INPUT,
            UIntAttr::MaxDoubleBitBinaryInputIndex => var::MAX_DOUBLE_BIT_BINARY_INPUT_INDEX,
            UIntAttr::NumDoubleBitBinaryInput => var::NUM_DOUBLE_BIT_BINARY_INPUT,
            UIntAttr::MaxBinaryInputIndex => var::MAX_BINARY_INPUT_INDEX,
            UIntAttr::NumBinaryInput => var::NUM_BINARY_INPUT,
            UIntAttr::MaxTxFragmentSize => var::MAX_TX_FRAGMENT_SIZE,
            UIntAttr::MaxRxFragmentSize => var::MAX_RX_FRAGMENT_SIZE,
        }
    }

    /// Construct an ['OwnedAttribute'] given a value
    pub fn with_value(self, value: u32) -> OwnedAttribute {
        OwnedAttribute::new(
            AttrSet::Default,
            self.variation(),
            OwnedAttrValue::UnsignedInt(value),
        )
    }
}

impl From<UIntAttr> for u8 {
    fn from(value: UIntAttr) -> Self {
        value.variation()
    }
}

/// Enumeration of all the known boolean attributes
///
/// Boolean attributes are actually just encoded as signed integer attributes where 1 == true
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BoolAttr {
    /// Variation 219 - Supports analog output events
    SupportsAnalogOutputEvents,
    /// Variation 222 - Supports binary output events
    SupportsBinaryOutputEvents,
    /// Variation 225 - Support frozen counter events
    SupportsFrozenCounterEvents,
    /// Variation 226 - Support frozen counters
    SupportsFrozenCounters,
    /// Variation 227 - Support counter events
    SupportsCounterEvents,
    /// Variation 230 - Support frozen analog input events
    SupportsFrozenAnalogInputs,
    /// Variation 231 - Support analog input events
    SupportsAnalogInputEvents,
    /// Variation 234 - Support double-bit binary input events
    SupportsDoubleBitBinaryInputEvents,
    /// Variation 237 - Support binary input events
    SupportsBinaryInputEvents,
}

impl BoolAttr {
    fn extract(self, value: AttrValue) -> Result<KnownAttribute, TypeError> {
        Ok(KnownAttribute::Bool(self, value.expect_bool()?))
    }

    /// The variation associated with this string attribute
    pub fn variation(self) -> u8 {
        match self {
            BoolAttr::SupportsAnalogOutputEvents => var::SUPPORTS_ANALOG_OUTPUT_EVENTS,
            BoolAttr::SupportsBinaryOutputEvents => var::SUPPORTS_BINARY_OUTPUT_EVENTS,
            BoolAttr::SupportsFrozenCounterEvents => var::SUPPORTS_FROZEN_COUNTER_EVENTS,
            BoolAttr::SupportsFrozenCounters => var::SUPPORTS_FROZEN_COUNTERS,
            BoolAttr::SupportsCounterEvents => var::SUPPORTS_COUNTER_EVENTS,
            BoolAttr::SupportsFrozenAnalogInputs => var::SUPPORTS_FROZEN_ANALOG_INPUTS,
            BoolAttr::SupportsAnalogInputEvents => var::SUPPORTS_ANALOG_INPUT_EVENTS,
            BoolAttr::SupportsDoubleBitBinaryInputEvents => {
                var::SUPPORTS_DOUBLE_BIT_BINARY_INPUT_EVENTS
            }
            BoolAttr::SupportsBinaryInputEvents => var::SUPPORTS_BINARY_INPUT_EVENTS,
        }
    }

    /// Construct an ['OwnedAttribute'] given a value
    pub fn with_value(self, value: bool) -> OwnedAttribute {
        OwnedAttribute::new(
            AttrSet::Default,
            self.variation(),
            OwnedAttrValue::SignedInt(i32::from(value)),
        )
    }
}

impl From<BoolAttr> for u8 {
    fn from(value: BoolAttr) -> Self {
        value.variation()
    }
}

/// Enumeration of all the known DNP3 Time attributes
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TimeAttr {
    /// Variation 198 - Time and date that the outstation's current configuration was built defined
    ConfigBuildDate,
    /// Variation 199 - Time and date that the outstation's configuration was last modified
    ConfigLastChangeDate,
}

impl TimeAttr {
    fn extract(self, value: AttrValue) -> Result<KnownAttribute, TypeError> {
        Ok(KnownAttribute::DNP3Time(self, value.expect_time()?))
    }

    /// The variation associated with this string attribute
    pub fn variation(self) -> u8 {
        match self {
            TimeAttr::ConfigBuildDate => var::CONFIG_BUILD_DATE,
            TimeAttr::ConfigLastChangeDate => var::CONFIG_LAST_CHANGE_DATE,
        }
    }

    /// Construct an ['OwnedAttribute'] given a value
    pub fn with_value(self, value: Timestamp) -> OwnedAttribute {
        OwnedAttribute::new(
            AttrSet::Default,
            self.variation(),
            OwnedAttrValue::Dnp3Time(value),
        )
    }
}

impl From<TimeAttr> for u8 {
    fn from(value: TimeAttr) -> Self {
        value.variation()
    }
}

/// Enumeration of all known octet-string attributes
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum OctetStringAttr {
    /// Variation 201 - Digest (aka fingerprint) of the configuration using a CRC, HASH, MAC, or public key signature
    ConfigDigest,
}

impl OctetStringAttr {
    fn extract(self, value: AttrValue) -> Result<KnownAttribute, TypeError> {
        Ok(KnownAttribute::OctetString(
            self,
            value.expect_octet_string()?,
        ))
    }

    /// The variation associated with this string attribute
    pub fn variation(self) -> u8 {
        match self {
            OctetStringAttr::ConfigDigest => var::CONFIG_DIGEST,
        }
    }

    /// Construct an ['OwnedAttribute'] given a value
    pub fn with_value(self, value: Vec<u8>) -> OwnedAttribute {
        OwnedAttribute::new(
            AttrSet::Default,
            self.variation(),
            OwnedAttrValue::OctetString(value),
        )
    }
}

impl From<OctetStringAttr> for u8 {
    fn from(value: OctetStringAttr) -> Self {
        value.variation()
    }
}

/// Enumeration of all known float attributes
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FloatAttr {
    /// Variation 203 - Altitude of the device in meters
    DeviceLocationAltitude,
    /// Variation 204 - Longitude of the device from reference meridian (-180.0 to 180.0 deg)
    DeviceLocationLongitude,
    /// Variation 205 - Latitude of the device from the equator (90.0 to -90.0 deg)
    DeviceLocationLatitude,
}

impl FloatAttr {
    fn extract(self, value: AttrValue) -> Result<KnownAttribute, TypeError> {
        Ok(KnownAttribute::Float(self, value.expect_float()?))
    }

    /// The variation associated with this string attribute
    pub fn variation(self) -> u8 {
        match self {
            FloatAttr::DeviceLocationAltitude => var::DEVICE_LOCATION_ALTITUDE,
            FloatAttr::DeviceLocationLongitude => var::DEVICE_LOCATION_LONGITUDE,
            FloatAttr::DeviceLocationLatitude => var::DEVICE_LOCATION_LATITUDE,
        }
    }

    /// Construct an ['OwnedAttribute'] given a value
    pub fn with_value(self, value: FloatType) -> OwnedAttribute {
        OwnedAttribute::new(
            AttrSet::Default,
            self.variation(),
            OwnedAttrValue::FloatingPoint(value),
        )
    }
}

impl From<FloatAttr> for u8 {
    fn from(value: FloatAttr) -> Self {
        value.variation()
    }
}

/// An enumeration that represents all of attributes defined in the default set (0)
///
/// Each type has its own enum which represents only the valid variants for that type,
/// e.g. see [StringAttr] or [UIntAttr].
#[derive(Clone, Debug, PartialEq)]
pub enum KnownAttribute<'a> {
    /// List of attribute variations
    AttributeList(VariationListAttr, VariationList<'a>),
    /// VStr attributes
    String(StringAttr, &'a str),
    /// Float attributes
    Float(FloatAttr, FloatType),
    /// UInt attributes
    UInt(UIntAttr, u32),
    /// Bool attributes
    Bool(BoolAttr, bool),
    /// Octet-string attributes
    OctetString(OctetStringAttr, &'a [u8]),
    /// DNP3Time attributes
    DNP3Time(TimeAttr, Timestamp),
}

impl<'a> AnyAttribute<'a> {
    /// Try to construct from a raw attribute
    pub fn try_from(attr: &Attribute<'a>) -> Result<Self, TypeError> {
        if let AttrSet::Private(_) = attr.set {
            return Ok(AnyAttribute::Other(*attr));
        }

        let known = match attr.variation {
            var::CONFIG_ID => StringAttr::ConfigId.extract(attr.value)?,
            var::CONFIG_VERSION => StringAttr::ConfigVersion.extract(attr.value)?,
            var::CONFIG_BUILD_DATE => TimeAttr::ConfigBuildDate.extract(attr.value)?,
            var::CONFIG_LAST_CHANGE_DATE => TimeAttr::ConfigLastChangeDate.extract(attr.value)?,
            var::CONFIG_DIGEST => OctetStringAttr::ConfigDigest.extract(attr.value)?,
            var::CONFIG_DIGEST_ALGORITHM => {
                StringAttr::ConfigDigestAlgorithm.extract(attr.value)?
            }
            var::MASTER_RESOURCE_ID => StringAttr::MasterResourceId.extract(attr.value)?,
            var::DEVICE_LOCATION_ALTITUDE => {
                FloatAttr::DeviceLocationAltitude.extract(attr.value)?
            }
            var::DEVICE_LOCATION_LONGITUDE => {
                FloatAttr::DeviceLocationLongitude.extract(attr.value)?
            }
            var::DEVICE_LOCATION_LATITUDE => {
                FloatAttr::DeviceLocationLatitude.extract(attr.value)?
            }
            var::USER_ASSIGNED_SECONDARY_OPERATOR_NAME => {
                StringAttr::UserAssignedSecondaryOperatorName.extract(attr.value)?
            }
            var::USER_ASSIGNED_PRIMARY_OPERATOR_NAME => {
                StringAttr::UserAssignedPrimaryOperatorName.extract(attr.value)?
            }
            var::USER_ASSIGNED_SYSTEM_NAME => {
                StringAttr::UserAssignedSystemName.extract(attr.value)?
            }
            var::SECURE_AUTH_VERSION => UIntAttr::SecureAuthVersion.extract(attr.value)?,
            var::NUM_SECURITY_STAT_PER_ASSOC => {
                UIntAttr::NumSecurityStatsPerAssoc.extract(attr.value)?
            }
            var::USER_SPECIFIC_ATTRIBUTES => {
                StringAttr::UserSpecificAttributes.extract(attr.value)?
            }
            var::NUM_MASTER_DEFINED_DATA_SET_PROTO => {
                UIntAttr::NumMasterDefinedDataSetProto.extract(attr.value)?
            }
            var::NUM_OUTSTATION_DEFINED_DATA_SET_PROTO => {
                UIntAttr::NumOutstationDefinedDataSetProto.extract(attr.value)?
            }
            var::NUM_MASTER_DEFINED_DATA_SETS => {
                UIntAttr::NumMasterDefinedDataSets.extract(attr.value)?
            }
            var::NUM_OUTSTATION_DEFINED_DATA_SETS => {
                UIntAttr::NumOutstationDefinedDataSets.extract(attr.value)?
            }
            var::MAX_BINARY_OUTPUT_PER_REQUEST => {
                UIntAttr::MaxBinaryOutputPerRequest.extract(attr.value)?
            }
            var::LOCAL_TIMING_ACCURACY => UIntAttr::LocalTimingAccuracy.extract(attr.value)?,
            var::DURATION_OF_TIME_ACCURACY => {
                UIntAttr::DurationOfTimeAccuracy.extract(attr.value)?
            }
            var::SUPPORTS_ANALOG_OUTPUT_EVENTS => {
                BoolAttr::SupportsAnalogOutputEvents.extract(attr.value)?
            }
            var::MAX_ANALOG_OUTPUT_INDEX => UIntAttr::MaxAnalogOutputIndex.extract(attr.value)?,
            var::NUM_ANALOG_OUTPUT => UIntAttr::NumAnalogOutputs.extract(attr.value)?,
            var::SUPPORTS_BINARY_OUTPUT_EVENTS => {
                BoolAttr::SupportsBinaryOutputEvents.extract(attr.value)?
            }
            var::MAX_BINARY_OUTPUT_INDEX => UIntAttr::MaxBinaryOutputIndex.extract(attr.value)?,
            var::NUM_BINARY_OUTPUT => UIntAttr::NumBinaryOutputs.extract(attr.value)?,
            var::SUPPORTS_FROZEN_COUNTER_EVENTS => {
                BoolAttr::SupportsFrozenCounterEvents.extract(attr.value)?
            }
            var::SUPPORTS_FROZEN_COUNTERS => {
                BoolAttr::SupportsFrozenCounters.extract(attr.value)?
            }
            var::SUPPORTS_COUNTER_EVENTS => BoolAttr::SupportsCounterEvents.extract(attr.value)?,
            var::MAX_COUNTER_INDEX => UIntAttr::MaxCounterIndex.extract(attr.value)?,
            var::NUM_COUNTER => UIntAttr::NumCounter.extract(attr.value)?,
            var::SUPPORTS_FROZEN_ANALOG_INPUTS => {
                BoolAttr::SupportsFrozenAnalogInputs.extract(attr.value)?
            }
            var::SUPPORTS_ANALOG_INPUT_EVENTS => {
                BoolAttr::SupportsAnalogInputEvents.extract(attr.value)?
            }
            var::MAX_ANALOG_INPUT_INDEX => UIntAttr::MaxAnalogInputIndex.extract(attr.value)?,
            var::NUM_ANALOG_INPUT => UIntAttr::NumAnalogInput.extract(attr.value)?,
            var::SUPPORTS_DOUBLE_BIT_BINARY_INPUT_EVENTS => {
                BoolAttr::SupportsDoubleBitBinaryInputEvents.extract(attr.value)?
            }
            var::MAX_DOUBLE_BIT_BINARY_INPUT_INDEX => {
                UIntAttr::MaxDoubleBitBinaryInputIndex.extract(attr.value)?
            }
            var::NUM_DOUBLE_BIT_BINARY_INPUT => {
                UIntAttr::NumDoubleBitBinaryInput.extract(attr.value)?
            }
            var::SUPPORTS_BINARY_INPUT_EVENTS => {
                BoolAttr::SupportsBinaryInputEvents.extract(attr.value)?
            }
            var::MAX_BINARY_INPUT_INDEX => UIntAttr::MaxBinaryInputIndex.extract(attr.value)?,
            var::NUM_BINARY_INPUT => UIntAttr::NumBinaryInput.extract(attr.value)?,
            var::MAX_TX_FRAGMENT_SIZE => UIntAttr::MaxTxFragmentSize.extract(attr.value)?,
            var::MAX_RX_FRAGMENT_SIZE => UIntAttr::MaxRxFragmentSize.extract(attr.value)?,
            var::DEVICE_MANUFACTURER_SOFTWARE_VERSION => {
                StringAttr::DeviceManufacturerSoftwareVersion.extract(attr.value)?
            }
            var::DEVICE_MANUFACTURER_HARDWARE_VERSION => {
                StringAttr::DeviceManufacturerHardwareVersion.extract(attr.value)?
            }
            var::USER_ASSIGNED_OWNER_NAME => {
                StringAttr::UserAssignedOwnerName.extract(attr.value)?
            }
            var::USER_ASSIGNED_LOCATION => StringAttr::UserAssignedLocation.extract(attr.value)?,
            var::USER_ASSIGNED_ID => StringAttr::UserAssignedId.extract(attr.value)?,
            var::USER_ASSIGNED_DEVICE_NAME => {
                StringAttr::UserAssignedDeviceName.extract(attr.value)?
            }
            var::DEVICE_SERIAL_NUMBER => StringAttr::DeviceSerialNumber.extract(attr.value)?,
            var::DEVICE_SUBSET_AND_CONFORMANCE => {
                StringAttr::DeviceSubsetAndConformance.extract(attr.value)?
            }
            var::PRODUCT_NAME_AND_MODEL => StringAttr::ProductNameAndModel.extract(attr.value)?,
            var::DEVICE_MANUFACTURER_NAME => {
                StringAttr::DeviceManufacturersName.extract(attr.value)?
            }
            var::LIST_OF_ATTRIBUTE_VARIATIONS => {
                VariationListAttr::ListOfVariations.extract(attr.value)?
            }
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
pub(crate) enum AttrDataType {
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
    Dnp3Time,
    /// List of UINT8-BSTR8 pairs
    AttrList,
    /// Extended list of UINT8-BSTR8 pairs
    ExtAttrList,
}

impl From<AttrDataType> for u8 {
    fn from(value: AttrDataType) -> Self {
        match value {
            AttrDataType::VisibleString => VISIBLE_STRING,
            AttrDataType::UnsignedInt => UNSIGNED_INT,
            AttrDataType::SignedInt => SIGNED_INT,
            AttrDataType::FloatingPoint => FLOATING_POINT,
            AttrDataType::OctetString => OCTET_STRING,
            AttrDataType::BitString => BIT_STRING,
            AttrDataType::Dnp3Time => DNP3_TIME,
            AttrDataType::AttrList => ATTR_LIST,
            AttrDataType::ExtAttrList => EXT_ATTR_LIST,
        }
    }
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
            DNP3_TIME => Some(Self::Dnp3Time),
            ATTR_LIST => Some(Self::AttrList),
            EXT_ATTR_LIST => Some(Self::ExtAttrList),
            _ => None,
        }
    }
}

/// A list of attributes returned from the outstation. This type is
/// the payload of g0v255. It implements an iterator over [`AttrItem`] values.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct VariationList<'a> {
    data: &'a [u8],
}

impl<'a> VariationList<'a> {
    /// Create an iterator of the list
    pub fn iter(&self) -> VariationListIter<'a> {
        VariationListIter { data: self.data }
    }
}

/// An iterator over a [VariationList] that yields [AttrItem]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct VariationListIter<'a> {
    data: &'a [u8],
}

/// Single entry in a [VariationList]
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
    pub const fn writable() -> Self {
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

impl From<AttrProp> for u8 {
    fn from(value: AttrProp) -> Self {
        value.is_writable.into()
    }
}

impl<'a> Iterator for VariationListIter<'a> {
    type Item = AttrItem;

    fn next(&mut self) -> Option<Self::Item> {
        let variation = *self.data.first()?;
        let prop = *self.data.get(1)?;

        self.data = self.data.get(2..).unwrap_or_default();

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

impl FloatType {
    /// Extract the value, widening f32 to f64
    pub fn value(self) -> f64 {
        match self {
            FloatType::F32(x) => x as f64,
            FloatType::F64(x) => x,
        }
    }
}

/// Represents the value of a device attribute parsed from the underlying buffer
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
    Dnp3Time(Timestamp),
    /// BSTR - Bit string
    BitString(&'a [u8]),
    /// List of UINT8-BSTR8
    AttrList(VariationList<'a>),
}

impl<'a> AttrValue<'a> {
    pub(crate) fn data_type(&self) -> AttrDataType {
        match self {
            Self::VisibleString(_) => AttrDataType::VisibleString,
            Self::UnsignedInt(_) => AttrDataType::UnsignedInt,
            Self::SignedInt(_) => AttrDataType::SignedInt,
            Self::FloatingPoint(_) => AttrDataType::FloatingPoint,
            Self::OctetString(_) => AttrDataType::OctetString,
            Self::Dnp3Time(_) => AttrDataType::Dnp3Time,
            Self::BitString(_) => AttrDataType::BitString,
            Self::AttrList(_) => AttrDataType::AttrList,
        }
    }

    pub(crate) fn to_owned(self) -> Option<OwnedAttrValue> {
        let attr = match self {
            AttrValue::VisibleString(x) => OwnedAttrValue::VisibleString(x.to_string()),
            AttrValue::UnsignedInt(x) => OwnedAttrValue::UnsignedInt(x),
            AttrValue::SignedInt(x) => OwnedAttrValue::SignedInt(x),
            AttrValue::FloatingPoint(x) => OwnedAttrValue::FloatingPoint(x),
            AttrValue::OctetString(x) => OwnedAttrValue::OctetString(x.to_vec()),
            AttrValue::Dnp3Time(x) => OwnedAttrValue::Dnp3Time(x),
            AttrValue::BitString(x) => OwnedAttrValue::BitString(x.to_vec()),
            AttrValue::AttrList(_) => return None,
        };

        Some(attr)
    }
}

/// Represents the value of an attribute
///
/// This type owns all of its data unlike [AttrValue].
#[derive(Clone, Debug, PartialEq)]
pub enum OwnedAttrValue {
    /// VSTR - Visible character suitable for print and display
    VisibleString(String),
    /// UINT - Unsigned integer
    UnsignedInt(u32),
    /// Signed integer
    SignedInt(i32),
    /// Int - Signed integer
    FloatingPoint(FloatType),
    /// OSTR - Octet string
    OctetString(Vec<u8>),
    /// DNP3 Time
    Dnp3Time(Timestamp),
    /// BSTR - Bit string
    BitString(Vec<u8>),
}

impl OwnedAttrValue {
    pub(crate) fn data_type(&self) -> AttrDataType {
        match self {
            OwnedAttrValue::VisibleString(_) => AttrDataType::VisibleString,
            OwnedAttrValue::UnsignedInt(_) => AttrDataType::UnsignedInt,
            OwnedAttrValue::SignedInt(_) => AttrDataType::SignedInt,
            OwnedAttrValue::FloatingPoint(_) => AttrDataType::FloatingPoint,
            OwnedAttrValue::OctetString(_) => AttrDataType::OctetString,
            OwnedAttrValue::Dnp3Time(_) => AttrDataType::Dnp3Time,
            OwnedAttrValue::BitString(_) => AttrDataType::BitString,
        }
    }

    pub(crate) fn view(&self) -> AttrValue {
        match self {
            OwnedAttrValue::VisibleString(x) => AttrValue::VisibleString(x.as_str()),
            OwnedAttrValue::UnsignedInt(x) => AttrValue::UnsignedInt(*x),
            OwnedAttrValue::SignedInt(x) => AttrValue::SignedInt(*x),
            OwnedAttrValue::FloatingPoint(x) => AttrValue::FloatingPoint(*x),
            OwnedAttrValue::OctetString(x) => AttrValue::OctetString(x.as_slice()),
            OwnedAttrValue::Dnp3Time(x) => AttrValue::Dnp3Time(*x),
            OwnedAttrValue::BitString(x) => AttrValue::BitString(x.as_slice()),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
/// Ways in which attribute encoding can fail
pub enum BadAttribute {
    /// length of the string attribute exceeds what's encodable (max 255)
    BadLength(usize),
}

impl Display for BadAttribute {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BadAttribute::BadLength(x) => write!(
                f,
                "Data length {x} can not be encoded in a device attribute (max = 255)"
            ),
        }
    }
}

pub(crate) enum AttrWriteError {
    /// underlying cursor error
    Cursor,
    /// attribute value could not be encoded
    BadAttribute(BadAttribute),
}

impl From<BadAttribute> for AttrWriteError {
    fn from(value: BadAttribute) -> Self {
        AttrWriteError::BadAttribute(value)
    }
}

impl From<WriteError> for AttrWriteError {
    fn from(_: WriteError) -> Self {
        AttrWriteError::Cursor
    }
}

#[derive(Copy, Clone)]
enum UInt {
    U8(u8),
    U16(u16),
    U32(u32),
}

impl UInt {
    fn new(value: u32) -> Self {
        if value <= u8::MAX as u32 {
            Self::U8(value as u8)
        } else if value <= u16::MAX as u32 {
            Self::U16(value as u16)
        } else {
            Self::U32(value)
        }
    }

    fn len(self) -> u8 {
        match self {
            UInt::U8(_) => 1,
            UInt::U16(_) => 2,
            UInt::U32(_) => 4,
        }
    }

    fn write(self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        match self {
            UInt::U8(x) => cursor.write_u8(x),
            UInt::U16(x) => cursor.write_u16_le(x),
            UInt::U32(x) => cursor.write_u32_le(x),
        }
    }
}

#[derive(Copy, Clone)]
enum Int {
    I8(u8),
    I16(i16),
    I32(i32),
}

impl Int {
    const I8_RANGE: core::ops::Range<i32> = i8::MIN as i32..i8::MAX as i32;
    const I16_RANGE: core::ops::Range<i32> = i16::MIN as i32..i16::MAX as i32;

    fn new(value: i32) -> Self {
        if Self::I8_RANGE.contains(&value) {
            Self::I8(value as u8)
        } else if Self::I16_RANGE.contains(&value) {
            Self::I16(value as i16)
        } else {
            Self::I32(value)
        }
    }

    fn len(self) -> u8 {
        match self {
            Self::I8(_) => 1,
            Self::I16(_) => 2,
            Self::I32(_) => 4,
        }
    }

    fn write(self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        match self {
            Self::I8(x) => cursor.write_u8(x),
            Self::I16(x) => cursor.write_i16_le(x),
            Self::I32(x) => cursor.write_i32_le(x),
        }
    }
}

impl OwnedAttrValue {
    pub(crate) fn write(&self, cursor: &mut WriteCursor) -> Result<(), AttrWriteError> {
        match self {
            OwnedAttrValue::VisibleString(s) => {
                Self::write_bytes(cursor, VISIBLE_STRING, s.as_bytes())
            }
            OwnedAttrValue::UnsignedInt(x) => Self::write_uint(cursor, *x),
            OwnedAttrValue::SignedInt(x) => Self::write_int(cursor, *x),
            OwnedAttrValue::FloatingPoint(x) => Self::write_float(cursor, *x),
            OwnedAttrValue::OctetString(x) => Self::write_bytes(cursor, OCTET_STRING, x.as_slice()),
            OwnedAttrValue::Dnp3Time(x) => Self::write_time(cursor, *x),
            OwnedAttrValue::BitString(x) => Self::write_bytes(cursor, BIT_STRING, x.as_slice()),
        }
    }

    fn write_header(cursor: &mut WriteCursor, code: u8, len: u8) -> Result<(), AttrWriteError> {
        cursor.write_u8(code)?;
        cursor.write_u8(len)?;
        Ok(())
    }

    fn write_bytes(cursor: &mut WriteCursor, code: u8, bytes: &[u8]) -> Result<(), AttrWriteError> {
        let len: u8 = bytes
            .len()
            .try_into()
            .map_err(|_| BadAttribute::BadLength(bytes.len()))?;
        Self::write_header(cursor, code, len)?;
        cursor.write_bytes(bytes)?;
        Ok(())
    }

    fn write_uint(cursor: &mut WriteCursor, x: u32) -> Result<(), AttrWriteError> {
        let x = UInt::new(x);
        Self::write_header(cursor, UNSIGNED_INT, x.len())?;
        x.write(cursor)?;
        Ok(())
    }

    fn write_time(cursor: &mut WriteCursor, x: Timestamp) -> Result<(), AttrWriteError> {
        Self::write_header(cursor, DNP3_TIME, 6)?;
        cursor.write_u48_le(x.raw_value())?;
        Ok(())
    }

    fn write_int(cursor: &mut WriteCursor, x: i32) -> Result<(), AttrWriteError> {
        let x = Int::new(x);
        Self::write_header(cursor, SIGNED_INT, x.len())?;
        x.write(cursor)?;
        Ok(())
    }

    fn write_float(cursor: &mut WriteCursor, x: FloatType) -> Result<(), AttrWriteError> {
        match x {
            FloatType::F32(x) => {
                Self::write_header(cursor, FLOATING_POINT, 4)?;
                cursor.write_f32_le(x)?;
            }
            FloatType::F64(x) => {
                Self::write_header(cursor, FLOATING_POINT, 8)?;
                cursor.write_f64_le(x)?;
            }
        }
        Ok(())
    }
}

/// Expected type X but received type Y
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TypeError {
    pub(crate) expected: AttrDataType,
    pub(crate) actual: AttrDataType,
}

impl TypeError {
    pub(crate) fn new(expected: AttrDataType, actual: AttrDataType) -> Self {
        Self { expected, actual }
    }
}

impl<'a> AttrValue<'a> {
    pub(crate) fn format(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            AttrValue::VisibleString(x) => write!(f, "\nvisible string: {x}"),
            AttrValue::UnsignedInt(x) => write!(f, "\nunsigned int: {x}"),
            AttrValue::SignedInt(x) => write!(f, "\nsigned int: {x}"),
            AttrValue::FloatingPoint(x) => match x {
                FloatType::F32(x) => write!(f, "\nfloat32: {x}"),
                FloatType::F64(x) => write!(f, "\nfloat64: {x}"),
            },
            AttrValue::OctetString(x) => write!(f, "\noctet string len == {}", x.len()),
            AttrValue::BitString(x) => write!(f, "\nbit string len == {}", x.len()),
            AttrValue::Dnp3Time(x) => write!(f, "\n{x}"),
            AttrValue::AttrList(list) => {
                for x in list.iter() {
                    write!(
                        f,
                        "\nvariation: {} writeable: {}",
                        x.variation, x.properties.is_writable
                    )?;
                }
                Ok(())
            }
        }
    }

    pub(crate) fn expect_octet_string(&self) -> Result<&'a [u8], TypeError> {
        match self {
            AttrValue::OctetString(x) => Ok(x),
            _ => Err(TypeError::new(AttrDataType::OctetString, self.get_type())),
        }
    }

    pub(crate) fn expect_time(&self) -> Result<Timestamp, TypeError> {
        match self {
            AttrValue::Dnp3Time(t) => Ok(*t),
            _ => Err(TypeError::new(AttrDataType::Dnp3Time, self.get_type())),
        }
    }

    pub(crate) fn expect_float(&self) -> Result<FloatType, TypeError> {
        match self {
            AttrValue::FloatingPoint(x) => Ok(*x),
            _ => Err(TypeError::new(AttrDataType::FloatingPoint, self.get_type())),
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

    pub(crate) fn expect_attr_list(&self) -> Result<VariationList<'a>, TypeError> {
        match self {
            AttrValue::AttrList(x) => Ok(*x),
            _ => Err(TypeError::new(AttrDataType::AttrList, self.get_type())),
        }
    }
}

/// Attribute value and the set to which it belongs parsed the underlying buffer.
///
/// This type is zero-allocation and variable sized content is borrowed from the underlying buffer.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Attribute<'a> {
    /// Set to which the attribute belongs
    pub set: AttrSet,
    /// The variation of the attribute
    pub variation: u8,
    /// Value of the attribute borrowed from the underlying buffer
    pub value: AttrValue<'a>,
}

impl<'a> Attribute<'a> {
    pub(crate) fn to_owned(self) -> Option<OwnedAttribute> {
        let value = self.value.to_owned()?;
        Some(OwnedAttribute {
            set: self.set,
            variation: self.variation,
            value,
        })
    }
}

/// Attribute value and the set to which it belongs
///
/// This type owns all of its data unlike [Attribute].
#[derive(Clone, Debug, PartialEq)]
pub struct OwnedAttribute {
    /// Set to which the attribute belongs
    pub set: AttrSet,
    /// The variation of the attribute
    pub variation: u8,
    /// Value of the attribute
    pub value: OwnedAttrValue,
}

impl OwnedAttribute {
    /// construct an OwnedAttribute from its members
    pub fn new(set: AttrSet, variation: u8, value: OwnedAttrValue) -> Self {
        Self {
            set,
            variation,
            value,
        }
    }

    pub(crate) fn view(&self) -> Attribute {
        Attribute {
            set: self.set,
            variation: self.variation,
            value: self.value.view(),
        }
    }
}

fn get_default_desc(var: u8) -> &'static str {
    match var {
        var::CONFIG_ID => "Configuration ID",
        var::CONFIG_VERSION => "Configuration version",
        var::CONFIG_BUILD_DATE => "Configuration build date",
        var::CONFIG_LAST_CHANGE_DATE => "Configuration last change date",
        var::CONFIG_DIGEST => "Configuration digest",
        var::CONFIG_DIGEST_ALGORITHM => "Configuration digest algorithm",
        var::MASTER_RESOURCE_ID => "Master resource ID",
        var::DEVICE_LOCATION_ALTITUDE => "Device location altitude",
        var::DEVICE_LOCATION_LONGITUDE => "Device location longitude",
        var::DEVICE_LOCATION_LATITUDE => "Device location latitude",
        var::USER_ASSIGNED_SECONDARY_OPERATOR_NAME => "User-assigned secondary operator name",
        var::USER_ASSIGNED_PRIMARY_OPERATOR_NAME => "User-assigned primary operator name",
        var::USER_ASSIGNED_SYSTEM_NAME => "User-assigned system name",
        var::SECURE_AUTH_VERSION => "Secure authentication version",
        var::NUM_SECURITY_STAT_PER_ASSOC => "Number of security statistics per association",
        var::USER_SPECIFIC_ATTRIBUTES => "Identification of support for user-specific attributes",
        var::NUM_MASTER_DEFINED_DATA_SET_PROTO => "Number of master-defined data set prototypes",
        var::NUM_OUTSTATION_DEFINED_DATA_SET_PROTO => {
            "Number of outstation-defined data set prototypes"
        }
        var::NUM_MASTER_DEFINED_DATA_SETS => "Number of master-defined data sets",
        var::NUM_OUTSTATION_DEFINED_DATA_SETS => "Number of outstation-defined data sets",
        var::MAX_BINARY_OUTPUT_PER_REQUEST => "Maximum number of binary output objects per request",
        var::LOCAL_TIMING_ACCURACY => "Local timing accuracy",
        var::DURATION_OF_TIME_ACCURACY => "Duration of time accuracy",
        var::SUPPORTS_ANALOG_OUTPUT_EVENTS => "Support for analog output events",
        var::MAX_ANALOG_OUTPUT_INDEX => "Maximum analog output index",
        var::NUM_ANALOG_OUTPUT => "Number of analog outputs",
        var::SUPPORTS_BINARY_OUTPUT_EVENTS => "Support for binary output events",
        var::MAX_BINARY_OUTPUT_INDEX => "Maximum binary output index",
        var::NUM_BINARY_OUTPUT => "Number of binary outputs",
        var::SUPPORTS_FROZEN_COUNTER_EVENTS => "Support for frozen counter events",
        var::SUPPORTS_FROZEN_COUNTERS => "Support for frozen counters",
        var::SUPPORTS_COUNTER_EVENTS => "Support for counter events",
        var::MAX_COUNTER_INDEX => "Maximum counter index",
        var::NUM_COUNTER => "Number of counter points",
        var::SUPPORTS_FROZEN_ANALOG_INPUTS => "Support for frozen analog inputs",
        var::SUPPORTS_ANALOG_INPUT_EVENTS => "Support for analog input events",
        var::MAX_ANALOG_INPUT_INDEX => "Maximum analog input index",
        var::NUM_ANALOG_INPUT => "Number of analog input points",
        var::SUPPORTS_DOUBLE_BIT_BINARY_INPUT_EVENTS => {
            "Support for double-bit binary input events"
        }
        var::MAX_DOUBLE_BIT_BINARY_INPUT_INDEX => "Maximum double-bit binary input index",
        var::NUM_DOUBLE_BIT_BINARY_INPUT => "Number of double-bit binary input points",
        var::SUPPORTS_BINARY_INPUT_EVENTS => "Support for binary input events",
        var::MAX_BINARY_INPUT_INDEX => "Maximum binary input index",
        var::NUM_BINARY_INPUT => "Number of binary input points",
        var::MAX_TX_FRAGMENT_SIZE => "Maximum transmit fragment size",
        var::MAX_RX_FRAGMENT_SIZE => "Maximum receive fragment size",
        var::DEVICE_MANUFACTURER_SOFTWARE_VERSION => "Device manufacturer's software version",
        var::DEVICE_MANUFACTURER_HARDWARE_VERSION => "Device manufacturer's hardware version",
        var::USER_ASSIGNED_OWNER_NAME => "User-assigned owner name",
        var::USER_ASSIGNED_LOCATION => "User-assigned location name",
        var::USER_ASSIGNED_ID => "User-assigned ID code/number",
        var::USER_ASSIGNED_DEVICE_NAME => "User-assigned device name",
        var::DEVICE_SERIAL_NUMBER => "Device serial number",
        var::DEVICE_SUBSET_AND_CONFORMANCE => "DNP3 subset and conformance",
        var::PRODUCT_NAME_AND_MODEL => "Device manufacturer's product name and model",
        var::DEVICE_MANUFACTURER_NAME => "Device manufacturer's name",
        var::LIST_OF_ATTRIBUTE_VARIATIONS => "List of attribute variations",
        _ => "Unknown",
    }
}

impl<'a> Attribute<'a> {
    pub(crate) fn format(&self, f: &mut Formatter) -> std::fmt::Result {
        match self.set {
            AttrSet::Default => {
                // lookup description
                let desc = get_default_desc(self.variation);
                write!(f, "\nDefault set - variation {} - {desc}", self.variation)?;
            }
            AttrSet::Private(x) => {
                write!(f, "\nPrivate set ({x}) - variation {}", self.variation)?;
            }
        }
        self.value.format(f)
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

impl From<AttrWriteError> for TaskError {
    fn from(value: AttrWriteError) -> Self {
        match value {
            AttrWriteError::Cursor => TaskError::WriteError,
            AttrWriteError::BadAttribute(x) => TaskError::BadEncoding(BadEncoding::Attribute(x)),
        }
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
            AttrDataType::Dnp3Time => {
                if len != 6 {
                    return Err(AttrParseError::BadTimeLength(len));
                }
                Self::Dnp3Time(Timestamp::new(cursor.read_u48_le()?))
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
    pub(crate) fn get_type(&self) -> AttrDataType {
        match self {
            AttrValue::VisibleString(_) => AttrDataType::VisibleString,
            AttrValue::UnsignedInt(_) => AttrDataType::UnsignedInt,
            AttrValue::SignedInt(_) => AttrDataType::SignedInt,
            AttrValue::FloatingPoint(_) => AttrDataType::FloatingPoint,
            AttrValue::OctetString(_) => AttrDataType::OctetString,
            AttrValue::BitString(_) => AttrDataType::BitString,
            AttrValue::AttrList(_) => AttrDataType::AttrList,
            AttrValue::Dnp3Time(_) => AttrDataType::Dnp3Time,
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
    ) -> Result<VariationList<'a>, AttrParseError> {
        if len % 2 != 0 {
            return Err(AttrParseError::BadAttrListLength(len));
        }

        let data = cursor.read_bytes(len as usize)?;

        Ok(VariationList { data })
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
