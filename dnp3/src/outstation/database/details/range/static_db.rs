use std::collections::{BTreeMap, Bound, VecDeque};
use std::ops::RangeBounds;

use crate::app::measurement::*;
use crate::app::Iin2;
use crate::outstation::config::OutstationConfig;
use crate::outstation::database::config::*;
use crate::outstation::database::details::event::buffer::Insertable;
use crate::outstation::database::details::range::traits::StaticVariation;
use crate::outstation::database::details::range::writer::RangeWriter;
use crate::outstation::database::read::StaticReadHeader;
use crate::outstation::database::{ClassZeroConfig, EventClass, EventMode, UpdateOptions};
use crate::util::cursor::{WriteCursor, WriteError};

pub(crate) trait EventDetector<T>
where
    T: Updatable,
{
    fn is_event(&self, new: &T, old: &T) -> bool;
}

pub(crate) trait Updatable: Insertable + Clone + Default {
    type StaticVariation: StaticVariation<Self>;
    type Detector: EventDetector<Self>;
    fn get_map(maps: &mut StaticDatabase) -> &mut PointMap<Self>;
    fn wrap(range: IndexRange, variation: Option<Self::StaticVariation>) -> VariationRange;
    fn enabled_class_zero(config: &ClassZeroConfig) -> bool;
}

#[derive(Copy, Clone)]
pub(crate) struct IndexRange {
    start: u16,
    stop: u16,
}

impl IndexRange {
    pub(crate) fn new(start: u16, stop: u16) -> Self {
        Self { start, stop }
    }
}

impl RangeBounds<u16> for IndexRange {
    fn start_bound(&self) -> Bound<&u16> {
        Bound::Included(&self.start)
    }
    fn end_bound(&self) -> Bound<&u16> {
        Bound::Included(&self.stop)
    }
}

#[derive(Copy, Clone)]
pub(crate) struct VariationRange {
    range: IndexRange,
    variation: SpecificVariation,
}

impl VariationRange {
    fn new(range: IndexRange, variation: SpecificVariation) -> Self {
        Self { range, variation }
    }
}

#[derive(Copy, Clone)]
pub(crate) enum SpecificVariation {
    Binary(Option<StaticBinaryVariation>),
    DoubleBitBinary(Option<StaticDoubleBitBinaryVariation>),
    BinaryOutputStatus(Option<StaticBinaryOutputStatusVariation>),
    Counter(Option<StaticCounterVariation>),
    FrozenCounter(Option<StaticFrozenCounterVariation>),
    Analog(Option<StaticAnalogVariation>),
    AnalogOutputStatus(Option<StaticAnalogOutputStatusVariation>),
    OctetString,
}

impl SpecificVariation {
    fn with(self, range: IndexRange) -> VariationRange {
        VariationRange::new(range, self)
    }
}

struct SelectionQueue {
    queue: VecDeque<VariationRange>,
    capacity_exceeded: usize,
}

impl SelectionQueue {
    fn new(max_selections: u16) -> Self {
        Self {
            queue: VecDeque::with_capacity(max_selections as usize),
            capacity_exceeded: 0,
        }
    }

    fn peek(&self) -> Option<VariationRange> {
        self.queue.front().copied()
    }

    fn pop(&mut self) {
        self.queue.pop_front();
    }

    fn update_front(&mut self, range: VariationRange) -> bool {
        if let Some(front) = self.queue.front_mut() {
            *front = range;
            true
        } else {
            false
        }
    }

    fn push_back(&mut self, range: VariationRange) -> bool {
        if self.queue.len() == self.queue.capacity() {
            self.capacity_exceeded += 1;
            return false;
        }
        self.queue.push_back(range);
        true
    }

    fn reset(&mut self) {
        self.capacity_exceeded = 0;
        self.queue.clear();
    }
}

