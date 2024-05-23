use dnp3::app::attr::{AttrProp, AttrSet, FloatType, OwnedAttrValue, OwnedAttribute};
use dnp3::app::measurement::*;
use dnp3::app::Timestamp;
pub use dnp3::outstation::database::Database;
pub use dnp3::outstation::database::DatabaseHandle;
use dnp3::outstation::database::*;
use std::ffi::CStr;

use crate::ffi;

impl Default for ffi::UpdateInfoFields {
    fn default() -> Self {
        Self {
            result: ffi::UpdateResult::NoPoint,
            created: 0,
            discarded: 0,
        }
    }
}

impl From<dnp3::outstation::database::UpdateInfo> for ffi::UpdateInfo {
    fn from(value: UpdateInfo) -> Self {
        match value {
            UpdateInfo::NoPoint => ffi::UpdateInfoFields {
                result: ffi::UpdateResult::NoPoint,
                created: 0,
                discarded: 0,
            }
            .into(),
            UpdateInfo::NoEvent => ffi::UpdateInfoFields {
                result: ffi::UpdateResult::NoEvent,
                created: 0,
                discarded: 0,
            }
            .into(),
            UpdateInfo::Created(id) => ffi::UpdateInfoFields {
                result: ffi::UpdateResult::Created,
                created: id,
                discarded: 0,
            }
            .into(),
            UpdateInfo::Overflow { created, discarded } => ffi::UpdateInfoFields {
                result: ffi::UpdateResult::Overflow,
                created,
                discarded,
            }
            .into(),
        }
    }
}

macro_rules! implement_database_point_operations {
    (
        $add_name:ident, $remove_name:ident, $update_name:ident, $update_name_2:ident, $get_name:ident,
        $lib_point_type:ty, $lib_config_type:ty,
        $ffi_point_type:ty, $ffi_config_type:ty,
    ) => {
        pub(crate) unsafe fn $add_name(
            database: *mut Database,
            index: u16,
            point_class: ffi::EventClass,
            config: $ffi_config_type,
        ) -> bool {
            if let Some(database) = database.as_mut() {
                return database.add(index, point_class.into(), <$lib_config_type>::from(config));
            }
            false
        }

        pub(crate) unsafe fn $remove_name(database: *mut Database, index: u16) -> bool {
            if let Some(database) = database.as_mut() {
                return Remove::<$lib_point_type>::remove(database, index);
            }
            false
        }

        pub(crate) unsafe fn $update_name(
            database: *mut Database,
            value: $ffi_point_type,
            options: ffi::UpdateOptions,
        ) -> bool {
            if let Some(database) = database.as_mut() {
                return database
                    .update(value.index, &<$lib_point_type>::from(value), options.into())
                    .into();
            }
            false
        }

        pub(crate) unsafe fn $update_name_2(
            database: *mut Database,
            value: $ffi_point_type,
            options: ffi::UpdateOptions,
        ) -> ffi::UpdateInfo {
            if let Some(database) = database.as_mut() {
                return database
                    .update2(value.index, &<$lib_point_type>::from(value), options.into())
                    .into();
            }
            ffi::UpdateInfoFields::default().into()
        }

        pub(crate) unsafe fn $get_name(
            database: *mut Database,
            index: u16,
        ) -> Result<$ffi_point_type, ffi::ParamError> {
            let database = database.as_mut().ok_or(ffi::ParamError::NullParameter)?;

            if let Some(point) = Get::<$lib_point_type>::get(database, index) {
                Ok(<$ffi_point_type>::new(index, point))
            } else {
                Err(ffi::ParamError::PointDoesNotExist)
            }
        }
    };
}

