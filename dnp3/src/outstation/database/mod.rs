use std::sync::{Arc, Mutex};

pub use config::*;
use details::range::static_db::{Deadband, FlagsDetector, OctetStringDetector, PointConfig};

use crate::app::measurement::*;
use crate::app::parse::parser::HeaderCollection;
use crate::app::Iin2;
use crate::master::EventClasses;
use crate::outstation::database::read::ReadHeader;

use crate::app::attr::{AttrProp, AttrSet, OwnedAttribute, TypeError};
use crate::outstation::OutstationApplication;
use scursor::WriteCursor;

mod config;
/// private internal control only needed by the parent module
mod details;
/// read headers
pub(crate) mod read;

/// Controls how events are processed when updating values in the database
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum EventMode {
    /// Detect events in a type dependent fashion. This is the default mode that should be used.
    Detect,
    /// Produce an event whether the value has changed or not
    Force,
    /// Never produce an event regardless of change
    Suppress,
}

/// Event class (1/2/3) assignment
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum EventClass {
    /// Class 1 data per the protocol specification
    Class1,
    /// Class 2 data per the protocol specification
    Class2,
    /// Class 3 data per the protocol specification
    Class3,
}

/// Controls which types are reported during a class 0 READ
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct ClassZeroConfig {
    /// If true, Binary Inputs are reported in Class 0 READ requests
    pub binary: bool,
    /// If true, Double-bit Binary Inputs are reported in Class 0 READ requests
    pub double_bit_binary: bool,
    /// If true, Binary Output Status points are reported in Class 0 READ requests
    pub binary_output_status: bool,
    /// If true, Counters are reported in Class 0 READ requests
    pub counter: bool,
    /// If true, Frozen Counters are reported in Class 0 READ requests
    pub frozen_counter: bool,
    /// If true, Analog Inputs are reported in Class 0 READ requests
    pub analog: bool,
    /// If true, Analog Output Status points are reported in Class 0 READ requests
    pub analog_output_status: bool,
    /// If true, Octet Strings are reported in Class 0 READ requests
    /// This field defaults to `false` for conformance to the standard
    pub octet_string: bool,
}

impl ClassZeroConfig {
    /// construct a `ClassZeroConfig` from its fields
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        binary: bool,
        double_bit_binary: bool,
        binary_output_status: bool,
        counter: bool,
        frozen_counter: bool,
        analog: bool,
        analog_output_status: bool,
        octet_string: bool,
    ) -> Self {
        ClassZeroConfig {
            binary,
            double_bit_binary,
            binary_output_status,
            counter,
            frozen_counter,
            analog,
            analog_output_status,
            octet_string,
        }
    }
}

impl Default for ClassZeroConfig {
    fn default() -> Self {
        Self {
            binary: true,
            double_bit_binary: true,
            binary_output_status: true,
            counter: true,
            frozen_counter: true,
            analog: true,
            analog_output_status: true,
            octet_string: false,
        }
    }
}

/// Maximum number of events for each type.
///
/// A value of zero means that events will not be buffered for that type.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct EventBufferConfig {
    /// maximum number of binary input events (g2)
    #[cfg_attr(feature = "serialization", serde(default))]
    pub max_binary: u16,
    /// maximum number of double bit binary input events (g4)
    #[cfg_attr(feature = "serialization", serde(default))]
    pub max_double_binary: u16,
    /// maximum number of binary output status events (g11)
    #[cfg_attr(feature = "serialization", serde(default))]
    pub max_binary_output_status: u16,
    /// maximum number of counter events (g22)
    #[cfg_attr(feature = "serialization", serde(default))]
    pub max_counter: u16,
    /// maximum number of frozen counter events (g23)
    #[cfg_attr(feature = "serialization", serde(default))]
    pub max_frozen_counter: u16,
    /// maximum number of analog events (g32)
    #[cfg_attr(feature = "serialization", serde(default))]
    pub max_analog: u16,
    /// maximum number of analog output status events (g42)
    #[cfg_attr(feature = "serialization", serde(default))]
    pub max_analog_output_status: u16,
    /// maximum number of octet string events (g111)
    #[cfg_attr(feature = "serialization", serde(default))]
    pub max_octet_string: u16,
}