pub(crate) struct PointConfig<T>
where
    T: Updatable,
{
    class: Option<EventClass>,
    detector: T::Detector,
    s_var: T::StaticVariation,
    e_var: T::EventVariation,
}

impl<T> PointConfig<T>
where
    T: Updatable,
{
    pub(crate) fn new(
        class: Option<EventClass>,
        detector: T::Detector,
        s_var: T::StaticVariation,
        e_var: T::EventVariation,
    ) -> Self {
        Self {
            class,
            detector,
            s_var,
            e_var,
        }
    }
}

pub(crate) struct Point<T>
where
    T: Updatable,
{
    // current value
    current: T,
    // value that is frozen during READ requests to be reported
    selected: T,
    // last value that produced an event
    last_event: T,
    // configuration
    config: PointConfig<T>,
}

impl<T> Point<T>
where
    T: Updatable + Default,
{
    pub(crate) fn new(config: PointConfig<T>) -> Self {
        Self {
            current: T::default(),
            selected: T::default(),
            last_event: T::default(),
            config,
        }
    }
}

pub(crate) struct PointMap<T>
where
    T: Updatable,
{
    inner: BTreeMap<u16, Point<T>>,
}

impl<T> PointMap<T>
where
    T: Updatable,
{
    fn empty() -> Self {
        Self {
            inner: BTreeMap::new(),
        }
    }

    fn get_mut(&mut self, index: u16) -> Option<&mut Point<T>> {
        self.inner.get_mut(&index)
    }

    fn select_all(&mut self) -> Option<VariationRange> {
        self.select_all_with_variation(None)
    }

    fn select_all_with_variation(
        &mut self,
        variation: Option<T::StaticVariation>,
    ) -> Option<VariationRange> {
        self.inner
            .values_mut()
            .for_each(|x| x.selected = x.current.clone());

        /*
          TODO - when this feature is stabilized we can use it here
          TODO - #![feature(map_first_last)]
          TODO - It'll be more efficient than finding first/last using iterators
        */
        let start = self.inner.iter().next().map(|(key, _)| *key)?;
        let stop = self.inner.iter().next_back().map(|(key, _)| *key)?;
        Some(T::wrap(IndexRange::new(start, stop), variation))
    }
}

pub(crate) struct StaticDatabase {
    class_zero: ClassZeroConfig,
    selected: SelectionQueue,
    // maps for the various types
    binary: PointMap<Binary>,
    double_bit_binary: PointMap<DoubleBitBinary>,
    binary_output_status: PointMap<BinaryOutputStatus>,
    counter: PointMap<Counter>,
    frozen_counter: PointMap<FrozenCounter>,
    analog: PointMap<Analog>,
    analog_output_status: PointMap<AnalogOutputStatus>,
    octet_strings: PointMap<OctetString>,
}

impl Default for StaticDatabase {
    fn default() -> Self {
        Self::new(None, ClassZeroConfig::default())
    }
}

impl StaticDatabase {
    pub(crate) fn new(max_read_selection: Option<u16>, class_zero: ClassZeroConfig) -> Self {
        // don't allow values smaller than the default
        let max_read_selection = max_read_selection
            .map(|x| x.max(OutstationConfig::DEFAULT_MAX_READ_REQUEST_HEADERS))
            .unwrap_or(OutstationConfig::DEFAULT_MAX_READ_REQUEST_HEADERS);

        Self {
            class_zero,
            selected: SelectionQueue::new(max_read_selection),
            binary: PointMap::empty(),
            double_bit_binary: PointMap::empty(),
            binary_output_status: PointMap::empty(),
            counter: PointMap::empty(),
            frozen_counter: PointMap::empty(),
            analog: PointMap::empty(),
            analog_output_status: PointMap::empty(),
            octet_strings: PointMap::empty(),
        }
    }
    /*
       pub(crate) fn exceeded_capacity(&self) -> Option<usize> {
           if self.selected.capacity_exceeded > 0 {
               Some(self.selected.capacity_exceeded)
           } else {
               None
           }
       }
    */