pub(crate) unsafe fn database_update_flags(
    instance: *mut crate::Database,
    index: u16,
    flags_type: ffi::UpdateFlagsType,
    flags: ffi::Flags,
    time: ffi::Timestamp,
    options: ffi::UpdateOptions,
) -> ffi::UpdateInfo {
    let db = match instance.as_mut() {
        None => return ffi::UpdateInfoFields::default().into(),
        Some(db) => db,
    };

    db.update_flags(
        index,
        flags_type.into(),
        (&flags).into(),
        (&time).into(),
        options.into(),
    )
    .into()
}

impl From<ffi::UpdateFlagsType> for UpdateFlagsType {
    fn from(value: ffi::UpdateFlagsType) -> Self {
        match value {
            ffi::UpdateFlagsType::BinaryInput => Self::BinaryInput,
            ffi::UpdateFlagsType::DoubleBitBinaryInput => Self::DoubleBitBinaryInput,
            ffi::UpdateFlagsType::BinaryOutputStatus => Self::BinaryOutputStatus,
            ffi::UpdateFlagsType::Counter => Self::Counter,
            ffi::UpdateFlagsType::FrozenCounter => Self::FrozenCounter,
            ffi::UpdateFlagsType::AnalogInput => Self::AnalogInput,
            ffi::UpdateFlagsType::AnalogOutputStatus => Self::AnalogOutputStatus,
        }
    }
}

implement_database_point_operations!(
    database_add_binary_input,
    database_remove_binary_input,
    database_update_binary_input,
    database_update_binary_input_2,
    database_get_binary_input,
    BinaryInput,
    BinaryInputConfig,
    ffi::BinaryInput,
    ffi::BinaryInputConfig,
);

implement_database_point_operations!(
    database_add_double_bit_binary_input,
    database_remove_double_bit_binary_input,
    database_update_double_bit_binary_input,
    database_update_double_bit_binary_input_2,
    database_get_double_bit_binary_input,
    DoubleBitBinaryInput,
    DoubleBitBinaryInputConfig,
    ffi::DoubleBitBinaryInput,
    ffi::DoubleBitBinaryInputConfig,
);

implement_database_point_operations!(
    database_add_binary_output_status,
    database_remove_binary_output_status,
    database_update_binary_output_status,
    database_update_binary_output_status_2,
    database_get_binary_output_status,
    BinaryOutputStatus,
    BinaryOutputStatusConfig,
    ffi::BinaryOutputStatus,
    ffi::BinaryOutputStatusConfig,
);

implement_database_point_operations!(
    database_add_counter,
    database_remove_counter,
    database_update_counter,
    database_update_counter_2,
    database_get_counter,
    Counter,
    CounterConfig,
    ffi::Counter,
    ffi::CounterConfig,
);

implement_database_point_operations!(
    database_add_frozen_counter,
    database_remove_frozen_counter,
    database_update_frozen_counter,
    database_update_frozen_counter_2,
    database_get_frozen_counter,
    FrozenCounter,
    FrozenCounterConfig,
    ffi::FrozenCounter,
    ffi::FrozenCounterConfig,
);

implement_database_point_operations!(
    database_add_analog_input,
    database_remove_analog_input,
    database_update_analog_input,
    database_update_analog_input_2,
    database_get_analog_input,
    AnalogInput,
    AnalogInputConfig,
    ffi::AnalogInput,
    ffi::AnalogInputConfig,
);

implement_database_point_operations!(
    database_add_analog_output_status,
    database_remove_analog_output_status,
    database_update_analog_output_status,
    database_update_analog_output_status_2,
    database_get_analog_output_status,
    AnalogOutputStatus,
    AnalogOutputStatusConfig,
    ffi::AnalogOutputStatus,
    ffi::AnalogOutputStatusConfig,
);

pub(crate) unsafe fn database_add_octet_string(
    database: *mut Database,
    index: u16,
    point_class: ffi::EventClass,
) -> bool {
    if let Some(database) = database.as_mut() {
        return database.add(index, point_class.into(), OctetStringConfig);
    }
    false
}

