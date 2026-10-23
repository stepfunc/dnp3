// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::attr::{AttrSet, OwnedAttribute};
use std::ops::BitAnd;

use crate::app::control::CommandStatus;
use crate::app::format::write::HeaderWriter;
use crate::app::gen::prefixed::PrefixedVariation;
use crate::app::parse::count::CountSequence;
use crate::app::parse::parser::{HeaderCollection, ObjectHeader};
use crate::app::parse::prefix::Prefix;
use crate::app::parse::traits::Index;
use crate::app::variations::*;
use crate::app::CommandObject;
use crate::app::Timestamp;
use crate::app::Variation::Group0;
use crate::master::error::CommandResponseError;
use crate::master::{BadEncoding, TaskError};
use crate::outstation::FreezeInterval;

/// Controls how a command request is issued
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum CommandMode {
    /// Master will use the `DIRECT_OPERATE` function code in a single request/response
    DirectOperate,
    /// Master will use the `SELECT` function code followed by `OPERATE` in two pass request/response
    SelectBeforeOperate,
}

/// Controls which time synchronization procedure is used
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
#[non_exhaustive]
pub enum TimeSyncProcedure {
    /// Master will use the LAN procedure: RECORD_CURRENT_TIME followed by WRITE g50v3
    Lan,
    /// Master will use the non-LAN procedure: DELAY_MEASUREMENT followed by WRITE g50v1
    NonLan,
    /// Master will directly WRITE g50v1 (absolute time) without delay measurement
    ///
    /// **WARNING**: This is a non-conformant workaround for outstations that do not support
    /// the DELAY_MEASURE function code. This method does NOT account for network latency
    /// and may result in inaccurate time synchronization. Use only when the outstation
    /// does not support standard time synchronization procedures.
    DirectWriteAbsTime,
}

/// struct recording which event classes are enabled
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct EventClasses {
    /// enable Class 1
    pub class1: bool,
    /// enable Class 2
    pub class2: bool,
    /// enable Class 3
    pub class3: bool,
}

/// struct recording which event classes and class 0 are enabled
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct Classes {
    /// enable class zero
    pub class0: bool,
    /// enabled event classes
    pub events: EventClasses,
}

/// struct representing a one-byte range scan
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct OneByteRangeScan {
    /// variation to READ
    pub variation: Variation,
    /// start address of the READ
    pub start: u8,
    /// stop address of the READ
    pub stop: u8,
}

/// struct representing a two-byte range scan
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TwoByteRangeScan {
    /// variation to READ
    pub variation: Variation,
    /// start address of the READ
    pub start: u16,
    /// stop address of the READ
    pub stop: u16,
}

/// struct representing an "all objects" (QC = 0x06) scan
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AllObjectsScan {
    /// variation to READ
    pub variation: Variation,
}

/// struct representing a one-byte limited count scan
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct OneByteLimitedCountScan {
    /// variation to READ
    pub variation: Variation,
    /// maximum number of events
    pub count: u8,
}

/// struct representing a two-byte limited count scan
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TwoByteLimitedCountScan {
    /// variation to READ
    pub variation: Variation,
    /// maximum number of events
    pub count: u16,
}

/// Represents a single header in a WRITE request to modify dead-bands within the outstation
#[derive(Debug, Clone)]
pub struct DeadBandHeader {
    // hidden implementation
    pub(crate) inner: DeadBandHeaderVariants,
}

impl DeadBandHeader {
    /// Group 34 variation 1 with 8-bit index
    pub fn group34_var1_u8(dead_bands: Vec<(u8, u16)>) -> Self {
        let objects = objects(dead_bands, |value| Group34Var1 { value });
        Self::u8(DeadBands::V1(objects))
    }

    /// Group 34 variation 1 with 16-bit index
    pub fn group34_var1_u16(dead_bands: Vec<(u16, u16)>) -> Self {
        let objects = objects(dead_bands, |value| Group34Var1 { value });
        Self::u16(DeadBands::V1(objects))
    }

    /// Group 34 variation 2 with 8-bit index
    pub fn group34_var2_u8(dead_bands: Vec<(u8, u32)>) -> Self {
        let objects = objects(dead_bands, |value| Group34Var2 { value });
        Self::u8(DeadBands::V2(objects))
    }

    /// Group 34 variation 2 with 16-bit index
    pub fn group34_var2_u16(dead_bands: Vec<(u16, u32)>) -> Self {
        let objects = objects(dead_bands, |value| Group34Var2 { value });
        Self::u16(DeadBands::V2(objects))
    }

    /// Group 34 variation 3 with 8-bit index
    pub fn group34_var3_u8(dead_bands: Vec<(u8, f32)>) -> Self {
        let objects = objects(dead_bands, |value| Group34Var3 { value });
        Self::u8(DeadBands::V3(objects))
    }

    /// Group 34 variation 3 with 16-bit index
    pub fn group34_var3_u16(dead_bands: Vec<(u16, f32)>) -> Self {
        let objects = objects(dead_bands, |value| Group34Var3 { value });
        Self::u16(DeadBands::V3(objects))
    }

    fn u8(dead_bands: DeadBands<u8>) -> Self {
        Self {
            inner: DeadBandHeaderVariants::U8(dead_bands),
        }
    }

    fn u16(dead_bands: DeadBands<u16>) -> Self {
        Self {
            inner: DeadBandHeaderVariants::U16(dead_bands),
        }
    }
}

/// The objects for (index, value) pairs, each paired with its index
fn objects<I, T, V>(dead_bands: Vec<(I, T)>, object: impl Fn(T) -> V) -> Vec<(V, I)> {
    dead_bands
        .into_iter()
        .map(|(index, value)| (object(value), index))
        .collect()
}

/// A header of dead-bands, by the width of its indices
#[derive(Debug, Clone)]
pub(crate) enum DeadBandHeaderVariants {
    /// 8-bit indices
    U8(DeadBands<u8>),
    /// 16-bit indices
    U16(DeadBands<u16>),
}

/// The dead-bands of a header, by variation
#[derive(Debug, Clone)]
pub(crate) enum DeadBands<I> {
    /// Group 34 variation 1
    V1(Vec<(Group34Var1, I)>),
    /// Group 34 variation 2
    V2(Vec<(Group34Var2, I)>),
    /// Group 34 variation 3
    V3(Vec<(Group34Var3, I)>),
}

impl EventClasses {
    /// construct an `EventClasses` from its fields
    pub fn new(class1: bool, class2: bool, class3: bool) -> Self {
        Self {
            class1,
            class2,
            class3,
        }
    }

    /// test if any of the event classes are enabled
    pub fn any(self) -> bool {
        self.class1 || self.class2 || self.class3
    }

    /// construct an `EventClasses` with all three classes enabled
    pub const fn all() -> Self {
        Self {
            class1: true,
            class2: true,
            class3: true,
        }
    }

    /// construct an `EventClasses` with all three classes disabled
    pub const fn none() -> Self {
        Self {
            class1: false,
            class2: false,
            class3: false,
        }
    }

    pub(crate) fn write(self, writer: &mut HeaderWriter) -> Result<(), scursor::WriteError> {
        if self.class1 {
            writer.write_all_objects_header(Variation::Group60Var2)?;
        }
        if self.class2 {
            writer.write_all_objects_header(Variation::Group60Var3)?;
        }
        if self.class3 {
            writer.write_all_objects_header(Variation::Group60Var4)?;
        }
        Ok(())
    }
}