    #[cfg(test)]
    pub(crate) fn selection_capacity(&self) -> usize {
        self.selected.queue.capacity()
    }

    pub(crate) fn reset(&mut self) {
        self.selected.reset();
    }

    pub(crate) fn add<T>(&mut self, index: u16, config: PointConfig<T>) -> bool
    where
        T: Updatable,
    {
        let map = self.get_map::<T>();

        if map.inner.contains_key(&index) {
            return false;
        }

        map.inner.insert(index, Point::new(config));

        true
    }

    pub(crate) fn remove<T>(&mut self, index: u16) -> bool
    where
        T: Updatable,
    {
        self.get_map::<T>().inner.remove(&index).is_some()
    }

    pub(crate) fn update<T>(
        &mut self,
        value: &T,
        index: u16,
        options: UpdateOptions,
    ) -> (bool, Option<(T::EventVariation, EventClass)>)
    where
        T: Updatable,
    {
        match self.get_map::<T>().get_mut(index) {
            None => (false, None),
            Some(x) => {
                if options.update_static {
                    x.current = value.clone();
                }

                // event detection
                let event = match options.event_mode {
                    EventMode::Suppress => None,
                    EventMode::Force => {
                        x.last_event = value.clone();
                        x.config.class.map(|ec| (x.config.e_var, ec))
                    }
                    EventMode::Detect => {
                        if x.config.detector.is_event(&x.last_event, &value) {
                            x.last_event = value.clone();
                            x.config.class.map(|ec| (x.config.e_var, ec))
                        } else {
                            None
                        }
                    }
                };

                (true, event)
            }
        }
    }

    pub(crate) fn write(&mut self, cursor: &mut WriteCursor) -> Result<(), WriteError> {
        while let Some(range) = self.selected.peek() {
            match self.write_range(cursor, range) {
                // done with this header
                Ok(()) => {
                    self.selected.pop();
                }
                // ran out of space
                Err(s) => {
                    // possibly wrote some data though, so update the selection
                    self.selected.update_front(s);
                    return Err(WriteError);
                }
            };
        }

        Ok(())
    }

    fn write_range(
        &mut self,
        cursor: &mut WriteCursor,
        range: VariationRange,
    ) -> Result<(), VariationRange> {
        match range.variation {
            SpecificVariation::Binary(var) => {
                self.write_typed_range::<Binary>(cursor, range.range, var)
            }
            SpecificVariation::DoubleBitBinary(var) => {
                self.write_typed_range::<DoubleBitBinary>(cursor, range.range, var)
            }
            SpecificVariation::BinaryOutputStatus(var) => {
                self.write_typed_range::<BinaryOutputStatus>(cursor, range.range, var)
            }
            SpecificVariation::Counter(var) => {
                self.write_typed_range::<Counter>(cursor, range.range, var)
            }
            SpecificVariation::FrozenCounter(var) => {
                self.write_typed_range::<FrozenCounter>(cursor, range.range, var)
            }
            SpecificVariation::Analog(var) => {
                self.write_typed_range::<Analog>(cursor, range.range, var)
            }
            SpecificVariation::AnalogOutputStatus(var) => {
                self.write_typed_range::<AnalogOutputStatus>(cursor, range.range, var)
            }
            SpecificVariation::OctetString => {
                self.write_typed_range::<OctetString>(cursor, range.range, None)
            }
        }
    }

    fn write_typed_range<T>(
        &mut self,
        cursor: &mut WriteCursor,
        range: IndexRange,
        variation: Option<T::StaticVariation>,
    ) -> Result<(), VariationRange>
    where
        T: Updatable,
    {
        let mut writer = RangeWriter::new();
        for (index, item) in self.get_map::<T>().inner.range(range) {
            // first determine what variation should be written
            let info = variation
                .unwrap_or(item.config.s_var)
                .promote(&item.selected)
                .get_write_info(&item.selected);

            if writer.write(cursor, *index, &item.selected, info).is_err() {
                // ran out of space, tell calling code to resume at this index
                return Err(T::wrap(IndexRange::new(*index, range.stop), variation));
            }
        }

        Ok(())
    }