pub(crate) unsafe fn database_remove_octet_string(database: *mut Database, index: u16) -> bool {
    if let Some(database) = database.as_mut() {
        return Remove::<OctetString>::remove(database, index);
    }
    false
}

pub(crate) unsafe fn database_update_octet_string(
    database: *mut Database,
    index: u16,
    value: *mut OctetStringValue,
    options: ffi::UpdateOptions,
) -> bool {
    if let Some(database) = database.as_mut() {
        if let Some(value) = value.as_ref() {
            if let Some(value) = value.into() {
                return database.update(index, &value, options.into());
            }
        }
    }
    false
}

pub(crate) unsafe fn database_update_octet_string_2(
    database: *mut crate::Database,
    index: u16,
    value: *mut crate::OctetStringValue,
    options: ffi::UpdateOptions,
) -> ffi::UpdateInfo {
    if let Some(database) = database.as_mut() {
        if let Some(value) = value.as_ref() {
            if let Some(value) = value.into() {
                return database.update2(index, &value, options.into()).into();
            }
        }
    }
    ffi::UpdateInfoFields::default().into()
}

pub(crate) unsafe fn database_define_string_attr(
    instance: *mut crate::Database,
    set: u8,
    writable: bool,
    variation: u8,
    value: &CStr,
) -> ffi::AttrDefError {
    let value = || OwnedAttrValue::VisibleString(value.to_string_lossy().to_string());
    define_any_attr(instance, set, writable, variation, value)
}

pub(crate) unsafe fn database_define_uint_attr(
    instance: *mut crate::Database,
    set: u8,
    writable: bool,
    variation: u8,
    value: u32,
) -> ffi::AttrDefError {
    define_any_attr(instance, set, writable, variation, || {
        OwnedAttrValue::UnsignedInt(value)
    })
}

pub(crate) unsafe fn database_define_int_attr(
    instance: *mut crate::Database,
    set: u8,
    writable: bool,
    variation: u8,
    value: i32,
) -> ffi::AttrDefError {
    define_any_attr(instance, set, writable, variation, || {
        OwnedAttrValue::SignedInt(value)
    })
}

pub(crate) unsafe fn database_define_time_attr(
    instance: *mut crate::Database,
    set: u8,
    writable: bool,
    variation: u8,
    value: u64,
) -> ffi::AttrDefError {
    define_any_attr(instance, set, writable, variation, || {
        OwnedAttrValue::Dnp3Time(Timestamp::new(value))
    })
}

pub(crate) unsafe fn database_define_bool_attr(
    instance: *mut crate::Database,
    set: u8,
    writable: bool,
    variation: u8,
    value: bool,
) -> ffi::AttrDefError {
    define_any_attr(instance, set, writable, variation, || {
        OwnedAttrValue::SignedInt(value.into())
    })
}

pub(crate) unsafe fn database_define_float_attr(
    instance: *mut crate::Database,
    set: u8,
    writable: bool,
    variation: u8,
    value: f32,
) -> ffi::AttrDefError {
    define_any_attr(instance, set, writable, variation, || {
        OwnedAttrValue::FloatingPoint(FloatType::F32(value))
    })
}

pub(crate) unsafe fn database_define_double_attr(
    instance: *mut crate::Database,
    set: u8,
    writable: bool,
    variation: u8,
    value: f64,
) -> ffi::AttrDefError {
    define_any_attr(instance, set, writable, variation, || {
        OwnedAttrValue::FloatingPoint(FloatType::F64(value))
    })
}

unsafe fn define_any_attr<F>(
    instance: *mut crate::Database,
    set: u8,
    writable: bool,
    variation: u8,
    value: F,
) -> ffi::AttrDefError
where
    F: FnOnce() -> OwnedAttrValue,
{
    let db = match instance.as_mut() {
        None => return ffi::AttrDefError::Ok,
        Some(x) => x,
    };

    let prop = if writable {
        AttrProp::writable()
    } else {
        AttrProp::default()
    };

    let attr = OwnedAttribute::new(AttrSet::new(set), variation, value());

    if let Err(err) = db.define_attr(prop, attr) {
        return err.into();
    }

    ffi::AttrDefError::Ok
}

