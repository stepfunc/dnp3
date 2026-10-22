use crate::app::Iin2;
use crate::master::EventClasses;
use crate::outstation::database::details::event::buffer::{EventBuffer, InsertError};
use crate::outstation::database::details::family::{Family, UpdatableFlags};
use crate::outstation::database::details::range::static_db::{PointConfig, StaticDatabase};
use crate::outstation::database::read::{EventReadHeader, ReadHeader};
use crate::outstation::database::{
    ClassZeroConfig, EventBufferConfig, ResponseInfo, UpdateFlagsType, UpdateInfo, UpdateOptions,
};

use crate::app::measurement::{
    AnalogInput, AnalogOutputStatus, BinaryInput, BinaryOutputStatus, Counter,
    DoubleBitBinaryInput, Flags, FrozenCounter, OctetString, Time,
};
use crate::outstation::database::details::attrs::map::SetMap;
use crate::outstation::database::details::WriteProgress;
use crate::outstation::{BufferState, ClassCount, OutstationApplication};
use scursor::WriteCursor;

pub(crate) struct Database {
    static_db: StaticDatabase,
    event_buffer: EventBuffer,
    attrs: super::attrs::AttrHandler,
}

impl Database {
    pub(crate) fn new(
        max_read_selection: u16,
        class_zero_config: ClassZeroConfig,
        config: EventBufferConfig,
    ) -> Self {
        Self {
            static_db: StaticDatabase::new(max_read_selection, class_zero_config),
            event_buffer: EventBuffer::new(config),
            attrs: super::attrs::AttrHandler::new(super::attrs::AttrHandler::MAX_SELECTED),
        }
    }

    pub(crate) fn get_attr_map(&mut self) -> &mut SetMap {
        self.attrs.get_attr_map()
    }

    pub(crate) fn reset(&mut self) {
        self.static_db.reset();
        self.event_buffer.reset();
        self.attrs.reset();
    }

    pub(crate) fn set_analog_deadband(&mut self, index: u16, deadband: f64) -> bool {
        self.static_db.set_analog_deadband(index, deadband)
    }

    pub(crate) fn clear_written_events(
        &mut self,
        app: &mut dyn OutstationApplication,
    ) -> BufferState {
        self.event_buffer.clear_written(app);
        self.event_buffer.buffer_state()
    }

    pub(crate) fn unwritten_classes(&self) -> EventClasses {
        self.event_buffer.unwritten_classes()
    }

    pub(crate) fn is_overflown(&self) -> bool {
        self.event_buffer.is_overflown()
    }

    pub(crate) fn select_by_header(&mut self, header: ReadHeader) -> Iin2 {
        match header {
            ReadHeader::Static(header) => self.static_db.select(header),
            ReadHeader::Event(header) => {
                self.event_buffer.select_by_header(header);
                self.event_type_iin(header)
            }
            ReadHeader::Attr(header) => self.attrs.select(header),
        }
    }

    /// IIN2.1 for a read of an event type with no points (TB2016-001)
    ///
    /// Events buffered for points that have since been removed are still reported,
    /// without an error. This is independent of how many events the header selected,
    /// because an earlier header in the same request may have selected them already.
    fn event_type_iin(&self, header: EventReadHeader) -> Iin2 {
        let missing = match header {
            EventReadHeader::Class1(_)
            | EventReadHeader::Class2(_)
            | EventReadHeader::Class3(_) => false,
            EventReadHeader::Binary(_, _) => self.is_missing::<BinaryInput>(),
            EventReadHeader::DoubleBitBinary(_, _) => self.is_missing::<DoubleBitBinaryInput>(),
            EventReadHeader::BinaryOutputStatus(_, _) => self.is_missing::<BinaryOutputStatus>(),
            EventReadHeader::Counter(_, _) => self.is_missing::<Counter>(),
            EventReadHeader::FrozenCounter(_, _) => self.is_missing::<FrozenCounter>(),
            EventReadHeader::Analog(_, _) => self.is_missing::<AnalogInput>(),
            EventReadHeader::AnalogOutputStatus(_, _) => self.is_missing::<AnalogOutputStatus>(),
            EventReadHeader::OctetString(_) => self.is_missing::<OctetString>(),
            // we know what it is, but don't support it
            EventReadHeader::FrozenAnalog(_, _) => true,
        };

        if missing {
            Iin2::OBJECT_UNKNOWN
        } else {
            Iin2::default()
        }
    }