    pub(crate) fn select(&mut self, variation: StaticReadHeader) -> Iin2 {
        match variation {
            StaticReadHeader::Class0 => self.select_class_zero(),
            StaticReadHeader::Binary(variation, range) => {
                self.select_by_type::<Binary>(variation, range)
            }
            StaticReadHeader::DoubleBitBinary(variation, range) => {
                self.select_by_type::<DoubleBitBinary>(variation, range)
            }
            StaticReadHeader::BinaryOutputStatus(variation, range) => {
                self.select_by_type::<BinaryOutputStatus>(variation, range)
            }
            StaticReadHeader::Counter(variation, range) => {
                self.select_by_type::<Counter>(variation, range)
            }
            StaticReadHeader::FrozenCounter(variation, range) => {
                self.select_by_type::<FrozenCounter>(variation, range)
            }
            StaticReadHeader::Analog(variation, range) => {
                self.select_by_type::<Analog>(variation, range)
            }
            StaticReadHeader::AnalogOutputStatus(variation, range) => {
                self.select_by_type::<AnalogOutputStatus>(variation, range)
            }
            StaticReadHeader::OctetString(range) => self.select_by_type::<OctetString>(None, range),
        }
    }

    fn select_by_type<T>(
        &mut self,
        variation: Option<T::StaticVariation>,
        range: Option<IndexRange>,
    ) -> Iin2
    where
        T: Updatable,
    {
        match range {
            Some(range) => self.push_selection(T::wrap(range, variation)),
            None => {
                if let Some(x) = T::get_map(self).select_all_with_variation(variation) {
                    self.push_selection(x)
                } else {
                    Iin2::default()
                }
            }
        }
    }

    fn push_selection(&mut self, range: VariationRange) -> Iin2 {
        if self.selected.push_back(range) {
            Iin2::default()
        } else {
            Iin2::PARAMETER_ERROR
        }
    }

    fn get_map<T>(&mut self) -> &mut PointMap<T>
    where
        T: Updatable,
    {
        T::get_map(self)
    }

    fn select_class_zero_type<T>(&mut self) -> Iin2
    where
        T: Updatable,
    {
        if T::enabled_class_zero(&self.class_zero) {
            let full_range = match T::get_map(self).select_all() {
                None => return Iin2::default(),
                Some(x) => x,
            };

            if self.selected.push_back(full_range) {
                Iin2::default()
            } else {
                // out of space for read headers
                Iin2::PARAMETER_ERROR
            }
        } else {
            Iin2::default()
        }
    }

    fn select_class_zero(&mut self) -> Iin2 {
        self.select_class_zero_type::<Binary>()
            | self.select_class_zero_type::<DoubleBitBinary>()
            | self.select_class_zero_type::<BinaryOutputStatus>()
            | self.select_class_zero_type::<Counter>()
            | self.select_class_zero_type::<FrozenCounter>()
            | self.select_class_zero_type::<Analog>()
            | self.select_class_zero_type::<AnalogOutputStatus>()
            | self.select_class_zero_type::<OctetString>()
    }
}

pub(crate) struct FlagsDetector;
pub(crate) struct Deadband<N>
where
    N: std::ops::Sub<N, Output = N> + PartialOrd<N>,
{
    deadband: N,
}

pub(crate) struct OctetStringDetector;

impl<N> Deadband<N>
where
    N: std::ops::Sub<N, Output = N> + PartialOrd<N>,
{
    pub(crate) fn new(value: N) -> Self {
        Self { deadband: value }
    }

    fn exceeded(&self, lhs: N, rhs: N) -> bool {
        let diff = if lhs > rhs { lhs - rhs } else { rhs - lhs };

        diff > self.deadband
    }
}