impl BitAnd for EventClasses {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self::new(
            self.class1 && rhs.class1,
            self.class2 && rhs.class2,
            self.class3 && rhs.class3,
        )
    }
}

impl Classes {
    /// construct a `Classes` from its fields
    pub const fn new(class0: bool, events: EventClasses) -> Self {
        Self { class0, events }
    }

    /// construct a `Classes` with everything enabled
    pub const fn all() -> Self {
        Self::new(true, EventClasses::all())
    }

    /// construct a `Classes` with all events, without class 0.
    pub fn class123() -> Self {
        Self::new(false, EventClasses::all())
    }

    /// construct a `Classes` with class 0 and no events
    pub fn class0() -> Self {
        Self::new(true, EventClasses::none())
    }

    /// construct a `Classes` with nothing enabled
    pub fn none() -> Self {
        Self::new(false, EventClasses::none())
    }

    /// test if any classes (0/1/2/3) are enabled
    pub(crate) fn any(&self) -> bool {
        self.class0 || self.events.any()
    }

    pub(crate) fn write(self, writer: &mut HeaderWriter) -> Result<(), scursor::WriteError> {
        self.events.write(writer)?;
        if self.class0 {
            writer.write_all_objects_header(Variation::Group60Var1)?;
        }
        Ok(())
    }
}

impl OneByteRangeScan {
    /// construct a `OneByteRangeScan` from its fields
    pub fn new(variation: Variation, start: u8, stop: u8) -> Self {
        Self {
            variation,
            start,
            stop,
        }
    }

    pub(crate) fn write(self, writer: &mut HeaderWriter) -> Result<(), scursor::WriteError> {
        writer.write_range_only(self.variation, self.start, self.stop)
    }
}

impl TwoByteRangeScan {
    /// construct a `TwoByteRangeScan` from its fields
    pub fn new(variation: Variation, start: u16, stop: u16) -> Self {
        Self {
            variation,
            start,
            stop,
        }
    }

    pub(crate) fn write(self, writer: &mut HeaderWriter) -> Result<(), scursor::WriteError> {
        writer.write_range_only(self.variation, self.start, self.stop)
    }
}

impl AllObjectsScan {
    /// construct an `AllObjectsScan` from the variation
    pub fn new(variation: Variation) -> Self {
        Self { variation }
    }

    pub(crate) fn write(self, writer: &mut HeaderWriter) -> Result<(), scursor::WriteError> {
        writer.write_all_objects_header(self.variation)
    }
}

impl OneByteLimitedCountScan {
    /// construct an [`OneByteLimitedCountScan`] from the variation
    pub fn new(variation: Variation, count: u8) -> Self {
        Self { variation, count }
    }

    pub(crate) fn write(self, writer: &mut HeaderWriter) -> Result<(), scursor::WriteError> {
        writer.write_limited_count(self.variation, self.count)
    }
}

impl TwoByteLimitedCountScan {
    /// construct an [`TwoByteLimitedCountScan`] from the variation
    pub fn new(variation: Variation, count: u16) -> Self {
        Self { variation, count }
    }

    pub(crate) fn write(self, writer: &mut HeaderWriter) -> Result<(), scursor::WriteError> {
        writer.write_limited_count(self.variation, self.count)
    }
}

/// Enum representing all of the allowed scan types
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ReadHeader {
    /// variant for one byte range scans
    Range8(OneByteRangeScan),
    /// variant for two byte range scans
    Range16(TwoByteRangeScan),
    /// variant for all objects scans
    AllObjects(AllObjectsScan),
    /// variant for one byte limited count
    LimitedCount8(OneByteLimitedCountScan),
    /// variant for two byte limited count
    LimitedCount16(TwoByteLimitedCountScan),
}

impl ReadHeader {
    /// construct a one byte range [`ReadHeader`]
    pub fn one_byte_range(variation: Variation, start: u8, stop: u8) -> Self {
        ReadHeader::Range8(OneByteRangeScan::new(variation, start, stop))
    }

    /// construct a two range [`ReadHeader`]
    pub fn two_byte_range(variation: Variation, start: u16, stop: u16) -> Self {
        ReadHeader::Range16(TwoByteRangeScan::new(variation, start, stop))
    }

    /// construct an all objects [`ReadHeader`]
    pub fn all_objects(variation: Variation) -> Self {
        ReadHeader::AllObjects(AllObjectsScan::new(variation))
    }

    /// construct a one byte limited count scan [`ReadHeader`]
    pub fn one_byte_limited_count(variation: Variation, count: u8) -> Self {
        ReadHeader::LimitedCount8(OneByteLimitedCountScan::new(variation, count))
    }

    /// construct a two byte limited count scan [`ReadHeader`]
    pub fn two_byte_limited_count(variation: Variation, count: u16) -> Self {
        ReadHeader::LimitedCount16(TwoByteLimitedCountScan::new(variation, count))
    }

    pub(crate) fn format(self, writer: &mut HeaderWriter) -> Result<(), scursor::WriteError> {
        match self {
            ReadHeader::Range8(scan) => scan.write(writer),
            ReadHeader::Range16(scan) => scan.write(writer),
            ReadHeader::AllObjects(scan) => scan.write(writer),
            ReadHeader::LimitedCount8(scan) => scan.write(writer),
            ReadHeader::LimitedCount16(scan) => scan.write(writer),
        }
    }
}

/// Builder for write requests that hides the underlying type
#[derive(Clone, Debug, Default)]
pub struct Headers {
    headers: Vec<Header>,
}

#[derive(Clone, Debug)]
enum Header {
    Read(ReadHeader),
    TimeAndInterval(FreezeInterval),
    Attribute(OwnedAttribute),
}

impl Header {
    pub(crate) fn format(&self, writer: &mut HeaderWriter) -> Result<(), TaskError> {
        match self {
            Header::Read(x) => {
                x.format(writer)?;
            }
            Header::TimeAndInterval(x) => {
                let g50v2: Group50Var2 = <FreezeInterval as Into<Group50Var2>>::into(*x);
                writer.write_count_of_one(g50v2)?;
            }
            Header::Attribute(x) => {
                writer.write_attribute(x)?;
            }
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), BadEncoding> {
        match self {
            Header::Read(_) | Header::TimeAndInterval(_) => Ok(()),
            Header::Attribute(x) => x.check_encodable().map_err(BadEncoding::Attribute),
        }
    }

    #[cfg(feature = "ffi")]
    pub(crate) fn to_read_header(&self) -> Option<ReadHeader> {
        match self {
            Header::Read(x) => Some(*x),
            Header::TimeAndInterval(_) => None,
            Header::Attribute(_) => None,
        }
    }
}

impl Headers {
    /// Construct an empty set of request headers
    pub fn new() -> Self {
        Default::default()
    }

    /// Convert this generic request into a ReadRequest dropping irrelevant headers
    #[cfg(feature = "ffi")]
    pub fn to_read_request(&self) -> ReadRequest {
        ReadRequest::MultipleHeader(
            self.headers
                .iter()
                .filter_map(|x| x.to_read_header())
                .collect(),
        )
    }

    /// Add a header to the collection
    #[cfg(feature = "ffi")]
    pub fn push_read_header(&mut self, header: ReadHeader) {
        self.headers.push(header.into());
    }

