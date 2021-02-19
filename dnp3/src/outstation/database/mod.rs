use std::sync::{Arc, Mutex};

pub use config::*;
use details::range::static_db::{Deadband, FlagsDetector, OctetStringDetector, PointConfig};

use crate::app::measurement::*;
use crate::app::parse::parser::HeaderCollection;
use crate::app::Iin2;
use crate::master::EventClasses;
use crate::outstation::database::read::ReadHeader;
use crate::util::cursor::WriteCursor;

mod config;
/// private internal control only needed by the parent module
mod details;
/// read headers
pub(crate) mod read;

/// Controls how events are processed when updating values in the database
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum EventMode {
    /// Detect events in a type dependent fashion. This is the default mode that should be used.
    Detect,
    /// Produce an event whether the value has changed or not
    Force,
    /// Never produce an event regardless of change
    Suppress,
}

/// Event class (1/2/3) assignment
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum EventClass {
    /// Class 1 data per the protocol specification
    Class1,
    /// Class 2 data per the protocol specification
    Class2,
    /// Class 3 data per the protocol specification
    Class3,
}

/// Controls which types are reported during a class 0 READ
#[derive(Debug, Copy, Clone, PartialEq)]
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
    pub octet_strings: bool,
}

impl ClassZeroConfig {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        binary: bool,
        double_bit_binary: bool,
        binary_output_status: bool,
        counter: bool,
        frozen_counter: bool,
        analog: bool,
        analog_output_status: bool,
        octet_strings: bool,
    ) -> Self {
        ClassZeroConfig {
            binary,
            double_bit_binary,
            binary_output_status,
            counter,
            frozen_counter,
            analog,
            analog_output_status,
            octet_strings,
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
            octet_strings: false,
        }
    }
}

/// Maximum number of events for each type.
///
/// A value of zero means that events will not be buffered for that type.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct EventBufferConfig {
    /// maximum number of binary input events (g2)
    pub max_binary: u16,
    /// maximum number of double bit binary input events (g4)
    pub max_double_binary: u16,
    /// maximum number of binary output status events (g11)
    pub max_binary_output_status: u16,
    /// maximum number of counter events (g22)
    pub max_counter: u16,
    /// maximum number of frozen counter events (g23)
    pub max_frozen_counter: u16,
    /// maximum number of analog events (g32)
    pub max_analog: u16,
    /// maximum number of analog output status events (g42)
    pub max_analog_output_status: u16,
    /// maximum number of octet string events (g111)
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
    }
}

pub(crate) struct ResponseInfo {
    /// true if the written response contains events
    pub(crate) has_events: bool,
    /// true if all selected data has been written (FIN == 1)
    pub(crate) complete: bool,
    /*
    /// flags for IIN
    pub(crate) unwritten: EventClasses,
     */
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
    pub fn initialize() -> Self {
        Self {
            update_static: true,
            event_mode: EventMode::Suppress,
        }
    }
}

impl Default for UpdateOptions {
    fn default() -> Self {
        Self {
            update_static: true,
            event_mode: EventMode::Detect,
        }
    }
}

impl ResponseInfo {
    pub(crate) fn need_confirm(&self) -> bool {
        self.has_events || !self.complete
    }
}

/// trait for adding a type to the database by index/class/configuration
///
/// Setting class to None means that the value will not produce events (static only).
/// The value is initialized to the default of 0.0/false with flags == RESTART.
pub trait Add<T> {
    /// add a measurement to the database
    fn add(&mut self, index: u16, class: Option<EventClass>, config: T) -> bool;
}

/// trait for removing a type from the database
pub trait Remove<T> {
    /// remove a type by index, return true of the value existed, false otherwise
    ///
    /// Note: this remove the static value and configuration only. Any previously
    /// buffered events will be reported normally
    fn remove(&mut self, index: u16) -> bool;
}

/// trait for updating an existing value in the database
pub trait Update<T> {
    /// Update a value at a particular index. The options control
    /// how static/event data is modified
    fn update(&mut self, index: u16, value: &T, options: UpdateOptions) -> bool;
}

/// Core database implementation shared between an outstation task and the user facing API.
/// This type is always guarded by a `DatabaseHandle` which provides a transactional API.
pub struct Database {
    pub(crate) inner: crate::outstation::database::details::database::Database,
}

