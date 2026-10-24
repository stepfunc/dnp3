// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use std::fmt::Debug;
use std::ops::BitOr;

use crate::app::measurement::{self};
use crate::master::EventClasses;
use crate::outstation::database::config::*;
use crate::outstation::database::variations::*;
use crate::outstation::database::{EventBufferConfig, EventClass};

use crate::util::BadWrite;

use super::writer::EventWriter;

use crate::outstation::database::details::family::Family;
use crate::outstation::database::details::WriteProgress;
use crate::outstation::{BufferState, ClassCount, OutstationApplication, TypeCount};
use records::{EventRecord, EventState, Events};
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

fn class_count_mut(count: &mut ClassCount, class: EventClass) -> &mut usize {
    match class {
        EventClass::Class1 => &mut count.num_class_1,
        EventClass::Class2 => &mut count.num_class_2,
        EventClass::Class3 => &mut count.num_class_3,
    }
}

fn increment(count: &mut usize) {
    *count = count.saturating_add(1);
}

fn decrement(count: &mut usize) {
    *count = count.saturating_sub(1);
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum EventType {
    Binary,
    DoubleBitBinary,
    BinaryOutputStatus,
    Counter,
    FrozenCounter,
    Analog,
    AnalogOutputStatus,
    OctetString,
}

impl EventType {
    pub(crate) const ALL: [EventType; 8] = [
        EventType::Binary,
        EventType::DoubleBitBinary,
        EventType::BinaryOutputStatus,
        EventType::Counter,
        EventType::FrozenCounter,
        EventType::Analog,
        EventType::AnalogOutputStatus,
        EventType::OctetString,
    ];

    pub(crate) fn max(self, config: &EventBufferConfig) -> usize {
        usize::from(match self {
            EventType::Binary => config.max_binary,
            EventType::DoubleBitBinary => config.max_double_binary,
            EventType::BinaryOutputStatus => config.max_binary_output_status,
            EventType::Counter => config.max_counter,
            EventType::FrozenCounter => config.max_frozen_counter,
            EventType::Analog => config.max_analog,
            EventType::AnalogOutputStatus => config.max_analog_output_status,
            EventType::OctetString => config.max_octet_string,
        })
    }

    fn count_mut(self, count: &mut TypeCount) -> &mut usize {
        match self {
            EventType::Binary => &mut count.num_binary_input,
            EventType::DoubleBitBinary => &mut count.num_double_bit_binary_input,
            EventType::BinaryOutputStatus => &mut count.num_binary_output_status,
            EventType::Counter => &mut count.num_counter,
            EventType::FrozenCounter => &mut count.num_frozen_counter,
            EventType::Analog => &mut count.num_analog,
            EventType::AnalogOutputStatus => &mut count.num_analog_output_status,
            EventType::OctetString => &mut count.num_octet_string,
        }
    }

    fn count(self, mut count: TypeCount) -> usize {
        *self.count_mut(&mut count)
    }
}

/// The event records, and the counts that change with each of their insertions, removals and
/// state changes. Nothing outside this module changes a record's state or a count.
mod records {
    use std::ops::ControlFlow;

    use scursor::WriteCursor;

    use super::super::list::VecList;
    use super::super::writer::EventWriter;
    use super::{class_count_mut, decrement, increment, Event, EventType};
    use crate::master::EventClasses;
    use crate::outstation::database::details::WriteProgress;
    use crate::outstation::database::EventClass;
    use crate::outstation::{BufferState, ClassCount, TypeCount};

    #[derive(Copy, Clone, Debug, PartialEq)]
    pub(super) enum EventState {
        Unselected,
        Selected,
        Written,
    }

    #[derive(Debug, PartialEq)]
    pub(super) struct EventRecord {
        index: u16,
        id: u64,
        class: EventClass,
        event: Event,
        state: EventState,
    }

    impl EventRecord {
        pub(super) fn new(index: u16, id: u64, class: EventClass, event: Event) -> Self {
            Self {
                index,
                id,
                class,
                event,
                state: EventState::Unselected,
            }
        }

        pub(super) fn id(&self) -> u64 {
            self.id
        }

        pub(super) fn class(&self) -> EventClass {
            self.class
        }

        pub(super) fn state(&self) -> EventState {
            self.state
        }
    }

    pub(super) struct Events {
        list: VecList<EventRecord>,
        total: BufferState,
        // records in the Written state
        written: ClassCount,
    }

    impl Events {
        pub(super) fn new(capacity: usize) -> Self {
            Self {
                list: VecList::new(capacity),
                total: BufferState {
                    classes: ClassCount::default(),
                    types: TypeCount {
                        num_binary_input: 0,
                        num_double_bit_binary_input: 0,
                        num_binary_output_status: 0,
                        num_counter: 0,
                        num_frozen_counter: 0,
                        num_analog: 0,
                        num_analog_output_status: 0,
                        num_octet_string: 0,
                    },
                },
                written: ClassCount::default(),
            }
        }

        pub(super) fn total(&self) -> BufferState {
            self.total
        }

        pub(super) fn type_count(&self, event_type: EventType) -> usize {
            event_type.count(self.total.types)
        }

        pub(super) fn unwritten_classes(&self) -> EventClasses {
            let total = self.total.classes;
            let written = self.written;
            EventClasses::new(
                total.num_class_1 > written.num_class_1,
                total.num_class_2 > written.num_class_2,
                total.num_class_3 > written.num_class_3,
            )
        }

        /// append an unselected record
        ///
        /// The list always has room for it. Its capacity is the sum of the type maxima, and
        /// `EventBuffer::insert` pushes a record only when its type holds fewer than its maximum,
        /// after evicting the type's oldest record if the type was full, so the other types
        /// can't fill the list. If the list were full anyway, the record would be dropped and the
        /// counts left as they are, still matching the list.
        pub(super) fn push(&mut self, record: EventRecord) {
            let event_type = record.event.event_type();
            let class = record.class;
            if self.list.add(record) {
                increment(event_type.count_mut(&mut self.total.types));
                increment(class_count_mut(&mut self.total.classes, class));
            }
        }

        /// remove the oldest record of a type, whatever its state, returning its id
        #[cold]
        pub(super) fn remove_oldest(&mut self, event_type: EventType) -> Option<u64> {
            let record = self
                .list
                .remove_first(|r| r.event.event_type() == event_type)?;
            Self::uncount(&mut self.total, &mut self.written, record);
            Some(record.id)
        }

        /// remove every record matching the predicate, passing each one to `removed`
        pub(super) fn remove_all<F, R>(&mut self, predicate: F, mut removed: R) -> usize
        where
            F: FnMut(&EventRecord) -> bool,
            R: FnMut(&EventRecord),
        {
            let total = &mut self.total;
            let written = &mut self.written;
            self.list.remove_all(predicate, |record| {
                Self::uncount(total, written, record);
                removed(record);
            })
        }

        /// select unselected records the selector accepts, oldest first, up to the limit
        pub(super) fn select<F>(&mut self, limit: Option<usize>, mut selector: F) -> usize
        where
            F: FnMut(EventClass, &mut Event) -> bool,
        {
            let limit = limit.unwrap_or(usize::MAX);
            let mut count: usize = 0;
            self.list.for_each_mut(|record| {
                if count >= limit {
                    return ControlFlow::Break(());
                }
                if record.state == EventState::Unselected
                    && selector(record.class, &mut record.event)
                {
                    record.state = EventState::Selected;
                    count = count.saturating_add(1);
                }
                ControlFlow::Continue(())
            });
            count
        }

        /// write the selected records in order until the cursor is full, marking them written,
        /// and return how many were written
        pub(super) fn write(&mut self, cursor: &mut WriteCursor) -> (usize, WriteProgress) {
            let written = &mut self.written;
            let mut writer = EventWriter::new();
            let mut count: usize = 0;
            let mut progress = WriteProgress::Complete;
            self.list.for_each_mut(|record| {
                if record.state != EventState::Selected {
                    return ControlFlow::Continue(());
                }
                if record
                    .event
                    .write(record.index, cursor, &mut writer)
                    .is_err()
                {
                    progress = WriteProgress::Partial;
                    return ControlFlow::Break(());
                }
                Self::set_state(written, record, EventState::Written);
                count = count.saturating_add(1);
                ControlFlow::Continue(())
            });

            (count, progress)
        }

        /// return every record to the unselected state
        pub(super) fn reset(&mut self) {
            let written = &mut self.written;
            self.list.for_each_mut(|record| {
                Self::set_state(written, record, EventState::Unselected);
                ControlFlow::Continue(())
            });
        }

        fn set_state(written: &mut ClassCount, record: &mut EventRecord, state: EventState) {
            let count = class_count_mut(written, record.class);
            if record.state == EventState::Written {
                decrement(count);
            }
            if state == EventState::Written {
                increment(count);
            }
            record.state = state;
        }

        fn uncount(total: &mut BufferState, written: &mut ClassCount, record: &EventRecord) {
            decrement(record.event.event_type().count_mut(&mut total.types));
            decrement(class_count_mut(&mut total.classes, record.class));
            if record.state == EventState::Written {
                decrement(class_count_mut(written, record.class));
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct Variation<T>
where
    T: Copy,
{
    default: T,
    selected: T,
}

impl<T> Variation<T>
where
    T: Copy,
{
    pub(crate) fn new(default: T) -> Self {
        Self {
            default,
            selected: default,
        }
    }

    fn select(&mut self, variation: T) {
        self.selected = variation
    }

    fn select_default(&mut self) {
        self.selected = self.default
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
    fn event_type(&self) -> EventType {
        match self {
            Event::Binary(_, _) => EventType::Binary,
            Event::DoubleBitBinary(_, _) => EventType::DoubleBitBinary,
            Event::BinaryOutputStatus(_, _) => EventType::BinaryOutputStatus,
            Event::Counter(_, _) => EventType::Counter,
            Event::FrozenCounter(_, _) => EventType::FrozenCounter,
            Event::Analog(_, _) => EventType::Analog,
            Event::AnalogOutputStatus(_, _) => EventType::AnalogOutputStatus,
            Event::OctetString(_, _) => EventType::OctetString,
        }
    }

    fn select_default_variation(&mut self) {
        match self {
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

    #[inline]
    fn write(
        &self,
        index: u16,
        cursor: &mut WriteCursor,
        writer: &mut EventWriter,
    ) -> Result<(), BadWrite> {
        match &self {
            Event::Binary(evt, v) => writer.write(cursor, evt, index, v.selected),
            Event::DoubleBitBinary(evt, v) => writer.write(cursor, evt, index, v.selected),
            Event::BinaryOutputStatus(evt, v) => writer.write(cursor, evt, index, v.selected),
            Event::Counter(evt, v) => writer.write(cursor, evt, index, v.selected),
            Event::FrozenCounter(evt, v) => writer.write(cursor, evt, index, v.selected),
            Event::Analog(evt, v) => writer.write(cursor, evt, index, v.selected),
            Event::AnalogOutputStatus(evt, v) => writer.write(cursor, evt, index, v.selected),
            Event::OctetString(evt, v) => writer.write(cursor, evt, index, v.selected),
        }
    }
}

pub(crate) struct EventBuffer {
    config: EventBufferConfig,
    events: Events,
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
            events: Events::new(max_size),
            is_overflown: false,
            next: 0,
        }
    }

    pub(crate) fn unwritten_classes(&self) -> EventClasses {
        self.events.unwritten_classes()
    }

    pub(crate) fn insert<T>(
        &mut self,
        index: u16,
        class: EventClass,
        event: &T,
        default_variation: T::EventVariation,
    ) -> Result<u64, InsertError>
    where
        T: Family,
    {
        let max = T::EVENT_TYPE.max(&self.config);

        if max == 0 {
            return Err(InsertError::TypeMaxIsZero);
        }

        let id = self.next;
        self.next = self.next.wrapping_add(1);

        let discarded = if self.events.type_count(T::EVENT_TYPE) >= max {
            self.events.remove_oldest(T::EVENT_TYPE)
        } else {
            None
        };

        let record = EventRecord::new(index, id, class, event.create_event(default_variation));
        self.events.push(record);

        match discarded {
            Some(discarded) => {
                self.is_overflown = true;
                Err(InsertError::Overflow {
                    created: id,
                    discarded,
                })
            }
            None => Ok(id),
        }
    }

    /// true if the buffer holds at least one event of the type, whatever its state
    pub(crate) fn has_events<T>(&self) -> bool
    where
        T: Family,
    {
        self.events.type_count(T::EVENT_TYPE) > 0
    }

    pub(crate) fn select_by_class(&mut self, classes: EventClasses, limit: Option<usize>) -> usize {
        self.events.select(limit, |class, event| {
            if classes.matches(class) {
                event.select_default_variation();
                true
            } else {
                false
            }
        })
    }

    pub(crate) fn select_by_type<T>(
        &mut self,
        specific_variation: Option<T::EventVariation>,
        limit: Option<usize>,
    ) -> usize
    where
        T: Family,
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
        T: Family,
    {
        self.events
            .select(limit, |_, event| match T::event_variation(event) {
                Some(v) => {
                    v.select(variation);
                    true
                }
                None => false,
            })
    }

    fn select_default_variation<T>(&mut self, limit: Option<usize>) -> usize
    where
        T: Family,
    {
        self.events.select(limit, |_, event| {
            if event.event_type() == T::EVENT_TYPE {
                event.select_default_variation();
                true
            } else {
                false
            }
        })
    }

    /// write the selected events until the cursor is full, returning how many were written
    pub(crate) fn write_events(&mut self, cursor: &mut WriteCursor) -> (usize, WriteProgress) {
        self.events.write(cursor)
    }

    pub(crate) fn clear_written(&mut self, app: &mut dyn OutstationApplication) -> usize {
        let count = self.events.remove_all(
            |record| record.state() == EventState::Written,
            |record| app.event_cleared(record.id()),
        );
        self.update_overflow();
        count
    }

    pub(crate) fn remove_unselected_by_class(&mut self, classes: EventClasses) -> ClassCount {
        let mut count = ClassCount::default();
        self.events.remove_all(
            |record| record.state() == EventState::Unselected && classes.matches(record.class()),
            |record| increment(class_count_mut(&mut count, record.class())),
        );
        self.update_overflow();
        count
    }

    pub(crate) fn buffer_state(&self) -> BufferState {
        self.events.total()
    }

    pub(crate) fn reset(&mut self) {
        self.events.reset();
    }

    pub(crate) fn is_overflown(&self) -> bool {
        self.is_overflown
    }

    /// the overflow persists until no type is full
    fn update_overflow(&mut self) {
        let is_any_full = EventType::ALL.into_iter().any(|event_type| {
            let max = event_type.max(&self.config);
            max > 0 && self.events.type_count(event_type) >= max
        });

        if !is_any_full {
            self.is_overflown = false;
        }
    }
}