    /// Add a header to the collection
    #[cfg(feature = "ffi")]
    pub fn push_freeze_interval(&mut self, interval: FreezeInterval) {
        self.headers.push(Header::TimeAndInterval(interval));
    }

    /// Add an attribute header to the collection
    #[cfg(feature = "ffi")]
    pub fn push_attr(&mut self, attr: OwnedAttribute) {
        self.headers.push(Header::Attribute(attr));
    }

    /// Add an all objects header (0x06) with the specified variation
    pub fn add_all_objects(self, variation: Variation) -> Self {
        self.add(ReadHeader::all_objects(variation).into())
    }

    /// Add 8-bit start/stop header (0x00) with the specified variation
    pub fn add_range_8(self, variation: Variation, start: u8, stop: u8) -> Self {
        self.add(ReadHeader::one_byte_range(variation, start, stop).into())
    }

    /// Add 16-bit start/stop header (0x01) with the specified variation
    pub fn add_range_16(self, variation: Variation, start: u16, stop: u16) -> Self {
        self.add(ReadHeader::two_byte_range(variation, start, stop).into())
    }

    /// add a one byte limited count header (0x7) with the specified count and variation
    pub fn add_one_byte_limited_count(self, variation: Variation, count: u8) -> Self {
        self.add(ReadHeader::one_byte_limited_count(variation, count).into())
    }

    /// add a two byte limited count (0x08) header with the specified count and variation
    pub fn add_two_byte_limited_count(self, variation: Variation, count: u16) -> Self {
        self.add(ReadHeader::two_byte_limited_count(variation, count).into())
    }

    /// Add a limited count (0x07) with a single g50v2
    ///
    /// This is useful when constructing freeze-at-requests
    pub fn add_time_and_interval(self, time: Timestamp, interval_ms: u32) -> Self {
        self.add_freeze_interval(FreezeInterval::new(time, interval_ms))
    }

    /// Add a limited count (0x07) with a single g50v2
    ///
    /// This is useful when constructing freeze-at-requests
    ///
    /// This can also be accomplished using the `add_time_and_interval` method although
    /// this method provides cleaner semantics.
    pub fn add_freeze_interval(self, interval: FreezeInterval) -> Self {
        self.add(Header::TimeAndInterval(interval))
    }

    /// Add an attribute header that will be encoded using 0x00 - one byte start/stop
    pub fn add_attribute(self, attr: OwnedAttribute) -> Self {
        self.add(Header::Attribute(attr))
    }

    pub(crate) fn write(&self, writer: &mut HeaderWriter) -> Result<(), TaskError> {
        for header in self.headers.iter() {
            header.validate().map_err(TaskError::BadEncoding)?;
        }

        for header in self.headers.iter() {
            header.format(writer)?;
        }
        Ok(())
    }

    fn add(mut self, header: Header) -> Self {
        self.headers.push(header);
        self
    }
}

/// Enum representing all of the READ request types available from the master API
#[derive(Clone, Debug)]
pub enum ReadRequest {
    /// Read a single header
    SingleHeader(ReadHeader),
    /// Read class data
    ClassScan(Classes),
    /// Read multiple headers
    MultipleHeader(Vec<ReadHeader>),
}

impl ReadRequest {
    /// construct a `ReadRequest` from a `Classes` instance
    pub fn class_scan(scan: Classes) -> Self {
        Self::ClassScan(scan)
    }

    /// construct a `ReadRequest` consisting of a single one-byte range
    pub fn one_byte_range(variation: Variation, start: u8, stop: u8) -> Self {
        Self::SingleHeader(ReadHeader::one_byte_range(variation, start, stop))
    }

    /// construct a `ReadRequest` consisting of a single one-byte range specifying a specific device attribute
    pub fn device_attribute<T: Into<u8>>(variation: T, set: AttrSet) -> Self {
        Self::one_byte_range(Group0(variation.into()), set.value(), set.value())
    }

    /// construct a `ReadRequest` consisting of a single two-byte range
    pub fn two_byte_range(variation: Variation, start: u16, stop: u16) -> Self {
        Self::SingleHeader(ReadHeader::two_byte_range(variation, start, stop))
    }

    /// construct an all objects `ReadRequest` for a particular variation
    pub fn all_objects(variation: Variation) -> Self {
        Self::SingleHeader(ReadHeader::all_objects(variation))
    }

    /// construct a `ReadRequest` consisting of multiple headers
    pub fn multiple_headers(headers: &[ReadHeader]) -> Self {
        Self::MultipleHeader(headers.to_vec())
    }

    pub(crate) fn format(&self, writer: &mut HeaderWriter) -> Result<(), scursor::WriteError> {
        match self {
            ReadRequest::SingleHeader(req) => req.format(writer),
            ReadRequest::ClassScan(req) => req.write(writer),
            ReadRequest::MultipleHeader(reqs) => {
                for req in reqs {
                    req.format(writer)?;
                }
                Ok(())
            }
        }
    }
}

impl From<ReadHeader> for Header {
    fn from(value: ReadHeader) -> Self {
        Header::Read(value)
    }
}

/// A header of commands with a one-byte or two-byte count and index prefix
#[derive(Clone)]
pub(crate) enum CommandHeader {
    U8(CommandItems<u8>),
    U16(CommandItems<u16>),
}

/// The commands in a header, by object type
#[derive(Clone)]
pub(crate) enum CommandItems<I> {
    G12V1(Vec<(Group12Var1, I)>),
    G41V1(Vec<(Group41Var1, I)>),
    G41V2(Vec<(Group41Var2, I)>),
    G41V3(Vec<(Group41Var3, I)>),
    G41V4(Vec<(Group41Var4, I)>),
}

/// A command object that `CommandBuilder` adds to a header of its type
pub(crate) trait Command: CommandObject {
    /// a header of `items`
    fn header<I>(items: Vec<(Self, I)>) -> CommandItems<I>;

    /// the items of `header` if it's of this type
    fn items<I>(header: &mut CommandItems<I>) -> Option<&mut Vec<(Self, I)>>;
}

/// An index width that `CommandBuilder` addresses commands with
pub(crate) trait CommandIndex: Index {
    /// a header of `items`
    fn header(items: CommandItems<Self>) -> CommandHeader;

    /// the items of `header` if it has this index width
    fn items(header: &mut CommandHeader) -> Option<&mut CommandItems<Self>>;
}

impl Command for Group12Var1 {
    fn header<I>(items: Vec<(Self, I)>) -> CommandItems<I> {
        CommandItems::G12V1(items)
    }

    fn items<I>(header: &mut CommandItems<I>) -> Option<&mut Vec<(Self, I)>> {
        match header {
            CommandItems::G12V1(items) => Some(items),
            _ => None,
        }
    }
}

impl Command for Group41Var1 {
    fn header<I>(items: Vec<(Self, I)>) -> CommandItems<I> {
        CommandItems::G41V1(items)
    }

    fn items<I>(header: &mut CommandItems<I>) -> Option<&mut Vec<(Self, I)>> {
        match header {
            CommandItems::G41V1(items) => Some(items),
            _ => None,
        }
    }
}

impl Command for Group41Var2 {
    fn header<I>(items: Vec<(Self, I)>) -> CommandItems<I> {
        CommandItems::G41V2(items)
    }

