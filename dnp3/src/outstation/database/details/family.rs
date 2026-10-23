// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::measurement::*;
use crate::outstation::database::config::*;
use crate::outstation::database::details::event::buffer::{Event, EventType, Variation};
use crate::outstation::database::details::range::static_db::{
    Deadband, EventDetector, FlagsDetector, OctetStringDetector, PointMap, SpecificVariation,
    StaticDatabase,
};
use crate::outstation::database::details::range::traits::StaticVariation;
use crate::outstation::database::ClassZeroConfig;

/// A measurement type, with everything the database associates with it: its event, its
/// variations, how it detects events, its static points and whether class 0 reports them
pub(crate) trait Family: Clone + Default {
    type EventVariation: Copy;
    type StaticVariation: StaticVariation<Self>;
    type Detector: EventDetector<Self>;

    const EVENT_TYPE: EventType;

    /// the event for the value, in its default variation
    fn create_event(&self, default_variation: Self::EventVariation) -> Event;
    /// the variation of the event, if it's of this type
    fn event_variation(event: &mut Event) -> Option<&mut Variation<Self::EventVariation>>;

    fn points(db: &StaticDatabase) -> &PointMap<Self>;
    fn points_mut(db: &mut StaticDatabase) -> &mut PointMap<Self>;
    /// a selection of the points in the variation a READ asked for, if any
    fn selection(variation: Option<Self::StaticVariation>) -> SpecificVariation;
    fn in_class_zero(config: &ClassZeroConfig) -> bool;
}

/// A measurement type whose flags and time can change without its value
pub(crate) trait UpdatableFlags: Family {
    fn update_flags(&mut self, flags: Flags, time: Option<Time>);
}

impl Family for BinaryInput {
    type EventVariation = EventBinaryInputVariation;
    type StaticVariation = StaticBinaryInputVariation;
    type Detector = FlagsDetector;

    const EVENT_TYPE: EventType = EventType::Binary;

    fn create_event(&self, default_variation: Self::EventVariation) -> Event {
        Event::Binary(*self, Variation::new(default_variation))
    }

    fn event_variation(event: &mut Event) -> Option<&mut Variation<Self::EventVariation>> {
        if let Event::Binary(_, v) = event {
            Some(v)
        } else {
            None
        }
    }

    fn points(db: &StaticDatabase) -> &PointMap<Self> {
        &db.binary
    }

    fn points_mut(db: &mut StaticDatabase) -> &mut PointMap<Self> {
        &mut db.binary
    }

    fn selection(variation: Option<Self::StaticVariation>) -> SpecificVariation {
        SpecificVariation::Binary(variation)
    }

    fn in_class_zero(config: &ClassZeroConfig) -> bool {
        config.binary
    }
}

impl UpdatableFlags for BinaryInput {
    fn update_flags(&mut self, flags: Flags, time: Option<Time>) {
        self.flags = flags;
        self.time = time;
    }
}

impl Default for BinaryInput {
    fn default() -> Self {
        Self::new(false, Flags::RESTART, Time::unsynchronized(0))
    }
}

impl Family for DoubleBitBinaryInput {
    type EventVariation = EventDoubleBitBinaryInputVariation;
    type StaticVariation = StaticDoubleBitBinaryInputVariation;
    type Detector = FlagsDetector;

    const EVENT_TYPE: EventType = EventType::DoubleBitBinary;

    fn create_event(&self, default_variation: Self::EventVariation) -> Event {
        Event::DoubleBitBinary(*self, Variation::new(default_variation))
    }

    fn event_variation(event: &mut Event) -> Option<&mut Variation<Self::EventVariation>> {
        if let Event::DoubleBitBinary(_, v) = event {
            Some(v)
        } else {
            None
        }
    }

    fn points(db: &StaticDatabase) -> &PointMap<Self> {
        &db.double_bit_binary
    }

    fn points_mut(db: &mut StaticDatabase) -> &mut PointMap<Self> {
        &mut db.double_bit_binary
    }

    fn selection(variation: Option<Self::StaticVariation>) -> SpecificVariation {
        SpecificVariation::DoubleBitBinary(variation)
    }

    fn in_class_zero(config: &ClassZeroConfig) -> bool {
        config.double_bit_binary
    }
}

impl UpdatableFlags for DoubleBitBinaryInput {
    fn update_flags(&mut self, flags: Flags, time: Option<Time>) {
        self.flags = flags;
        self.time = time;
    }
}