impl Database {
    /// Create a database by specified how it will buffer events
    pub(crate) fn new(
        max_read_selection: Option<u16>,
        class_zero_config: ClassZeroConfig,
        config: EventBufferConfig,
    ) -> Self {
        Self {
            inner: crate::outstation::database::details::database::Database::new(
                max_read_selection,
                class_zero_config,
                config,
            ),
        }
    }
}

/// Handle type that can be used to perform transactions on an underlying database
#[derive(Clone)]
pub struct DatabaseHandle {
    inner: Arc<Mutex<Database>>,
    notify: Arc<crate::tokio::sync::Notify>,
}

impl DatabaseHandle {
    /// Perform a transaction on the underlying database using a closure
    pub fn transaction<F, R>(&self, mut func: F) -> R
    where
        F: FnMut(&mut Database) -> R,
    {
        let ret = func(&mut self.inner.lock().unwrap());
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
            notify: Arc::new(crate::tokio::sync::Notify::new()),
        }
    }

    pub(crate) fn clear_written_events(&mut self) {
        self.inner.lock().unwrap().inner.clear_written_events();
    }

    pub(crate) fn unwritten_classes(&self) -> EventClasses {
        self.inner.lock().unwrap().inner.unwritten_classes()
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

impl Update<Binary> for Database {
    fn update(&mut self, index: u16, value: &Binary, options: UpdateOptions) -> bool {
        self.inner.update(value, index, options)
    }
}

impl Update<DoubleBitBinary> for Database {
    fn update(&mut self, index: u16, value: &DoubleBitBinary, options: UpdateOptions) -> bool {
        self.inner.update(value, index, options)
    }
}

impl Update<BinaryOutputStatus> for Database {
    fn update(&mut self, index: u16, value: &BinaryOutputStatus, options: UpdateOptions) -> bool {
        self.inner.update(value, index, options)
    }
}

impl Update<Counter> for Database {
    fn update(&mut self, index: u16, value: &Counter, options: UpdateOptions) -> bool {
        self.inner.update(value, index, options)
    }
}

impl Update<FrozenCounter> for Database {
    fn update(&mut self, index: u16, value: &FrozenCounter, options: UpdateOptions) -> bool {
        self.inner.update(value, index, options)
    }
}

impl Update<Analog> for Database {
    fn update(&mut self, index: u16, value: &Analog, options: UpdateOptions) -> bool {
        self.inner.update(value, index, options)
    }
}

impl Update<AnalogOutputStatus> for Database {
    fn update(&mut self, index: u16, value: &AnalogOutputStatus, options: UpdateOptions) -> bool {
        self.inner.update(value, index, options)
    }
}

impl Update<OctetString> for Database {
    fn update(&mut self, index: u16, value: &OctetString, options: UpdateOptions) -> bool {
        self.inner.update(value, index, options)
    }
}

impl Add<BinaryConfig> for Database {
    fn add(&mut self, index: u16, class: Option<EventClass>, config: BinaryConfig) -> bool {
        let config =
            PointConfig::<Binary>::new(class, FlagsDetector {}, config.s_var, config.e_var);
        self.inner.add(index, config)
    }
}

impl Add<DoubleBitBinaryConfig> for Database {
    fn add(
        &mut self,
        index: u16,
        class: Option<EventClass>,
        config: DoubleBitBinaryConfig,
    ) -> bool {
        let config = PointConfig::<DoubleBitBinary>::new(
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

impl Add<AnalogConfig> for Database {
    fn add(&mut self, index: u16, class: Option<EventClass>, config: AnalogConfig) -> bool {
        let config = PointConfig::<Analog>::new(
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

impl Remove<Binary> for Database {
    fn remove(&mut self, index: u16) -> bool {
        self.inner.remove::<Binary>(index)
    }
}

impl Remove<DoubleBitBinary> for Database {
    fn remove(&mut self, index: u16) -> bool {
        self.inner.remove::<DoubleBitBinary>(index)
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

impl Remove<Analog> for Database {
    fn remove(&mut self, index: u16) -> bool {
        self.inner.remove::<Analog>(index)
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