    fn items<I>(header: &mut CommandItems<I>) -> Option<&mut Vec<(Self, I)>> {
        match header {
            CommandItems::G41V2(items) => Some(items),
            _ => None,
        }
    }
}

impl Command for Group41Var3 {
    fn header<I>(items: Vec<(Self, I)>) -> CommandItems<I> {
        CommandItems::G41V3(items)
    }

    fn items<I>(header: &mut CommandItems<I>) -> Option<&mut Vec<(Self, I)>> {
        match header {
            CommandItems::G41V3(items) => Some(items),
            _ => None,
        }
    }
}

impl Command for Group41Var4 {
    fn header<I>(items: Vec<(Self, I)>) -> CommandItems<I> {
        CommandItems::G41V4(items)
    }

    fn items<I>(header: &mut CommandItems<I>) -> Option<&mut Vec<(Self, I)>> {
        match header {
            CommandItems::G41V4(items) => Some(items),
            _ => None,
        }
    }
}

impl CommandIndex for u8 {
    fn header(items: CommandItems<Self>) -> CommandHeader {
        CommandHeader::U8(items)
    }

    fn items(header: &mut CommandHeader) -> Option<&mut CommandItems<Self>> {
        match header {
            CommandHeader::U8(items) => Some(items),
            CommandHeader::U16(_) => None,
        }
    }
}

impl CommandIndex for u16 {
    fn header(items: CommandItems<Self>) -> CommandHeader {
        CommandHeader::U16(items)
    }

    fn items(header: &mut CommandHeader) -> Option<&mut CommandItems<Self>> {
        match header {
            CommandHeader::U8(_) => None,
            CommandHeader::U16(items) => Some(items),
        }
    }
}

/// Collection of command headers sent from the master API
pub struct CommandHeaders {
    headers: Vec<CommandHeader>,
}

impl std::fmt::Debug for CommandHeaders {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandHeaders")
            .field("num_headers", &self.headers.len())
            .finish()
    }
}

impl CommandHeaders {
    pub(crate) fn single(header: CommandHeader) -> Self {
        Self {
            headers: vec![header],
        }
    }

    pub(crate) fn write(&self, writer: &mut HeaderWriter) -> Result<(), TaskError> {
        for header in self.headers.iter() {
            header.validate().map_err(TaskError::BadEncoding)?;
        }

        for header in self.headers.iter() {
            header.write(writer)?;
        }

        Ok(())
    }

    pub(crate) fn compare(&self, headers: HeaderCollection) -> Result<(), CommandResponseError> {
        let mut iter = headers.iter();

        for sent in &self.headers {
            match iter.next() {
                None => return Err(CommandResponseError::HeaderCountMismatch),
                Some(received) => sent.compare(received)?,
            }
        }

        if iter.next().is_some() {
            return Err(CommandResponseError::HeaderCountMismatch);
        }

        Ok(())
    }
}

/// Builder object used to create a [CommandHeaders](CommandHeaders)
#[derive(Clone)]
pub struct CommandBuilder {
    headers: Vec<CommandHeader>,
    partial: Option<CommandHeader>,
}

impl std::fmt::Debug for CommandBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandBuilder")
            .field("num_headers", &self.headers.len())
            .finish_non_exhaustive()
    }
}

/// Trait that provides builder support for a particular command type
pub trait CommandSupport<T> {
    /// add a command using one byte addressing
    fn add_u8(&mut self, command: T, index: u8);
    /// add a command using two byte addressing
    fn add_u16(&mut self, command: T, index: u16);
    /// construct a `CommandHeaders` instance consisting of single command with one byte addressing
    fn single_header_u8(command: T, index: u8) -> CommandHeaders;
    /// construct a `CommandHeaders` instance consisting of single command with two byte addressing
    fn single_header_u16(command: T, index: u16) -> CommandHeaders;
}

impl CommandBuilder {
    /// construct a new `CommandBuilder` instance
    pub fn new() -> Self {
        Self {
            headers: Vec::new(),
            partial: None,
        }
    }

    /// Manually complete any partially built header.
    ///
    /// This allows for building multiple headers of the same type,
    /// e.g. two g12v1 values in two separate headers
    pub fn finish_header(&mut self) {
        if let Some(header) = self.partial.take() {
            self.headers.push(header);
        }
    }

    /// Add the command to the partial header if it has the same type and index width, otherwise
    /// finish that header and start another
    fn push<C: Command, I: CommandIndex>(&mut self, command: C, index: I) {
        if let Some(items) = self.partial.as_mut().and_then(I::items).and_then(C::items) {
            items.push((command, index));
        } else {
            self.finish_header();
            self.partial = Some(CommandHeader::new(command, index));
        }
    }

    /// Consume the instance and return a fully built `CommandHeaders`
    pub fn build(mut self) -> CommandHeaders {
        self.finish_header();
        CommandHeaders {
            headers: self.headers,
        }
    }
}

impl CommandSupport<Group12Var1> for CommandBuilder {
    fn add_u8(&mut self, command: Group12Var1, index: u8) {
        self.push(command, index);
    }

    fn add_u16(&mut self, command: Group12Var1, index: u16) {
        self.push(command, index);
    }

    fn single_header_u8(command: Group12Var1, index: u8) -> CommandHeaders {
        CommandHeaders::single(CommandHeader::new(command, index))
    }

    fn single_header_u16(command: Group12Var1, index: u16) -> CommandHeaders {
        CommandHeaders::single(CommandHeader::new(command, index))
    }
}

impl CommandSupport<Group41Var1> for CommandBuilder {
    fn add_u8(&mut self, command: Group41Var1, index: u8) {
        self.push(command, index);
    }

    fn add_u16(&mut self, command: Group41Var1, index: u16) {
        self.push(command, index);
    }

    fn single_header_u8(command: Group41Var1, index: u8) -> CommandHeaders {
        CommandHeaders::single(CommandHeader::new(command, index))
    }

    fn single_header_u16(command: Group41Var1, index: u16) -> CommandHeaders {
        CommandHeaders::single(CommandHeader::new(command, index))
    }
}

impl CommandSupport<Group41Var2> for CommandBuilder {
    fn add_u8(&mut self, command: Group41Var2, index: u8) {
        self.push(command, index);
    }

    fn add_u16(&mut self, command: Group41Var2, index: u16) {
        self.push(command, index);
    }

    fn single_header_u8(command: Group41Var2, index: u8) -> CommandHeaders {
        CommandHeaders::single(CommandHeader::new(command, index))
    }

    fn single_header_u16(command: Group41Var2, index: u16) -> CommandHeaders {
        CommandHeaders::single(CommandHeader::new(command, index))
    }
}

impl CommandSupport<Group41Var3> for CommandBuilder {
    fn add_u8(&mut self, command: Group41Var3, index: u8) {
        self.push(command, index);
    }

    fn add_u16(&mut self, command: Group41Var3, index: u16) {
        self.push(command, index);
    }

    fn single_header_u8(command: Group41Var3, index: u8) -> CommandHeaders {
        CommandHeaders::single(CommandHeader::new(command, index))
    }

    fn single_header_u16(command: Group41Var3, index: u16) -> CommandHeaders {
        CommandHeaders::single(CommandHeader::new(command, index))
    }
}

impl CommandSupport<Group41Var4> for CommandBuilder {
    fn add_u8(&mut self, command: Group41Var4, index: u8) {
        self.push(command, index);
    }