impl Default for DoubleBitBinaryInput {
    fn default() -> Self {
        Self::new(
            DoubleBit::Indeterminate,
            Flags::RESTART,
            Time::unsynchronized(0),
        )
    }
}

impl Family for BinaryOutputStatus {
    type EventVariation = EventBinaryOutputStatusVariation;
    type StaticVariation = StaticBinaryOutputStatusVariation;
    type Detector = FlagsDetector;

    const EVENT_TYPE: EventType = EventType::BinaryOutputStatus;

    fn create_event(&self, default_variation: Self::EventVariation) -> Event {
        Event::BinaryOutputStatus(*self, Variation::new(default_variation))
    }

    fn event_variation(event: &mut Event) -> Option<&mut Variation<Self::EventVariation>> {
        if let Event::BinaryOutputStatus(_, v) = event {
            Some(v)
        } else {
            None
        }
    }

    fn points(db: &StaticDatabase) -> &PointMap<Self> {
        &db.binary_output_status
    }

    fn points_mut(db: &mut StaticDatabase) -> &mut PointMap<Self> {
        &mut db.binary_output_status
    }

    fn selection(variation: Option<Self::StaticVariation>) -> SpecificVariation {
        SpecificVariation::BinaryOutputStatus(variation)
    }

    fn in_class_zero(config: &ClassZeroConfig) -> bool {
        config.binary_output_status
    }
}

impl UpdatableFlags for BinaryOutputStatus {
    fn update_flags(&mut self, flags: Flags, time: Option<Time>) {
        self.flags = flags;
        self.time = time;
    }
}

impl Default for BinaryOutputStatus {
    fn default() -> Self {
        Self::new(false, Flags::RESTART, Time::unsynchronized(0))
    }
}

impl Family for Counter {
    type EventVariation = EventCounterVariation;
    type StaticVariation = StaticCounterVariation;
    type Detector = Deadband<u32>;

    const EVENT_TYPE: EventType = EventType::Counter;

    fn create_event(&self, default_variation: Self::EventVariation) -> Event {
        Event::Counter(*self, Variation::new(default_variation))
    }

    fn event_variation(event: &mut Event) -> Option<&mut Variation<Self::EventVariation>> {
        if let Event::Counter(_, v) = event {
            Some(v)
        } else {
            None
        }
    }

    fn points(db: &StaticDatabase) -> &PointMap<Self> {
        &db.counter
    }

    fn points_mut(db: &mut StaticDatabase) -> &mut PointMap<Self> {
        &mut db.counter
    }

    fn selection(variation: Option<Self::StaticVariation>) -> SpecificVariation {
        SpecificVariation::Counter(variation)
    }

    fn in_class_zero(config: &ClassZeroConfig) -> bool {
        config.counter
    }
}

impl UpdatableFlags for Counter {
    fn update_flags(&mut self, flags: Flags, time: Option<Time>) {
        self.flags = flags;
        self.time = time;
    }
}

impl Default for Counter {
    fn default() -> Self {
        Self::new(0, Flags::RESTART, Time::unsynchronized(0))
    }
}

impl Family for FrozenCounter {
    type EventVariation = EventFrozenCounterVariation;
    type StaticVariation = StaticFrozenCounterVariation;
    type Detector = Deadband<u32>;

    const EVENT_TYPE: EventType = EventType::FrozenCounter;

    fn create_event(&self, default_variation: Self::EventVariation) -> Event {
        Event::FrozenCounter(*self, Variation::new(default_variation))
    }

    fn event_variation(event: &mut Event) -> Option<&mut Variation<Self::EventVariation>> {
        if let Event::FrozenCounter(_, v) = event {
            Some(v)
        } else {
            None
        }
    }

    fn points(db: &StaticDatabase) -> &PointMap<Self> {
        &db.frozen_counter
    }

    fn points_mut(db: &mut StaticDatabase) -> &mut PointMap<Self> {
        &mut db.frozen_counter
    }

    fn selection(variation: Option<Self::StaticVariation>) -> SpecificVariation {
        SpecificVariation::FrozenCounter(variation)
    }

    fn in_class_zero(config: &ClassZeroConfig) -> bool {
        config.frozen_counter
    }
}

