use std::cell::Cell;
use std::fmt::Debug;
use std::ops::BitOr;

use crate::app::measurement::{self};
use crate::master::EventClasses;
use crate::outstation::database::config::*;
use crate::outstation::database::read::EventReadHeader;
use crate::outstation::database::{EventBufferConfig, EventClass};

use crate::util::BadWrite;

use super::list::VecList;
use super::writer::EventWriter;

use crate::outstation::database::details::event::traits::OctetStringLength;
use crate::outstation::{BufferState, ClassCount, OutstationApplication, TypeCount};
use scursor::WriteCursor;

impl From<EventClass> for EventClasses {
    fn from(x: EventClass) -> Self {
        match x {
            EventClass::Class1 => EventClasses::new(true, false, false),
            EventClass::Class2 => EventClasses::new(false, true, false),
            EventClass::Class3 => EventClasses::new(false, false, true),
        }
    }
}

impl BitOr<EventClass> for EventClass {
    type Output = EventClasses;

    fn bitor(self, rhs: EventClass) -> Self::Output {
        let lhs: EventClasses = self.into();
        let rhs: EventClasses = rhs.into();
        lhs | rhs
    }
}

impl EventClasses {
    fn matches(&self, class: EventClass) -> bool {
        match class {
            EventClass::Class1 => self.class1,
            EventClass::Class2 => self.class2,
            EventClass::Class3 => self.class3,
        }
    }
}

impl BitOr<EventClasses> for EventClasses {
    type Output = EventClasses;

    fn bitor(self, rhs: EventClasses) -> Self::Output {
        EventClasses::new(
            self.class1 | rhs.class1,
            self.class2 | rhs.class2,
            self.class3 | rhs.class3,
        )
    }
}

