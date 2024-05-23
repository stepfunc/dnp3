use std::collections::{BTreeMap, Bound, VecDeque};
use std::fmt::Formatter;
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

use crate::app::attr::AttrSet;
use crate::util::BadWrite;
use scursor::WriteCursor;

pub(crate) trait EventDetector<T>
where
    T: Updatable,
{
    fn is_event(&self, new: &T, old: &T) -> bool;
}

pub(crate) trait Updatable: Insertable + Clone + Default {
    type StaticVariation: StaticVariation<Self>;
    type Detector: EventDetector<Self>;
    fn get_map(maps: &StaticDatabase) -> &PointMap<Self>;
    fn get_mut_map(maps: &mut StaticDatabase) -> &mut PointMap<Self>;
    fn wrap(range: IndexRange, variation: Option<Self::StaticVariation>) -> VariationRange;
    fn enabled_class_zero(config: &ClassZeroConfig) -> bool;
}

pub(crate) trait UpdatableFlags: Updatable {
    fn update_flags(&mut self, flags: Flags, time: Option<Time>);
}

#[derive(Copy, Clone)]
pub(crate) struct IndexRange {
    start: u16,
    stop: u16,
}

impl std::fmt::Display for IndexRange {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "start: {} stop: {}", self.start, self.stop)
    }
}

impl IndexRange {
    pub(crate) fn new(start: u16, stop: u16) -> Self {
        Self { start, stop }
    }

    pub(crate) fn to_attr_set(self) -> Option<AttrSet> {
        if self.start != self.stop {
            return None;
        }

        self.start.try_into().ok().map(AttrSet::new)
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
    Binary(Option<StaticBinaryInputVariation>),
    DoubleBitBinary(Option<StaticDoubleBitBinaryInputVariation>),
    BinaryOutputStatus(Option<StaticBinaryOutputStatusVariation>),
    Counter(Option<StaticCounterVariation>),
    FrozenCounter(Option<StaticFrozenCounterVariation>),
    Analog(Option<StaticAnalogInputVariation>),
    AnalogOutputStatus(Option<StaticAnalogOutputStatusVariation>),
    OctetString,
    AnalogDeadBand(Option<AnalogInputDeadBandVariation>),
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

    fn full_range(&self) -> Option<IndexRange> {
        let start = *self.inner.first_key_value()?.0;
        let stop = *self.inner.last_key_value()?.0;
        Some(IndexRange::new(start, stop))
    }

    fn get_mut(&mut self, index: u16) -> Option<&mut Point<T>> {
        self.inner.get_mut(&index)
    }

    fn select_range_with_variation(
        &mut self,
        range: IndexRange,
        variation: Option<T::StaticVariation>,
    ) -> Option<VariationRange> {
        for (_index, point) in self
            .inner
            .range_mut((Bound::Included(&range.start), Bound::Included(&range.stop)))
        {
            // for every point in the range, we copy the current value into a distinct 'selected' cell
            // when writing the response(s) we use the selected value
            // this allows the outstation to send consistent snapshot of the values when a multi-fragment response is required
            point.selected = point.current.clone();
        }
        Some(T::wrap(range, variation))
    }

    fn select_all(&mut self) -> Option<VariationRange> {
        self.select_all_with_variation(None)
    }

    fn select_all_with_variation(
        &mut self,
        variation: Option<T::StaticVariation>,
    ) -> Option<VariationRange> {
        let range = self.full_range()?;

        // as far at the processing goes, we treat this just like a range scan over all the values
        self.select_range_with_variation(range, variation)
    }
}

pub(crate) struct StaticDatabase {
    class_zero: ClassZeroConfig,
    selected: SelectionQueue,
    // maps for the various types
    binary: PointMap<BinaryInput>,
    double_bit_binary: PointMap<DoubleBitBinaryInput>,
    binary_output_status: PointMap<BinaryOutputStatus>,
    counter: PointMap<Counter>,
    frozen_counter: PointMap<FrozenCounter>,
    analog: PointMap<AnalogInput>,
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

    pub(crate) fn set_analog_deadband(&mut self, index: u16, deadband: f64) -> bool {
        fn zero_or_positive(value: f64) -> bool {
            value == 0.0 || (value.is_normal() && value.is_sign_positive())
        }

        if !zero_or_positive(deadband) {
            return false;
        }

        match self.analog.get_mut(index) {
            None => false,
            Some(x) => {
                x.config.detector.deadband = deadband;
                true
            }
        }
    }

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
        let map = self.get_mut_map::<T>();

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
        self.get_mut_map::<T>().inner.remove(&index).is_some()
    }

