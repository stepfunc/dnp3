// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::Iin2;
use crate::master::EventClasses;
use crate::outstation::database::details::event::buffer::{EventBuffer, InsertError};
use crate::outstation::database::details::family::{Family, UpdatableFlags};
use crate::outstation::database::details::range::static_db::{PointConfig, StaticDatabase};
use crate::outstation::database::read::{EventTarget, ReadHeader};
use crate::outstation::database::{
    ClassZeroConfig, EventBufferConfig, ResponseInfo, UpdateFlagsType, UpdateInfo, UpdateOptions,
};

use crate::app::measurement::{
    AnalogInput, AnalogOutputStatus, BinaryInput, BinaryOutputStatus, Counter,
    DoubleBitBinaryInput, Flags, FrozenCounter, OctetString, Time,
};
use crate::app::parse::parser::HeaderCollection;
use crate::app::FunctionCode;
use crate::outstation::database::assign::Assignments;
use crate::outstation::database::details::attrs::map::SetMap;
use crate::outstation::database::details::budget::{self, Pool, WorkBudget};
use crate::outstation::database::details::{Visits, WriteProgress};
use crate::outstation::read_log::ReadLog;
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

    /// the event buffer, which the no-panic check drives directly
    #[cfg(all(test, dnp3_no_panic))]
    pub(crate) fn event_buffer(&mut self) -> &mut EventBuffer {
        &mut self.event_buffer
    }

    pub(crate) fn get_attr_map(&mut self) -> &mut SetMap {
        self.attrs.get_attr_map()
    }

    pub(crate) fn reset(&mut self) {
        self.static_db.reset();
        self.event_buffer.reset();
        self.attrs.reset();
    }

    pub(crate) fn set_analog_deadband(&mut self, index: u16, deadband: f64) -> Option<f64> {
        self.static_db.set_analog_deadband(index, deadband)
    }

    pub(crate) fn clear_written_events(
        &mut self,
        app: &mut dyn OutstationApplication,
    ) -> BufferState {
        self.event_buffer.clear_written(|id| app.event_cleared(id));
        self.event_buffer.buffer_state()
    }

    pub(crate) fn unwritten_classes(&self) -> EventClasses {
        self.event_buffer.unwritten_classes()
    }

    pub(crate) fn is_overflown(&self) -> bool {
        self.event_buffer.is_overflown()
    }

    /// Start the selection for a READ from a reset, so that it never inherits what an earlier
    /// selection left selected or written
    ///
    /// A READ whose selection visits more records than its budget allows is rejected after the
    /// header that exceeds it: the selection is reset, so the response has no objects, and only
    /// IIN2.2 is returned
    pub(crate) fn select_read<L: ReadLog>(
        &mut self,
        headers: impl Iterator<Item = ReadHeader>,
    ) -> Iin2 {
        self.reset();
        let mut budget = self.work_budget();
        let mut iin2 = Iin2::default();
        for header in headers {
            let (pool, (header_iin2, visits)) = self.select_by_header::<L>(header);
            if let Err(over) = budget.charge(pool, visits) {
                L::read_over_budget(over);
                self.reset();
                return Iin2::PARAMETER_ERROR;
            }
            iin2 |= header_iin2;
        }
        iin2
    }

    /// The budget of a request, scaled to the points, attribute sets and event capacity
    pub(crate) fn work_budget(&self) -> WorkBudget {
        let static_records = self
            .static_db
            .point_count()
            .saturating_add(self.attrs.set_count());
        WorkBudget::new(static_records, self.event_buffer.capacity())
    }

    /// Apply an ASSIGN_CLASS request: check every assignment against the installed points and the
    /// budget first, then change the class of their points, so a request with any error changes
    /// nothing
    ///
    /// Each assignment is charged for counting and changing its points, and for its callback
    pub(crate) fn assign_classes(&mut self, headers: &HeaderCollection) -> Result<(), Iin2> {
        let mut budget = self.work_budget();
        for assignment in Assignments::new(headers) {
            let visits = self.static_db.check_assignment(assignment?)?;
            budget
                .charge(Pool::Static, visits.plus(Visits::new(1)))
                .map_err(|over| {
                    budget::log_over_budget(FunctionCode::AssignClass, over);
                    Iin2::PARAMETER_ERROR
                })?;
        }
        for assignment in Assignments::new(headers).flatten() {
            self.static_db.assign(assignment);
        }
        Ok(())
    }

    /// Start the selection of the events in `classes` for an unsolicited response from a reset,
    /// returning how many it selected
    pub(crate) fn select_unsolicited(&mut self, classes: EventClasses) -> usize {
        self.reset();
        let (count, _visits) = self.event_buffer.select_by_class(classes, None);
        count
    }

    /// Select what a header asks for, with the pool its work counts against and the records it
    /// visited
    fn select_by_header<L: ReadLog>(&mut self, header: ReadHeader) -> (Pool, (Iin2, Visits)) {
        match header {
            ReadHeader::Class0 => (Pool::Static, self.static_db.select_class_zero()),
            ReadHeader::Static(target, range) => {
                (Pool::Static, self.static_db.select(target, range))
            }
            ReadHeader::Event(target, limit) => (Pool::Event, self.select_events(target, limit)),
            ReadHeader::Attr(header) => (Pool::Static, self.attrs.select::<L>(header)),
        }
    }

    /// Select the events a header asks for, with the records visited
    pub(crate) fn select_events(
        &mut self,
        target: EventTarget,
        limit: Option<usize>,
    ) -> (Iin2, Visits) {
        match target {
            EventTarget::Class(class) => {
                let (_count, visits) = self.event_buffer.select_by_class(class.into(), limit);
                (Iin2::default(), visits)
            }
            EventTarget::Binary(v) => self.select_events_by_type::<BinaryInput>(v, limit),
            EventTarget::DoubleBitBinary(v) => {
                self.select_events_by_type::<DoubleBitBinaryInput>(v, limit)
            }
            EventTarget::BinaryOutputStatus(v) => {
                self.select_events_by_type::<BinaryOutputStatus>(v, limit)
            }
            EventTarget::Counter(v) => self.select_events_by_type::<Counter>(v, limit),
            EventTarget::FrozenCounter(v) => self.select_events_by_type::<FrozenCounter>(v, limit),
            EventTarget::Analog(v) => self.select_events_by_type::<AnalogInput>(v, limit),
            EventTarget::AnalogOutputStatus(v) => {
                self.select_events_by_type::<AnalogOutputStatus>(v, limit)
            }
            EventTarget::OctetString => self.select_events_by_type::<OctetString>(None, limit),
            // we know what it is, but don't support it
            EventTarget::FrozenAnalog(_) => (Iin2::OBJECT_UNKNOWN, Visits::NONE),
        }
    }

    /// Select the events of a type, with IIN2.1 if the type has no points (TB2016-001)
    ///
    /// Events buffered for points that have since been removed are still reported,
    /// without an error. This is independent of how many events the header selected,
    /// because an earlier header in the same request may have selected them already.
    fn select_events_by_type<T>(
        &mut self,
        variation: Option<T::EventVariation>,
        limit: Option<usize>,
    ) -> (Iin2, Visits)
    where
        T: Family,
    {
        let (_count, visits) = self.event_buffer.select_by_type::<T>(variation, limit);
        let iin2 = if !self.static_db.has_points::<T>() && !self.event_buffer.has_events::<T>() {
            Iin2::OBJECT_UNKNOWN
        } else {
            Iin2::default()
        };
        (iin2, visits)
    }

    pub(crate) fn discard_unselected_events(&mut self, classes: EventClasses) -> ClassCount {
        self.event_buffer.remove_unselected_by_class(classes)
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
    pub(crate) fn write_response_headers<L: ReadLog>(
        &mut self,
        cursor: &mut WriteCursor,
    ) -> ResponseInfo {
        let (num_events, progress) = self.write_events(cursor);
        let progress = progress
            .then(|| self.static_db.write(cursor))
            .then(|| self.attrs.write::<L>(cursor));

        ResponseInfo {
            has_events: num_events > 0,
            complete: progress == WriteProgress::Complete,
        }
    }

    /// write the selected events, returning how many and whether they all fit
    pub(crate) fn write_events(&mut self, cursor: &mut WriteCursor) -> (usize, WriteProgress) {
        self.event_buffer.write_events(cursor)
    }

    pub(crate) fn write_events_only(&mut self, cursor: &mut WriteCursor) -> usize {
        // doesn't matter if we wrote all of them or not
        let (num_events, _) = self.event_buffer.write_events(cursor);
        num_events
    }
}