impl EventBufferConfig {
    /// initialize with the same maximum values for all types
    pub fn all_types(max: u16) -> Self {
        Self::new(max, max, max, max, max, max, max, max)
    }

    /// initialize the configuration to support no events
    pub fn no_events() -> Self {
        Self::all_types(0)
    }

    /// create a configuration specifying the max for each type individually
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        max_binary: u16,
        max_double_binary: u16,
        max_binary_output_status: u16,
        max_counter: u16,
        max_frozen_counter: u16,
        max_analog: u16,
        max_analog_output_status: u16,
        max_octet_string: u16,
    ) -> Self {
        Self {
            max_binary,
            max_double_binary,
            max_binary_output_status,
            max_counter,
            max_frozen_counter,
            max_analog,
            max_analog_output_status,
            max_octet_string,
        }
    }

    fn max_events(&self) -> usize {
        self.max_binary as usize
            + self.max_double_binary as usize
            + self.max_binary_output_status as usize
            + self.max_counter as usize
            + self.max_frozen_counter as usize
            + self.max_analog as usize
            + self.max_analog_output_status as usize
            + self.max_octet_string as usize
    }
}

pub(crate) struct ResponseInfo {
    /// true if the written response contains events
    pub(crate) has_events: bool,
    /// true if all selected data has been written (FIN == 1)
    pub(crate) complete: bool,
}

pub(crate) struct EventsInfo {
    /// which classes have unwritten events
    pub(crate) unwritten_classes: EventClasses,
    /// True if an overflow occurred
    pub(crate) is_overflown: bool,
}

/// Options that control how the update is performed. 99% of the time
/// the default() method should be used to initialize this struct. Very
/// few applications need to use the other options.
#[derive(Debug, Copy, Clone)]
pub struct UpdateOptions {
    /// optionally bypass updating the static_db (current value)
    update_static: bool,
    /// determines how/if an event is produced
    event_mode: EventMode,
}

impl UpdateOptions {
    /// fully specify custom UpdateOptions
    pub const fn new(update_static: bool, event_mode: EventMode) -> Self {
        Self {
            update_static,
            event_mode,
        }
    }

    /// options that will only update the static value in the database, but produce no events
    /// useful for setting a first value in the outstation during database initialization
    pub fn no_event() -> Self {
        Self {
            update_static: true,
            event_mode: EventMode::Suppress,
        }
    }

    /// Update the static value and automatically detect event. This is the default value.
    pub fn detect_event() -> Self {
        Self {
            update_static: true,
            event_mode: EventMode::Detect,
        }
    }
}

impl Default for UpdateOptions {
    fn default() -> Self {
        Self::detect_event()
    }
}

impl ResponseInfo {
    pub(crate) fn need_confirm(&self) -> bool {
        self.has_events || !self.complete
    }
}

/// Trait for adding a type to the database by index/class/configuration
///
/// Setting class to None means that the value will not produce events (static only).
/// The value is initialized to the default of 0.0/false with flags == RESTART.
pub trait Add<T> {
    /// Add a measurement to the database
    fn add(&mut self, index: u16, class: Option<EventClass>, config: T) -> bool;
}

/// Trait for removing a type from the database
pub trait Remove<T> {
    /// Remove a type by index, return true of the value existed, false otherwise
    ///
    /// Note: this remove the static value and configuration only. Any previously
    /// buffered events will be reported normally
    fn remove(&mut self, index: u16) -> bool;
}

/// Information about what occurred during a point update
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UpdateInfo {
    /// No point exists for this type and index
    NoPoint,
    /// The point exists, but the update did not create an event
    NoEvent,
    /// An event was created with this id
    Created(u64),
    /// An event was created with this id, but inserting it caused an overflow
    Overflow {
        /// Id of the event that was created
        created: u64,
        /// Id of the previously inserted event that was discarded
        discarded: u64,
    },
}

/// Trait for updating an existing value in the database
pub trait Update<T> {
    /// Update a value at a particular index. The options control
    /// how static/event data is modified
    /// Returns true if the update succeeded (i.e. the point exists)
    fn update(&mut self, index: u16, value: &T, options: UpdateOptions) -> bool {
        self.update2(index, value, options) != UpdateInfo::NoPoint
    }