    fn add_u16(&mut self, command: Group41Var4, index: u16) {
        self.push(command, index);
    }

    fn single_header_u8(command: Group41Var4, index: u8) -> CommandHeaders {
        CommandHeaders::single(CommandHeader::new(command, index))
    }

    fn single_header_u16(command: Group41Var4, index: u16) -> CommandHeaders {
        CommandHeaders::single(CommandHeader::new(command, index))
    }
}

impl Default for CommandBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandHeader {
    /// a header of one command
    pub(crate) fn new<C: Command, I: CommandIndex>(command: C, index: I) -> Self {
        I::header(C::header(vec![(command, index)]))
    }

    fn validate(&self) -> Result<(), BadEncoding> {
        match self {
            CommandHeader::U8(items) => items.validate(),
            CommandHeader::U16(items) => items.validate(),
        }
    }

    pub(crate) fn write(&self, writer: &mut HeaderWriter) -> Result<(), scursor::WriteError> {
        match self {
            CommandHeader::U8(items) => items.write(writer),
            CommandHeader::U16(items) => items.write(writer),
        }
    }

    pub(crate) fn compare(&self, response: ObjectHeader) -> Result<(), CommandResponseError> {
        match (self, response) {
            (CommandHeader::U8(items), ObjectHeader::CountAndPrefix8(_, objects)) => {
                items.compare(objects)
            }
            (CommandHeader::U16(items), ObjectHeader::CountAndPrefix16(_, objects)) => {
                items.compare(objects)
            }
            _ => Err(CommandResponseError::HeaderTypeMismatch),
        }
    }
}

impl<I: Index> CommandItems<I> {
    fn validate(&self) -> Result<(), BadEncoding> {
        match self {
            CommandItems::G12V1(items) => {
                match items.iter().find(|(x, _)| !x.code.is_canonical()) {
                    Some((x, _)) => Err(BadEncoding::ControlCode(x.code)),
                    None => Ok(()),
                }
            }
            CommandItems::G41V1(_)
            | CommandItems::G41V2(_)
            | CommandItems::G41V3(_)
            | CommandItems::G41V4(_) => Ok(()),
        }
    }

    fn write(&self, writer: &mut HeaderWriter) -> Result<(), scursor::WriteError> {
        match self {
            CommandItems::G12V1(items) => writer.write_prefixed_items(items.iter()),
            CommandItems::G41V1(items) => writer.write_prefixed_items(items.iter()),
            CommandItems::G41V2(items) => writer.write_prefixed_items(items.iter()),
            CommandItems::G41V3(items) => writer.write_prefixed_items(items.iter()),
            CommandItems::G41V4(items) => writer.write_prefixed_items(items.iter()),
        }
    }

    fn compare(&self, response: PrefixedVariation<I>) -> Result<(), CommandResponseError> {
        match (self, response) {
            (CommandItems::G12V1(items), PrefixedVariation::Group12Var1(seq)) => {
                compare_items(seq, items)
            }
            (CommandItems::G41V1(items), PrefixedVariation::Group41Var1(seq)) => {
                compare_items(seq, items)
            }
            (CommandItems::G41V2(items), PrefixedVariation::Group41Var2(seq)) => {
                compare_items(seq, items)
            }
            (CommandItems::G41V3(items), PrefixedVariation::Group41Var3(seq)) => {
                compare_items(seq, items)
            }
            (CommandItems::G41V4(items), PrefixedVariation::Group41Var4(seq)) => {
                compare_items(seq, items)
            }
            _ => Err(CommandResponseError::HeaderTypeMismatch),
        }
    }
}

