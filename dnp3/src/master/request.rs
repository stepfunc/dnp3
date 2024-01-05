use crate::app::attr::{AttrSet, OwnedAttribute};
use std::ops::BitAnd;

use crate::app::control::CommandStatus;
use crate::app::format::write::HeaderWriter;
use crate::app::gen::prefixed::PrefixedVariation;
use crate::app::parse::count::CountSequence;
use crate::app::parse::parser::{HeaderCollection, HeaderDetails};
use crate::app::parse::prefix::Prefix;
use crate::app::parse::traits::{FixedSizeVariation, Index};
use crate::app::variations::*;
use crate::app::Timestamp;
use crate::app::Variation::Group0;
use crate::master::error::CommandResponseError;
use crate::master::TaskError;
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
pub enum TimeSyncProcedure {
    /// Master will use the LAN procedure: RECORD_CURRENT_TIME followed by WRITE g50v3
    Lan,
    /// Master will use the non-LAN procedure: DELAY_MEASUREMENT followed by WRITE g50v1
    NonLan,
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
        Self {
            inner: DeadBandHeaderVariants::G34V1U8(
                dead_bands
                    .iter()
                    .map(|(i, v)| (Group34Var1 { value: *v }, *i))
                    .collect(),
            ),
        }
    }

    /// Group 34 variation 1 with 16-bit index
    pub fn group34_var1_u16(dead_bands: Vec<(u16, u16)>) -> Self {
        Self {
            inner: DeadBandHeaderVariants::G34V1U16(
                dead_bands
                    .iter()
                    .map(|(i, v)| (Group34Var1 { value: *v }, *i))
                    .collect(),
            ),
        }
    }

    /// Group 34 variation 2 with 8-bit index
    pub fn group34_var2_u8(dead_bands: Vec<(u8, u32)>) -> Self {
        Self {
            inner: DeadBandHeaderVariants::G34V2U8(
                dead_bands
                    .iter()
                    .map(|(i, v)| (Group34Var2 { value: *v }, *i))
                    .collect(),
            ),
        }
    }

    /// Group 34 variation 2 with 16-bit index
    pub fn group34_var2_u16(dead_bands: Vec<(u16, u32)>) -> Self {
        Self {
            inner: DeadBandHeaderVariants::G34V2U16(
                dead_bands
                    .iter()
                    .map(|(i, v)| (Group34Var2 { value: *v }, *i))
                    .collect(),
            ),
        }
    }

    /// Group 34 variation 3 with 8-bit index
    pub fn group34_var3_u8(dead_bands: Vec<(u8, f32)>) -> Self {
        Self {
            inner: DeadBandHeaderVariants::G34V3U8(
                dead_bands
                    .iter()
                    .map(|(i, v)| (Group34Var3 { value: *v }, *i))
                    .collect(),
            ),
        }
    }

    /// Group 34 variation 3 with 16-bit index
    pub fn group34_var3_u16(dead_bands: Vec<(u16, f32)>) -> Self {
        Self {
            inner: DeadBandHeaderVariants::G34V3U16(
                dead_bands
                    .iter()
                    .map(|(i, v)| (Group34Var3 { value: *v }, *i))
                    .collect(),
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum DeadBandHeaderVariants {
    /// Group 34 variation 1 with 8-bit index
    G34V1U8(Vec<(Group34Var1, u8)>),
    /// Group 34 variation 1 with 16-bit index
    G34V1U16(Vec<(Group34Var1, u16)>),
    /// Group 34 variation 2 with 8-bit index
    G34V2U8(Vec<(Group34Var2, u8)>),
    /// Group 34 variation 2 with 16-bit index
    G34V2U16(Vec<(Group34Var2, u16)>),
    /// Group 34 variation 3 with 8-bit index
    G34V3U8(Vec<(Group34Var3, u8)>),
    /// Group 34 variation 3 with 16-bit index
    G34V3U16(Vec<(Group34Var3, u16)>),
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

#[derive(Clone)]
pub(crate) enum CommandHeader {
    G12V1U8(Vec<(Group12Var1, u8)>),
    G41V1U8(Vec<(Group41Var1, u8)>),
    G41V2U8(Vec<(Group41Var2, u8)>),
    G41V3U8(Vec<(Group41Var3, u8)>),
    G41V4U8(Vec<(Group41Var4, u8)>),
    G12V1U16(Vec<(Group12Var1, u16)>),
    G41V1U16(Vec<(Group41Var1, u16)>),
    G41V2U16(Vec<(Group41Var2, u16)>),
    G41V3U16(Vec<(Group41Var3, u16)>),
    G41V4U16(Vec<(Group41Var4, u16)>),
}

pub(crate) trait Command {
    fn status(&self) -> CommandStatus;
    fn to_header_u8(&self, index: u8) -> CommandHeader;
    fn to_header_u16(&self, index: u16) -> CommandHeader;
}

impl Command for Group12Var1 {
    fn status(&self) -> CommandStatus {
        self.status
    }

    fn to_header_u8(&self, index: u8) -> CommandHeader {
        CommandHeader::G12V1U8(vec![(*self, index)])
    }

    fn to_header_u16(&self, index: u16) -> CommandHeader {
        CommandHeader::G12V1U16(vec![(*self, index)])
    }
}

impl Command for Group41Var1 {
    fn status(&self) -> CommandStatus {
        self.status
    }

    fn to_header_u8(&self, index: u8) -> CommandHeader {
        CommandHeader::G41V1U8(vec![(*self, index)])
    }

    fn to_header_u16(&self, index: u16) -> CommandHeader {
        CommandHeader::G41V1U16(vec![(*self, index)])
    }
}

impl Command for Group41Var2 {
    fn status(&self) -> CommandStatus {
        self.status
    }
    fn to_header_u8(&self, index: u8) -> CommandHeader {
        CommandHeader::G41V2U8(vec![(*self, index)])
    }
    fn to_header_u16(&self, index: u16) -> CommandHeader {
        CommandHeader::G41V2U16(vec![(*self, index)])
    }
}

impl Command for Group41Var3 {
    fn status(&self) -> CommandStatus {
        self.status
    }
    fn to_header_u8(&self, index: u8) -> CommandHeader {
        CommandHeader::G41V3U8(vec![(*self, index)])
    }
    fn to_header_u16(&self, index: u16) -> CommandHeader {
        CommandHeader::G41V3U16(vec![(*self, index)])
    }
}

impl Command for Group41Var4 {
    fn status(&self) -> CommandStatus {
        self.status
    }
    fn to_header_u8(&self, index: u8) -> CommandHeader {
        CommandHeader::G41V4U8(vec![(*self, index)])
    }

    fn to_header_u16(&self, index: u16) -> CommandHeader {
        CommandHeader::G41V4U16(vec![(*self, index)])
    }
}

/// Collection of command headers sent from the master API
pub struct CommandHeaders {
    headers: Vec<CommandHeader>,
}

impl CommandHeaders {
    pub(crate) fn single(header: CommandHeader) -> Self {
        Self {
            headers: vec![header],
        }
    }

    pub(crate) fn write(&self, writer: &mut HeaderWriter) -> Result<(), scursor::WriteError> {
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
                Some(received) => sent.compare(received.details)?,
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

    fn add_g12v1_u8(&mut self, command: Group12Var1, index: u8) {
        if let Some(partial) = self.partial.take() {
            if let CommandHeader::G12V1U8(mut vec) = partial {
                vec.push((command, index));
                self.partial = Some(CommandHeader::G12V1U8(vec));
            } else {
                self.headers.push(partial);
                self.partial = Some(command.to_header_u8(index));
            }
        } else {
            self.partial = Some(command.to_header_u8(index));
        }
    }

    fn add_g12v1_u16(&mut self, command: Group12Var1, index: u16) {
        if let Some(partial) = self.partial.take() {
            if let CommandHeader::G12V1U16(mut vec) = partial {
                vec.push((command, index));
                self.partial = Some(CommandHeader::G12V1U16(vec));
            } else {
                self.headers.push(partial);
                self.partial = Some(command.to_header_u16(index));
            }
        } else {
            self.partial = Some(command.to_header_u16(index));
        }
    }

    fn add_g41v1_u8(&mut self, command: Group41Var1, index: u8) {
        if let Some(partial) = self.partial.take() {
            if let CommandHeader::G41V1U8(mut vec) = partial {
                vec.push((command, index));
                self.partial = Some(CommandHeader::G41V1U8(vec));
            } else {
                self.headers.push(partial);
                self.partial = Some(command.to_header_u8(index));
            }
        } else {
            self.partial = Some(command.to_header_u8(index));
        }
    }

    fn add_g41v1_u16(&mut self, command: Group41Var1, index: u16) {
        if let Some(partial) = self.partial.take() {
            if let CommandHeader::G41V1U16(mut vec) = partial {
                vec.push((command, index));
                self.partial = Some(CommandHeader::G41V1U16(vec));
            } else {
                self.headers.push(partial);
                self.partial = Some(command.to_header_u16(index));
            }
        } else {
            self.partial = Some(command.to_header_u16(index));
        }
    }

    fn add_g41v2_u8(&mut self, command: Group41Var2, index: u8) {
        if let Some(partial) = self.partial.take() {
            if let CommandHeader::G41V2U8(mut vec) = partial {
                vec.push((command, index));
                self.partial = Some(CommandHeader::G41V2U8(vec));
            } else {
                self.headers.push(partial);
                self.partial = Some(command.to_header_u8(index));
            }
        } else {
            self.partial = Some(command.to_header_u8(index));
        }
    }

    fn add_g41v2_u16(&mut self, command: Group41Var2, index: u16) {
        if let Some(partial) = self.partial.take() {
            if let CommandHeader::G41V2U16(mut vec) = partial {
                vec.push((command, index));
                self.partial = Some(CommandHeader::G41V2U16(vec));
            } else {
                self.headers.push(partial);
                self.partial = Some(command.to_header_u16(index));
            }
        } else {
            self.partial = Some(command.to_header_u16(index));
        }
    }

    fn add_g41v3_u8(&mut self, command: Group41Var3, index: u8) {
        if let Some(partial) = self.partial.take() {
            if let CommandHeader::G41V3U8(mut vec) = partial {
                vec.push((command, index));
                self.partial = Some(CommandHeader::G41V3U8(vec));
            } else {
                self.headers.push(partial);
                self.partial = Some(command.to_header_u8(index));
            }
        } else {
            self.partial = Some(command.to_header_u8(index));
        }
    }

    fn add_g41v3_u16(&mut self, command: Group41Var3, index: u16) {
        if let Some(partial) = self.partial.take() {
            if let CommandHeader::G41V3U16(mut vec) = partial {
                vec.push((command, index));
                self.partial = Some(CommandHeader::G41V3U16(vec));
            } else {
                self.headers.push(partial);
                self.partial = Some(command.to_header_u16(index));
            }
        } else {
            self.partial = Some(command.to_header_u16(index));
        }
    }

    fn add_g41v4_u8(&mut self, command: Group41Var4, index: u8) {
        if let Some(partial) = self.partial.take() {
            if let CommandHeader::G41V4U8(mut vec) = partial {
                vec.push((command, index));
                self.partial = Some(CommandHeader::G41V4U8(vec));
            } else {
                self.headers.push(partial);
                self.partial = Some(command.to_header_u8(index));
            }
        } else {
            self.partial = Some(command.to_header_u8(index));
        }
    }

    fn add_g41v4_u16(&mut self, command: Group41Var4, index: u16) {
        if let Some(partial) = self.partial.take() {
            if let CommandHeader::G41V4U16(mut vec) = partial {
                vec.push((command, index));
                self.partial = Some(CommandHeader::G41V4U16(vec));
            } else {
                self.headers.push(partial);
                self.partial = Some(command.to_header_u16(index));
            }
        } else {
            self.partial = Some(command.to_header_u16(index));
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
        self.add_g12v1_u8(command, index);
    }

    fn add_u16(&mut self, command: Group12Var1, index: u16) {
        self.add_g12v1_u16(command, index);
    }

    fn single_header_u8(command: Group12Var1, index: u8) -> CommandHeaders {
        CommandHeaders::single(command.to_header_u8(index))
    }

    fn single_header_u16(command: Group12Var1, index: u16) -> CommandHeaders {
        CommandHeaders::single(command.to_header_u16(index))
    }
}

impl CommandSupport<Group41Var1> for CommandBuilder {
    fn add_u8(&mut self, command: Group41Var1, index: u8) {
        self.add_g41v1_u8(command, index);
    }

    fn add_u16(&mut self, command: Group41Var1, index: u16) {
        self.add_g41v1_u16(command, index);
    }

    fn single_header_u8(command: Group41Var1, index: u8) -> CommandHeaders {
        CommandHeaders::single(command.to_header_u8(index))
    }

    fn single_header_u16(command: Group41Var1, index: u16) -> CommandHeaders {
        CommandHeaders::single(command.to_header_u16(index))
    }
}

impl CommandSupport<Group41Var2> for CommandBuilder {
    fn add_u8(&mut self, command: Group41Var2, index: u8) {
        self.add_g41v2_u8(command, index);
    }

    fn add_u16(&mut self, command: Group41Var2, index: u16) {
        self.add_g41v2_u16(command, index);
    }

    fn single_header_u8(command: Group41Var2, index: u8) -> CommandHeaders {
        CommandHeaders::single(command.to_header_u8(index))
    }

    fn single_header_u16(command: Group41Var2, index: u16) -> CommandHeaders {
        CommandHeaders::single(command.to_header_u16(index))
    }
}

impl CommandSupport<Group41Var3> for CommandBuilder {
    fn add_u8(&mut self, command: Group41Var3, index: u8) {
        self.add_g41v3_u8(command, index);
    }

    fn add_u16(&mut self, command: Group41Var3, index: u16) {
        self.add_g41v3_u16(command, index);
    }

    fn single_header_u8(command: Group41Var3, index: u8) -> CommandHeaders {
        CommandHeaders::single(command.to_header_u8(index))
    }

    fn single_header_u16(command: Group41Var3, index: u16) -> CommandHeaders {
        CommandHeaders::single(command.to_header_u16(index))
    }
}

impl CommandSupport<Group41Var4> for CommandBuilder {
    fn add_u8(&mut self, command: Group41Var4, index: u8) {
        self.add_g41v4_u8(command, index);
    }

    fn add_u16(&mut self, command: Group41Var4, index: u16) {
        self.add_g41v4_u16(command, index);
    }

    fn single_header_u8(command: Group41Var4, index: u8) -> CommandHeaders {
        CommandHeaders::single(command.to_header_u8(index))
    }

    fn single_header_u16(command: Group41Var4, index: u16) -> CommandHeaders {
        CommandHeaders::single(command.to_header_u16(index))
    }
}

impl Default for CommandBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandHeader {
    pub(crate) fn write(&self, writer: &mut HeaderWriter) -> Result<(), scursor::WriteError> {
        match self {
            CommandHeader::G12V1U8(items) => writer.write_prefixed_items(items.iter()),
            CommandHeader::G41V1U8(items) => writer.write_prefixed_items(items.iter()),
            CommandHeader::G41V2U8(items) => writer.write_prefixed_items(items.iter()),
            CommandHeader::G41V3U8(items) => writer.write_prefixed_items(items.iter()),
            CommandHeader::G41V4U8(items) => writer.write_prefixed_items(items.iter()),
            CommandHeader::G12V1U16(items) => writer.write_prefixed_items(items.iter()),
            CommandHeader::G41V1U16(items) => writer.write_prefixed_items(items.iter()),
            CommandHeader::G41V2U16(items) => writer.write_prefixed_items(items.iter()),
            CommandHeader::G41V3U16(items) => writer.write_prefixed_items(items.iter()),
            CommandHeader::G41V4U16(items) => writer.write_prefixed_items(items.iter()),
        }
    }

    fn compare_items<V, I>(
        seq: CountSequence<'_, Prefix<I, V>>,
        sent: &[(V, I)],
    ) -> Result<(), CommandResponseError>
    where
        V: FixedSizeVariation + Command,
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
                    if !x.equals(item) {
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

    pub(crate) fn compare(&self, response: HeaderDetails) -> Result<(), CommandResponseError> {
        match self {
            CommandHeader::G12V1U8(items) => match response {
                HeaderDetails::OneByteCountAndPrefix(_, PrefixedVariation::Group12Var1(seq)) => {
                    Self::compare_items(seq, items)
                }
                _ => Err(CommandResponseError::HeaderTypeMismatch),
            },
            CommandHeader::G12V1U16(items) => match response {
                HeaderDetails::TwoByteCountAndPrefix(_, PrefixedVariation::Group12Var1(seq)) => {
                    Self::compare_items(seq, items)
                }
                _ => Err(CommandResponseError::HeaderTypeMismatch),
            },
            CommandHeader::G41V1U8(items) => match response {
                HeaderDetails::OneByteCountAndPrefix(_, PrefixedVariation::Group41Var1(seq)) => {
                    Self::compare_items(seq, items)
                }
                _ => Err(CommandResponseError::HeaderTypeMismatch),
            },
            CommandHeader::G41V1U16(items) => match response {
                HeaderDetails::TwoByteCountAndPrefix(_, PrefixedVariation::Group41Var1(seq)) => {
                    Self::compare_items(seq, items)
                }
                _ => Err(CommandResponseError::HeaderTypeMismatch),
            },
            CommandHeader::G41V2U8(items) => match response {
                HeaderDetails::OneByteCountAndPrefix(_, PrefixedVariation::Group41Var2(seq)) => {
                    Self::compare_items(seq, items)
                }
                _ => Err(CommandResponseError::HeaderTypeMismatch),
            },
            CommandHeader::G41V2U16(items) => match response {
                HeaderDetails::TwoByteCountAndPrefix(_, PrefixedVariation::Group41Var2(seq)) => {
                    Self::compare_items(seq, items)
                }
                _ => Err(CommandResponseError::HeaderTypeMismatch),
            },
            CommandHeader::G41V3U8(items) => match response {
                HeaderDetails::OneByteCountAndPrefix(_, PrefixedVariation::Group41Var3(seq)) => {
                    Self::compare_items(seq, items)
                }
                _ => Err(CommandResponseError::HeaderTypeMismatch),
            },
            CommandHeader::G41V3U16(items) => match response {
                HeaderDetails::TwoByteCountAndPrefix(_, PrefixedVariation::Group41Var3(seq)) => {
                    Self::compare_items(seq, items)
                }
                _ => Err(CommandResponseError::HeaderTypeMismatch),
            },
            CommandHeader::G41V4U8(items) => match response {
                HeaderDetails::OneByteCountAndPrefix(_, PrefixedVariation::Group41Var4(seq)) => {
                    Self::compare_items(seq, items)
                }
                _ => Err(CommandResponseError::HeaderTypeMismatch),
            },
            CommandHeader::G41V4U16(items) => match response {
                HeaderDetails::TwoByteCountAndPrefix(_, PrefixedVariation::Group41Var4(seq)) => {
                    Self::compare_items(seq, items)
                }
                _ => Err(CommandResponseError::HeaderTypeMismatch),
            },
        }
    }
}