    /// An overload of [`Update::update()`] that provides more information about what occurred
    fn update2(&mut self, index: u16, value: &T, options: UpdateOptions) -> UpdateInfo;
}

/// Point type on which to update the flags
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum UpdateFlagsType {
    /// Binary input
    BinaryInput,
    /// Double-bit binary input
    DoubleBitBinaryInput,
    /// Binary output status
    BinaryOutputStatus,
    /// Counter
    Counter,
    /// Frozen counter
    FrozenCounter,
    /// Analog input
    AnalogInput,
    /// Analog output status
    AnalogOutputStatus,
}

/// Trait for updating flags on a point without changing the current value of the point
pub trait UpdateFlags {
    /// Update the flags for the specified point without changing the value
    ///
    /// This is equivalent to getting the current value, changing the flags and the timestamp, then calling update
    fn update_flags(
        &mut self,
        index: u16,
        flags_type: UpdateFlagsType,
        flags: Flags,
        time: Option<Time>,
        options: UpdateOptions,
    ) -> UpdateInfo;
}

/// Trait for getting the current value in the database
pub trait Get<T> {
    /// retrieve the current value off the database.
    fn get(&self, index: u16) -> Option<T>;
}

/// Errors that can occur when manipulating attributes
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(not(feature = "ffi"), non_exhaustive)]
pub enum AttrDefError {
    /// The attribute is already defined
    AlreadyDefined,
    /// The attribute does not match the type expected for set 0
    BadType(TypeError),
    /// The variation is reserved (254 or 255) and cannot be defined, written, or retrieved
    ReservedVariation(u8),
    /// The attribute is not writable
    NotWritable(AttrSet, u8),
}

impl std::fmt::Display for AttrDefError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::BadType(x) => write!(
                f,
                "The type {:?} does not match the expected type {:?}",
                x.actual, x.expected
            ),
            Self::ReservedVariation(x) => {
                write!(f, "Reserved variation cannot be defined: {x}")
            }
            Self::NotWritable(set, var) => write!(
                f,
                "Attribute with set = {set:?} and var = {var} cannot be written"
            ),
            Self::AlreadyDefined => write!(f, "The requested attribute is already defined"),
        }
    }
}

impl From<TypeError> for AttrDefError {
    fn from(value: TypeError) -> Self {
        Self::BadType(value)
    }
}

/// Core database implementation shared between an outstation task and the user facing API.
/// This type is always guarded by a `DatabaseHandle` which provides a transactional API.
pub struct Database {
    pub(crate) inner: details::database::Database,
}

impl Database {
    /// Create a database by specified how it will buffer events
    pub(crate) fn new(
        max_read_selection: Option<u16>,
        class_zero_config: ClassZeroConfig,
        config: EventBufferConfig,
    ) -> Self {
        Self {
            inner: details::database::Database::new(max_read_selection, class_zero_config, config),
        }
    }

    /// Define an attribute that will be exposed to the master
    pub fn define_attr(
        &mut self,
        prop: AttrProp,
        attr: OwnedAttribute,
    ) -> Result<(), AttrDefError> {
        self.inner.get_attr_map().define(prop, attr)
    }
}

/// Handle type that can be used to perform transactions on an underlying database
#[derive(Clone)]
pub struct DatabaseHandle {
    inner: Arc<Mutex<Database>>,
    notify: Arc<tokio::sync::Notify>,
}

impl DatabaseHandle {
    /// Acquire a mutex on the underlying database and apply a set of changes as a transaction
    pub fn transaction<F, R>(&self, mut func: F) -> R
    where
        F: FnMut(&mut Database) -> R,
    {
        let ret = {
            let mut db = self.inner.lock().unwrap();
            func(&mut db)
        };
        self.notify.notify_one();
        ret
    }

    pub(crate) async fn wait_for_change(&self) {
        self.notify.notified().await
    }