fn compare_items<V, I>(
    seq: CountSequence<'_, Prefix<I, V>>,
    sent: &[(V, I)],
) -> Result<(), CommandResponseError>
where
    V: CommandObject,
    I: Index,
{
    let mut received = seq.iter();

    for item in sent {
        match received.next() {
            None => return Err(CommandResponseError::ObjectCountMismatch),
            Some(x) => {
                if x.value.status() != CommandStatus::Success {
                    return Err(CommandResponseError::BadStatus(x.value.status()));
                }
                if x.index != item.1 || !x.value.echoes(&item.0) {
                    return Err(CommandResponseError::ObjectValueMismatch);
                }
            }
        }
    }

    if received.next().is_some() {
        return Err(CommandResponseError::ObjectCountMismatch);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::control::{ControlCode, OpType};
    use crate::app::parse::options::ParseOptions;
    use crate::app::FunctionCode;
    use scursor::WriteCursor;

    fn check(
        sent: &CommandHeader,
        echo: &CommandHeader,
        expected: Result<(), CommandResponseError>,
    ) {
        let mut buffer = [0u8; 64];
        let mut cursor = WriteCursor::new(&mut buffer);
        echo.write(&mut HeaderWriter::new(&mut cursor)).unwrap();
        let headers = HeaderCollection::parse(
            ParseOptions::default(),
            FunctionCode::Response,
            cursor.written(),
        )
        .unwrap();
        assert_eq!(
            CommandHeaders::single(sent.clone()).compare(headers),
            expected
        );
    }

    /// each command with 1-byte and then 2-byte addressing
    fn commands() -> [(CommandHeader, CommandHeader); 5] {
        fn both<C: Command>(command: C) -> (CommandHeader, CommandHeader) {
            (
                CommandHeader::new(command, 3u8),
                CommandHeader::new(command, 259u16),
            )
        }
        [
            both(Group12Var1::from_op_type(OpType::LatchOn)),
            both(Group41Var1::new(-7)),
            both(Group41Var2::new(-7)),
            both(Group41Var3::new(2.5)),
            both(Group41Var4::new(2.5)),
        ]
    }

    #[test]
    fn later_invalid_header_leaves_the_cursor_untouched() {
        let code = ControlCode::from_op_type(OpType::Unknown(0x11));
        let mut builder = CommandBuilder::new();
        builder.add_u8(Group12Var1::from_op_type(OpType::LatchOn), 1);
        builder.finish_header();
        builder.add_u8(Group12Var1::from_code(code), 2);
        let headers = builder.build();

        let mut buffer = [0u8; 64];
        let mut cursor = WriteCursor::new(&mut buffer);
        cursor.write_u8(0xAA).unwrap();

        assert_eq!(
            headers.write(&mut HeaderWriter::new(&mut cursor)),
            Err(TaskError::BadEncoding(BadEncoding::ControlCode(code)))
        );
        assert_eq!(cursor.written(), &[0xAA]);
    }

    #[test]
    fn echo_with_the_same_width_matches() {
        for (one, two) in commands() {
            check(&one, &one, Ok(()));
            check(&two, &two, Ok(()));
        }
    }

    #[test]
    fn echo_with_the_other_width_is_a_header_type_mismatch() {
        for (one, two) in commands() {
            check(&one, &two, Err(CommandResponseError::HeaderTypeMismatch));
            check(&two, &one, Err(CommandResponseError::HeaderTypeMismatch));
        }
    }

    const OTHER_NAN_F32: f32 = f32::from_bits(0x7FC0_0001);
    const OTHER_NAN_F64: f64 = f64::from_bits(0x7FF8_0000_0000_0001);

    /// a sent command and its echo, with 1-byte and then 2-byte addressing
    fn echo_pairs<C: Command>(sent: C, echo: C) -> [(CommandHeader, CommandHeader); 2] {
        [
            (CommandHeader::new(sent, 3u8), CommandHeader::new(echo, 3u8)),
            (
                CommandHeader::new(sent, 259u16),
                CommandHeader::new(echo, 259u16),
            ),
        ]
    }

    /// each pair of g41v3 and g41v4 values, with both index widths
    fn float_pairs(
        values: &[(f32, f32)],
        values64: &[(f64, f64)],
    ) -> Vec<(CommandHeader, CommandHeader)> {
        values
            .iter()
            .flat_map(|&(sent, echo)| echo_pairs(Group41Var3::new(sent), Group41Var3::new(echo)))
            .chain(values64.iter().flat_map(|&(sent, echo)| {
                echo_pairs(Group41Var4::new(sent), Group41Var4::new(echo))
            }))
            .collect()
    }

    #[test]
    fn exact_nan_echo_matches() {
        assert_ne!(f32::NAN.to_bits(), OTHER_NAN_F32.to_bits());
        assert_ne!(f64::NAN.to_bits(), OTHER_NAN_F64.to_bits());
        let pairs = float_pairs(
            &[(f32::NAN, f32::NAN), (OTHER_NAN_F32, OTHER_NAN_F32)],
            &[(f64::NAN, f64::NAN), (OTHER_NAN_F64, OTHER_NAN_F64)],
        );
        assert_eq!(pairs.len(), 8);
        for (sent, echo) in pairs {
            check(&sent, &echo, Ok(()));
        }
    }

    #[test]
    fn changed_float_value_is_an_object_value_mismatch() {
        let pairs = float_pairs(
            &[
                (2.5, 3.5),
                (f32::NAN, 2.5),
                (2.5, f32::NAN),
                (f32::NAN, OTHER_NAN_F32),
            ],
            &[
                (2.5, 3.5),
                (f64::NAN, 2.5),
                (2.5, f64::NAN),
                (f64::NAN, OTHER_NAN_F64),
            ],
        );
        assert_eq!(pairs.len(), 16);
        for (sent, echo) in pairs {
            check(&sent, &echo, Err(CommandResponseError::ObjectValueMismatch));
        }
    }

    #[test]
    fn echo_with_the_other_signed_zero_is_an_object_value_mismatch() {
        let pairs = float_pairs(&[(0.0, -0.0), (-0.0, 0.0)], &[(0.0, -0.0), (-0.0, 0.0)]);
        assert_eq!(pairs.len(), 8);
        for (sent, echo) in pairs {
            check(&sent, &echo, Err(CommandResponseError::ObjectValueMismatch));
        }
    }

    #[test]
    fn attribute_with_a_variation_that_cannot_carry_a_value_leaves_the_cursor_untouched() {
        use crate::app::attr::{var, BadAttribute, OwnedAttrValue};

        let headers = |variation| {
            Headers::new()
                .add_all_objects(Variation::Group60Var1)
                .add_attribute(OwnedAttribute::new(
                    AttrSet::Default,
                    variation,
                    OwnedAttrValue::UnsignedInt(0),
                ))
        };

        for variation in [
            0,
            var::ALL_ATTRIBUTES_REQUEST,
            var::LIST_OF_ATTRIBUTE_VARIATIONS,
        ] {
            let mut buffer = [0u8; 64];
            let mut cursor = WriteCursor::new(&mut buffer);
            cursor.write_u8(0xAA).unwrap();

            assert_eq!(
                headers(variation).write(&mut HeaderWriter::new(&mut cursor)),
                Err(TaskError::BadEncoding(BadEncoding::Attribute(
                    BadAttribute::BadVariation(variation)
                )))
            );
            assert_eq!(cursor.written(), &[0xAA]);
        }

        let mut buffer = [0u8; 64];
        let mut cursor = WriteCursor::new(&mut buffer);
        headers(var::DEVICE_MANUFACTURER_NAME)
            .write(&mut HeaderWriter::new(&mut cursor))
            .unwrap();
        assert_eq!(
            cursor.written(),
            &[60, 1, 0x06, 0, 252, 0x00, 0, 0, 2, 1, 0]
        );
    }

    mod round_trip {
        use super::*;
        use crate::app::gen::count::CountVariation;
        use crate::app::parse::parser::{ReadQualifier, Width};
        use crate::app::{ObjectParseError, QualifierCode};

        use proptest::prelude::*;

        /// The qualifier of a READ header as the master should write it
        #[derive(Clone, Copy, Debug, PartialEq)]
        enum Qualifier {
            AllObjects,
            Range8(u8, u8),
            Range16(u16, u16),
            Count8(u8),
            Count16(u16),
        }

        impl Qualifier {
            fn code(self) -> QualifierCode {
                match self {
                    Qualifier::AllObjects => QualifierCode::AllObjects,
                    Qualifier::Range8(..) => QualifierCode::Range8,
                    Qualifier::Range16(..) => QualifierCode::Range16,
                    Qualifier::Count8(_) => QualifierCode::Count8,
                    Qualifier::Count16(_) => QualifierCode::Count16,
                }
            }

            /// the qualifier byte, then the start and stop or the count
            fn bytes(self) -> Vec<u8> {
                match self {
                    Qualifier::AllObjects => vec![0x06],
                    Qualifier::Range8(start, stop) => vec![0x00, start, stop],
                    Qualifier::Range16(start, stop) => {
                        [&[0x01][..], &start.to_le_bytes(), &stop.to_le_bytes()].concat()
                    }
                    Qualifier::Count8(count) => vec![0x07, count],
                    Qualifier::Count16(count) => [&[0x08][..], &count.to_le_bytes()].concat(),
                }
            }

            fn parsed(qualifier: ReadQualifier) -> Self {
                let narrow = |x: u16| u8::try_from(x).unwrap();
                match qualifier {
                    ReadQualifier::AllObjects => Qualifier::AllObjects,
                    ReadQualifier::Range(Width::One, range) => {
                        Qualifier::Range8(narrow(range.get_start()), narrow(range.get_stop()))
                    }
                    ReadQualifier::Range(Width::Two, range) => {
                        Qualifier::Range16(range.get_start(), range.get_stop())
                    }
                    ReadQualifier::Count(Width::One, count) => Qualifier::Count8(narrow(count)),
                    ReadQualifier::Count(Width::Two, count) => Qualifier::Count16(count),
                }
            }
        }

        /// A READ header: the group and variation on the wire, and the qualifier
        type ModelHeader = ((u8, u8), Qualifier);

        fn model_header(header: ReadHeader) -> ModelHeader {
            let (variation, qualifier) = match header {
                ReadHeader::Range8(x) => (x.variation, Qualifier::Range8(x.start, x.stop)),
                ReadHeader::Range16(x) => (x.variation, Qualifier::Range16(x.start, x.stop)),
                ReadHeader::AllObjects(x) => (x.variation, Qualifier::AllObjects),
                ReadHeader::LimitedCount8(x) => (x.variation, Qualifier::Count8(x.count)),
                ReadHeader::LimitedCount16(x) => (x.variation, Qualifier::Count16(x.count)),
            };
            (variation.to_group_and_var(), qualifier)
        }

        fn model_request(request: &ReadRequest) -> Vec<ModelHeader> {
            match request {
                ReadRequest::SingleHeader(header) => vec![model_header(*header)],
                ReadRequest::MultipleHeader(headers) => {
                    headers.iter().copied().map(model_header).collect()
                }
                // the event classes first, then class 0
                ReadRequest::ClassScan(classes) => [
                    (classes.events.class1, (60, 2)),
                    (classes.events.class2, (60, 3)),
                    (classes.events.class3, (60, 4)),
                    (classes.class0, (60, 1)),
                ]
                .into_iter()
                .filter(|(enabled, _)| *enabled)
                .map(|(_, group_var)| (group_var, Qualifier::AllObjects))
                .collect(),
            }
        }

        fn model_bytes(&((group, var), qualifier): &ModelHeader) -> Vec<u8> {
            [vec![group, var], qualifier.bytes()].concat()
        }

        /// The headers of a READ request as the outstation parses them
        fn parse_read(bytes: &[u8]) -> Result<Vec<ModelHeader>, ObjectParseError> {
            HeaderCollection::parse(ParseOptions::default(), FunctionCode::Read, bytes)
                .map(read_headers)
        }

        /// the headers of a parsed READ request; the iterator stays exhausted once it ends
        fn read_headers(headers: HeaderCollection) -> Vec<ModelHeader> {
            let mut iter = headers.iter();
            let parsed = iter
                .by_ref()
                .map(|header| match header {
                    ObjectHeader::Read(variation, qualifier) => {
                        (variation.to_group_and_var(), Qualifier::parsed(qualifier))
                    }
                    other => panic!("not a READ header: {other:?}"),
                })
                .collect();
            assert!(iter.next().is_none());
            assert!(iter.next().is_none());
            parsed
        }

        /// The kinds of READ qualifier, which accept different variations
        #[derive(Clone, Copy, Debug)]
        enum Kind {
            AllObjects,
            Range,
            Count,
        }

        impl Qualifier {
            fn kind(self) -> Kind {
                match self {
                    Qualifier::AllObjects => Kind::AllObjects,
                    Qualifier::Range8(..) | Qualifier::Range16(..) => Kind::Range,
                    Qualifier::Count8(_) | Qualifier::Count16(_) => Kind::Count,
                }
            }
        }

        // The variations of each group that a READ accepts with each kind of qualifier, written
        // out independently of the parser. Groups 0, 110 and 111 are in `read_accepts`.
        const ALL_OBJECTS: &[(u8, &[u8])] = &[
            (1, &[0, 1, 2]),
            (2, &[0, 1, 2, 3]),
            (3, &[0, 1, 2]),
            (4, &[0, 1, 2, 3]),
            (10, &[0, 1, 2]),
            (11, &[0, 1, 2]),
            (13, &[1, 2]),
            (20, &[0, 1, 2, 5, 6]),
            (21, &[0, 1, 2, 5, 6, 9, 10]),
            (22, &[0, 1, 2, 5, 6]),
            (23, &[0, 1, 2, 5, 6]),
            (30, &[0, 1, 2, 3, 4, 5, 6]),
            (31, &[0, 1, 2, 3, 4, 5, 6, 7, 8]),
            (32, &[0, 1, 2, 3, 4, 5, 6, 7, 8]),
            (33, &[0, 1, 2, 3, 4, 5, 6, 7, 8]),
            (34, &[0, 1, 2, 3]),
            (40, &[0, 1, 2, 3, 4]),
            (42, &[0, 1, 2, 3, 4, 5, 6, 7, 8]),
            (43, &[1, 2, 3, 4, 5, 6, 7, 8]),
            (60, &[1, 2, 3, 4]),
            (80, &[1]),
            (102, &[0, 1]),
        ];

        const RANGE: &[(u8, &[u8])] = &[
            (1, &[0, 1, 2]),
            (3, &[0, 1, 2]),
            (10, &[0, 1, 2]),
            (20, &[0, 1, 2, 5, 6]),
            (21, &[0, 1, 2, 5, 6, 9, 10]),
            (30, &[0, 1, 2, 3, 4, 5, 6]),
            (31, &[0, 1, 2, 3, 4, 5, 6, 7, 8]),
            (34, &[0, 1, 2, 3]),
            (40, &[0, 1, 2, 3, 4]),
            (80, &[1]),
            (102, &[0, 1]),
        ];

        const COUNT: &[(u8, &[u8])] = &[
            (2, &[0, 1, 2, 3]),
            (4, &[0, 1, 2, 3]),
            (11, &[0, 1, 2]),
            (13, &[1, 2]),
            (22, &[0, 1, 2, 5, 6]),
            (23, &[0, 1, 2, 5, 6]),
            (32, &[0, 1, 2, 3, 4, 5, 6, 7, 8]),
            (33, &[0, 1, 2, 3, 4, 5, 6, 7, 8]),
            (42, &[0, 1, 2, 3, 4, 5, 6, 7, 8]),
            (43, &[1, 2, 3, 4, 5, 6, 7, 8]),
            (50, &[1, 2, 3, 4]),
            (51, &[1, 2]),
            (52, &[1, 2]),
            (60, &[2, 3, 4]),
        ];

        /// true if a READ accepts the defined variation with this kind of qualifier
        fn read_accepts(kind: Kind, (group, var): (u8, u8)) -> bool {
            let table = match (kind, group) {
                (Kind::Count, 0) => return false,
                (_, 0) => return true,
                (Kind::AllObjects, 110 | 111) | (Kind::Range, 110) => return var == 0,
                (Kind::Count, 111) => return true,
                (_, 110 | 111) => return false,
                (Kind::AllObjects, _) => ALL_OBJECTS,
                (Kind::Range, _) => RANGE,
                (Kind::Count, _) => COUNT,
            };
            table
                .iter()
                .any(|(g, vars)| *g == group && vars.contains(&var))
        }

        /// what the outstation should parse from the header alone
        fn expected(header: &ModelHeader) -> Result<ModelHeader, ObjectParseError> {
            let ((group, var), qualifier) = *header;
            match Variation::lookup(group, var) {
                None => Err(ObjectParseError::UnknownGroupVariation(group, var)),
                Some(_) if read_accepts(qualifier.kind(), (group, var)) => Ok(*header),
                Some(variation) => Err(ObjectParseError::InvalidQualifierForVariation(
                    variation,
                    qualifier.code(),
                )),
            }
        }

        /// one of the variations, with groups 0, 110 and 111, which have a variation for every
        /// byte, no more likely than the others
        fn one_of(variations: Vec<Variation>) -> impl Strategy<Value = Variation> {
            let (strings, others): (Vec<Variation>, Vec<Variation>) = variations
                .into_iter()
                .partition(|v| matches!(v.to_group_and_var().0, 0 | 110 | 111));
            prop_oneof![
                3 => prop::sample::select(others),
                1 => prop::sample::select(strings),
            ]
        }

        /// a variation a READ accepts with this kind of qualifier, or less often, one it doesn't
        fn variation(kind: Kind) -> impl Strategy<Value = Variation> {
            let (accepted, rejected): (Vec<Variation>, Vec<Variation>) =
                Variation::all().partition(|v| read_accepts(kind, v.to_group_and_var()));
            prop_oneof![
                3 => one_of(accepted),
                1 => one_of(rejected),
            ]
        }

        /// a start and stop, with single indices, the whole range and its ends more likely
        fn range<T>(max: T) -> impl Strategy<Value = (T, T)>
        where
            T: Arbitrary + Copy + Ord + Default + std::fmt::Debug + 'static,
        {
            let zero = T::default();
            prop_oneof![
                2 => (any::<T>(), any::<T>()).prop_map(|(a, b)| (a.min(b), a.max(b))),
                1 => any::<T>().prop_map(|a| (a, a)),
                1 => prop::sample::select(vec![(zero, zero), (zero, max), (max, max)]),
            ]
        }

        /// a count, with these boundary values more likely
        fn count<T>(boundaries: Vec<T>) -> impl Strategy<Value = T>
        where
            T: Arbitrary + Clone + std::fmt::Debug + 'static,
        {
            prop_oneof![
                2 => any::<T>(),
                1 => prop::sample::select(boundaries),
            ]
        }

        fn read_header() -> impl Strategy<Value = ReadHeader> {
            prop_oneof![
                (variation(Kind::Range), range(u8::MAX))
                    .prop_map(|(v, (start, stop))| { ReadHeader::one_byte_range(v, start, stop) }),
                (variation(Kind::Range), range(u16::MAX))
                    .prop_map(|(v, (start, stop))| { ReadHeader::two_byte_range(v, start, stop) }),
                variation(Kind::AllObjects).prop_map(ReadHeader::all_objects),
                (variation(Kind::Count), count(vec![0, 1, u8::MAX]))
                    .prop_map(|(v, count)| ReadHeader::one_byte_limited_count(v, count)),
                (
                    variation(Kind::Count),
                    count(vec![0, 1, 255, 256, u16::MAX])
                )
                    .prop_map(|(v, count)| ReadHeader::two_byte_limited_count(v, count)),
            ]
        }

        fn read_request() -> impl Strategy<Value = ReadRequest> {
            prop_oneof![
                read_header().prop_map(ReadRequest::SingleHeader),
                prop::collection::vec(read_header(), 0..6)
                    .prop_map(|headers| ReadRequest::multiple_headers(&headers)),
                any::<[bool; 4]>().prop_map(|[class0, class1, class2, class3]| {
                    ReadRequest::class_scan(Classes::new(
                        class0,
                        EventClasses::new(class1, class2, class3),
                    ))
                }),
                (count(vec![0, 254, 255]), any::<u8>()).prop_map(|(variation, set)| {
                    ReadRequest::device_attribute(variation, AttrSet::new(set))
                }),
            ]
        }

        #[derive(Clone, Debug)]
        enum Item {
            Read(ReadHeader),
            /// the time and interval of a g50v2
            Freeze(u64, u32),
        }

        fn item() -> impl Strategy<Value = Item> {
            prop_oneof![
                read_header().prop_map(Item::Read),
                (
                    prop_oneof![Just(0), 0..=Timestamp::MAX_VALUE],
                    prop_oneof![Just(0), any::<u32>()],
                )
                    .prop_map(|(time, interval)| Item::Freeze(time, interval)),
            ]
        }

        fn add(headers: Headers, item: &Item) -> Headers {
            match *item {
                Item::Read(ReadHeader::Range8(x)) => {
                    headers.add_range_8(x.variation, x.start, x.stop)
                }
                Item::Read(ReadHeader::Range16(x)) => {
                    headers.add_range_16(x.variation, x.start, x.stop)
                }
                Item::Read(ReadHeader::AllObjects(x)) => headers.add_all_objects(x.variation),
                Item::Read(ReadHeader::LimitedCount8(x)) => {
                    headers.add_one_byte_limited_count(x.variation, x.count)
                }
                Item::Read(ReadHeader::LimitedCount16(x)) => {
                    headers.add_two_byte_limited_count(x.variation, x.count)
                }
                Item::Freeze(time, interval) => {
                    headers.add_time_and_interval(Timestamp::new(time), interval)
                }
            }
        }

        /// a g50v2 with a count of one: the 48-bit time, then the 32-bit interval
        fn freeze_bytes(time: u64, interval: u32) -> Vec<u8> {
            [
                &[50, 2, 0x07, 1][..],
                &time.to_le_bytes()[..6],
                &interval.to_le_bytes(),
            ]
            .concat()
        }

        /// found by `read_requests_parse_to_the_headers_written`: the master writes the variation
        /// it's given, and the outstation rejects one that isn't defined
        #[test]
        fn device_attribute_read_of_an_undefined_variation_is_written_as_given() {
            let mut buffer = [0; 16];
            let mut cursor = WriteCursor::new(&mut buffer);
            ReadRequest::device_attribute(0u8, AttrSet::Private(7))
                .format(&mut HeaderWriter::new(&mut cursor))
                .unwrap();
            assert_eq!(cursor.written(), [0, 0, 0x00, 7, 7]);
            assert_eq!(
                parse_read(cursor.written()),
                Err(ObjectParseError::UnknownGroupVariation(0, 0))
            );
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(512))]

            /// A READ request is written as its headers, and the outstation parses each header
            /// back to what was written, alone or together, unless a READ doesn't accept the
            /// variation with that qualifier
            #[test]
            fn read_requests_parse_to_the_headers_written(request in read_request()) {
                let mut buffer = [0; 256];
                let mut cursor = WriteCursor::new(&mut buffer);
                request.format(&mut HeaderWriter::new(&mut cursor)).unwrap();
                let model = model_request(&request);
                let written = cursor.written();
                prop_assert_eq!(written, model.iter().flat_map(model_bytes).collect::<Vec<u8>>());

                for header in &model {
                    prop_assert_eq!(
                        parse_read(&model_bytes(header)),
                        expected(header).map(|header| vec![header])
                    );
                }
                let expected: Result<Vec<ModelHeader>, ObjectParseError> =
                    model.iter().map(expected).collect();
                prop_assert_eq!(parse_read(written), expected);
            }

            /// Generic request headers are written as the READ headers and g50v2 freeze times
            /// they hold, in order, and each g50v2 parses back to its time and interval
            #[test]
            fn headers_are_written_in_order(items in prop::collection::vec(item(), 0..6)) {
                let headers = items.iter().fold(Headers::new(), add);
                let mut buffer = [0; 256];
                let mut cursor = WriteCursor::new(&mut buffer);
                headers.write(&mut HeaderWriter::new(&mut cursor)).unwrap();
                let expected: Vec<u8> = items
                    .iter()
                    .flat_map(|item| match *item {
                        Item::Read(header) => model_bytes(&model_header(header)),
                        Item::Freeze(time, interval) => freeze_bytes(time, interval),
                    })
                    .collect();
                prop_assert_eq!(cursor.written(), expected);

                for item in &items {
                    let Item::Freeze(time, interval) = *item else {
                        continue;
                    };
                    let bytes = freeze_bytes(time, interval);
                    let parsed = HeaderCollection::parse(
                        ParseOptions::default(),
                        FunctionCode::FreezeAtTime,
                        &bytes,
                    )
                    .unwrap()
                    .get_only_header()
                    .unwrap();
                    let ObjectHeader::Count(Width::One, 1, CountVariation::Group50Var2(objects)) =
                        parsed
                    else {
                        return Err(TestCaseError::fail(format!("{parsed:?}")));
                    };
                    prop_assert_eq!(
                        objects.single(),
                        Some(Group50Var2 {
                            time: Timestamp::new(time),
                            interval
                        })
                    );
                }
            }
        }
    }
}
