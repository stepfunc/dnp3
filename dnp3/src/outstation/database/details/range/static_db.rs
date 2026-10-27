// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use std::collections::{btree_map, BTreeMap, Bound};
use std::fmt::Formatter;
use std::ops::RangeBounds;

use crate::app::measurement::*;
use crate::app::parse::range::Range;
use crate::app::Iin2;
use crate::outstation::config::OutstationConfig;
use crate::outstation::database::assign::Assignment;
use crate::outstation::database::details::family::Family;
use crate::outstation::database::details::range::traits::StaticVariation;
use crate::outstation::database::details::range::writer::RangeWriter;
use crate::outstation::database::details::{Visits, WriteProgress};
use crate::outstation::database::read::StaticTarget;
use crate::outstation::database::variations::*;
use crate::outstation::database::{ClassZeroConfig, EventClass, EventMode, UpdateOptions};
use crate::outstation::{PointIndices, PointType};

use crate::app::attr::AttrSet;
use crate::util::bounded::{BoundedQueue, Full};
use scursor::WriteCursor;

pub(crate) trait EventDetector<T>
where
    T: Family,
{
    fn is_event(&self, new: &T, old: &T) -> bool;
}

/// A range of point indices, `start` through `stop` inclusive, never empty
///
/// Its constructors keep `start <= stop`: [`IndexRange::new`] checks it, and a [`Range`] is
/// already checked when it's parsed
#[derive(Copy, Clone)]
pub(crate) struct IndexRange {
    start: u16,
    stop: u16,
}

impl From<Range> for IndexRange {
    fn from(range: Range) -> Self {
        Self {
            start: range.get_start(),
            stop: range.get_stop(),
        }
    }
}

impl std::fmt::Display for IndexRange {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "start: {} stop: {}", self.start, self.stop)
    }
}

impl IndexRange {
    /// The range from `start` through `stop`, or `None` if `start > stop`
    pub(crate) fn new(start: u16, stop: u16) -> Option<Self> {
        (start <= stop).then_some(Self { start, stop })
    }

    /// The rest of the range from `index`, or `None` if `index` is past its end
    fn resume_at(self, index: u16) -> Option<Self> {
        Self::new(index, self.stop)
    }

    pub(crate) fn to_attr_set(self) -> Option<AttrSet> {
        if self.start != self.stop {
            return None;
        }

        self.start.try_into().ok().map(AttrSet::new)
    }

