pub mod config;
pub(crate) mod details;

use crate::app::header::IIN2;
use crate::app::measurement::*;
use crate::app::parse::parser::HeaderCollection;
use crate::outstation::database::config::*;
use crate::outstation::database::details::event::buffer::EventClasses;
use crate::outstation::database::details::range::static_db::{
    Deadband, FlagsDetector, PointConfig,
};
use crate::outstation::types::EventClass;
use crate::util::cursor::WriteCursor;

use std::sync::{Arc, Mutex};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum EventMode {
    /// Detect events in a type dependent fashion. This is the default mode that should be used.
    Detect,
    /// Produce an event whether the value has changed or not
    Force,
    /// Never produce an event regardless of change
    Suppress,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct EventBufferConfig {
    pub max_binary: u16,
    pub max_double_binary: u16,
    pub max_binary_output_status: u16,
    pub max_counter: u16,
    pub max_frozen_counter: u16,
    pub max_analog: u16,
    pub max_analog_output_status: u16,
}

impl EventBufferConfig {
    pub fn uniform(max: u16) -> Self {
        Self::new(max, max, max, max, max, max, max)
    }

    pub fn no_events() -> Self {
        Self::uniform(0)
    }

    pub fn new(
        max_binary: u16,
        max_double_binary: u16,
        max_binary_output_status: u16,
        max_counter: u16,
        max_frozen_counter: u16,
        max_analog: u16,
        max_analog_output_status: u16,
    ) -> Self {
        Self {
            max_binary,
            max_double_binary,
            max_binary_output_status,
            max_counter,
            max_frozen_counter,
            max_analog,
            max_analog_output_status,
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
    /// flags for IIN
    pub(crate) unwritten: EventClasses,
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
    pub fn new(update_static: bool, event_mode: EventMode) -> Self {
        Self {
            update_static,
            event_mode,
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

pub trait Update<T> {
    fn update(&mut self, value: &T, index: u16, options: UpdateOptions) -> bool;
}

pub trait Add<T> {
    fn add(&mut self, index: u16, class: EventClass, config: T) -> bool;
}

pub struct Database {
    pub(crate) inner: crate::outstation::database::details::database::Database,
}

impl Database {
    pub(crate) fn new(config: EventBufferConfig) -> Self {
        Self {
            inner: crate::outstation::database::details::database::Database::new(config),
        }
    }
}

#[derive(Clone)]
pub struct DatabaseHandle {
    inner: Arc<Mutex<Database>>,
}

impl DatabaseHandle {
    pub fn new(config: EventBufferConfig) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Database::new(config))),
        }
    }

    pub fn update<F, R>(&mut self, mut func: F) -> R
    where
        F: FnMut(&mut Database) -> R,
    {
        let mut lock = self.inner.lock().unwrap();
        func(&mut *lock)
    }

    pub(crate) fn clear_written_events(&mut self) {
        self.inner.lock().unwrap().inner.clear_written_events();
    }

    pub(crate) fn select(&mut self, headers: &HeaderCollection) -> IIN2 {
        self.inner.lock().unwrap().inner.select(headers)
    }

    pub(crate) fn write_response_headers(&mut self, cursor: &mut WriteCursor) -> ResponseInfo {
        self.inner
            .lock()
            .unwrap()
            .inner
            .write_response_headers(cursor)
    }

    pub(crate) fn reset(&mut self) {
        self.inner.lock().unwrap().inner.reset()
    }
}

impl Update<Binary> for Database {
    fn update(&mut self, value: &Binary, index: u16, options: UpdateOptions) -> bool {
        self.inner.update(value, index, options)
    }
}

impl Update<DoubleBitBinary> for Database {
    fn update(&mut self, value: &DoubleBitBinary, index: u16, options: UpdateOptions) -> bool {
        self.inner.update(value, index, options)
    }
}

impl Update<BinaryOutputStatus> for Database {
    fn update(&mut self, value: &BinaryOutputStatus, index: u16, options: UpdateOptions) -> bool {
        self.inner.update(value, index, options)
    }
}

impl Update<Counter> for Database {
    fn update(&mut self, value: &Counter, index: u16, options: UpdateOptions) -> bool {
        self.inner.update(value, index, options)
    }
}

impl Update<FrozenCounter> for Database {
    fn update(&mut self, value: &FrozenCounter, index: u16, options: UpdateOptions) -> bool {
        self.inner.update(value, index, options)
    }
}

impl Update<Analog> for Database {
    fn update(&mut self, value: &Analog, index: u16, options: UpdateOptions) -> bool {
        self.inner.update(value, index, options)
    }
}

impl Update<AnalogOutputStatus> for Database {
    fn update(&mut self, value: &AnalogOutputStatus, index: u16, options: UpdateOptions) -> bool {
        self.inner.update(value, index, options)
    }
}

impl Add<BinaryConfig> for Database {
    fn add(&mut self, index: u16, class: EventClass, config: BinaryConfig) -> bool {
        let config =
            PointConfig::<Binary>::new(class, FlagsDetector {}, config.s_var, config.e_var);
        self.inner.add(index, config)
    }
}

impl Add<DoubleBitBinaryConfig> for Database {
    fn add(&mut self, index: u16, class: EventClass, config: DoubleBitBinaryConfig) -> bool {
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
    fn add(&mut self, index: u16, class: EventClass, config: BinaryOutputStatusConfig) -> bool {
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
    fn add(&mut self, index: u16, class: EventClass, config: CounterConfig) -> bool {
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
    fn add(&mut self, index: u16, class: EventClass, config: FrozenCounterConfig) -> bool {
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
    fn add(&mut self, index: u16, class: EventClass, config: AnalogConfig) -> bool {
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
    fn add(&mut self, index: u16, class: EventClass, config: AnalogOutputStatusConfig) -> bool {
        let config = PointConfig::<AnalogOutputStatus>::new(
            class,
            Deadband::new(config.deadband),
            config.s_var,
            config.e_var,
        );
        self.inner.add(index, config)
    }
}