impl From<AttrDefError> for ffi::AttrDefError {
    fn from(value: AttrDefError) -> Self {
        match value {
            AttrDefError::AlreadyDefined => ffi::AttrDefError::AlreadyDefined,
            AttrDefError::BadType(_) => ffi::AttrDefError::BadType,
            AttrDefError::ReservedVariation(_) => ffi::AttrDefError::ReservedVariation,
            AttrDefError::NotWritable(_, _) => ffi::AttrDefError::NotWritable,
        }
    }
}

pub fn update_options_default() -> ffi::UpdateOptions {
    ffi::UpdateOptionsFields {
        update_static: true,
        event_mode: ffi::EventMode::Detect,
    }
    .into()
}

pub struct OctetStringValue {
    inner: Vec<u8>,
}

impl OctetStringValue {
    fn new() -> Self {
        Self { inner: Vec::new() }
    }

    fn into(&self) -> Option<OctetString> {
        OctetString::new(&self.inner).ok()
    }
}

pub unsafe fn octet_string_value_create() -> *mut OctetStringValue {
    Box::into_raw(Box::new(OctetStringValue::new()))
}

pub unsafe fn octet_string_value_destroy(octet_string: *mut OctetStringValue) {
    if !octet_string.is_null() {
        drop(Box::from_raw(octet_string));
    }
}

pub unsafe fn octet_string_value_add(octet_string: *mut OctetStringValue, value: u8) {
    if let Some(octet_string) = octet_string.as_mut() {
        octet_string.inner.push(value);
    }
}

impl From<ffi::EventClass> for Option<EventClass> {
    fn from(from: ffi::EventClass) -> Self {
        match from {
            ffi::EventClass::None => None,
            ffi::EventClass::Class1 => Some(EventClass::Class1),
            ffi::EventClass::Class2 => Some(EventClass::Class2),
            ffi::EventClass::Class3 => Some(EventClass::Class3),
        }
    }
}

impl From<ffi::UpdateOptions> for UpdateOptions {
    fn from(from: ffi::UpdateOptions) -> Self {
        let update_static = from.update_static();
        let event_mode = match from.event_mode() {
            ffi::EventMode::Detect => EventMode::Detect,
            ffi::EventMode::Force => EventMode::Force,
            ffi::EventMode::Suppress => EventMode::Suppress,
        };

        Self::new(update_static, event_mode)
    }
}

impl From<&ffi::Flags> for Flags {
    fn from(from: &ffi::Flags) -> Self {
        Self {
            value: from.value(),
        }
    }
}

impl From<&ffi::Timestamp> for Option<Time> {
    fn from(from: &ffi::Timestamp) -> Self {
        match from.quality() {
            ffi::TimeQuality::InvalidTime => None,
            ffi::TimeQuality::SynchronizedTime => {
                Some(Time::Synchronized(Timestamp::new(from.value())))
            }
            ffi::TimeQuality::UnsynchronizedTime => {
                Some(Time::Unsynchronized(Timestamp::new(from.value())))
            }
        }
    }
}

impl From<ffi::BinaryInputConfig> for BinaryInputConfig {
    fn from(from: ffi::BinaryInputConfig) -> Self {
        Self {
            s_var: match from.static_variation() {
                ffi::StaticBinaryInputVariation::Group1Var1 => {
                    StaticBinaryInputVariation::Group1Var1
                }
                ffi::StaticBinaryInputVariation::Group1Var2 => {
                    StaticBinaryInputVariation::Group1Var2
                }
            },
            e_var: match from.event_variation() {
                ffi::EventBinaryInputVariation::Group2Var1 => EventBinaryInputVariation::Group2Var1,
                ffi::EventBinaryInputVariation::Group2Var2 => EventBinaryInputVariation::Group2Var2,
                ffi::EventBinaryInputVariation::Group2Var3 => EventBinaryInputVariation::Group2Var3,
            },
        }
    }
}