impl Default for EventClasses {
    fn default() -> Self {
        Self::new(false, false, false)
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct Count {
    value: usize,
}

impl Count {
    fn new() -> Self {
        Self { value: 0 }
    }

    fn subtract(&self, other: &Count) -> Self {
        Self {
            value: self.value - other.value,
        }
    }

    fn zero(&mut self) {
        self.value = 0;
    }

    fn get(&self) -> usize {
        self.value
    }

    fn increment(&mut self) {
        self.value += 1;
    }

    fn decrement(&mut self) {
        self.value -= 1;
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct ClassCounter {
    num_class_1: Count,
    num_class_2: Count,
    num_class_3: Count,
}

impl From<ClassCounter> for ClassCount {
    fn from(value: ClassCounter) -> Self {
        Self {
            num_class_1: value.num_class_1.value,
            num_class_2: value.num_class_2.value,
            num_class_3: value.num_class_3.value,
        }
    }
}

impl ClassCounter {
    fn new() -> Self {
        Self {
            num_class_1: Count::new(),
            num_class_2: Count::new(),
            num_class_3: Count::new(),
        }
    }

    fn zero(&mut self) {
        self.num_class_1.zero();
        self.num_class_2.zero();
        self.num_class_3.zero();
    }

    fn subtract(&self, other: &Self) -> Self {
        Self {
            num_class_1: self.num_class_1.subtract(&other.num_class_1),
            num_class_2: self.num_class_2.subtract(&other.num_class_2),
            num_class_3: self.num_class_3.subtract(&other.num_class_3),
        }
    }

    fn increment(&mut self, class: EventClass) {
        match class {
            EventClass::Class1 => self.num_class_1.increment(),
            EventClass::Class2 => self.num_class_2.increment(),
            EventClass::Class3 => self.num_class_3.increment(),
        };
    }

    fn decrement(&mut self, class: EventClass) {
        match class {
            EventClass::Class1 => self.num_class_1.decrement(),
            EventClass::Class2 => self.num_class_2.decrement(),
            EventClass::Class3 => self.num_class_3.decrement(),
        }
    }
}

#[derive(Copy, Clone)]
pub(crate) struct TypeCounter {
    num_binary: Count,
    num_double_binary: Count,
    num_binary_output_status: Count,
    num_counter: Count,
    num_frozen_counter: Count,
    num_analog: Count,
    num_analog_output_status: Count,
    num_octet_string: Count,
}

impl From<TypeCounter> for TypeCount {
    fn from(value: TypeCounter) -> Self {
        Self {
            num_binary_input: value.num_binary.value,
            num_double_bit_binary_input: value.num_double_binary.value,
            num_binary_output_status: value.num_binary_output_status.value,
            num_counter: value.num_counter.value,
            num_frozen_counter: value.num_frozen_counter.value,
            num_analog: value.num_analog.value,
            num_analog_output_status: value.num_analog_output_status.value,
            num_octet_string: value.num_octet_string.value,
        }
    }
}

impl TypeCounter {
    fn new() -> Self {
        Self {
            num_binary: Count::new(),
            num_double_binary: Count::new(),
            num_binary_output_status: Count::new(),
            num_counter: Count::new(),
            num_frozen_counter: Count::new(),
            num_analog: Count::new(),
            num_analog_output_status: Count::new(),
            num_octet_string: Count::new(),
        }
    }

    fn zero(&mut self) {
        self.num_binary.zero();
        self.num_double_binary.zero();
        self.num_binary_output_status.zero();
        self.num_counter.zero();
        self.num_frozen_counter.zero();
        self.num_analog.zero();
        self.num_analog_output_status.zero();
        self.num_octet_string.zero();
    }

    fn increment(&mut self, event: &Event) {
        self.modify(event, |cnt| cnt.increment())
    }

    /*
       fn decrement(&mut self, event: &Event) {
           self.modify(event, |cnt| cnt.decrement())
       }
    */

    fn modify<F>(&mut self, event: &Event, op: F)
    where
        F: Fn(&mut Count),
    {
        match event {
            Event::Binary(_, _) => op(&mut self.num_binary),
            Event::DoubleBitBinary(_, _) => op(&mut self.num_double_binary),
            Event::BinaryOutputStatus(_, _) => op(&mut self.num_binary_output_status),
            Event::Counter(_, _) => op(&mut self.num_counter),
            Event::FrozenCounter(_, _) => op(&mut self.num_frozen_counter),
            Event::Analog(_, _) => op(&mut self.num_analog),
            Event::AnalogOutputStatus(_, _) => op(&mut self.num_analog_output_status),
            Event::OctetString(_, _) => op(&mut self.num_octet_string),
        }
    }
}

#[derive(Copy, Clone)]
struct Counters {
    types: TypeCounter,
    classes: ClassCounter,
}

impl From<Counters> for BufferState {
    fn from(value: Counters) -> Self {
        Self {
            classes: value.classes.into(),
            types: value.types.into(),
        }
    }
}

impl Counters {
    fn new() -> Self {
        Self {
            types: TypeCounter::new(),
            classes: ClassCounter::new(),
        }
    }

    fn zero(&mut self) {
        self.types.zero();
        self.classes.zero();
    }

    fn increment(&mut self, record: &EventRecord) {
        self.types.increment(&record.event);
        self.classes.increment(record.class);
    }

    fn decrement(&mut self, record: &EventRecord) {
        self.classes.decrement(record.class);
        match record.event {
            Event::Binary(_, _) => self.types.num_binary.decrement(),
            Event::DoubleBitBinary(_, _) => self.types.num_double_binary.decrement(),
            Event::BinaryOutputStatus(_, _) => self.types.num_binary_output_status.decrement(),
            Event::Counter(_, _) => self.types.num_counter.decrement(),
            Event::FrozenCounter(_, _) => self.types.num_frozen_counter.decrement(),
            Event::Analog(_, _) => self.types.num_analog.decrement(),
            Event::AnalogOutputStatus(_, _) => self.types.num_analog_output_status.decrement(),
            Event::OctetString(_, _) => self.types.num_octet_string.decrement(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct Variation<T>
where
    T: Copy,
{
    default: T,
    selected: Cell<T>,
}

impl<T> Variation<T>
where
    T: Copy,
{
    fn new(default: T) -> Self {
        Self {
            default,
            selected: Cell::new(default),
        }
    }

    fn select_default(&self) {
        self.selected.set(self.default)
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum Event {
    Binary(
        measurement::BinaryInput,
        Variation<EventBinaryInputVariation>,
    ),
    DoubleBitBinary(
        measurement::DoubleBitBinaryInput,
        Variation<EventDoubleBitBinaryInputVariation>,
    ),
    BinaryOutputStatus(
        measurement::BinaryOutputStatus,
        Variation<EventBinaryOutputStatusVariation>,
    ),
    Counter(measurement::Counter, Variation<EventCounterVariation>),
    FrozenCounter(
        measurement::FrozenCounter,
        Variation<EventFrozenCounterVariation>,
    ),
    Analog(
        measurement::AnalogInput,
        Variation<EventAnalogInputVariation>,
    ),
    AnalogOutputStatus(
        measurement::AnalogOutputStatus,
        Variation<EventAnalogOutputStatusVariation>,
    ),
    OctetString(Box<[u8]>, Variation<EventOctetStringVariation>),
}

impl Event {
    fn select_default_variation(&self) {
        match &self {
            Event::Binary(_, v) => v.select_default(),
            Event::DoubleBitBinary(_, v) => v.select_default(),
            Event::BinaryOutputStatus(_, v) => v.select_default(),
            Event::Counter(_, v) => v.select_default(),
            Event::FrozenCounter(_, v) => v.select_default(),
            Event::Analog(_, v) => v.select_default(),
            Event::AnalogOutputStatus(_, v) => v.select_default(),
            Event::OctetString(_, v) => v.select_default(),
        }
    }

    fn write(
        &self,
        index: u16,
        cursor: &mut WriteCursor,
        writer: &mut EventWriter,
    ) -> Result<(), BadWrite> {
        match &self {
            Event::Binary(evt, v) => writer.write(cursor, evt, index, v.selected.get()),
            Event::DoubleBitBinary(evt, v) => writer.write(cursor, evt, index, v.selected.get()),
            Event::BinaryOutputStatus(evt, v) => writer.write(cursor, evt, index, v.selected.get()),
            Event::Counter(evt, v) => writer.write(cursor, evt, index, v.selected.get()),
            Event::FrozenCounter(evt, v) => writer.write(cursor, evt, index, v.selected.get()),
            Event::Analog(evt, v) => writer.write(cursor, evt, index, v.selected.get()),
            Event::AnalogOutputStatus(evt, v) => writer.write(cursor, evt, index, v.selected.get()),
            Event::OctetString(evt, _) => {
                writer.write(cursor, evt, index, OctetStringLength(evt.len()))
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum EventState {
    Unselected,
    Selected,
    Written,
}

#[derive(Debug, PartialEq)]
pub(crate) struct EventRecord {
    index: u16,
    id: u64,
    class: EventClass,
    event: Event,
    state: Cell<EventState>,
}

impl EventRecord {
    fn new(index: u16, id: u64, class: EventClass, event: Event) -> Self {
        Self {
            index,
            id,
            class,
            event,
            state: Cell::new(EventState::Unselected),
        }
    }
}

pub(crate) trait Insertable: Sized {
    type EventVariation: Copy;

    fn get_max(config: &EventBufferConfig) -> u16;
    fn get_type_count(counter: &TypeCounter) -> usize;
    fn is_type(record: &EventRecord) -> bool;
    fn decrement_type(counter: &mut TypeCounter);
    fn increment_type(counter: &mut TypeCounter);
    fn create_event(&self, default_variation: Self::EventVariation) -> Event;
    // set the selected variation if the record is of this type
    fn select_variation(record: &EventRecord, variation: Self::EventVariation) -> bool;
}

pub(crate) struct EventBuffer {
    config: EventBufferConfig,
    events: VecList<EventRecord>,
    total: Counters,
    written: Counters,
    is_overflown: bool,
    next: u64,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum InsertError {
    TypeMaxIsZero,
    Overflow { created: u64, discarded: u64 },
}

impl EventBuffer {
    pub(crate) fn new(config: EventBufferConfig) -> Self {
        let max_size = config.max_events();
        Self {
            config,
            events: VecList::new(max_size),
            total: Counters::new(),
            written: Counters::new(),
            is_overflown: false,
            next: 0,
        }
    }

    pub(crate) fn unwritten_classes(&self) -> EventClasses {
        let unwritten = self.total.classes.subtract(&self.written.classes);
        EventClasses::new(
            unwritten.num_class_1.value > 0,
            unwritten.num_class_2.value > 0,
            unwritten.num_class_3.value > 0,
        )
    }

    pub(crate) fn insert<T>(
        &mut self,
        index: u16,
        class: EventClass,
        event: &T,
        default_variation: T::EventVariation,
    ) -> Result<u64, InsertError>
    where
        T: Insertable,
    {
        let max = T::get_max(&self.config);

        if max == 0 {
            return Err(InsertError::TypeMaxIsZero);
        }

        let id = self.next;
        self.next += 1;

        let ret = if T::get_type_count(&self.total.types) == max as usize {
            if let Some(record) = self.events.remove_first(T::is_type) {
                T::decrement_type(&mut self.total.types);
                self.total.classes.decrement(record.class);
                self.is_overflown = true;
                Err(InsertError::Overflow {
                    created: id,
                    discarded: record.id,
                })
            } else {
                Ok(id)
            }
        } else {
            Ok(id)
        };

        let record = EventRecord::new(index, id, class, event.create_event(default_variation));

        self.events.add(record);
        self.total.classes.increment(class);
        T::increment_type(&mut self.total.types);

        ret
    }

    pub(crate) fn select_by_header(&mut self, header: EventReadHeader) -> usize {
        match header {
            EventReadHeader::Class1(limit) => {
                self.select_by_class(EventClass::Class1.into(), limit)
            }
            EventReadHeader::Class2(limit) => {
                self.select_by_class(EventClass::Class2.into(), limit)
            }
            EventReadHeader::Class3(limit) => {
                self.select_by_class(EventClass::Class3.into(), limit)
            }
            EventReadHeader::Binary(v, limit) => {
                self.select_by_type::<measurement::BinaryInput>(v, limit)
            }
            EventReadHeader::DoubleBitBinary(v, limit) => {
                self.select_by_type::<measurement::DoubleBitBinaryInput>(v, limit)
            }
            EventReadHeader::BinaryOutputStatus(v, limit) => {
                self.select_by_type::<measurement::BinaryOutputStatus>(v, limit)
            }
            EventReadHeader::Counter(v, limit) => {
                self.select_by_type::<measurement::Counter>(v, limit)
            }
            EventReadHeader::FrozenCounter(v, limit) => {
                self.select_by_type::<measurement::FrozenCounter>(v, limit)
            }
            EventReadHeader::Analog(v, limit) => {
                self.select_by_type::<measurement::AnalogInput>(v, limit)
            }
            EventReadHeader::AnalogOutputStatus(v, limit) => {
                self.select_by_type::<measurement::AnalogOutputStatus>(v, limit)
            }
            EventReadHeader::OctetString(limit) => {
                self.select_by_type::<measurement::OctetString>(None, limit)
            }
            EventReadHeader::FrozenAnalog(_, _) => {
                // not currently supported
                0
            }
        }
    }

    pub(crate) fn select_by_class(&mut self, classes: EventClasses, limit: Option<usize>) -> usize {
        self.select(limit, |e| {
            if classes.matches(e.class) {
                e.event.select_default_variation();
                true
            } else {
                false
            }
        })
    }

    fn select_by_type<T>(
        &mut self,
        specific_variation: Option<T::EventVariation>,
        limit: Option<usize>,
    ) -> usize
    where
        T: Insertable,
    {
        match specific_variation {
            Some(x) => self.select_specific_variation::<T>(limit, x),
            None => self.select_default_variation::<T>(limit),
        }
    }

    fn select_specific_variation<T>(
        &mut self,
        limit: Option<usize>,
        variation: T::EventVariation,
    ) -> usize
    where
        T: Insertable,
    {
        self.select(limit, |e| T::select_variation(e, variation))
    }

    fn select_default_variation<T>(&mut self, limit: Option<usize>) -> usize
    where
        T: Insertable,
    {
        self.select(limit, |rec| {
            if T::is_type(rec) {
                rec.event.select_default_variation();
                true
            } else {
                false
            }
        })
    }

    pub(crate) fn write_events(&mut self, cursor: &mut WriteCursor) -> Result<usize, usize> {
        let mut count = 0;
        let mut writer = EventWriter::new();
        let mut counters = self.written;
        let mut complete = true;
        for record in self.selected_iter() {
            if record
                .event
                .write(record.index, cursor, &mut writer)
                .is_err()
            {
                complete = false;
                break; // out of space
            }

            counters.increment(record);
            record.state.set(EventState::Written);
            count += 1;
        }
        self.written = counters;

        if complete {
            Ok(count)
        } else {
            Err(count)
        }
    }

    pub(crate) fn clear_written(&mut self, app: &mut dyn OutstationApplication) -> usize {
        let total = &mut self.total;
        let count = self.events.remove_all(|event| {
            if event.state.get() == EventState::Written {
                app.event_cleared(event.id);
                total.decrement(event);
                true
            } else {
                false
            }
        });

        self.written.zero();
        if !self.is_any_full() {
            self.is_overflown = false;
        }
        count
    }

    pub(crate) fn buffer_state(&self) -> BufferState {
        self.total.into()
    }

    pub(crate) fn reset(&mut self) {
        for (_, r) in self.events.iter() {
            r.state.set(EventState::Unselected);
        }
        self.written.zero();
    }

    pub(crate) fn is_overflown(&self) -> bool {
        self.is_overflown
    }

    fn is_any_full(&self) -> bool {
        self.is_full::<measurement::BinaryInput>()
            || self.is_full::<measurement::DoubleBitBinaryInput>()
            || self.is_full::<measurement::BinaryOutputStatus>()
            || self.is_full::<measurement::Counter>()
            || self.is_full::<measurement::FrozenCounter>()
            || self.is_full::<measurement::AnalogInput>()
            || self.is_full::<measurement::AnalogOutputStatus>()
            || self.is_full::<measurement::OctetString>()
    }

    fn is_full<T>(&self) -> bool
    where
        T: Insertable,
    {
        let max = T::get_max(&self.config);

        if max == 0 {
            return false;
        }

        T::get_type_count(&self.total.types) >= max as usize
    }

    fn select<F>(&mut self, limit: Option<usize>, selector: F) -> usize
    where
        F: Fn(&EventRecord) -> bool,
    {
        let mut count = 0;

        for (_, evt) in self
            .events
            .iter()
            .filter(|(_, e)| e.state.get() == EventState::Unselected && selector(e))
            .take(limit.unwrap_or(usize::MAX))
        {
            evt.state.set(EventState::Selected);
            count += 1;
        }

        count
    }

    /// iterator over selected values that need to be written
    fn selected_iter(&self) -> impl Iterator<Item = &EventRecord> {
        self.events
            .iter()
            .map(|x| x.1)
            .filter(|x| x.state.get() == EventState::Selected)
    }
}

impl Insertable for measurement::BinaryInput {
    type EventVariation = EventBinaryInputVariation;

    fn get_max(config: &EventBufferConfig) -> u16 {
        config.max_binary
    }

    fn get_type_count(counter: &TypeCounter) -> usize {
        counter.num_binary.get()
    }

    fn is_type(record: &EventRecord) -> bool {
        std::matches!(record.event, Event::Binary(_, _))
    }

    fn decrement_type(counter: &mut TypeCounter) {
        counter.num_binary.decrement();
    }

    fn increment_type(counter: &mut TypeCounter) {
        counter.num_binary.increment();
    }

    fn create_event(&self, default_variation: EventBinaryInputVariation) -> Event {
        Event::Binary(*self, Variation::new(default_variation))
    }

    fn select_variation(record: &EventRecord, variation: Self::EventVariation) -> bool {
        if let Event::Binary(_, v) = &record.event {
            v.selected.set(variation);
            true
        } else {
            false
        }
    }
}

impl Insertable for measurement::DoubleBitBinaryInput {
    type EventVariation = EventDoubleBitBinaryInputVariation;

    fn get_max(config: &EventBufferConfig) -> u16 {
        config.max_double_binary
    }

    fn get_type_count(counter: &TypeCounter) -> usize {
        counter.num_double_binary.get()
    }

    fn is_type(record: &EventRecord) -> bool {
        std::matches!(record.event, Event::DoubleBitBinary(_, _))
    }

    fn decrement_type(counter: &mut TypeCounter) {
        counter.num_double_binary.decrement();
    }

    fn increment_type(counter: &mut TypeCounter) {
        counter.num_double_binary.increment();
    }

    fn create_event(&self, default_variation: EventDoubleBitBinaryInputVariation) -> Event {
        Event::DoubleBitBinary(*self, Variation::new(default_variation))
    }

    fn select_variation(record: &EventRecord, variation: Self::EventVariation) -> bool {
        if let Event::DoubleBitBinary(_, v) = &record.event {
            v.selected.set(variation);
            true
        } else {
            false
        }
    }
}

impl Insertable for measurement::BinaryOutputStatus {
    type EventVariation = EventBinaryOutputStatusVariation;

    fn get_max(config: &EventBufferConfig) -> u16 {
        config.max_binary_output_status
    }

    fn get_type_count(counter: &TypeCounter) -> usize {
        counter.num_binary_output_status.get()
    }

    fn is_type(record: &EventRecord) -> bool {
        std::matches!(record.event, Event::BinaryOutputStatus(_, _))
    }

    fn decrement_type(counter: &mut TypeCounter) {
        counter.num_binary_output_status.decrement();
    }

    fn increment_type(counter: &mut TypeCounter) {
        counter.num_binary_output_status.increment();
    }

    fn create_event(&self, default_variation: EventBinaryOutputStatusVariation) -> Event {
        Event::BinaryOutputStatus(*self, Variation::new(default_variation))
    }

    fn select_variation(record: &EventRecord, variation: Self::EventVariation) -> bool {
        if let Event::BinaryOutputStatus(_, v) = &record.event {
            v.selected.set(variation);
            true
        } else {
            false
        }
    }
}

impl Insertable for measurement::Counter {
    type EventVariation = EventCounterVariation;

    fn get_max(config: &EventBufferConfig) -> u16 {
        config.max_counter
    }

    fn get_type_count(counter: &TypeCounter) -> usize {
        counter.num_counter.get()
    }

    fn is_type(record: &EventRecord) -> bool {
        std::matches!(record.event, Event::Counter(_, _))
    }

    fn decrement_type(counter: &mut TypeCounter) {
        counter.num_counter.decrement();
    }

    fn increment_type(counter: &mut TypeCounter) {
        counter.num_counter.increment();
    }

    fn create_event(&self, default_variation: EventCounterVariation) -> Event {
        Event::Counter(*self, Variation::new(default_variation))
    }

    fn select_variation(record: &EventRecord, variation: Self::EventVariation) -> bool {
        if let Event::Counter(_, v) = &record.event {
            v.selected.set(variation);
            true
        } else {
            false
        }
    }
}

impl Insertable for measurement::FrozenCounter {
    type EventVariation = EventFrozenCounterVariation;

    fn get_max(config: &EventBufferConfig) -> u16 {
        config.max_frozen_counter
    }

    fn get_type_count(counter: &TypeCounter) -> usize {
        counter.num_frozen_counter.get()
    }

    fn is_type(record: &EventRecord) -> bool {
        std::matches!(record.event, Event::FrozenCounter(_, _))
    }

    fn decrement_type(counter: &mut TypeCounter) {
        counter.num_frozen_counter.decrement();
    }

    fn increment_type(counter: &mut TypeCounter) {
        counter.num_frozen_counter.increment();
    }

    fn create_event(&self, default_variation: EventFrozenCounterVariation) -> Event {
        Event::FrozenCounter(*self, Variation::new(default_variation))
    }

    fn select_variation(record: &EventRecord, variation: Self::EventVariation) -> bool {
        if let Event::FrozenCounter(_, v) = &record.event {
            v.selected.set(variation);
            true
        } else {
            false
        }
    }
}

impl Insertable for measurement::AnalogInput {
    type EventVariation = EventAnalogInputVariation;

    fn get_max(config: &EventBufferConfig) -> u16 {
        config.max_analog
    }

    fn get_type_count(counter: &TypeCounter) -> usize {
        counter.num_analog.get()
    }

    fn is_type(record: &EventRecord) -> bool {
        std::matches!(record.event, Event::Analog(_, _))
    }

    fn decrement_type(counter: &mut TypeCounter) {
        counter.num_analog.decrement();
    }

    fn increment_type(counter: &mut TypeCounter) {
        counter.num_analog.increment();
    }

    fn create_event(&self, default_variation: EventAnalogInputVariation) -> Event {
        Event::Analog(*self, Variation::new(default_variation))
    }

    fn select_variation(record: &EventRecord, variation: Self::EventVariation) -> bool {
        if let Event::Analog(_, v) = &record.event {
            v.selected.set(variation);
            true
        } else {
            false
        }
    }
}

impl Insertable for measurement::AnalogOutputStatus {
    type EventVariation = EventAnalogOutputStatusVariation;

    fn get_max(config: &EventBufferConfig) -> u16 {
        config.max_analog_output_status
    }

    fn get_type_count(counter: &TypeCounter) -> usize {
        counter.num_analog_output_status.get()
    }

    fn is_type(record: &EventRecord) -> bool {
        std::matches!(record.event, Event::AnalogOutputStatus(_, _))
    }

    fn decrement_type(counter: &mut TypeCounter) {
        counter.num_analog_output_status.decrement();
    }

    fn increment_type(counter: &mut TypeCounter) {
        counter.num_analog_output_status.increment();
    }

    fn create_event(&self, default_variation: EventAnalogOutputStatusVariation) -> Event {
        Event::AnalogOutputStatus(*self, Variation::new(default_variation))
    }

    fn select_variation(record: &EventRecord, variation: Self::EventVariation) -> bool {
        if let Event::AnalogOutputStatus(_, v) = &record.event {
            v.selected.set(variation);
            true
        } else {
            false
        }
    }
}

impl Insertable for measurement::OctetString {
    type EventVariation = EventOctetStringVariation;

    fn get_max(config: &EventBufferConfig) -> u16 {
        config.max_octet_string
    }

    fn get_type_count(counter: &TypeCounter) -> usize {
        counter.num_octet_string.get()
    }

    fn is_type(record: &EventRecord) -> bool {
        std::matches!(record.event, Event::OctetString(_, _))
    }

    fn decrement_type(counter: &mut TypeCounter) {
        counter.num_octet_string.decrement();
    }

    fn increment_type(counter: &mut TypeCounter) {
        counter.num_octet_string.increment();
    }

    fn create_event(&self, default_variation: EventOctetStringVariation) -> Event {
        Event::OctetString(self.as_boxed_slice(), Variation::new(default_variation))
    }

    fn select_variation(record: &EventRecord, variation: Self::EventVariation) -> bool {
        if let Event::OctetString(_, v) = &record.event {
            v.selected.set(variation);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::measurement::*;
    use crate::app::MaybeAsync;
    use std::collections::VecDeque;

    use super::*;

    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    enum Event {
        Clear(u64),
    }

    #[derive(Default)]
    struct MockApplication {
        events: VecDeque<Event>,
    }

    impl OutstationApplication for MockApplication {
        fn begin_confirm(&mut self) {
            unreachable!()
        }
        fn event_cleared(&mut self, id: u64) {
            self.events.push_back(Event::Clear(id));
        }
        fn end_confirm(&mut self, _state: BufferState) -> MaybeAsync<()> {
            unreachable!()
        }
    }

    fn insert_events(buffer: &mut EventBuffer) {
        buffer
            .insert(
                1,
                EventClass::Class1,
                &BinaryInput::new(true, Flags::ONLINE, Time::synchronized(0)),
                EventBinaryInputVariation::Group2Var1,
            )
            .unwrap();

        buffer
            .insert(
                2,
                EventClass::Class2,
                &Counter::new(23, Flags::ONLINE, Time::synchronized(0)),
                EventCounterVariation::Group22Var2,
            )
            .unwrap();

        buffer
            .insert(
                3,
                EventClass::Class3,
                &DoubleBitBinaryInput::new(
                    DoubleBit::DeterminedOn,
                    Flags::ONLINE,
                    Time::synchronized(0),
                ),
                EventDoubleBitBinaryInputVariation::Group4Var3,
            )
            .unwrap();

        buffer
            .insert(
                4,
                EventClass::Class2,
                &BinaryInput::new(true, Flags::ONLINE, Time::synchronized(1234)),
                EventBinaryInputVariation::Group2Var1,
            )
            .unwrap();

        buffer
            .insert(
                5,
                EventClass::Class1,
                &AnalogInput::new(42.0, Flags::ONLINE, Time::synchronized(0)),
                EventAnalogInputVariation::Group32Var1,
            )
            .unwrap();
    }

    #[test]
    fn cannot_insert_if_max_for_type_is_zero() {
        let mut buffer = EventBuffer::new(EventBufferConfig::no_events());

        assert_matches!(
            buffer.insert(
                1,
                EventClass::Class1,
                &BinaryInput::new(true, Flags::ONLINE, Time::synchronized(0)),
                EventBinaryInputVariation::Group2Var1,
            ),
            Err(InsertError::TypeMaxIsZero)
        )
    }

    #[test]
    fn overflows_when_max_for_type_is_exceeded() {
        let mut buffer = EventBuffer::new(EventBufferConfig::all_types(1));

        let binary = BinaryInput::new(true, Flags::ONLINE, Time::synchronized(0));

        buffer
            .insert(
                1,
                EventClass::Class1,
                &binary,
                EventBinaryInputVariation::Group2Var1,
            )
            .unwrap();

        let expected = InsertError::Overflow {
            created: 1,
            discarded: 0,
        };

        assert_eq!(
            buffer.insert(
                1,
                EventClass::Class1,
                &binary,
                EventBinaryInputVariation::Group2Var1
            ),
            Err(expected)
        )
    }

    #[test]
    fn can_select_events_by_class_and_write_some() {
        let mut buffer = EventBuffer::new(EventBufferConfig::all_types(3));

        insert_events(&mut buffer);

        // ignore the class 2 events
        assert_eq!(
            3,
            buffer.select_by_class(EventClass::Class1 | EventClass::Class3, None)
        );

        let mut backing = [0u8; 24];

        let mut mock = MockApplication::default();

        {
            let mut cursor = WriteCursor::new(backing.as_mut());
            assert_eq!(buffer.write_events(&mut cursor), Err(1)); // not enough space to write both events
            let remaining_classes = EventClasses::all();
            assert_eq!(buffer.unwritten_classes(), remaining_classes);
            assert_eq!(buffer.clear_written(&mut mock), 1);
            assert_eq!(buffer.unwritten_classes(), remaining_classes);
        }

        {
            let mut cursor = WriteCursor::new(backing.as_mut());
            assert_eq!(buffer.write_events(&mut cursor), Err(1));
            let remaining_classes = EventClass::Class1 | EventClass::Class2; //  we just wrote the only class 3 event
            assert_eq!(buffer.unwritten_classes(), remaining_classes);
            assert_eq!(buffer.clear_written(&mut mock), 1);
            assert_eq!(buffer.unwritten_classes(), remaining_classes);
        }
    }

    #[test]
    fn can_select_events_by_type() {
        let mut buffer = EventBuffer::new(EventBufferConfig::all_types(3));

        insert_events(&mut buffer);

        // ignore the 2nd binary
        assert_eq!(1, buffer.select_default_variation::<BinaryInput>(Some(1)));

        // select remaining binary events using g2v2
        assert_eq!(
            1,
            buffer.select_specific_variation::<BinaryInput>(
                None,
                EventBinaryInputVariation::Group2Var2
            )
        );

        let mut backing = [0u8; 64];
        let mut cursor = WriteCursor::new(backing.as_mut());

        assert_eq!(2, buffer.write_events(&mut cursor).unwrap());

        assert_eq!(
            cursor.written(),
            [
                // g2v1 (count == 1) (index == 01)
                02, 01, 0x28, 0x01, 0x00, 0x01, 0x00, 0x81,
                // g2v2 (count == 1) (index == 04) (time == 1234 == 0x4D2)
                02, 02, 0x28, 0x01, 0x00, 0x04, 0x00, 0x81, 0xD2, 0x04, 0, 0, 0, 0
            ]
        );

        let mut mock = MockApplication::default();

        assert_eq!(2, buffer.clear_written(&mut mock));
        assert_eq!(mock.events.pop_front(), Some(Event::Clear(0)));
        assert_eq!(mock.events.pop_front(), Some(Event::Clear(3)));
        assert_eq!(mock.events.pop_front(), None);
    }
}