impl EventDetector<Binary> for FlagsDetector {
    fn is_event(&self, new: &Binary, old: &Binary) -> bool {
        new.get_wire_flags() != old.get_wire_flags()
    }
}

impl EventDetector<BinaryOutputStatus> for FlagsDetector {
    fn is_event(&self, new: &BinaryOutputStatus, old: &BinaryOutputStatus) -> bool {
        new.get_wire_flags() != old.get_wire_flags()
    }
}

impl EventDetector<DoubleBitBinary> for FlagsDetector {
    fn is_event(&self, new: &DoubleBitBinary, old: &DoubleBitBinary) -> bool {
        new.get_wire_flags() != old.get_wire_flags()
    }
}

pub(crate) trait HasValue<T> {
    fn value(&self) -> T;
}

impl HasValue<u32> for Counter {
    fn value(&self) -> u32 {
        self.value
    }
}

impl HasValue<u32> for FrozenCounter {
    fn value(&self) -> u32 {
        self.value
    }
}

impl HasValue<f64> for Analog {
    fn value(&self) -> f64 {
        self.value
    }
}

impl HasValue<f64> for AnalogOutputStatus {
    fn value(&self) -> f64 {
        self.value
    }
}

impl<T, N> EventDetector<T> for Deadband<N>
where
    T: Updatable + HasValue<N> + WireFlags,
    N: std::ops::Sub<N, Output = N> + PartialOrd<N>,
{
    fn is_event(&self, new: &T, old: &T) -> bool {
        if new.get_wire_flags() != old.get_wire_flags() {
            return true;
        }

        self.exceeded(new.value(), old.value())
    }
}

impl EventDetector<OctetString> for OctetStringDetector {
    fn is_event(&self, new: &OctetString, old: &OctetString) -> bool {
        new.value() != old.value()
    }
}

impl Updatable for Binary {
    type StaticVariation = StaticBinaryVariation;
    type Detector = FlagsDetector;

    fn get_map(maps: &mut StaticDatabase) -> &mut PointMap<Self> {
        &mut maps.binary
    }

    fn wrap(range: IndexRange, variation: Option<Self::StaticVariation>) -> VariationRange {
        SpecificVariation::Binary(variation).with(range)
    }

    fn enabled_class_zero(config: &ClassZeroConfig) -> bool {
        config.binary
    }
}

impl Updatable for DoubleBitBinary {
    type StaticVariation = StaticDoubleBitBinaryVariation;
    type Detector = FlagsDetector;

    fn get_map(maps: &mut StaticDatabase) -> &mut PointMap<Self> {
        &mut maps.double_bit_binary
    }

    fn wrap(range: IndexRange, variation: Option<Self::StaticVariation>) -> VariationRange {
        SpecificVariation::DoubleBitBinary(variation).with(range)
    }

    fn enabled_class_zero(config: &ClassZeroConfig) -> bool {
        config.double_bit_binary
    }
}

impl Updatable for BinaryOutputStatus {
    type StaticVariation = StaticBinaryOutputStatusVariation;
    type Detector = FlagsDetector;

    fn get_map(maps: &mut StaticDatabase) -> &mut PointMap<Self> {
        &mut maps.binary_output_status
    }

    fn wrap(range: IndexRange, variation: Option<Self::StaticVariation>) -> VariationRange {
        SpecificVariation::BinaryOutputStatus(variation).with(range)
    }

    fn enabled_class_zero(config: &ClassZeroConfig) -> bool {
        config.binary_output_status
    }
}

impl Updatable for Counter {
    type StaticVariation = StaticCounterVariation;
    type Detector = Deadband<u32>;

    fn get_map(maps: &mut StaticDatabase) -> &mut PointMap<Self> {
        &mut maps.counter
    }