    fn is_missing<T>(&self) -> bool
    where
        T: Family,
    {
        !self.static_db.has_points::<T>() && !self.event_buffer.has_events::<T>()
    }

    pub(crate) fn discard_unselected_events(&mut self, classes: EventClasses) -> ClassCount {
        self.event_buffer.remove_unselected_by_class(classes)
    }

    pub(crate) fn select_event_classes(&mut self, classes: EventClasses) -> usize {
        self.event_buffer.select_by_class(classes, None)
    }

    pub(crate) fn add<T>(&mut self, index: u16, config: PointConfig<T>) -> bool
    where
        T: Family,
    {
        self.static_db.add(index, config)
    }

    pub(crate) fn remove<T>(&mut self, index: u16) -> bool
    where
        T: Family,
    {
        self.static_db.remove::<T>(index)
    }

    pub(crate) fn get<T>(&self, index: u16) -> Option<T>
    where
        T: Family,
    {
        self.static_db.get::<T>(index)
    }

    pub(crate) fn update_flags(
        &mut self,
        index: u16,
        flags_type: UpdateFlagsType,
        flags: Flags,
        time: Option<Time>,
        options: UpdateOptions,
    ) -> UpdateInfo {
        match flags_type {
            UpdateFlagsType::BinaryInput => {
                self.update_flags_by_type::<BinaryInput>(index, flags, time, options)
            }
            UpdateFlagsType::DoubleBitBinaryInput => {
                self.update_flags_by_type::<DoubleBitBinaryInput>(index, flags, time, options)
            }
            UpdateFlagsType::BinaryOutputStatus => {
                self.update_flags_by_type::<BinaryOutputStatus>(index, flags, time, options)
            }
            UpdateFlagsType::Counter => {
                self.update_flags_by_type::<Counter>(index, flags, time, options)
            }
            UpdateFlagsType::FrozenCounter => {
                self.update_flags_by_type::<FrozenCounter>(index, flags, time, options)
            }
            UpdateFlagsType::AnalogInput => {
                self.update_flags_by_type::<AnalogInput>(index, flags, time, options)
            }
            UpdateFlagsType::AnalogOutputStatus => {
                self.update_flags_by_type::<AnalogOutputStatus>(index, flags, time, options)
            }
        }
    }

    pub(crate) fn update_flags_by_type<T: UpdatableFlags>(
        &mut self,
        index: u16,
        flags: Flags,
        timestamp: Option<Time>,
        options: UpdateOptions,
    ) -> UpdateInfo {
        match self.static_db.get::<T>(index) {
            None => UpdateInfo::NoPoint,
            Some(mut x) => {
                x.update_flags(flags, timestamp);
                self.update::<T>(&x, index, options)
            }
        }
    }

    pub(crate) fn update<T>(&mut self, value: &T, index: u16, options: UpdateOptions) -> UpdateInfo
    where
        T: Family,
    {
        let (exists, event_data) = self.static_db.update(value, index, options);

        // if an event should be produced, insert it into the buffer
        if let Some((variation, class)) = event_data {
            // Overflow is handled in the event buffer
            return match self.event_buffer.insert(index, class, value, variation) {
                Ok(x) => UpdateInfo::Created(x),
                Err(InsertError::TypeMaxIsZero) => UpdateInfo::NoEvent,
                Err(InsertError::Overflow { created, discarded }) => {
                    UpdateInfo::Overflow { created, discarded }
                }
            };
        }

        if exists {
            UpdateInfo::NoEvent
        } else {
            UpdateInfo::NoPoint
        }
    }

    /// write events, then static data, then attributes, each only once the part before it is
    /// complete
    pub(crate) fn write_response_headers(&mut self, cursor: &mut WriteCursor) -> ResponseInfo {
        let (num_events, progress) = self.event_buffer.write_events(cursor);
        let progress = progress
            .then(|| self.static_db.write(cursor))
            .then(|| self.attrs.write(cursor));

        ResponseInfo {
            has_events: num_events > 0,
            complete: progress == WriteProgress::Complete,
        }
    }

    pub(crate) fn write_events_only(&mut self, cursor: &mut WriteCursor) -> usize {
        // doesn't matter if we wrote all of them or not
        let (num_events, _) = self.event_buffer.write_events(cursor);
        num_events
    }
}