    /// The entries of `map` in this range
    #[expect(
        clippy::disallowed_methods,
        reason = "IndexRange's constructors keep start <= stop, so the bounds are in order"
    )]
    fn entries<V>(self, map: &BTreeMap<u16, V>) -> btree_map::Range<'_, u16, V> {
        map.range(self)
    }

    /// The entries of `map` in this range, mutably
    #[expect(
        clippy::disallowed_methods,
        reason = "IndexRange's constructors keep start <= stop, so the bounds are in order"
    )]
    fn entries_mut<V>(self, map: &mut BTreeMap<u16, V>) -> btree_map::RangeMut<'_, u16, V> {
        map.range_mut(self)
    }

    /// number of indices in the range
    fn len(&self) -> usize {
        usize::from(self.stop)
            .saturating_add(1)
            .saturating_sub(usize::from(self.start))
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

pub(crate) struct PointConfig<T>
where
    T: Family,
{
    class: Option<EventClass>,
    detector: T::Detector,
    s_var: T::StaticVariation,
    e_var: T::EventVariation,
}

impl<T> PointConfig<T>
where
    T: Family,
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
    T: Family,
{
    // current value
    current: T,
    // value that is frozen during READ requests to be reported
    selected: T,
    // whether a READ has selected the point since it was added: fragments only write the ranges
    // of the current READ, which marks every point in them, so a point added after the READ
    // selected its range isn't part of that response
    was_selected: bool,
    // last value that produced an event
    last_event: T,
    // configuration
    config: PointConfig<T>,
}

impl<T> Point<T>
where
    T: Family + Default,
{
    pub(crate) fn new(config: PointConfig<T>) -> Self {
        Self {
            current: T::default(),
            selected: T::default(),
            was_selected: false,
            last_event: T::default(),
            config,
        }
    }
}

pub(crate) struct PointMap<T>
where
    T: Family,
{
    inner: BTreeMap<u16, Point<T>>,
}

impl<T> PointMap<T>
where
    T: Family,
{
    fn empty() -> Self {
        Self {
            inner: BTreeMap::new(),
        }
    }

    fn full_range(&self) -> Option<IndexRange> {
        let start = *self.inner.first_key_value()?.0;
        let stop = *self.inner.last_key_value()?.0;
        IndexRange::new(start, stop)
    }

    fn get_mut(&mut self, index: u16) -> Option<&mut Point<T>> {
        self.inner.get_mut(&index)
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// The points an assignment to the indices visits, counting them and then changing them, or
    /// the error for a type without points (IIN2.1) or a range that isn't fully installed
    /// (IIN2.2)
    fn check_assignment(&self, indices: PointIndices) -> Result<Visits, Iin2> {
        if self.inner.is_empty() {
            return Err(Iin2::OBJECT_UNKNOWN);
        }
        match indices {
            // the count of all of them doesn't walk them
            PointIndices::All => Ok(Visits::new(self.inner.len())),
            PointIndices::Range(start, stop) => {
                let range = IndexRange::new(start, stop).ok_or(Iin2::PARAMETER_ERROR)?;
                match self.check_range(range) {
                    (Ok(iin2), counted) if iin2 == Iin2::default() => Ok(counted.plus(counted)),
                    _ => Err(Iin2::PARAMETER_ERROR),
                }
            }
        }
    }

    /// Assign the events of the points at the indices to a class, or to none
    fn assign(&mut self, indices: PointIndices, class: Option<EventClass>) {
        let range = match indices {
            PointIndices::All => self.full_range(),
            PointIndices::Range(start, stop) => IndexRange::new(start, stop),
        };
        if let Some(range) = range {
            for (_index, point) in range.entries_mut(&mut self.inner) {
                point.config.class = class;
            }
        }
    }

    /// IIN2 bits for a read of a specific range of this type, or the error to
    /// respond with if nothing in the range can be selected (TB2016-001), with the points
    /// counted:
    ///
    /// * IIN2.1 if there are no points of this type
    /// * IIN2.2 if some or all of the points in the range aren't installed
    fn check_range(&self, range: IndexRange) -> (Result<Iin2, Iin2>, Visits) {
        if self.inner.is_empty() {
            return (Err(Iin2::OBJECT_UNKNOWN), Visits::NONE);
        }

        let installed = range.entries(&self.inner).count();
        let result = match installed {
            0 => Err(Iin2::PARAMETER_ERROR),
            installed if installed < range.len() => Ok(Iin2::PARAMETER_ERROR),
            _ => Ok(Iin2::default()),
        };
        (result, Visits::new(installed))
    }

    fn select_range_with_variation(
        &mut self,
        range: IndexRange,
        variation: Option<T::StaticVariation>,
    ) -> (VariationRange, Visits) {
        let mut visits = Visits::NONE;
        for (_index, point) in range.entries_mut(&mut self.inner) {
            // for every point in the range, we copy the current value into a distinct 'selected' cell
            // when writing the response(s) we use the selected value
            // this allows the outstation to send consistent snapshot of the values when a multi-fragment response is required
            point.selected = point.current.clone();
            point.was_selected = true;
            visits.increment();
        }
        (T::selection(variation).with(range), visits)
    }

    fn select_all_with_variation(
        &mut self,
        variation: Option<T::StaticVariation>,
    ) -> Option<(VariationRange, Visits)> {
        let range = self.full_range()?;

        // as far at the processing goes, we treat this just like a range scan over all the values
        Some(self.select_range_with_variation(range, variation))
    }

    /// mark the points in the range selected without copying their values, for writing
    /// something other than their values
    fn select_membership(&mut self, range: IndexRange) -> Visits {
        let mut visits = Visits::NONE;
        for (_index, point) in range.entries_mut(&mut self.inner) {
            point.was_selected = true;
            visits.increment();
        }
        visits
    }
}

pub(crate) struct StaticDatabase {
    class_zero: ClassZeroConfig,
    selected: BoundedQueue<VariationRange>,
    // maps for the various types
    pub(crate) binary: PointMap<BinaryInput>,
    pub(crate) double_bit_binary: PointMap<DoubleBitBinaryInput>,
    pub(crate) binary_output_status: PointMap<BinaryOutputStatus>,
    pub(crate) counter: PointMap<Counter>,
    pub(crate) frozen_counter: PointMap<FrozenCounter>,
    pub(crate) analog: PointMap<AnalogInput>,
    pub(crate) analog_output_status: PointMap<AnalogOutputStatus>,
    pub(crate) octet_strings: PointMap<OctetString>,
}

impl Default for StaticDatabase {
    fn default() -> Self {
        Self::new(
            OutstationConfig::DEFAULT_MAX_READ_REQUEST_HEADERS,
            ClassZeroConfig::default(),
        )
    }
}

impl StaticDatabase {
    pub(crate) fn new(max_read_selection: u16, class_zero: ClassZeroConfig) -> Self {
        Self {
            class_zero,
            selected: BoundedQueue::new(max_read_selection.into()),
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

    /// Set the deadband a master wrote, returning the value stored, or None if the deadband is NaN
    /// or there's no analog input at the index
    ///
    /// A negative deadband is stored as 0 (IEEE 1815-2012 A.18)
    pub(crate) fn set_analog_deadband(&mut self, index: u16, deadband: f64) -> Option<f64> {
        let deadband = if deadband < 0.0 { 0.0 } else { deadband };
        let detector = Deadband::analog(deadband)?;
        let point = self.analog.get_mut(index)?;
        point.config.detector = detector;
        Some(deadband)
    }

    #[cfg(test)]
    pub(crate) fn selection_capacity(&self) -> usize {
        self.selected.max()
    }

    #[cfg(test)]
    pub(crate) fn selection_count(&self) -> usize {
        self.selected.len()
    }

    pub(crate) fn reset(&mut self) {
        self.selected.clear();
    }

    /// The points an assignment visits, or its error; see [`PointMap::check_assignment`]
    pub(crate) fn check_assignment(&self, assignment: Assignment) -> Result<Visits, Iin2> {
        let indices = assignment.indices;
        match assignment.point_type {
            PointType::BinaryInput => self.binary.check_assignment(indices),
            PointType::DoubleBitBinaryInput => self.double_bit_binary.check_assignment(indices),
            PointType::BinaryOutputStatus => self.binary_output_status.check_assignment(indices),
            PointType::Counter => self.counter.check_assignment(indices),
            PointType::FrozenCounter => self.frozen_counter.check_assignment(indices),
            PointType::AnalogInput => self.analog.check_assignment(indices),
            PointType::AnalogOutputStatus => self.analog_output_status.check_assignment(indices),
            PointType::OctetString => self.octet_strings.check_assignment(indices),
        }
    }

    /// Apply an assignment that [`Self::check_assignment`] accepted
    pub(crate) fn assign(&mut self, assignment: Assignment) {
        let Assignment {
            point_type,
            indices,
            class,
        } = assignment;
        match point_type {
            PointType::BinaryInput => self.binary.assign(indices, class),
            PointType::DoubleBitBinaryInput => self.double_bit_binary.assign(indices, class),
            PointType::BinaryOutputStatus => self.binary_output_status.assign(indices, class),
            PointType::Counter => self.counter.assign(indices, class),
            PointType::FrozenCounter => self.frozen_counter.assign(indices, class),
            PointType::AnalogInput => self.analog.assign(indices, class),
            PointType::AnalogOutputStatus => self.analog_output_status.assign(indices, class),
            PointType::OctetString => self.octet_strings.assign(indices, class),
        }
    }

    /// the number of installed points of every type
    pub(crate) fn point_count(&self) -> usize {
        [
            self.binary.inner.len(),
            self.double_bit_binary.inner.len(),
            self.binary_output_status.inner.len(),
            self.counter.inner.len(),
            self.frozen_counter.inner.len(),
            self.analog.inner.len(),
            self.analog_output_status.inner.len(),
            self.octet_strings.inner.len(),
        ]
        .into_iter()
        .fold(0, usize::saturating_add)
    }

    pub(crate) fn add<T>(&mut self, index: u16, config: PointConfig<T>) -> bool
    where
        T: Family,
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
        T: Family,
    {
        self.get_mut_map::<T>().inner.remove(&index).is_some()
    }

    pub(crate) fn get<T>(&self, index: u16) -> Option<T>
    where
        T: Family,
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
        T: Family,
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

    pub(crate) fn write(&mut self, cursor: &mut WriteCursor) -> WriteProgress {
        while let Some(range) = self.selected.front().copied() {
            match self.write_range(cursor, range) {
                // done with this header
                Ok(()) => {
                    self.selected.pop_front();
                }
                // ran out of space
                Err(s) => {
                    // possibly wrote some data though, so update the selection
                    if let Some(front) = self.selected.front_mut() {
                        *front = s;
                    }
                    return WriteProgress::Partial;
                }
            };
        }

        WriteProgress::Complete
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
        T: Family,
    {
        let points = range
            .entries(&self.get_map::<T>().inner)
            .filter(|(_, point)| point.was_selected)
            .map(|(index, point)| (*index, &point.selected, point.config.s_var));
        write_points(cursor, range, variation, points)
    }

    fn write_analog_dead_bands(
        &mut self,
        cursor: &mut WriteCursor,
        range: IndexRange,
        variation: Option<AnalogInputDeadBandVariation>,
    ) -> Result<(), VariationRange> {
        let dead_bands = range
            .entries(&self.analog.inner)
            .filter(|(_, point)| point.was_selected)
            .map(|(index, point)| (*index, &point.config.detector.deadband));
        write_dead_bands(cursor, range, variation, dead_bands)
    }

    /// Select the points a static header asks for, with the points visited
    ///
    /// A header for a type that has points, while the selection is full, gets IIN2.2 without
    /// visiting them, since its range couldn't be pushed
    pub(crate) fn select(
        &mut self,
        target: StaticTarget,
        range: Option<IndexRange>,
    ) -> (Iin2, Visits) {
        match target {
            StaticTarget::Binary(variation) => self.select_by_type::<BinaryInput>(variation, range),
            StaticTarget::DoubleBitBinary(variation) => {
                self.select_by_type::<DoubleBitBinaryInput>(variation, range)
            }
            StaticTarget::BinaryOutputStatus(variation) => {
                self.select_by_type::<BinaryOutputStatus>(variation, range)
            }
            StaticTarget::Counter(variation) => self.select_by_type::<Counter>(variation, range),
            StaticTarget::FrozenCounter(variation) => {
                self.select_by_type::<FrozenCounter>(variation, range)
            }
            StaticTarget::Analog(variation) => self.select_by_type::<AnalogInput>(variation, range),
            StaticTarget::AnalogOutputStatus(variation) => {
                self.select_by_type::<AnalogOutputStatus>(variation, range)
            }
            StaticTarget::OctetString => self.select_by_type::<OctetString>(None, range),
            StaticTarget::FrozenAnalog(_) => {
                // we know what it is, but don't support it
                (Iin2::OBJECT_UNKNOWN, Visits::NONE)
            }
            StaticTarget::AnalogInputDeadBand(var) => {
                if self.analog.is_empty() {
                    return (Iin2::OBJECT_UNKNOWN, Visits::NONE);
                }
                if self.selected.is_full() {
                    return (Iin2::PARAMETER_ERROR, Visits::NONE);
                }

                let (range, iin2, counted) = match range {
                    None => match self.analog.full_range() {
                        Some(range) => (range, Iin2::default(), Visits::NONE),
                        // no analog inputs
                        None => return (Iin2::OBJECT_UNKNOWN, Visits::NONE),
                    },
                    Some(range) => match self.analog.check_range(range) {
                        (Ok(iin2), counted) => (range, iin2, counted),
                        (Err(iin2), counted) => return (iin2, counted),
                    },
                };

                let marked = self.analog.select_membership(range);
                let iin2 = iin2
                    | self.push_selection(VariationRange::new(
                        range,
                        SpecificVariation::AnalogDeadBand(var),
                    ));
                (iin2, counted.plus(marked))
            }
        }
    }

    fn select_by_type<T>(
        &mut self,
        variation: Option<T::StaticVariation>,
        range: Option<IndexRange>,
    ) -> (Iin2, Visits)
    where
        T: Family,
    {
        let is_full = self.selected.is_full();
        let map = T::points_mut(self);

        if map.is_empty() {
            return (Iin2::OBJECT_UNKNOWN, Visits::NONE);
        }
        if is_full {
            return (Iin2::PARAMETER_ERROR, Visits::NONE);
        }

        let (selected, iin2, visits) = match range {
            Some(range) => match map.check_range(range) {
                (Ok(iin2), counted) => {
                    let (selected, snapshot) = map.select_range_with_variation(range, variation);
                    (selected, iin2, counted.plus(snapshot))
                }
                (Err(iin2), counted) => return (iin2, counted),
            },
            None => match map.select_all_with_variation(variation) {
                Some((selected, snapshot)) => (selected, Iin2::default(), snapshot),
                // no points of this type
                None => return (Iin2::OBJECT_UNKNOWN, Visits::NONE),
            },
        };

        (iin2 | self.push_selection(selected), visits)
    }

    /// true if at least one point of the type is installed
    pub(crate) fn has_points<T>(&self) -> bool
    where
        T: Family,
    {
        !self.get_map::<T>().inner.is_empty()
    }

    fn push_selection(&mut self, range: VariationRange) -> Iin2 {
        match self.selected.try_push(range) {
            Ok(()) => Iin2::default(),
            Err(Full) => Iin2::PARAMETER_ERROR,
        }
    }

    fn get_map<T>(&self) -> &PointMap<T>
    where
        T: Family,
    {
        T::points(self)
    }

    fn get_mut_map<T>(&mut self) -> &mut PointMap<T>
    where
        T: Family,
    {
        T::points_mut(self)
    }

    /// Select every point of a type that class 0 includes, as a header for all of them does,
    /// with the points visited
    ///
    /// Class 0 skips a type that has no points, rather than setting IIN2.1
    fn select_class_zero_type<T>(&mut self) -> (Iin2, Visits)
    where
        T: Family,
    {
        if !T::in_class_zero(&self.class_zero) || !self.has_points::<T>() {
            return (Iin2::default(), Visits::NONE);
        }
        self.select_by_type::<T>(None, None)
    }

    pub(crate) fn select_class_zero(&mut self) -> (Iin2, Visits) {
        [
            self.select_class_zero_type::<BinaryInput>(),
            self.select_class_zero_type::<DoubleBitBinaryInput>(),
            self.select_class_zero_type::<BinaryOutputStatus>(),
            self.select_class_zero_type::<Counter>(),
            self.select_class_zero_type::<FrozenCounter>(),
            self.select_class_zero_type::<AnalogInput>(),
            self.select_class_zero_type::<AnalogOutputStatus>(),
            self.select_class_zero_type::<OctetString>(),
        ]
        .into_iter()
        .fold((Iin2::default(), Visits::NONE), |(iin2, visits), (i, v)| {
            (iin2 | i, visits.plus(v))
        })
    }
}

pub(crate) struct FlagsDetector;
pub(crate) struct Deadband<N> {
    deadband: N,
}

/// whether two values differ by more than a deadband
pub(crate) trait Distance: Copy {
    fn exceeds(self, other: Self, deadband: Self) -> bool;
}

impl Distance for u32 {
    fn exceeds(self, other: Self, deadband: Self) -> bool {
        self.abs_diff(other) > deadband
    }
}

/// a change between NaN and a number exceeds any deadband, and NaN to NaN never does, whatever
/// the payload
impl Distance for f64 {
    fn exceeds(self, other: Self, deadband: Self) -> bool {
        match (self.is_nan(), other.is_nan()) {
            (true, true) => false,
            (true, false) | (false, true) => true,
            // equal infinities differ by NaN, which exceeds nothing
            (false, false) => (self - other).abs() > deadband,
        }
    }
}

pub(crate) struct OctetStringDetector;

impl Deadband<u32> {
    pub(crate) fn new(value: u32) -> Self {
        Self { deadband: value }
    }
}

impl Deadband<f64> {
    /// Analog dead-bands must be zero or positive, which rejects NaN
    pub(crate) fn analog(value: f64) -> Option<Self> {
        (value >= 0.0).then_some(Self { deadband: value })
    }
}

impl<N: Distance> Deadband<N> {
    fn exceeded(&self, lhs: N, rhs: N) -> bool {
        lhs.exceeds(rhs, self.deadband)
    }
}

/// the state of a binary type is one of its wire flags
impl<T> EventDetector<T> for FlagsDetector
where
    T: Family + WireFlags,
{
    fn is_event(&self, new: &T, old: &T) -> bool {
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
    T: Family + HasValue<N> + WireFlags,
    N: Distance,
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

/// Write the selected `points` of a `range`, each as the requested variation or its own
/// default, promoted for its value, or the rest of the range to resume at if one doesn't fit
pub(crate) fn write_points<'a, T>(
    cursor: &mut WriteCursor,
    range: IndexRange,
    variation: Option<T::StaticVariation>,
    points: impl Iterator<Item = (u16, &'a T, T::StaticVariation)>,
) -> Result<(), VariationRange>
where
    T: Family + 'a,
{
    let mut writer = RangeWriter::new();
    for (index, value, default) in points {
        // first determine what variation should be written
        let selected = variation.unwrap_or(default).promote(value);

        if writer.write(cursor, index, value, selected).is_err() {
            // ran out of space, tell calling code to resume at this index, which the
            // range always includes
            return match range.resume_at(index) {
                Some(rest) => Err(T::selection(variation).with(rest)),
                None => Ok(()),
            };
        }
    }

    Ok(())
}

/// Write the analog input dead-bands of a `range`, as the requested variation or g34v3, or the
/// rest of the range to resume at if one doesn't fit
pub(crate) fn write_dead_bands<'a>(
    cursor: &mut WriteCursor,
    range: IndexRange,
    variation: Option<AnalogInputDeadBandVariation>,
    dead_bands: impl Iterator<Item = (u16, &'a f64)>,
) -> Result<(), VariationRange> {
    let mut writer = RangeWriter::new();
    // first determine what variation should be written
    let var = variation.unwrap_or(AnalogInputDeadBandVariation::Group34Var3);
    for (index, dead_band) in dead_bands {
        if writer.write(cursor, index, dead_band, var).is_err() {
            // ran out of space, tell calling code to resume at this index, which the
            // range always includes
            return match range.resume_at(index) {
                Some(rest) => Err(VariationRange::new(
                    rest,
                    SpecificVariation::AnalogDeadBand(variation),
                )),
                None => Ok(()),
            };
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn float_deadband_treats_nan_as_a_value() {
        const OTHER_NAN: f64 = f64::from_bits(0x7FF8_0000_0000_0001);
        let deadband = Deadband::analog(1.0).unwrap();
        assert!(!deadband.exceeded(2.0, 2.5));
        assert!(deadband.exceeded(-1.0, 1.0));
        assert!(deadband.exceeded(f64::MAX, f64::MIN));
        // a change between NaN and a number or an infinity exceeds any finite deadband
        assert!(deadband.exceeded(f64::NAN, 1.0));
        assert!(deadband.exceeded(1.0, f64::NAN));
        assert!(deadband.exceeded(f64::NAN, f64::INFINITY));
        assert!(Deadband::analog(f64::MAX).unwrap().exceeded(f64::NAN, 0.0));
        // NaN to NaN doesn't, whatever the payload
        assert!(!deadband.exceeded(f64::NAN, f64::NAN));
        assert!(!deadband.exceeded(f64::NAN, OTHER_NAN));
        // a change involving infinity does unless both are the same infinity
        assert!(!deadband.exceeded(f64::INFINITY, f64::INFINITY));
        assert!(!deadband.exceeded(f64::NEG_INFINITY, f64::NEG_INFINITY));
        assert!(deadband.exceeded(f64::INFINITY, f64::NEG_INFINITY));
        assert!(deadband.exceeded(f64::INFINITY, 1.0));
        // an infinite deadband suppresses every value change except to and from NaN
        let infinite = Deadband::analog(f64::INFINITY).unwrap();
        assert!(infinite.exceeded(f64::NAN, 1.0));
        assert!(infinite.exceeded(f64::INFINITY, f64::NAN));
        assert!(!infinite.exceeded(f64::NAN, OTHER_NAN));
        assert!(!infinite.exceeded(f64::MAX, f64::MIN));
        assert!(!infinite.exceeded(f64::INFINITY, f64::NEG_INFINITY));
        assert!(!infinite.exceeded(f64::INFINITY, 1.0));
    }

    #[test]
    fn float_deadband_treats_signed_zeros_as_the_same_value() {
        for deadband in [0.0, -0.0, 1.0] {
            let deadband = Deadband::analog(deadband).unwrap();
            assert!(!deadband.exceeded(0.0, -0.0));
            assert!(!deadband.exceeded(-0.0, 0.0));
        }
    }

    /// Deadband laws. The float deadband is symmetric, never exceeded by the same value, never
    /// exceeded by a larger deadband when a smaller one isn't, and exact at its limits: a zero
    /// deadband reports every change of value, an infinite one only a change to or from NaN, and
    /// a finite deadband equal to the distance isn't exceeded while the next smaller one is. An
    /// analog event is a change of flags or an exceeded deadband. The integer deadband is
    /// exceeded exactly when the distance, computed in u64, is larger.
    ///
    /// Oracles: value equality by `partial_cmp`, and u64 arithmetic for the integer deadband,
    /// are independent of the code. The finite float boundary takes the distance from the same
    /// subtraction the code uses, so it checks the comparison, not the distance.
    ///
    /// Generators: float pairs are any bits, special values, small values, or a value and a
    /// nearby one; deadbands are special values, small ones or any valid one. Integer cases are
    /// two independent values (often 0 or `u32::MAX`), or two values exactly the deadband apart,
    /// one closer or one further, in either order, so the boundary on each side is drawn at small
    /// and large deadbands alike.
    ///
    /// Known gaps: special float pairs come from drawing two special values, so a NaN and an
    /// infinity, opposite infinities, or 0.0 and -0.0 aren't drawn in every run;
    /// `float_deadband_treats_nan_as_a_value` pins the first two and
    /// `float_deadband_treats_signed_zeros_as_the_same_value` the last.
    mod deadband_properties {
        use super::*;
        use proptest::prelude::*;

        const OTHER_NAN: f64 = f64::from_bits(0x7FF8_0000_0000_0001);

        fn special() -> impl Strategy<Value = f64> {
            prop::sample::select(vec![
                0.0,
                -0.0,
                1.0,
                f64::MAX,
                f64::MIN,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NAN,
                -f64::NAN,
                OTHER_NAN,
                f64::from_bits(1),
            ])
        }

        fn value() -> impl Strategy<Value = f64> {
            prop_oneof![
                any::<u64>().prop_map(f64::from_bits),
                special(),
                -10.0f64..10.0,
            ]
        }

        /// two values, often close enough together that a small deadband decides
        fn pair() -> impl Strategy<Value = (f64, f64)> {
            prop_oneof![
                (value(), value()),
                (value(), -4.0f64..4.0).prop_map(|(a, delta)| (a, a + delta)),
            ]
        }

        /// a valid deadband: zero or positive, possibly infinite
        fn deadband() -> impl Strategy<Value = f64> {
            prop_oneof![
                prop::sample::select(vec![0.0, -0.0, 1.0, f64::MAX, f64::INFINITY]),
                0.0f64..5.0,
                any::<u64>().prop_filter_map("a valid deadband", |bits| {
                    let value = f64::from_bits(bits);
                    (value >= 0.0).then_some(value)
                }),
            ]
        }

        fn integer_value() -> impl Strategy<Value = u32> {
            prop_oneof![any::<u32>(), Just(0), Just(u32::MAX)]
        }

        fn integer_deadband() -> impl Strategy<Value = u32> {
            prop_oneof![any::<u32>(), 0u32..8]
        }

        /// two values and a deadband; half the time the values are the deadband apart, or one
        /// closer or one further, in either order
        fn integer_case() -> impl Strategy<Value = (u32, u32, u32)> {
            prop_oneof![
                (integer_value(), integer_value(), integer_deadband()),
                (any::<u32>(), integer_deadband(), -1i32..=1, any::<bool>()).prop_map(
                    |(start, deadband, step, down)| {
                        let distance = deadband.saturating_add_signed(step);
                        // exactly `distance` apart, moved down from the top if needed
                        let high = start.saturating_add(distance);
                        let low = high.saturating_sub(distance);
                        if down {
                            (high, low, deadband)
                        } else {
                            (low, high, deadband)
                        }
                    }
                ),
            ]
        }

        fn exceeded(a: f64, b: f64, deadband: f64) -> bool {
            Deadband::analog(deadband).unwrap().exceeded(a, b)
        }

        /// equal in value, where -0.0 equals 0.0 and every NaN equals every other
        fn same_value(a: f64, b: f64) -> bool {
            a.partial_cmp(&b) == Some(std::cmp::Ordering::Equal) || (a.is_nan() && b.is_nan())
        }

        proptest! {
            #![proptest_config(crate::util::test_support::proptest_config(512))]

            #[test]
            fn float_deadband_is_symmetric_and_never_exceeded_by_the_same_value(
                (a, b) in pair(),
                deadband in deadband(),
            ) {
                prop_assert_eq!(exceeded(a, b, deadband), exceeded(b, a, deadband));
                prop_assert!(!exceeded(a, a, deadband));
            }

            #[test]
            fn a_larger_float_deadband_never_adds_an_event(
                (a, b) in pair(),
                d1 in deadband(),
                d2 in deadband(),
            ) {
                let (small, large) = if d1 <= d2 { (d1, d2) } else { (d2, d1) };
                if exceeded(a, b, large) {
                    prop_assert!(exceeded(a, b, small));
                }
            }

            #[test]
            fn float_deadband_limits_are_exact(
                (a, b) in pair(),
                deadband in deadband(),
            ) {
                // a zero deadband reports every change of value, and nothing else
                prop_assert_eq!(exceeded(a, b, 0.0), !same_value(a, b));
                // a change to or from NaN exceeds every deadband, infinite included, and that
                // is the only change an infinite deadband reports
                let one_nan = a.is_nan() != b.is_nan();
                if one_nan {
                    prop_assert!(exceeded(a, b, deadband));
                }
                prop_assert_eq!(exceeded(a, b, f64::INFINITY), one_nan);
                // a finite deadband equal to the distance isn't exceeded, and the next smaller
                // one is
                let distance = (a - b).abs();
                if distance.is_finite() && distance > 0.0 {
                    prop_assert!(!exceeded(a, b, distance));
                    prop_assert!(exceeded(a, b, distance.next_down()));
                }
            }

            #[test]
            fn analog_event_when_flags_differ_or_the_deadband_is_exceeded(
                (a, b) in pair(),
                deadband in deadband(),
                flags in any::<u8>(),
                other_flags in any::<u8>(),
            ) {
                let detector = Deadband::analog(deadband).unwrap();
                let event = |new_flags: u8, old_flags: u8| {
                    let new = AnalogInput::new(a, Flags::new(new_flags), Time::unsynchronized(0));
                    let old = AnalogInput::new(b, Flags::new(old_flags), Time::unsynchronized(0));
                    detector.is_event(&new, &old)
                };
                // with the same flags, only the deadband decides
                prop_assert_eq!(event(flags, flags), exceeded(a, b, deadband));
                prop_assert_eq!(
                    event(flags, other_flags),
                    flags != other_flags || exceeded(a, b, deadband)
                );
            }

            #[test]
            fn integer_deadband_matches_the_distance((a, b, deadband) in integer_case()) {
                // the larger value is past the smaller one plus the deadband
                let (low, high) = if a <= b { (a, b) } else { (b, a) };
                let limit = u64::from(low).checked_add(u64::from(deadband));
                let detector = Deadband::new(deadband);
                prop_assert_eq!(
                    detector.exceeded(a, b),
                    limit.is_some_and(|limit| u64::from(high) > limit)
                );
                prop_assert_eq!(detector.exceeded(a, b), detector.exceeded(b, a));
            }
        }
    }

    #[test]
    fn written_analog_deadband_treats_negative_values_as_zero() {
        let mut db = StaticDatabase::default();
        assert!(db.add(0, analog_config(StaticAnalogInputVariation::Group30Var1)));

        let cases = [
            (0.0, Some(0.0)),
            (1.5, Some(1.5)),
            (1e-310, Some(1e-310)),
            (f64::INFINITY, Some(f64::INFINITY)),
            (-1.0, Some(0.0)),
            (f64::NEG_INFINITY, Some(0.0)),
            (f64::NAN, None),
        ];
        for (deadband, stored) in cases {
            assert_eq!(db.set_analog_deadband(0, deadband), stored, "{deadband}");
        }
        // -0.0 is stored as written
        assert_eq!(
            db.set_analog_deadband(0, -0.0).map(f64::to_bits),
            Some((-0.0f64).to_bits())
        );
        // no analog input at the index
        assert_eq!(db.set_analog_deadband(1, 1.0), None);
    }

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
            detector: Deadband::analog(0.0).unwrap(),
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

        assert_eq!(db.write(&mut cursor), WriteProgress::Complete);

        assert_eq!(
            cursor.written(),
            [
                // g1v2 - s/s == 0, restart
                1, 2, 0x01, 0, 0, 0, 0, 0x02, // g20v1 - s/s == 1, restart, value == 0
                20, 1, 0x01, 1, 0, 1, 0, 0x02, 0, 0, 0, 0,
                // g30v1 - s/s == 2, restart, value == 0
                30, 1, 0x01, 2, 0, 2, 0, 0x02, 0, 0, 0, 0,
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
            assert_eq!(db.write(&mut cursor), WriteProgress::Partial);

            assert_eq!(
                cursor.written(),
                [
                    // g1v2 - s/s == 0, restart
                    1, 2, 0x01, 00, 00, 00, 00, 0x02,
                ]
            )
        }

        {
            let mut cursor = WriteCursor::new(buffer.as_mut());
            assert_eq!(db.write(&mut cursor), WriteProgress::Partial);

            assert_eq!(
                cursor.written(),
                [
                    // g20v1 - s/s == 1, restart, value == 0
                    20, 1, 0x01, 1, 0, 1, 0, 0x02, 0, 0, 0, 0,
                ]
            )
        }

        {
            let mut cursor = WriteCursor::new(buffer.as_mut());
            assert_eq!(db.write(&mut cursor), WriteProgress::Complete);

            assert_eq!(
                cursor.written(),
                [
                    // g30v1 - s/s == 2, restart, value == 0
                    30, 1, 0x01, 2, 0, 2, 0, 0x02, 0, 0, 0, 0,
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

        assert_eq!(db.write(&mut cursor), WriteProgress::Complete);

        assert_eq!(
            cursor.written(),
            [
                // g1v2 - s/s == 0, restart
                1, 2, 0x01, 0, 0, 0, 0, 0x02,
            ]
        )
    }

    fn frozen_counter_config(var: StaticFrozenCounterVariation) -> PointConfig<FrozenCounter> {
        PointConfig {
            class: Some(EventClass::Class1),
            s_var: var,
            e_var: EventFrozenCounterVariation::Group23Var1,
            detector: Deadband::new(0),
        }
    }

    fn write_class_zero(db: &mut StaticDatabase) -> Vec<u8> {
        db.select_class_zero();

        let mut buffer = [0u8; 64];
        let mut cursor = WriteCursor::new(buffer.as_mut());

        assert_eq!(db.write(&mut cursor), WriteProgress::Complete);

        cursor.written().to_vec()
    }

    #[test]
    fn promotes_counter_variations_without_flags_if_flags_other_than_just_online() {
        let mut db = StaticDatabase::default();

        assert!(db.add(0, counter_config(StaticCounterVariation::Group20Var5)));
        assert!(db.add(1, counter_config(StaticCounterVariation::Group20Var6)));
        assert!(db.add(2, counter_config(StaticCounterVariation::Group20Var5)));

        let online = Counter::new(0, Flags::ONLINE, Time::unsynchronized(0));
        db.update(&online, 2, UpdateOptions::no_event());

        assert_eq!(
            write_class_zero(&mut db),
            [
                // g20v1 - s/s == 0, restart, value == 0
                20, 1, 0x01, 0, 0, 0, 0, 0x02, 0, 0, 0, 0,
                // g20v2 - s/s == 1, restart, value == 0
                20, 2, 0x01, 1, 0, 1, 0, 0x02, 0, 0, // g20v5 - s/s == 2, value == 0
                20, 5, 0x01, 2, 0, 2, 0, 0, 0, 0, 0,
            ]
        )
    }

    #[test]
    fn promotes_frozen_counter_variations_without_flags_if_flags_other_than_just_online() {
        let mut db = StaticDatabase::default();

        assert!(db.add(
            0,
            frozen_counter_config(StaticFrozenCounterVariation::Group21Var9)
        ));
        assert!(db.add(
            1,
            frozen_counter_config(StaticFrozenCounterVariation::Group21Var10)
        ));
        assert!(db.add(
            2,
            frozen_counter_config(StaticFrozenCounterVariation::Group21Var9)
        ));

        let online = FrozenCounter::new(0, Flags::ONLINE, Time::unsynchronized(0));
        db.update(&online, 2, UpdateOptions::no_event());

        assert_eq!(
            write_class_zero(&mut db),
            [
                // g21v1 - s/s == 0, restart, value == 0
                21, 1, 0x01, 0, 0, 0, 0, 0x02, 0, 0, 0, 0,
                // g21v2 - s/s == 1, restart, value == 0
                21, 2, 0x01, 1, 0, 1, 0, 0x02, 0, 0, // g21v9 - s/s == 2, value == 0
                21, 9, 0x01, 2, 0, 2, 0, 0, 0, 0, 0,
            ]
        )
    }

    #[test]
    fn promotes_analog_variations_without_flags_if_flags_other_than_just_online() {
        let mut db = StaticDatabase::default();

        assert!(db.add(0, analog_config(StaticAnalogInputVariation::Group30Var3)));
        assert!(db.add(1, analog_config(StaticAnalogInputVariation::Group30Var4)));
        assert!(db.add(2, analog_config(StaticAnalogInputVariation::Group30Var3)));

        // fits in 32 bits, but not in 16 bits
        let online = AnalogInput::new(40000.0, Flags::ONLINE, Time::unsynchronized(0));
        db.update(&online, 0, UpdateOptions::no_event());
        db.update(&online, 1, UpdateOptions::no_event());

        assert_eq!(
            write_class_zero(&mut db),
            [
                // g30v3 - s/s == 0, value == 40000
                30, 3, 0x01, 0, 0, 0, 0, 0x40, 0x9C, 0, 0,
                // g30v2 - s/s == 1, online and over range, value == i16::MAX
                30, 2, 0x01, 1, 0, 1, 0, 0x21, 0xFF, 0x7F,
                // g30v1 - s/s == 2, restart, value == 0
                30, 1, 0x01, 2, 0, 2, 0, 0x02, 0, 0, 0, 0,
            ]
        )
    }

    #[test]
    fn promotes_counter_variations_without_flags_unless_flags_are_exactly_online() {
        let mut db = StaticDatabase::default();

        assert!(db.add(0, counter_config(StaticCounterVariation::Group20Var5)));
        assert!(db.add(1, counter_config(StaticCounterVariation::Group20Var5)));
        assert!(db.add(2, counter_config(StaticCounterVariation::Group20Var6)));

        let forced = Counter::new(
            0,
            Flags::ONLINE | Flags::LOCAL_FORCED,
            Time::unsynchronized(0),
        );
        let no_flags = Counter::new(0, Flags::new(0), Time::unsynchronized(0));
        let online = Counter::new(0, Flags::ONLINE, Time::unsynchronized(0));
        db.update(&forced, 0, UpdateOptions::no_event());
        db.update(&no_flags, 1, UpdateOptions::no_event());
        db.update(&online, 2, UpdateOptions::no_event());

        assert_eq!(
            write_class_zero(&mut db),
            [
                // g20v1 - s/s == 0 to 1, online and local forced, then no flags, values == 0
                20, 1, 0x01, 0, 0, 1, 0, 0x11, 0, 0, 0, 0, 0x00, 0, 0, 0, 0,
                // g20v6 - s/s == 2, value == 0
                20, 6, 0x01, 2, 0, 2, 0, 0, 0,
            ]
        )
    }

    #[test]
    fn promotes_frozen_counter_variations_without_flags_unless_flags_are_exactly_online() {
        let mut db = StaticDatabase::default();

        assert!(db.add(
            0,
            frozen_counter_config(StaticFrozenCounterVariation::Group21Var9)
        ));
        assert!(db.add(
            1,
            frozen_counter_config(StaticFrozenCounterVariation::Group21Var10)
        ));
        assert!(db.add(
            2,
            frozen_counter_config(StaticFrozenCounterVariation::Group21Var10)
        ));

        let forced = FrozenCounter::new(
            0,
            Flags::ONLINE | Flags::LOCAL_FORCED,
            Time::unsynchronized(0),
        );
        let no_flags = FrozenCounter::new(0, Flags::new(0), Time::unsynchronized(0));
        let online = FrozenCounter::new(0, Flags::ONLINE, Time::unsynchronized(0));
        db.update(&forced, 0, UpdateOptions::no_event());
        db.update(&no_flags, 1, UpdateOptions::no_event());
        db.update(&online, 2, UpdateOptions::no_event());

        assert_eq!(
            write_class_zero(&mut db),
            [
                // g21v1 - s/s == 0, online and local forced, value == 0
                21, 1, 0x01, 0, 0, 0, 0, 0x11, 0, 0, 0, 0,
                // g21v2 - s/s == 1, no flags, value == 0
                21, 2, 0x01, 1, 0, 1, 0, 0x00, 0, 0, // g21v10 - s/s == 2, value == 0
                21, 10, 0x01, 2, 0, 2, 0, 0, 0,
            ]
        )
    }

    #[test]
    fn promotes_analog_variations_without_flags_on_the_flags_the_variation_with_flags_reports() {
        let mut db = StaticDatabase::default();

        assert!(db.add(0, analog_config(StaticAnalogInputVariation::Group30Var3)));
        assert!(db.add(1, analog_config(StaticAnalogInputVariation::Group30Var3)));
        assert!(db.add(2, analog_config(StaticAnalogInputVariation::Group30Var4)));
        assert!(db.add(3, analog_config(StaticAnalogInputVariation::Group30Var4)));

        let values = [3e9, -3e9, -40000.0, 100.0];
        for (index, value) in values.into_iter().enumerate() {
            let online = AnalogInput::new(value, Flags::ONLINE, Time::unsynchronized(0));
            db.update(
                &online,
                index.try_into().unwrap(),
                UpdateOptions::no_event(),
            );
        }

        assert_eq!(
            write_class_zero(&mut db),
            [
                // g30v1 - s/s == 0 to 1, online and over range, values == i32::MAX and i32::MIN
                30, 1, 0x01, 0, 0, 1, 0, 0x21, 0xFF, 0xFF, 0xFF, 0x7F, 0x21, 0x00, 0x00, 0x00,
                0x80, // g30v2 - s/s == 2, online and over range, value == i16::MIN
                30, 2, 0x01, 2, 0, 2, 0, 0x21, 0x00, 0x80,
                // g30v4 - s/s == 3, value == 100
                30, 4, 0x01, 3, 0, 3, 0, 0x64, 0x00,
            ]
        )
    }

    #[test]
    fn promotes_explicitly_requested_variation_without_flags_across_fragments() {
        let mut db = StaticDatabase::default();

        for index in 0..3 {
            assert!(db.add(index, counter_config(StaticCounterVariation::Group20Var1)));
        }

        // only the middle point has flags other than ONLINE
        db.update(
            &Counter::new(1, Flags::ONLINE, Time::unsynchronized(0)),
            0,
            UpdateOptions::no_event(),
        );
        db.update(
            &Counter::new(2, Flags::RESTART, Time::unsynchronized(0)),
            1,
            UpdateOptions::no_event(),
        );
        db.update(
            &Counter::new(3, Flags::ONLINE, Time::unsynchronized(0)),
            2,
            UpdateOptions::no_event(),
        );

        assert_eq!(
            db.select(
                StaticTarget::Counter(Some(StaticCounterVariation::Group20Var5)),
                None
            ),
            (Iin2::default(), Visits::new(3))
        );

        let mut buffer = [0u8; 12]; // can only fit one header at a time

        {
            let mut cursor = WriteCursor::new(buffer.as_mut());
            assert_eq!(db.write(&mut cursor), WriteProgress::Partial);

            assert_eq!(
                cursor.written(),
                [
                    // g20v5 - s/s == 0, value == 1
                    20, 5, 0x01, 0, 0, 0, 0, 1, 0, 0, 0,
                ]
            )
        }

        {
            let mut cursor = WriteCursor::new(buffer.as_mut());
            assert_eq!(db.write(&mut cursor), WriteProgress::Partial);

            assert_eq!(
                cursor.written(),
                [
                    // g20v1 - s/s == 1, restart, value == 2
                    20, 1, 0x01, 1, 0, 1, 0, 0x02, 2, 0, 0, 0,
                ]
            )
        }

        {
            let mut cursor = WriteCursor::new(buffer.as_mut());
            assert_eq!(db.write(&mut cursor), WriteProgress::Complete);

            assert_eq!(
                cursor.written(),
                [
                    // g20v5 - s/s == 2, value == 3
                    20, 5, 0x01, 2, 0, 2, 0, 3, 0, 0, 0,
                ]
            )
        }
    }

    fn write_selection(db: &mut StaticDatabase) -> Vec<u8> {
        let mut buffer = [0u8; 64];
        let mut cursor = WriteCursor::new(buffer.as_mut());

        assert_eq!(db.write(&mut cursor), WriteProgress::Complete);

        cursor.written().to_vec()
    }

    #[test]
    fn check_range_reports_installed_points() {
        let mut db = StaticDatabase::default();

        // no points of the type
        assert_eq!(
            db.binary.check_range(IndexRange::new(0, 0).unwrap()),
            (Err(Iin2::OBJECT_UNKNOWN), Visits::NONE)
        );

        for index in [0, 1, 2, 4] {
            assert!(db.add(index, binary_config(StaticBinaryInputVariation::Group1Var2)));
        }

        // every index installed
        assert_eq!(
            db.binary.check_range(IndexRange::new(0, 2).unwrap()),
            (Ok(Iin2::default()), Visits::new(3))
        );
        // missing index in the middle
        assert_eq!(
            db.binary.check_range(IndexRange::new(2, 4).unwrap()),
            (Ok(Iin2::PARAMETER_ERROR), Visits::new(2))
        );
        // past the last point
        assert_eq!(
            db.binary.check_range(IndexRange::new(4, 5).unwrap()),
            (Ok(Iin2::PARAMETER_ERROR), Visits::new(1))
        );
        // no installed index
        assert_eq!(
            db.binary.check_range(IndexRange::new(5, 10).unwrap()),
            (Err(Iin2::PARAMETER_ERROR), Visits::NONE)
        );
    }

    #[test]
    fn index_range_is_never_empty() {
        let bounds = |range: Option<IndexRange>| range.map(|range| (range.start, range.stop));

        assert_eq!(bounds(IndexRange::new(2, 1)), None);
        assert_eq!(bounds(IndexRange::new(1, 1)), Some((1, 1)));

        let range = IndexRange::new(1, 4).unwrap();
        assert_eq!(bounds(range.resume_at(3)), Some((3, 4)));
        assert_eq!(bounds(range.resume_at(4)), Some((4, 4)));
        assert_eq!(bounds(range.resume_at(5)), None);
    }

    #[test]
    fn check_range_counts_the_full_range_of_indices() {
        let mut db = StaticDatabase::default();

        for index in 0..=u16::MAX {
            assert!(db.add(index, binary_config(StaticBinaryInputVariation::Group1Var2)));
        }

        let full = IndexRange::new(0, u16::MAX).unwrap();
        assert_eq!(full.len(), 65536);
        assert_eq!(
            db.binary.check_range(full),
            (Ok(Iin2::default()), Visits::new(65536))
        );

        assert!(db.remove::<BinaryInput>(1000));
        assert_eq!(
            db.binary.check_range(full),
            (Ok(Iin2::PARAMETER_ERROR), Visits::new(65535))
        );
    }

    #[test]
    fn select_sets_object_unknown_for_types_without_points() {
        let mut db = StaticDatabase::default();

        assert_eq!(
            db.select(StaticTarget::Binary(None), None),
            (Iin2::OBJECT_UNKNOWN, Visits::NONE)
        );
        assert_eq!(
            db.select(
                StaticTarget::Counter(None),
                Some(IndexRange::new(0, 0).unwrap())
            ),
            (Iin2::OBJECT_UNKNOWN, Visits::NONE)
        );
        assert_eq!(
            db.select(StaticTarget::AnalogInputDeadBand(None), None),
            (Iin2::OBJECT_UNKNOWN, Visits::NONE)
        );
        assert_eq!(
            db.select(
                StaticTarget::AnalogInputDeadBand(None),
                Some(IndexRange::new(0, 0).unwrap())
            ),
            (Iin2::OBJECT_UNKNOWN, Visits::NONE)
        );
        assert_eq!(
            db.select(StaticTarget::FrozenAnalog(None), None),
            (Iin2::OBJECT_UNKNOWN, Visits::NONE)
        );

        // nothing selected
        assert_eq!(db.selection_count(), 0);
    }

    #[test]
    fn select_of_range_without_installed_points_selects_nothing() {
        let mut db = StaticDatabase::default();

        assert!(db.add(0, counter_config(StaticCounterVariation::Group20Var1)));

        assert_eq!(
            db.select(
                StaticTarget::Counter(None),
                Some(IndexRange::new(1, 2).unwrap())
            ),
            (Iin2::PARAMETER_ERROR, Visits::NONE)
        );
        assert_eq!(db.selection_count(), 0);
    }

    #[test]
    fn select_reports_parameter_error_when_out_of_selections() {
        let mut db = StaticDatabase::default();

        assert!(db.add(0, counter_config(StaticCounterVariation::Group20Var1)));

        // every index installed, so only running out of selections sets IIN2.2; the point is
        // counted and then snapshot
        let range = Some(IndexRange::new(0, 0).unwrap());
        for _ in 0..db.selection_capacity() {
            assert_eq!(
                db.select(StaticTarget::Counter(None), range),
                (Iin2::default(), Visits::new(2))
            );
        }
        // once the selection is full, the point isn't visited
        assert_eq!(
            db.select(StaticTarget::Counter(None), range),
            (Iin2::PARAMETER_ERROR, Visits::NONE)
        );
        // but a type without points still sets IIN2.1
        assert_eq!(
            db.select(StaticTarget::Binary(None), None),
            (Iin2::OBJECT_UNKNOWN, Visits::NONE)
        );
    }

    #[test]
    fn dead_band_selection_visits_each_point_it_marks() {
        let mut db = StaticDatabase::default();
        for index in 0..3 {
            assert!(db.add(
                index,
                analog_config(StaticAnalogInputVariation::Group30Var1)
            ));
        }

        // all of them are marked; a range is counted and then marked
        assert_eq!(
            db.select(StaticTarget::AnalogInputDeadBand(None), None),
            (Iin2::default(), Visits::new(3))
        );
        assert_eq!(
            db.select(
                StaticTarget::AnalogInputDeadBand(None),
                Some(IndexRange::new(0, 2).unwrap())
            ),
            (Iin2::default(), Visits::new(6))
        );
    }

    #[test]
    fn class_zero_with_one_selection_left_selects_the_first_type_with_points() {
        // populations differ, so the visits show which type was selected
        let mut db = StaticDatabase::default();
        for index in 0..3 {
            assert!(db.add(index, binary_config(StaticBinaryInputVariation::Group1Var2)));
        }
        for index in 0..5 {
            assert!(db.add(index, counter_config(StaticCounterVariation::Group20Var1)));
        }
        let range = Some(IndexRange::new(0, 0).unwrap());
        for _ in 1..db.selection_capacity() {
            assert_eq!(
                db.select(StaticTarget::Counter(None), range).0,
                Iin2::default()
            );
        }

        // the binary inputs take the last selection, and the counters don't fit, unvisited
        assert_eq!(
            db.select_class_zero(),
            (Iin2::PARAMETER_ERROR, Visits::new(3))
        );
        assert_eq!(db.selection_count(), db.selection_capacity());
    }

    #[test]
    fn select_of_partial_range_selects_installed_points() {
        let mut db = StaticDatabase::default();

        assert!(db.add(0, counter_config(StaticCounterVariation::Group20Var1)));

        assert_eq!(
            db.select(
                StaticTarget::Counter(None),
                Some(IndexRange::new(0, 1).unwrap())
            ),
            (Iin2::PARAMETER_ERROR, Visits::new(2))
        );
        assert_eq!(
            write_selection(&mut db),
            [
                // g20v1 - s/s == 0, restart, value == 0
                20, 1, 0x01, 0, 0, 0, 0, 0x02, 0, 0, 0, 0,
            ]
        );
    }

    #[test]
    fn select_of_all_points_with_sparse_indices_has_no_error() {
        let mut db = StaticDatabase::default();

        assert!(db.add(0, counter_config(StaticCounterVariation::Group20Var1)));
        assert!(db.add(2, counter_config(StaticCounterVariation::Group20Var1)));

        // the snapshot visits only the installed points
        assert_eq!(
            db.select(StaticTarget::Counter(None), None),
            (Iin2::default(), Visits::new(2))
        );
    }

    fn selections_until_parameter_error(db: &mut StaticDatabase) -> usize {
        let range = Some(IndexRange::new(0, 0).unwrap());
        std::iter::from_fn(|| {
            (db.select(StaticTarget::Counter(None), range).0 == Iin2::default()).then_some(())
        })
        .count()
    }

    fn db_with_one_counter() -> StaticDatabase {
        let mut db = StaticDatabase::default();
        assert!(db.add(0, counter_config(StaticCounterVariation::Group20Var1)));
        db
    }

    #[test]
    fn selection_limit_defaults_to_the_default_read_header_limit() {
        let mut db = db_with_one_counter();
        assert_eq!(
            selections_until_parameter_error(&mut db),
            usize::from(OutstationConfig::DEFAULT_MAX_READ_REQUEST_HEADERS)
        );
        // a selection past the limit selects nothing
        assert_eq!(
            db.selection_count(),
            usize::from(OutstationConfig::DEFAULT_MAX_READ_REQUEST_HEADERS)
        );
    }

    #[test]
    fn reset_makes_every_selection_available_again() {
        let mut db = db_with_one_counter();
        selections_until_parameter_error(&mut db);
        db.reset();
        assert_eq!(db.selection_count(), 0);
        assert_eq!(
            selections_until_parameter_error(&mut db),
            usize::from(OutstationConfig::DEFAULT_MAX_READ_REQUEST_HEADERS)
        );
    }
}