    pub(crate) fn new(
        max_read_selection: Option<u16>,
        class_zero_config: ClassZeroConfig,
        event_config: EventBufferConfig,
    ) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Database::new(
                max_read_selection,
                class_zero_config,
                event_config,
            ))),
            notify: Arc::new(tokio::sync::Notify::new()),
        }
    }

    pub(crate) async fn clear_written_events(&mut self, app: &mut dyn OutstationApplication) {
        app.begin_confirm();
        let state = self.inner.lock().unwrap().inner.clear_written_events(app);
        app.end_confirm(state).get().await;
    }

    pub(crate) fn get_events_info(&self) -> EventsInfo {
        let guard = self.inner.lock().unwrap();

        EventsInfo {
            unwritten_classes: guard.inner.unwritten_classes(),
            is_overflown: guard.inner.is_overflown(),
        }
    }

    pub(crate) fn select(&mut self, headers: &HeaderCollection) -> Iin2 {
        let mut iin2 = Iin2::default();
        let mut guard = self.inner.lock().unwrap();
        for header in headers.iter() {
            match ReadHeader::get(&header) {
                None => {
                    iin2 |= Iin2::NO_FUNC_CODE_SUPPORT;
                }
                Some(x) => iin2 |= guard.inner.select_by_header(x),
            }
        }
        iin2
    }

    pub(crate) fn write_response_headers(&mut self, cursor: &mut WriteCursor) -> ResponseInfo {
        self.inner
            .lock()
            .unwrap()
            .inner
            .write_response_headers(cursor)
    }

    pub(crate) fn write_unsolicited(
        &mut self,
        classes: EventClasses,
        cursor: &mut WriteCursor,
    ) -> usize {
        let mut guard = self.inner.lock().unwrap();
        guard.inner.reset();
        let count = guard.inner.select_event_classes(classes);
        if count == 0 {
            return 0;
        }
        guard.inner.write_events_only(cursor)
    }

    pub(crate) fn reset(&mut self) {
        self.inner.lock().unwrap().inner.reset()
    }
}

impl UpdateFlags for Database {
    fn update_flags(
        &mut self,
        index: u16,
        flags_type: UpdateFlagsType,
        flags: Flags,
        time: Option<Time>,
        options: UpdateOptions,
    ) -> UpdateInfo {
        self.inner
            .update_flags(index, flags_type, flags, time, options)
    }
}

impl Update<BinaryInput> for Database {
    fn update2(&mut self, index: u16, value: &BinaryInput, options: UpdateOptions) -> UpdateInfo {
        self.inner.update(value, index, options)
    }
}

impl Update<DoubleBitBinaryInput> for Database {
    fn update2(
        &mut self,
        index: u16,
        value: &DoubleBitBinaryInput,
        options: UpdateOptions,
    ) -> UpdateInfo {
        self.inner.update(value, index, options)
    }
}

impl Update<BinaryOutputStatus> for Database {
    fn update2(
        &mut self,
        index: u16,
        value: &BinaryOutputStatus,
        options: UpdateOptions,
    ) -> UpdateInfo {
        self.inner.update(value, index, options)
    }
}

impl Update<Counter> for Database {
    fn update2(&mut self, index: u16, value: &Counter, options: UpdateOptions) -> UpdateInfo {
        self.inner.update(value, index, options)
    }
}

impl Update<FrozenCounter> for Database {
    fn update2(&mut self, index: u16, value: &FrozenCounter, options: UpdateOptions) -> UpdateInfo {
        self.inner.update(value, index, options)
    }
}

impl Update<AnalogInput> for Database {
    fn update2(&mut self, index: u16, value: &AnalogInput, options: UpdateOptions) -> UpdateInfo {
        self.inner.update(value, index, options)
    }
}

impl Update<AnalogOutputStatus> for Database {
    fn update2(
        &mut self,
        index: u16,
        value: &AnalogOutputStatus,
        options: UpdateOptions,
    ) -> UpdateInfo {
        self.inner.update(value, index, options)
    }
}

impl Update<OctetString> for Database {
    fn update2(&mut self, index: u16, value: &OctetString, options: UpdateOptions) -> UpdateInfo {
        self.inner.update(value, index, options)
    }
}

impl Add<BinaryInputConfig> for Database {
    fn add(&mut self, index: u16, class: Option<EventClass>, config: BinaryInputConfig) -> bool {
        let config =
            PointConfig::<BinaryInput>::new(class, FlagsDetector {}, config.s_var, config.e_var);
        self.inner.add(index, config)
    }
}