impl UpdatableFlags for FrozenCounter {
    fn update_flags(&mut self, flags: Flags, time: Option<Time>) {
        self.flags = flags;
        self.time = time;
    }
}

impl Default for FrozenCounter {
    fn default() -> Self {
        Self::new(0, Flags::RESTART, Time::unsynchronized(0))
    }
}

impl Family for AnalogInput {
    type EventVariation = EventAnalogInputVariation;
    type StaticVariation = StaticAnalogInputVariation;
    type Detector = Deadband<f64>;

    const EVENT_TYPE: EventType = EventType::Analog;

    fn create_event(&self, default_variation: Self::EventVariation) -> Event {
        Event::Analog(*self, Variation::new(default_variation))
    }

    fn event_variation(event: &mut Event) -> Option<&mut Variation<Self::EventVariation>> {
        if let Event::Analog(_, v) = event {
            Some(v)
        } else {
            None
        }
    }

    fn points(db: &StaticDatabase) -> &PointMap<Self> {
        &db.analog
    }

    fn points_mut(db: &mut StaticDatabase) -> &mut PointMap<Self> {
        &mut db.analog
    }

    fn selection(variation: Option<Self::StaticVariation>) -> SpecificVariation {
        SpecificVariation::Analog(variation)
    }

    fn in_class_zero(config: &ClassZeroConfig) -> bool {
        config.analog
    }
}

impl UpdatableFlags for AnalogInput {
    fn update_flags(&mut self, flags: Flags, time: Option<Time>) {
        self.flags = flags;
        self.time = time;
    }
}

impl Default for AnalogInput {
    fn default() -> Self {
        Self::new(0.0, Flags::RESTART, Time::unsynchronized(0))
    }
}

impl Family for AnalogOutputStatus {
    type EventVariation = EventAnalogOutputStatusVariation;
    type StaticVariation = StaticAnalogOutputStatusVariation;
    type Detector = Deadband<f64>;

    const EVENT_TYPE: EventType = EventType::AnalogOutputStatus;

    fn create_event(&self, default_variation: Self::EventVariation) -> Event {
        Event::AnalogOutputStatus(*self, Variation::new(default_variation))
    }

    fn event_variation(event: &mut Event) -> Option<&mut Variation<Self::EventVariation>> {
        if let Event::AnalogOutputStatus(_, v) = event {
            Some(v)
        } else {
            None
        }
    }

    fn points(db: &StaticDatabase) -> &PointMap<Self> {
        &db.analog_output_status
    }

    fn points_mut(db: &mut StaticDatabase) -> &mut PointMap<Self> {
        &mut db.analog_output_status
    }

    fn selection(variation: Option<Self::StaticVariation>) -> SpecificVariation {
        SpecificVariation::AnalogOutputStatus(variation)
    }

    fn in_class_zero(config: &ClassZeroConfig) -> bool {
        config.analog_output_status
    }
}

impl UpdatableFlags for AnalogOutputStatus {
    fn update_flags(&mut self, flags: Flags, time: Option<Time>) {
        self.flags = flags;
        self.time = time;
    }
}

impl Default for AnalogOutputStatus {
    fn default() -> Self {
        Self::new(0.0, Flags::RESTART, Time::unsynchronized(0))
    }
}

/// The event buffer stores just the bytes of an octet string, rather than its full-size array
impl Family for OctetString {
    type EventVariation = EventOctetStringVariation;
    type StaticVariation = StaticOctetStringVariation;
    type Detector = OctetStringDetector;

    const EVENT_TYPE: EventType = EventType::OctetString;

    fn create_event(&self, default_variation: Self::EventVariation) -> Event {
        Event::OctetString(self.as_boxed_slice(), Variation::new(default_variation))
    }

    fn event_variation(event: &mut Event) -> Option<&mut Variation<Self::EventVariation>> {
        if let Event::OctetString(_, v) = event {
            Some(v)
        } else {
            None
        }
    }

    fn points(db: &StaticDatabase) -> &PointMap<Self> {
        &db.octet_strings
    }

    fn points_mut(db: &mut StaticDatabase) -> &mut PointMap<Self> {
        &mut db.octet_strings
    }

    // the variation is the length of each octet string
    fn selection(_variation: Option<Self::StaticVariation>) -> SpecificVariation {
        SpecificVariation::OctetString
    }

    fn in_class_zero(config: &ClassZeroConfig) -> bool {
        config.octet_string
    }
}