    pub(crate) fn get<T>(&self, index: u16) -> Option<T>
    where
        T: Updatable,
    {
        self.get_map::<T>()
            .inner
            .get(&index)
            .map(|point| point.current.clone())
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
        match self.get_mut_map::<T>().get_mut(index) {
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
                        if x.config.detector.is_event(&x.last_event, value) {
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

    pub(crate) fn write(&mut self, cursor: &mut WriteCursor) -> Result<(), BadWrite> {
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
                    return Err(BadWrite);
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
                self.write_typed_range::<BinaryInput>(cursor, range.range, var)
            }
            SpecificVariation::DoubleBitBinary(var) => {
                self.write_typed_range::<DoubleBitBinaryInput>(cursor, range.range, var)
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
                self.write_typed_range::<AnalogInput>(cursor, range.range, var)
            }
            SpecificVariation::AnalogOutputStatus(var) => {
                self.write_typed_range::<AnalogOutputStatus>(cursor, range.range, var)
            }
            SpecificVariation::OctetString => {
                self.write_typed_range::<OctetString>(cursor, range.range, None)
            }
            SpecificVariation::AnalogDeadBand(var) => {
                self.write_analog_dead_bands(cursor, range.range, var)
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

    fn write_analog_dead_bands(
        &mut self,
        cursor: &mut WriteCursor,
        range: IndexRange,
        variation: Option<AnalogInputDeadBandVariation>,
    ) -> Result<(), VariationRange> {
        let mut writer = RangeWriter::new();
        for (index, item) in self.analog.inner.range(range) {
            // first determine what variation should be written
            let var = variation.unwrap_or(AnalogInputDeadBandVariation::Group34Var3);

            let info = var.get_write_info();

            if writer
                .write(cursor, *index, &item.config.detector.deadband, info)
                .is_err()
            {
                // ran out of space, tell calling code to resume at this index
                return Err(VariationRange::new(
                    IndexRange::new(*index, range.stop),
                    SpecificVariation::AnalogDeadBand(variation),
                ));
            }
        }

        Ok(())
    }

    pub(crate) fn select(&mut self, variation: StaticReadHeader) -> Iin2 {
        match variation {
            StaticReadHeader::Class0 => self.select_class_zero(),
            StaticReadHeader::Binary(variation, range) => {
                self.select_by_type::<BinaryInput>(variation, range)
            }
            StaticReadHeader::DoubleBitBinary(variation, range) => {
                self.select_by_type::<DoubleBitBinaryInput>(variation, range)
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
                self.select_by_type::<AnalogInput>(variation, range)
            }
            StaticReadHeader::AnalogOutputStatus(variation, range) => {
                self.select_by_type::<AnalogOutputStatus>(variation, range)
            }
            StaticReadHeader::OctetString(range) => self.select_by_type::<OctetString>(None, range),
            StaticReadHeader::FrozenAnalog(_, _) => {
                // we don't support this, but we know what it is
                Iin2::default()
            }
            StaticReadHeader::AnalogInputDeadBand(var, range) => {
                match range {
                    None => {
                        if let Some(range) = self.analog.full_range() {
                            self.push_selection(VariationRange::new(
                                range,
                                SpecificVariation::AnalogDeadBand(var),
                            ))
                        } else {
                            // we don't have any of those
                            Iin2::default()
                        }
                    }
                    Some(range) => self.push_selection(VariationRange::new(
                        range,
                        SpecificVariation::AnalogDeadBand(var),
                    )),
                }
            }
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
        let selected = match range {
            Some(range) => T::get_mut_map(self).select_range_with_variation(range, variation),
            None => T::get_mut_map(self).select_all_with_variation(variation),
        };

        match selected {
            None => Iin2::default(),
            Some(range) => self.push_selection(range),
        }
    }

    fn push_selection(&mut self, range: VariationRange) -> Iin2 {
        if self.selected.push_back(range) {
            Iin2::default()
        } else {
            Iin2::PARAMETER_ERROR
        }
    }

    fn get_map<T>(&self) -> &PointMap<T>
    where
        T: Updatable,
    {
        T::get_map(self)
    }

    fn get_mut_map<T>(&mut self) -> &mut PointMap<T>
    where
        T: Updatable,
    {
        T::get_mut_map(self)
    }

    fn select_class_zero_type<T>(&mut self) -> Iin2
    where
        T: Updatable,
    {
        if T::enabled_class_zero(&self.class_zero) {
            let full_range = match T::get_mut_map(self).select_all() {
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
        self.select_class_zero_type::<BinaryInput>()
            | self.select_class_zero_type::<DoubleBitBinaryInput>()
            | self.select_class_zero_type::<BinaryOutputStatus>()
            | self.select_class_zero_type::<Counter>()
            | self.select_class_zero_type::<FrozenCounter>()
            | self.select_class_zero_type::<AnalogInput>()
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

impl EventDetector<BinaryInput> for FlagsDetector {
    fn is_event(&self, new: &BinaryInput, old: &BinaryInput) -> bool {
        new.get_wire_flags() != old.get_wire_flags()
    }
}

impl EventDetector<BinaryOutputStatus> for FlagsDetector {
    fn is_event(&self, new: &BinaryOutputStatus, old: &BinaryOutputStatus) -> bool {
        new.get_wire_flags() != old.get_wire_flags()
    }
}

impl EventDetector<DoubleBitBinaryInput> for FlagsDetector {
    fn is_event(&self, new: &DoubleBitBinaryInput, old: &DoubleBitBinaryInput) -> bool {
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

impl HasValue<f64> for AnalogInput {
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

impl Updatable for BinaryInput {
    type StaticVariation = StaticBinaryInputVariation;
    type Detector = FlagsDetector;

    fn get_map(maps: &StaticDatabase) -> &PointMap<Self> {
        &maps.binary
    }

    fn get_mut_map(maps: &mut StaticDatabase) -> &mut PointMap<Self> {
        &mut maps.binary
    }

    fn wrap(range: IndexRange, variation: Option<Self::StaticVariation>) -> VariationRange {
        SpecificVariation::Binary(variation).with(range)
    }

    fn enabled_class_zero(config: &ClassZeroConfig) -> bool {
        config.binary
    }
}

impl UpdatableFlags for BinaryInput {
    fn update_flags(&mut self, flags: Flags, time: Option<Time>) {
        self.flags = flags;
        self.time = time;
    }
}

impl Updatable for DoubleBitBinaryInput {
    type StaticVariation = StaticDoubleBitBinaryInputVariation;
    type Detector = FlagsDetector;

    fn get_map(maps: &StaticDatabase) -> &PointMap<Self> {
        &maps.double_bit_binary
    }

    fn get_mut_map(maps: &mut StaticDatabase) -> &mut PointMap<Self> {
        &mut maps.double_bit_binary
    }

    fn wrap(range: IndexRange, variation: Option<Self::StaticVariation>) -> VariationRange {
        SpecificVariation::DoubleBitBinary(variation).with(range)
    }

    fn enabled_class_zero(config: &ClassZeroConfig) -> bool {
        config.double_bit_binary
    }
}

impl UpdatableFlags for DoubleBitBinaryInput {
    fn update_flags(&mut self, flags: Flags, time: Option<Time>) {
        self.flags = flags;
        self.time = time;
    }
}

impl Updatable for BinaryOutputStatus {
    type StaticVariation = StaticBinaryOutputStatusVariation;
    type Detector = FlagsDetector;

    fn get_map(maps: &StaticDatabase) -> &PointMap<Self> {
        &maps.binary_output_status
    }

    fn get_mut_map(maps: &mut StaticDatabase) -> &mut PointMap<Self> {
        &mut maps.binary_output_status
    }

    fn wrap(range: IndexRange, variation: Option<Self::StaticVariation>) -> VariationRange {
        SpecificVariation::BinaryOutputStatus(variation).with(range)
    }

    fn enabled_class_zero(config: &ClassZeroConfig) -> bool {
        config.binary_output_status
    }
}

impl UpdatableFlags for BinaryOutputStatus {
    fn update_flags(&mut self, flags: Flags, time: Option<Time>) {
        self.flags = flags;
        self.time = time;
    }
}
impl Updatable for Counter {
    type StaticVariation = StaticCounterVariation;
    type Detector = Deadband<u32>;

    fn get_map(maps: &StaticDatabase) -> &PointMap<Self> {
        &maps.counter
    }

    fn get_mut_map(maps: &mut StaticDatabase) -> &mut PointMap<Self> {
        &mut maps.counter
    }

    fn wrap(range: IndexRange, variation: Option<Self::StaticVariation>) -> VariationRange {
        SpecificVariation::Counter(variation).with(range)
    }

    fn enabled_class_zero(config: &ClassZeroConfig) -> bool {
        config.counter
    }
}

impl UpdatableFlags for Counter {
    fn update_flags(&mut self, flags: Flags, time: Option<Time>) {
        self.flags = flags;
        self.time = time;
    }
}

impl Updatable for FrozenCounter {
    type StaticVariation = StaticFrozenCounterVariation;
    type Detector = Deadband<u32>;

    fn get_map(maps: &StaticDatabase) -> &PointMap<Self> {
        &maps.frozen_counter
    }

    fn get_mut_map(maps: &mut StaticDatabase) -> &mut PointMap<Self> {
        &mut maps.frozen_counter
    }

    fn wrap(range: IndexRange, variation: Option<Self::StaticVariation>) -> VariationRange {
        SpecificVariation::FrozenCounter(variation).with(range)
    }

    fn enabled_class_zero(config: &ClassZeroConfig) -> bool {
        config.frozen_counter
    }
}

impl UpdatableFlags for FrozenCounter {
    fn update_flags(&mut self, flags: Flags, time: Option<Time>) {
        self.flags = flags;
        self.time = time;
    }
}

impl Updatable for AnalogInput {
    type StaticVariation = StaticAnalogInputVariation;
    type Detector = Deadband<f64>;

    fn get_map(maps: &StaticDatabase) -> &PointMap<Self> {
        &maps.analog
    }

    fn get_mut_map(maps: &mut StaticDatabase) -> &mut PointMap<Self> {
        &mut maps.analog
    }

    fn wrap(range: IndexRange, variation: Option<Self::StaticVariation>) -> VariationRange {
        SpecificVariation::Analog(variation).with(range)
    }

    fn enabled_class_zero(config: &ClassZeroConfig) -> bool {
        config.analog
    }
}

impl UpdatableFlags for AnalogInput {
    fn update_flags(&mut self, flags: Flags, time: Option<Time>) {
        self.flags = flags;
        self.time = time;
    }
}

impl Updatable for AnalogOutputStatus {
    type StaticVariation = StaticAnalogOutputStatusVariation;
    type Detector = Deadband<f64>;

    fn get_map(maps: &StaticDatabase) -> &PointMap<Self> {
        &maps.analog_output_status
    }

    fn get_mut_map(maps: &mut StaticDatabase) -> &mut PointMap<Self> {
        &mut maps.analog_output_status
    }

    fn wrap(range: IndexRange, variation: Option<Self::StaticVariation>) -> VariationRange {
        SpecificVariation::AnalogOutputStatus(variation).with(range)
    }

    fn enabled_class_zero(config: &ClassZeroConfig) -> bool {
        config.analog_output_status
    }
}

impl UpdatableFlags for AnalogOutputStatus {
    fn update_flags(&mut self, flags: Flags, time: Option<Time>) {
        self.flags = flags;
        self.time = time;
    }
}

impl Updatable for OctetString {
    type StaticVariation = StaticOctetStringVariation;
    type Detector = OctetStringDetector;

    fn get_map(maps: &StaticDatabase) -> &PointMap<Self> {
        &maps.octet_strings
    }

    fn get_mut_map(maps: &mut StaticDatabase) -> &mut PointMap<Self> {
        &mut maps.octet_strings
    }

    fn wrap(range: IndexRange, _variation: Option<Self::StaticVariation>) -> VariationRange {
        SpecificVariation::OctetString.with(range)
    }

    fn enabled_class_zero(config: &ClassZeroConfig) -> bool {
        config.octet_string
    }
}

impl Default for BinaryInput {
    fn default() -> Self {
        Self::new(false, Flags::RESTART, Time::unsynchronized(0))
    }
}

impl Default for BinaryOutputStatus {
    fn default() -> Self {
        Self::new(false, Flags::RESTART, Time::unsynchronized(0))
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

impl Default for Counter {
    fn default() -> Self {
        Self::new(0, Flags::RESTART, Time::unsynchronized(0))
    }
}

impl Default for FrozenCounter {
    fn default() -> Self {
        Self::new(0, Flags::RESTART, Time::unsynchronized(0))
    }
}

impl Default for AnalogInput {
    fn default() -> Self {
        Self::new(0.0, Flags::RESTART, Time::unsynchronized(0))
    }
}

impl Default for AnalogOutputStatus {
    fn default() -> Self {
        Self::new(0.0, Flags::RESTART, Time::unsynchronized(0))
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

    fn binary_config(var: StaticBinaryInputVariation) -> PointConfig<BinaryInput> {
        PointConfig {
            class: Some(EventClass::Class1),
            s_var: var,
            e_var: EventBinaryInputVariation::Group2Var1,
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

    fn analog_config(var: StaticAnalogInputVariation) -> PointConfig<AnalogInput> {
        PointConfig {
            class: Some(EventClass::Class1),
            s_var: var,
            e_var: EventAnalogInputVariation::Group32Var1,
            detector: Deadband::new(0.0),
        }
    }

    #[test]
    fn can_write_integrity() {
        let mut db = StaticDatabase::default();

        assert!(db.add(0, binary_config(StaticBinaryInputVariation::Group1Var2)));
        assert!(db.add(1, counter_config(StaticCounterVariation::Group20Var1)));
        assert!(db.add(2, analog_config(StaticAnalogInputVariation::Group30Var1)));

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

        assert!(db.add(0, binary_config(StaticBinaryInputVariation::Group1Var2)));
        assert!(db.add(1, counter_config(StaticCounterVariation::Group20Var1)));
        assert!(db.add(2, analog_config(StaticAnalogInputVariation::Group30Var1)));

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

        assert!(db.add(0, binary_config(StaticBinaryInputVariation::Group1Var1)));

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