impl Add<DoubleBitBinaryInputConfig> for Database {
    fn add(
        &mut self,
        index: u16,
        class: Option<EventClass>,
        config: DoubleBitBinaryInputConfig,
    ) -> bool {
        let config = PointConfig::<DoubleBitBinaryInput>::new(
            class,
            FlagsDetector {},
            config.s_var,
            config.e_var,
        );
        self.inner.add(index, config)
    }
}

impl Add<BinaryOutputStatusConfig> for Database {
    fn add(
        &mut self,
        index: u16,
        class: Option<EventClass>,
        config: BinaryOutputStatusConfig,
    ) -> bool {
        let config = PointConfig::<BinaryOutputStatus>::new(
            class,
            FlagsDetector {},
            config.s_var,
            config.e_var,
        );
        self.inner.add(index, config)
    }
}

impl Add<CounterConfig> for Database {
    fn add(&mut self, index: u16, class: Option<EventClass>, config: CounterConfig) -> bool {
        let config = PointConfig::<Counter>::new(
            class,
            Deadband::new(config.deadband),
            config.s_var,
            config.e_var,
        );
        self.inner.add(index, config)
    }
}

impl Add<FrozenCounterConfig> for Database {
    fn add(&mut self, index: u16, class: Option<EventClass>, config: FrozenCounterConfig) -> bool {
        let config = PointConfig::<FrozenCounter>::new(
            class,
            Deadband::new(config.deadband),
            config.s_var,
            config.e_var,
        );
        self.inner.add(index, config)
    }
}

impl Add<AnalogInputConfig> for Database {
    fn add(&mut self, index: u16, class: Option<EventClass>, config: AnalogInputConfig) -> bool {
        let config = PointConfig::<AnalogInput>::new(
            class,
            Deadband::new(config.deadband),
            config.s_var,
            config.e_var,
        );
        self.inner.add(index, config)
    }
}

impl Add<AnalogOutputStatusConfig> for Database {
    fn add(
        &mut self,
        index: u16,
        class: Option<EventClass>,
        config: AnalogOutputStatusConfig,
    ) -> bool {
        let config = PointConfig::<AnalogOutputStatus>::new(
            class,
            Deadband::new(config.deadband),
            config.s_var,
            config.e_var,
        );
        self.inner.add(index, config)
    }
}

impl Add<OctetStringConfig> for Database {
    fn add(&mut self, index: u16, class: Option<EventClass>, _config: OctetStringConfig) -> bool {
        let config = PointConfig::<OctetString>::new(
            class,
            OctetStringDetector,
            StaticOctetStringVariation,
            EventOctetStringVariation,
        );
        self.inner.add(index, config)
    }
}

impl Remove<BinaryInput> for Database {
    fn remove(&mut self, index: u16) -> bool {
        self.inner.remove::<BinaryInput>(index)
    }
}

impl Remove<DoubleBitBinaryInput> for Database {
    fn remove(&mut self, index: u16) -> bool {
        self.inner.remove::<DoubleBitBinaryInput>(index)
    }
}

impl Remove<BinaryOutputStatus> for Database {
    fn remove(&mut self, index: u16) -> bool {
        self.inner.remove::<BinaryOutputStatus>(index)
    }
}

impl Remove<Counter> for Database {
    fn remove(&mut self, index: u16) -> bool {
        self.inner.remove::<Counter>(index)
    }
}

impl Remove<FrozenCounter> for Database {
    fn remove(&mut self, index: u16) -> bool {
        self.inner.remove::<FrozenCounter>(index)
    }
}

impl Remove<AnalogInput> for Database {
    fn remove(&mut self, index: u16) -> bool {
        self.inner.remove::<AnalogInput>(index)
    }
}

impl Remove<AnalogOutputStatus> for Database {
    fn remove(&mut self, index: u16) -> bool {
        self.inner.remove::<AnalogOutputStatus>(index)
    }
}

impl Remove<OctetString> for Database {
    fn remove(&mut self, index: u16) -> bool {
        self.inner.remove::<OctetString>(index)
    }
}

impl Get<BinaryInput> for Database {
    fn get(&self, index: u16) -> Option<BinaryInput> {
        self.inner.get::<BinaryInput>(index)
    }
}