impl From<ffi::BinaryInput> for BinaryInput {
    fn from(from: ffi::BinaryInput) -> Self {
        Self {
            value: from.value(),
            flags: from.flags().into(),
            time: from.time().into(),
        }
    }
}

impl From<ffi::DoubleBitBinaryInputConfig> for DoubleBitBinaryInputConfig {
    fn from(from: ffi::DoubleBitBinaryInputConfig) -> Self {
        Self {
            s_var: match from.static_variation() {
                ffi::StaticDoubleBitBinaryInputVariation::Group3Var1 => {
                    StaticDoubleBitBinaryInputVariation::Group3Var1
                }
                ffi::StaticDoubleBitBinaryInputVariation::Group3Var2 => {
                    StaticDoubleBitBinaryInputVariation::Group3Var2
                }
            },
            e_var: match from.event_variation() {
                ffi::EventDoubleBitBinaryInputVariation::Group4Var1 => {
                    EventDoubleBitBinaryInputVariation::Group4Var1
                }
                ffi::EventDoubleBitBinaryInputVariation::Group4Var2 => {
                    EventDoubleBitBinaryInputVariation::Group4Var2
                }
                ffi::EventDoubleBitBinaryInputVariation::Group4Var3 => {
                    EventDoubleBitBinaryInputVariation::Group4Var3
                }
            },
        }
    }
}

impl From<ffi::DoubleBitBinaryInput> for DoubleBitBinaryInput {
    fn from(from: ffi::DoubleBitBinaryInput) -> Self {
        Self {
            value: match from.value() {
                ffi::DoubleBit::Intermediate => DoubleBit::Intermediate,
                ffi::DoubleBit::DeterminedOff => DoubleBit::DeterminedOff,
                ffi::DoubleBit::DeterminedOn => DoubleBit::DeterminedOn,
                ffi::DoubleBit::Indeterminate => DoubleBit::Indeterminate,
            },
            flags: from.flags().into(),
            time: from.time().into(),
        }
    }
}

impl From<ffi::BinaryOutputStatusConfig> for BinaryOutputStatusConfig {
    fn from(from: ffi::BinaryOutputStatusConfig) -> Self {
        Self {
            s_var: match from.static_variation() {
                ffi::StaticBinaryOutputStatusVariation::Group10Var1 => {
                    StaticBinaryOutputStatusVariation::Group10Var1
                }
                ffi::StaticBinaryOutputStatusVariation::Group10Var2 => {
                    StaticBinaryOutputStatusVariation::Group10Var2
                }
            },
            e_var: match from.event_variation() {
                ffi::EventBinaryOutputStatusVariation::Group11Var1 => {
                    EventBinaryOutputStatusVariation::Group11Var1
                }
                ffi::EventBinaryOutputStatusVariation::Group11Var2 => {
                    EventBinaryOutputStatusVariation::Group11Var2
                }
            },
        }
    }
}

impl From<ffi::BinaryOutputStatus> for BinaryOutputStatus {
    fn from(from: ffi::BinaryOutputStatus) -> Self {
        Self {
            value: from.value(),
            flags: from.flags().into(),
            time: from.time().into(),
        }
    }
}