    fn wrap(range: IndexRange, variation: Option<Self::StaticVariation>) -> VariationRange {
        SpecificVariation::Counter(variation).with(range)
    }

    fn enabled_class_zero(config: &ClassZeroConfig) -> bool {
        config.counter
    }
}

impl Updatable for FrozenCounter {
    type StaticVariation = StaticFrozenCounterVariation;
    type Detector = Deadband<u32>;

    fn get_map(maps: &mut StaticDatabase) -> &mut PointMap<Self> {
        &mut maps.frozen_counter
    }

    fn wrap(range: IndexRange, variation: Option<Self::StaticVariation>) -> VariationRange {
        SpecificVariation::FrozenCounter(variation).with(range)
    }

    fn enabled_class_zero(config: &ClassZeroConfig) -> bool {
        config.frozen_counter
    }
}

impl Updatable for Analog {
    type StaticVariation = StaticAnalogVariation;
    type Detector = Deadband<f64>;

    fn get_map(maps: &mut StaticDatabase) -> &mut PointMap<Self> {
        &mut maps.analog
    }

    fn wrap(range: IndexRange, variation: Option<Self::StaticVariation>) -> VariationRange {
        SpecificVariation::Analog(variation).with(range)
    }

    fn enabled_class_zero(config: &ClassZeroConfig) -> bool {
        config.analog
    }
}

impl Updatable for AnalogOutputStatus {
    type StaticVariation = StaticAnalogOutputStatusVariation;
    type Detector = Deadband<f64>;

    fn get_map(maps: &mut StaticDatabase) -> &mut PointMap<Self> {
        &mut maps.analog_output_status
    }

    fn wrap(range: IndexRange, variation: Option<Self::StaticVariation>) -> VariationRange {
        SpecificVariation::AnalogOutputStatus(variation).with(range)
    }

    fn enabled_class_zero(config: &ClassZeroConfig) -> bool {
        config.analog_output_status
    }
}

impl Updatable for OctetString {
    type StaticVariation = StaticOctetStringVariation;
    type Detector = OctetStringDetector;

    fn get_map(maps: &mut StaticDatabase) -> &mut PointMap<Self> {
        &mut maps.octet_strings
    }

    fn wrap(range: IndexRange, _variation: Option<Self::StaticVariation>) -> VariationRange {
        SpecificVariation::OctetString.with(range)
    }

    fn enabled_class_zero(config: &ClassZeroConfig) -> bool {
        config.octet_strings
    }
}

impl Default for Binary {
    fn default() -> Self {
        Self::new(false, Flags::RESTART, Time::not_synchronized(0))
    }
}

impl Default for BinaryOutputStatus {
    fn default() -> Self {
        Self::new(false, Flags::RESTART, Time::not_synchronized(0))
    }
}

impl Default for DoubleBitBinary {
    fn default() -> Self {
        Self::new(
            DoubleBit::Indeterminate,
            Flags::RESTART,
            Time::not_synchronized(0),
        )
    }
}

impl Default for Counter {
    fn default() -> Self {
        Self::new(0, Flags::RESTART, Time::not_synchronized(0))
    }
}

impl Default for FrozenCounter {
    fn default() -> Self {
        Self::new(0, Flags::RESTART, Time::not_synchronized(0))
    }
}

impl Default for Analog {
    fn default() -> Self {
        Self::new(0.0, Flags::RESTART, Time::not_synchronized(0))
    }
}

impl Default for AnalogOutputStatus {
    fn default() -> Self {
        Self::new(0.0, Flags::RESTART, Time::not_synchronized(0))
    }
}

