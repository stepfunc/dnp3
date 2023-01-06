use crate::app::parse::range::Range;
use crate::app::parse::traits::{FixedSize, Index};
use crate::app::{ObjectParseError, Timestamp};
use crate::master::{BadEncoding, TaskError};
use scursor::{ReadCursor, ReadError, WriteCursor, WriteError};
use std::fmt::{Display, Formatter};
use std::str::Utf8Error;

/// constants defining attribute variations
pub(crate) mod var {
    pub(crate) const CONFIG_ID: u8 = 196;
    pub(crate) const CONFIG_VERSION: u8 = 197;
    pub(crate) const CONFIG_BUILD_DATE: u8 = 198;
    pub(crate) const CONFIG_LAST_CHANGE_DATE: u8 = 199;
    pub(crate) const CONFIG_DIGEST: u8 = 200;
    pub(crate) const CONFIG_DIGEST_ALGORITHM: u8 = 201;
    pub(crate) const MASTER_RESOURCE_ID: u8 = 202;
    pub(crate) const DEVICE_LOCATION_ALTITUDE: u8 = 203;
    pub(crate) const DEVICE_LOCATION_LONGITUDE: u8 = 204;
    pub(crate) const DEVICE_LOCATION_LATITUDE: u8 = 205;
    pub(crate) const USER_ASSIGNED_SECONDARY_OPERATOR_NAME: u8 = 206;
    pub(crate) const USER_ASSIGNED_PRIMARY_OPERATOR_NAME: u8 = 207;
    pub(crate) const USER_ASSIGNED_SYSTEM_NAME: u8 = 208;
    pub(crate) const SECURE_AUTH_VERSION: u8 = 209;
    pub(crate) const NUM_SECURITY_STAT_PER_ASSOC: u8 = 210;
    pub(crate) const USER_SPECIFIC_ATTRIBUTES: u8 = 211;
    pub(crate) const NUM_MASTER_DEFINED_DATA_SET_PROTO: u8 = 212;
    pub(crate) const NUM_OUTSTATION_DEFINED_DATA_SET_PROTO: u8 = 213;
    pub(crate) const NUM_MASTER_DEFINED_DATA_SETS: u8 = 214;
    pub(crate) const NUM_OUTSTATION_DEFINED_DATA_SETS: u8 = 215;
    pub(crate) const MAX_BINARY_OUTPUT_PER_REQUEST: u8 = 216;
    pub(crate) const LOCAL_TIMING_ACCURACY: u8 = 217;
    pub(crate) const DURATION_OF_TIME_ACCURACY: u8 = 218;
    pub(crate) const SUPPORTS_ANALOG_OUTPUT_EVENTS: u8 = 219;
    pub(crate) const MAX_ANALOG_OUTPUT_INDEX: u8 = 220;
    pub(crate) const NUM_ANALOG_OUTPUT: u8 = 221;
    pub(crate) const SUPPORTS_BINARY_OUTPUT_EVENTS: u8 = 222;
    pub(crate) const MAX_BINARY_OUTPUT_INDEX: u8 = 223;
    pub(crate) const NUM_BINARY_OUTPUT: u8 = 224;
    pub(crate) const SUPPORTS_FROZEN_COUNTER_EVENTS: u8 = 225;
    pub(crate) const SUPPORTS_FROZEN_COUNTERS: u8 = 226;
    pub(crate) const SUPPORTS_COUNTER_EVENTS: u8 = 227;
    pub(crate) const MAX_COUNTER_INDEX: u8 = 228;
    pub(crate) const NUM_COUNTER: u8 = 229;
    pub(crate) const SUPPORTS_FROZEN_ANALOG_INPUTS: u8 = 230;
    pub(crate) const SUPPORTS_ANALOG_INPUT_EVENTS: u8 = 231;
    pub(crate) const MAX_ANALOG_INPUT_INDEX: u8 = 232;
    pub(crate) const NUM_ANALOG_INPUT: u8 = 233;
    pub(crate) const SUPPORTS_DOUBLE_BIT_BINARY_INPUT_EVENTS: u8 = 234;
    pub(crate) const MAX_DOUBLE_BIT_BINARY_INPUT_INDEX: u8 = 235;
    pub(crate) const NUM_DOUBLE_BIT_BINARY_INPUT: u8 = 236;
    pub(crate) const SUPPORTS_BINARY_INPUT_EVENTS: u8 = 237;
    pub(crate) const MAX_BINARY_INPUT_INDEX: u8 = 238;
    pub(crate) const NUM_BINARY_INPUT: u8 = 239;
    pub(crate) const MAX_TX_FRAGMENT_SIZE: u8 = 240;
    pub(crate) const MAX_RX_FRAGMENT_SIZE: u8 = 241;
    pub(crate) const DEVICE_MANUFACTURER_SOFTWARE_VERSION: u8 = 242;
    pub(crate) const DEVICE_MANUFACTURER_HARDWARE_VERSION: u8 = 243;
    pub(crate) const USER_ASSIGNED_OWNER_NAME: u8 = 244;
    pub(crate) const USER_ASSIGNED_LOCATION: u8 = 245;
    pub(crate) const USER_ASSIGNED_ID: u8 = 246;
    pub(crate) const USER_ASSIGNED_DEVICE_NAME: u8 = 247;
    pub(crate) const DEVICE_SERIAL_NUMBER: u8 = 248;
    pub(crate) const DEVICE_SUBSET_AND_CONFORMANCE: u8 = 249;
    pub(crate) const PRODUCT_NAME_AND_MODEL: u8 = 250;
    pub(crate) const DEVICE_MANUFACTURER_NAME: u8 = 252;
    pub(crate) const LIST_OF_ATTRIBUTE_VARIATIONS: u8 = 255;
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
    fn extract_from(self, value: AttrValue) -> Result<KnownAttribute, TypeError> {
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
    fn extract_from(self, value: AttrValue) -> Result<KnownAttribute, TypeError> {
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
    fn extract_from(self, value: AttrValue) -> Result<KnownAttribute, TypeError> {
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
    fn extract_from(self, value: AttrValue) -> Result<KnownAttribute, TypeError> {
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
    fn extract_from(self, value: AttrValue) -> Result<KnownAttribute, TypeError> {
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
    fn extract_from(self, value: AttrValue) -> Result<KnownAttribute, TypeError> {
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
    /// Variation 203 - Altitude of the device
    DeviceLocationAltitude,
    /// Variation 204 - Longitude of the device from reference meridian (-180.0 to 180.0 deg)
    DeviceLocationLongitude,
    /// Variation 205 - Latitude of the device from the equator (90.0 to -90.0 deg)
    DeviceLocationLatitude,
}

impl FloatAttr {
    fn extract_from(self, value: AttrValue) -> Result<KnownAttribute, TypeError> {
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
    pub(crate) fn try_from(attr: &Attribute<'a>) -> Result<Self, TypeError> {
        if let AttrSet::Private(_) = attr.set {
            return Ok(AnyAttribute::Other(*attr));
        }

        let known = match attr.variation {
            var::CONFIG_ID => StringAttr::ConfigId.extract_from(attr.value)?,
            var::CONFIG_VERSION => StringAttr::ConfigVersion.extract_from(attr.value)?,
            var::CONFIG_BUILD_DATE => TimeAttr::ConfigBuildDate.extract_from(attr.value)?,
            var::CONFIG_LAST_CHANGE_DATE => {
                TimeAttr::ConfigLastChangeDate.extract_from(attr.value)?
            }
            var::CONFIG_DIGEST => OctetStringAttr::ConfigDigest.extract_from(attr.value)?,
            var::CONFIG_DIGEST_ALGORITHM => {
                StringAttr::ConfigDigestAlgorithm.extract_from(attr.value)?
            }
            var::MASTER_RESOURCE_ID => StringAttr::MasterResourceId.extract_from(attr.value)?,
            var::DEVICE_LOCATION_ALTITUDE => {
                FloatAttr::DeviceLocationAltitude.extract_from(attr.value)?
            }
            var::DEVICE_LOCATION_LONGITUDE => {
                FloatAttr::DeviceLocationLongitude.extract_from(attr.value)?
            }
            var::DEVICE_LOCATION_LATITUDE => {
                FloatAttr::DeviceLocationLatitude.extract_from(attr.value)?
            }
            var::USER_ASSIGNED_SECONDARY_OPERATOR_NAME => {
                StringAttr::UserAssignedSecondaryOperatorName.extract_from(attr.value)?
            }
            var::USER_ASSIGNED_PRIMARY_OPERATOR_NAME => {
                StringAttr::UserAssignedPrimaryOperatorName.extract_from(attr.value)?
            }
            var::USER_ASSIGNED_SYSTEM_NAME => {
                StringAttr::UserAssignedSystemName.extract_from(attr.value)?
            }
            var::SECURE_AUTH_VERSION => UIntAttr::SecureAuthVersion.extract_from(attr.value)?,
            var::NUM_SECURITY_STAT_PER_ASSOC => {
                UIntAttr::NumSecurityStatsPerAssoc.extract_from(attr.value)?
            }
            var::USER_SPECIFIC_ATTRIBUTES => {
                StringAttr::UserSpecificAttributes.extract_from(attr.value)?
            }
            var::NUM_MASTER_DEFINED_DATA_SET_PROTO => {
                UIntAttr::NumMasterDefinedDataSetProto.extract_from(attr.value)?
            }
            var::NUM_OUTSTATION_DEFINED_DATA_SET_PROTO => {
                UIntAttr::NumOutstationDefinedDataSetProto.extract_from(attr.value)?
            }
            var::NUM_MASTER_DEFINED_DATA_SETS => {
                UIntAttr::NumMasterDefinedDataSets.extract_from(attr.value)?
            }
            var::NUM_OUTSTATION_DEFINED_DATA_SETS => {
                UIntAttr::NumOutstationDefinedDataSets.extract_from(attr.value)?
            }
            var::MAX_BINARY_OUTPUT_PER_REQUEST => {
                UIntAttr::MaxBinaryOutputPerRequest.extract_from(attr.value)?
            }
            var::LOCAL_TIMING_ACCURACY => UIntAttr::LocalTimingAccuracy.extract_from(attr.value)?,
            var::DURATION_OF_TIME_ACCURACY => {
                UIntAttr::DurationOfTimeAccuracy.extract_from(attr.value)?
            }
            var::SUPPORTS_ANALOG_OUTPUT_EVENTS => {
                BoolAttr::SupportsAnalogOutputEvents.extract_from(attr.value)?
            }
            var::MAX_ANALOG_OUTPUT_INDEX => {
                UIntAttr::MaxAnalogOutputIndex.extract_from(attr.value)?
            }
            var::NUM_ANALOG_OUTPUT => UIntAttr::NumAnalogOutputs.extract_from(attr.value)?,
            var::SUPPORTS_BINARY_OUTPUT_EVENTS => {
                BoolAttr::SupportsBinaryOutputEvents.extract_from(attr.value)?
            }
            var::MAX_BINARY_OUTPUT_INDEX => {
                UIntAttr::MaxBinaryOutputIndex.extract_from(attr.value)?
            }
            var::NUM_BINARY_OUTPUT => UIntAttr::NumBinaryOutputs.extract_from(attr.value)?,
            var::SUPPORTS_FROZEN_COUNTER_EVENTS => {
                BoolAttr::SupportsFrozenCounterEvents.extract_from(attr.value)?
            }
            var::SUPPORTS_FROZEN_COUNTERS => {
                BoolAttr::SupportsFrozenCounters.extract_from(attr.value)?
            }
            var::SUPPORTS_COUNTER_EVENTS => {
                BoolAttr::SupportsCounterEvents.extract_from(attr.value)?
            }
            var::MAX_COUNTER_INDEX => UIntAttr::MaxCounterIndex.extract_from(attr.value)?,
            var::NUM_COUNTER => UIntAttr::NumCounter.extract_from(attr.value)?,
            var::SUPPORTS_FROZEN_ANALOG_INPUTS => {
                BoolAttr::SupportsFrozenAnalogInputs.extract_from(attr.value)?
            }
            var::SUPPORTS_ANALOG_INPUT_EVENTS => {
                BoolAttr::SupportsAnalogInputEvents.extract_from(attr.value)?
            }
            var::MAX_ANALOG_INPUT_INDEX => {
                UIntAttr::MaxAnalogInputIndex.extract_from(attr.value)?
            }
            var::NUM_ANALOG_INPUT => UIntAttr::NumAnalogInput.extract_from(attr.value)?,
            var::SUPPORTS_DOUBLE_BIT_BINARY_INPUT_EVENTS => {
                BoolAttr::SupportsDoubleBitBinaryInputEvents.extract_from(attr.value)?
            }
            var::MAX_DOUBLE_BIT_BINARY_INPUT_INDEX => {
                UIntAttr::MaxDoubleBitBinaryInputIndex.extract_from(attr.value)?
            }
            var::NUM_DOUBLE_BIT_BINARY_INPUT => {
                UIntAttr::NumDoubleBitBinaryInput.extract_from(attr.value)?
            }
            var::SUPPORTS_BINARY_INPUT_EVENTS => {
                BoolAttr::SupportsBinaryInputEvents.extract_from(attr.value)?
            }
            var::MAX_BINARY_INPUT_INDEX => {
                UIntAttr::MaxBinaryInputIndex.extract_from(attr.value)?
            }
            var::NUM_BINARY_INPUT => UIntAttr::NumBinaryInput.extract_from(attr.value)?,
            var::MAX_TX_FRAGMENT_SIZE => UIntAttr::MaxTxFragmentSize.extract_from(attr.value)?,
            var::MAX_RX_FRAGMENT_SIZE => UIntAttr::MaxRxFragmentSize.extract_from(attr.value)?,
            var::DEVICE_MANUFACTURER_SOFTWARE_VERSION => {
                StringAttr::DeviceManufacturerSoftwareVersion.extract_from(attr.value)?
            }
            var::DEVICE_MANUFACTURER_HARDWARE_VERSION => {
                StringAttr::DeviceManufacturerHardwareVersion.extract_from(attr.value)?
            }
            var::USER_ASSIGNED_OWNER_NAME => {
                StringAttr::UserAssignedOwnerName.extract_from(attr.value)?
            }
            var::USER_ASSIGNED_LOCATION => {
                StringAttr::UserAssignedLocation.extract_from(attr.value)?
            }
            var::USER_ASSIGNED_ID => StringAttr::UserAssignedId.extract_from(attr.value)?,
            var::USER_ASSIGNED_DEVICE_NAME => {
                StringAttr::UserAssignedDeviceName.extract_from(attr.value)?
            }
            var::DEVICE_SERIAL_NUMBER => StringAttr::DeviceSerialNumber.extract_from(attr.value)?,
            var::DEVICE_SUBSET_AND_CONFORMANCE => {
                StringAttr::DeviceSubsetAndConformance.extract_from(attr.value)?
            }
            var::PRODUCT_NAME_AND_MODEL => {
                StringAttr::ProductNameAndModel.extract_from(attr.value)?
            }
            var::DEVICE_MANUFACTURER_NAME => {
                StringAttr::DeviceManufacturersName.extract_from(attr.value)?
            }
            var::LIST_OF_ATTRIBUTE_VARIATIONS => {
                VariationListAttr::ListOfVariations.extract_from(attr.value)?
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

impl<'a> Iterator for VariationListIter<'a> {
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
    fn data_type(&self) -> AttrDataType {
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

    /// Modify a value if it is of the same type
    pub(crate) fn modify(&mut self, other: Self) -> Result<(), TypeError> {
        match self {
            Self::VisibleString(x) => {
                if let Self::VisibleString(y) = other {
                    *x = y;
                    return Ok(());
                }
            }
            Self::UnsignedInt(x) => {
                if let Self::UnsignedInt(y) = other {
                    *x = y;
                    return Ok(());
                }
            }
            Self::SignedInt(x) => {
                if let Self::SignedInt(y) = other {
                    *x = y;
                    return Ok(());
                }
            }
            Self::FloatingPoint(x) => {
                if let Self::FloatingPoint(y) = other {
                    *x = y;
                    return Ok(());
                }
            }
            Self::OctetString(x) => {
                if let Self::OctetString(y) = other {
                    *x = y;
                    return Ok(());
                }
            }
            Self::Dnp3Time(x) => {
                if let Self::Dnp3Time(y) = other {
                    *x = y;
                    return Ok(());
                }
            }
            Self::BitString(x) => {
                if let Self::BitString(y) = other {
                    *x = y;
                    return Ok(());
                }
            }
        }
        Err(TypeError::new(self.data_type(), other.data_type()))
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
    Cursor(WriteError),
    /// attribute value could not be encoded
    BadAttribute(BadAttribute),
}

impl From<BadAttribute> for AttrWriteError {
    fn from(value: BadAttribute) -> Self {
        AttrWriteError::BadAttribute(value)
    }
}

impl From<WriteError> for AttrWriteError {
    fn from(err: WriteError) -> Self {
        AttrWriteError::Cursor(err)
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
    fn new(expected: AttrDataType, actual: AttrDataType) -> Self {
        Self { expected, actual }
    }
}

impl<'a> AttrValue<'a> {
    pub(crate) fn format(&self, f: &mut Formatter) -> std::fmt::Result {
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
            AttrValue::Dnp3Time(x) => write!(f, "{x}"),
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
        196 => "Configuration ID",
        197 => "Configuration version",
        198 => "Configuration build date",
        199 => "Configuration last change date",
        200 => "Configuration digest",
        201 => "Configuration digest algorithm",
        202 => "Master resource ID",
        203 => "Device location altitude",
        204 => "Device location longitude",
        205 => "Device location latitude",
        206 => "User-assigned secondary operator name",
        207 => "User-assigned primary operator name",
        208 => "User-assigned system name",
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
        244 => "User-assigned owner name",
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
    pub(crate) fn format(&self, f: &mut Formatter) -> std::fmt::Result {
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
            AttrWriteError::Cursor(_) => TaskError::WriteError,
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