impl From<ffi::CounterConfig> for CounterConfig {
    fn from(from: ffi::CounterConfig) -> Self {
        Self {
            s_var: match from.static_variation() {
                ffi::StaticCounterVariation::Group20Var1 => StaticCounterVariation::Group20Var1,
                ffi::StaticCounterVariation::Group20Var2 => StaticCounterVariation::Group20Var2,
                ffi::StaticCounterVariation::Group20Var5 => StaticCounterVariation::Group20Var5,
                ffi::StaticCounterVariation::Group20Var6 => StaticCounterVariation::Group20Var6,
            },
            e_var: match from.event_variation() {
                ffi::EventCounterVariation::Group22Var1 => EventCounterVariation::Group22Var1,
                ffi::EventCounterVariation::Group22Var2 => EventCounterVariation::Group22Var2,
                ffi::EventCounterVariation::Group22Var5 => EventCounterVariation::Group22Var5,
                ffi::EventCounterVariation::Group22Var6 => EventCounterVariation::Group22Var6,
            },
            deadband: from.deadband(),
        }
    }
}

impl From<ffi::Counter> for Counter {
    fn from(from: ffi::Counter) -> Self {
        Self {
            value: from.value(),
            flags: from.flags().into(),
            time: from.time().into(),
        }
    }
}

impl From<ffi::FrozenCounterConfig> for FrozenCounterConfig {
    fn from(from: ffi::FrozenCounterConfig) -> Self {
        Self {
            s_var: match from.static_variation() {
                ffi::StaticFrozenCounterVariation::Group21Var1 => {
                    StaticFrozenCounterVariation::Group21Var1
                }
                ffi::StaticFrozenCounterVariation::Group21Var2 => {
                    StaticFrozenCounterVariation::Group21Var2
                }
                ffi::StaticFrozenCounterVariation::Group21Var5 => {
                    StaticFrozenCounterVariation::Group21Var5
                }
                ffi::StaticFrozenCounterVariation::Group21Var6 => {
                    StaticFrozenCounterVariation::Group21Var6
                }
                ffi::StaticFrozenCounterVariation::Group21Var9 => {
                    StaticFrozenCounterVariation::Group21Var9
                }
                ffi::StaticFrozenCounterVariation::Group21Var10 => {
                    StaticFrozenCounterVariation::Group21Var10
                }
            },
            e_var: match from.event_variation() {
                ffi::EventFrozenCounterVariation::Group23Var1 => {
                    EventFrozenCounterVariation::Group23Var1
                }
                ffi::EventFrozenCounterVariation::Group23Var2 => {
                    EventFrozenCounterVariation::Group23Var2
                }
                ffi::EventFrozenCounterVariation::Group23Var5 => {
                    EventFrozenCounterVariation::Group23Var5
                }
                ffi::EventFrozenCounterVariation::Group23Var6 => {
                    EventFrozenCounterVariation::Group23Var6
                }
            },
            deadband: from.deadband(),
        }
    }
}

impl From<ffi::FrozenCounter> for FrozenCounter {
    fn from(from: ffi::FrozenCounter) -> Self {
        Self {
            value: from.value(),
            flags: from.flags().into(),
            time: from.time().into(),
        }
    }
}

impl From<ffi::AnalogInputConfig> for AnalogInputConfig {
    fn from(from: ffi::AnalogInputConfig) -> Self {
        Self {
            s_var: match from.static_variation() {
                ffi::StaticAnalogInputVariation::Group30Var1 => {
                    StaticAnalogInputVariation::Group30Var1
                }
                ffi::StaticAnalogInputVariation::Group30Var2 => {
                    StaticAnalogInputVariation::Group30Var2
                }
                ffi::StaticAnalogInputVariation::Group30Var3 => {
                    StaticAnalogInputVariation::Group30Var3
                }
                ffi::StaticAnalogInputVariation::Group30Var4 => {
                    StaticAnalogInputVariation::Group30Var4
                }
                ffi::StaticAnalogInputVariation::Group30Var5 => {
                    StaticAnalogInputVariation::Group30Var5
                }
                ffi::StaticAnalogInputVariation::Group30Var6 => {
                    StaticAnalogInputVariation::Group30Var6
                }
            },
            e_var: match from.event_variation() {
                ffi::EventAnalogInputVariation::Group32Var1 => {
                    EventAnalogInputVariation::Group32Var1
                }
                ffi::EventAnalogInputVariation::Group32Var2 => {
                    EventAnalogInputVariation::Group32Var2
                }
                ffi::EventAnalogInputVariation::Group32Var3 => {
                    EventAnalogInputVariation::Group32Var3
                }
                ffi::EventAnalogInputVariation::Group32Var4 => {
                    EventAnalogInputVariation::Group32Var4
                }
                ffi::EventAnalogInputVariation::Group32Var5 => {
                    EventAnalogInputVariation::Group32Var5
                }
                ffi::EventAnalogInputVariation::Group32Var6 => {
                    EventAnalogInputVariation::Group32Var6
                }
                ffi::EventAnalogInputVariation::Group32Var7 => {
                    EventAnalogInputVariation::Group32Var7
                }
                ffi::EventAnalogInputVariation::Group32Var8 => {
                    EventAnalogInputVariation::Group32Var8
                }
            },
            deadband: from.deadband(),
        }
    }
}