impl Default for OctetString {
    fn default() -> Self {
        Self::new(&[0x00]).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binary_config(var: StaticBinaryVariation) -> PointConfig<Binary> {
        PointConfig {
            class: Some(EventClass::Class1),
            s_var: var,
            e_var: EventBinaryVariation::Group2Var1,
            detector: FlagsDetector {},
        }
    }

    fn counter_config(var: StaticCounterVariation) -> PointConfig<Counter> {
        PointConfig {
            class: Some(EventClass::Class1),
            s_var: var,
            e_var: EventCounterVariation::Group22Var1,
            detector: Deadband::new(0),
        }
    }

    fn analog_config(var: StaticAnalogVariation) -> PointConfig<Analog> {
        PointConfig {
            class: Some(EventClass::Class1),
            s_var: var,
            e_var: EventAnalogVariation::Group32Var1,
            detector: Deadband::new(0.0),
        }
    }

    #[test]
    fn can_write_integrity() {
        let mut db = StaticDatabase::default();

        assert!(db.add(0, binary_config(StaticBinaryVariation::Group1Var2)));
        assert!(db.add(1, counter_config(StaticCounterVariation::Group20Var1)));
        assert!(db.add(2, analog_config(StaticAnalogVariation::Group30Var1)));

        db.select_class_zero();

        let mut buffer = [0u8; 64];
        let mut cursor = WriteCursor::new(buffer.as_mut());

        db.write(&mut cursor).unwrap();

        assert_eq!(
            cursor.written(),
            [
                // g1v2 - s/s == 0, restart
                01, 02, 0x01, 00, 00, 00, 00, 0x02,
                // g20v1 - s/s == 1, restart, value == 0
                20, 01, 0x01, 01, 00, 01, 00, 0x02, 0, 0, 0, 0,
                // g30v1 - s/s == 2, restart, value == 0
                30, 01, 0x01, 02, 00, 02, 00, 0x02, 0, 0, 0, 0,
            ]
        )
    }

    #[test]
    fn can_write_multiple_cycles() {
        let mut db = StaticDatabase::default();

        assert!(db.add(0, binary_config(StaticBinaryVariation::Group1Var2)));
        assert!(db.add(1, counter_config(StaticCounterVariation::Group20Var1)));
        assert!(db.add(2, analog_config(StaticAnalogVariation::Group30Var1)));

        db.select_class_zero();

        let mut buffer = [0u8; 12]; // can only fit one header at a time

        {
            let mut cursor = WriteCursor::new(buffer.as_mut());
            db.write(&mut cursor).unwrap_err(); // incomplete !

            assert_eq!(
                cursor.written(),
                [
                    // g1v2 - s/s == 0, restart
                    01, 02, 0x01, 00, 00, 00, 00, 0x02,
                ]
            )
        }

        {
            let mut cursor = WriteCursor::new(buffer.as_mut());
            db.write(&mut cursor).unwrap_err(); // incomplete !

            assert_eq!(
                cursor.written(),
                [
                    // g20v1 - s/s == 1, restart, value == 0
                    20, 01, 0x01, 01, 00, 01, 00, 0x02, 0, 0, 0, 0,
                ]
            )
        }

        {
            let mut cursor = WriteCursor::new(buffer.as_mut());
            db.write(&mut cursor).unwrap(); // complete !

            assert_eq!(
                cursor.written(),
                [
                    // g30v1 - s/s == 2, restart, value == 0
                    30, 01, 0x01, 02, 00, 02, 00, 0x02, 0, 0, 0, 0,
                ]
            )
        }
    }

    #[test]
    fn promotes_g1v1_to_g1v2_if_flags_other_than_just_online() {
        let mut db = StaticDatabase::default();

        assert!(db.add(0, binary_config(StaticBinaryVariation::Group1Var1)));

        db.select_class_zero();

        let mut buffer = [0u8; 64];
        let mut cursor = WriteCursor::new(buffer.as_mut());

        db.write(&mut cursor).unwrap();

        assert_eq!(
            cursor.written(),
            [
                // g1v2 - s/s == 0, restart
                01, 02, 0x01, 00, 00, 00, 00, 0x02,
            ]
        )
    }
}