impl Get<DoubleBitBinaryInput> for Database {
    fn get(&self, index: u16) -> Option<DoubleBitBinaryInput> {
        self.inner.get::<DoubleBitBinaryInput>(index)
    }
}

impl Get<BinaryOutputStatus> for Database {
    fn get(&self, index: u16) -> Option<BinaryOutputStatus> {
        self.inner.get::<BinaryOutputStatus>(index)
    }
}

impl Get<Counter> for Database {
    fn get(&self, index: u16) -> Option<Counter> {
        self.inner.get::<Counter>(index)
    }
}

impl Get<FrozenCounter> for Database {
    fn get(&self, index: u16) -> Option<FrozenCounter> {
        self.inner.get::<FrozenCounter>(index)
    }
}

impl Get<AnalogInput> for Database {
    fn get(&self, index: u16) -> Option<AnalogInput> {
        self.inner.get::<AnalogInput>(index)
    }
}

impl Get<AnalogOutputStatus> for Database {
    fn get(&self, index: u16) -> Option<AnalogOutputStatus> {
        self.inner.get::<AnalogOutputStatus>(index)
    }
}

impl Get<OctetString> for Database {
    fn get(&self, index: u16) -> Option<OctetString> {
        self.inner.get::<OctetString>(index)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::app::Timestamp;

    const fn binary(val: bool) -> BinaryInput {
        BinaryInput::new(val, Flags::ONLINE, Time::Synchronized(Timestamp::zero()))
    }

    #[test]
    fn returns_no_point_if_point_not_added() {
        let mut db = Database::new(
            None,
            ClassZeroConfig::default(),
            EventBufferConfig::all_types(10),
        );
        assert_eq!(
            UpdateInfo::NoPoint,
            db.update2(0, &binary(true), UpdateOptions::default())
        );
    }

    #[test]
    fn returns_no_event_if_buffer_space_zero() {
        let mut db = Database::new(
            None,
            ClassZeroConfig::default(),
            EventBufferConfig::all_types(0),
        );
        db.add(0, Some(EventClass::Class1), BinaryInputConfig::default());
        assert_eq!(
            UpdateInfo::NoEvent,
            db.update2(0, &binary(true), UpdateOptions::default())
        );
    }

    #[test]
    fn returns_created_if_event_detected() {
        let mut db = Database::new(
            None,
            ClassZeroConfig::default(),
            EventBufferConfig::all_types(3),
        );
        db.add(0, Some(EventClass::Class1), BinaryInputConfig::default());
        assert_eq!(
            UpdateInfo::Created(0),
            db.update2(0, &binary(true), UpdateOptions::default())
        );
        assert_eq!(
            UpdateInfo::Created(1),
            db.update2(0, &binary(false), UpdateOptions::default())
        );
        assert_eq!(
            UpdateInfo::Created(2),
            db.update2(0, &binary(true), UpdateOptions::default())
        );
    }

    #[test]
    fn returns_overflow_when_event_discarded() {
        let mut db = Database::new(
            None,
            ClassZeroConfig::default(),
            EventBufferConfig::all_types(1),
        );
        db.add(0, Some(EventClass::Class1), BinaryInputConfig::default());
        assert_eq!(
            UpdateInfo::Created(0),
            db.update2(0, &binary(true), UpdateOptions::default())
        );
        assert_eq!(
            UpdateInfo::Overflow {
                created: 1,
                discarded: 0
            },
            db.update2(0, &binary(false), UpdateOptions::default())
        );
        assert_eq!(
            UpdateInfo::Overflow {
                created: 2,
                discarded: 1
            },
            db.update2(0, &binary(true), UpdateOptions::default())
        );
    }

    #[test]
    fn returns_overflow_no_event_if_no_change() {
        let mut db = Database::new(
            None,
            ClassZeroConfig::default(),
            EventBufferConfig::all_types(1),
        );
        db.add(0, Some(EventClass::Class1), BinaryInputConfig::default());
        assert_eq!(
            UpdateInfo::Created(0),
            db.update2(0, &binary(true), UpdateOptions::default())
        );
        assert_eq!(
            UpdateInfo::NoEvent,
            db.update2(0, &binary(true), UpdateOptions::default())
        );
    }
}