impl From<ffi::AnalogInput> for AnalogInput {
    fn from(from: ffi::AnalogInput) -> Self {
        Self {
            value: from.value(),
            flags: from.flags().into(),
            time: from.time().into(),
        }
    }
}

impl From<ffi::AnalogOutputStatusConfig> for AnalogOutputStatusConfig {
    fn from(from: ffi::AnalogOutputStatusConfig) -> Self {
        Self {
            s_var: match from.static_variation() {
                ffi::StaticAnalogOutputStatusVariation::Group40Var1 => {
                    StaticAnalogOutputStatusVariation::Group40Var1
                }
                ffi::StaticAnalogOutputStatusVariation::Group40Var2 => {
                    StaticAnalogOutputStatusVariation::Group40Var2
                }
                ffi::StaticAnalogOutputStatusVariation::Group40Var3 => {
                    StaticAnalogOutputStatusVariation::Group40Var3
                }
                ffi::StaticAnalogOutputStatusVariation::Group40Var4 => {
                    StaticAnalogOutputStatusVariation::Group40Var4
                }
            },
            e_var: match from.event_variation() {
                ffi::EventAnalogOutputStatusVariation::Group42Var1 => {
                    EventAnalogOutputStatusVariation::Group42Var1
                }
                ffi::EventAnalogOutputStatusVariation::Group42Var2 => {
                    EventAnalogOutputStatusVariation::Group42Var2
                }
                ffi::EventAnalogOutputStatusVariation::Group42Var3 => {
                    EventAnalogOutputStatusVariation::Group42Var3
                }
                ffi::EventAnalogOutputStatusVariation::Group42Var4 => {
                    EventAnalogOutputStatusVariation::Group42Var4
                }
                ffi::EventAnalogOutputStatusVariation::Group42Var5 => {
                    EventAnalogOutputStatusVariation::Group42Var5
                }
                ffi::EventAnalogOutputStatusVariation::Group42Var6 => {
                    EventAnalogOutputStatusVariation::Group42Var6
                }
                ffi::EventAnalogOutputStatusVariation::Group42Var7 => {
                    EventAnalogOutputStatusVariation::Group42Var7
                }
                ffi::EventAnalogOutputStatusVariation::Group42Var8 => {
                    EventAnalogOutputStatusVariation::Group42Var8
                }
            },
            deadband: from.deadband(),
        }
    }
}

impl From<ffi::AnalogOutputStatus> for AnalogOutputStatus {
    fn from(from: ffi::AnalogOutputStatus) -> Self {
        Self {
            value: from.value(),
            flags: from.flags().into(),
            time: from.time().into(),
        }
    }
}

pub(crate) unsafe fn database_handle_transaction(
    instance: *mut crate::DatabaseHandle,
    callback: crate::ffi::DatabaseTransaction,
) {
    if let Some(db) = instance.as_mut() {
        db.transaction(|db| callback.execute(db))
    }
}
