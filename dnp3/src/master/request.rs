use std::ops::BitAnd;

use crate::app::control::CommandStatus;
use crate::app::format::write::HeaderWriter;
use crate::app::gen::prefixed::PrefixedVariation;
use crate::app::parse::count::CountSequence;
use crate::app::parse::parser::{HeaderCollection, HeaderDetails};
use crate::app::parse::prefix::Prefix;
use crate::app::parse::traits::{FixedSizeVariation, Index};
use crate::app::variations::*;
use crate::master::error::CommandResponseError;
use crate::util::cursor::WriteError;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum CommandMode {
    DirectOperate,
    SelectBeforeOperate,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TimeSyncProcedure {
    Lan,
    NonLan,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct EventClasses {
    pub class1: bool,
    pub class2: bool,
    pub class3: bool,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Classes {
    pub class0: bool,
    pub events: EventClasses,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct OneByteRangeScan {
    pub variation: Variation,
    pub start: u8,
    pub stop: u8,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct TwoByteRangeScan {
    pub variation: Variation,
    pub start: u16,
    pub stop: u16,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AllObjectsScan {
    pub variation: Variation,
}

impl EventClasses {
    pub fn new(class1: bool, class2: bool, class3: bool) -> Self {
        Self {
            class1,
            class2,
            class3,
        }
    }

    pub fn to_classes(self) -> Classes {
        Classes::new(false, self)
    }

    pub fn any(self) -> bool {
        self.class1 || self.class2 || self.class3
    }

    pub fn all() -> Self {
        Self {
            class1: true,
            class2: true,
            class3: true,
        }
    }

    pub fn none() -> Self {
        Self {
            class1: false,
            class2: false,
            class3: false,
        }
    }

    pub(crate) fn write(self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
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
    pub fn new(class0: bool, events: EventClasses) -> Self {
        Self { events, class0 }
    }

    pub fn to_request(self) -> ReadRequest {
        ReadRequest::class_scan(self)
    }

    pub fn events(events: EventClasses) -> Self {
        Self::new(false, events)
    }

    pub fn integrity() -> Self {
        Self::new(true, EventClasses::all())
    }

    pub fn none() -> Self {
        Self::new(false, EventClasses::none())
    }

    pub fn any(&self) -> bool {
        self.class0 || self.events.any()
    }

    pub(crate) fn write(self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        self.events.write(writer)?;
        if self.class0 {
            writer.write_all_objects_header(Variation::Group60Var1)?;
        }
        Ok(())
    }
}

impl OneByteRangeScan {
    pub fn new(variation: Variation, start: u8, stop: u8) -> Self {
        Self {
            variation,
            start,
            stop,
        }
    }

    pub(crate) fn write(self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        writer.write_range_only(self.variation, self.start, self.stop)
    }
}

impl TwoByteRangeScan {
    pub fn new(variation: Variation, start: u16, stop: u16) -> Self {
        Self {
            variation,
            start,
            stop,
        }
    }

    pub(crate) fn write(self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        writer.write_range_only(self.variation, self.start, self.stop)
    }
}

impl AllObjectsScan {
    pub fn new(variation: Variation) -> Self {
        Self { variation }
    }

    pub(crate) fn write(self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        writer.write_all_objects_header(self.variation)
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ReadHeader {
    Range8(OneByteRangeScan),
    Range16(TwoByteRangeScan),
    AllObjects(AllObjectsScan),
}

impl ReadHeader {
    pub fn one_byte_range(variation: Variation, start: u8, stop: u8) -> Self {
        ReadHeader::Range8(OneByteRangeScan::new(variation, start, stop))
    }

    pub fn two_byte_range(variation: Variation, start: u16, stop: u16) -> Self {
        ReadHeader::Range16(TwoByteRangeScan::new(variation, start, stop))
    }

    pub fn all_objects(variation: Variation) -> Self {
        ReadHeader::AllObjects(AllObjectsScan::new(variation))
    }

    pub(crate) fn format(self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        match self {
            ReadHeader::Range8(scan) => scan.write(writer),
            ReadHeader::Range16(scan) => scan.write(writer),
            ReadHeader::AllObjects(scan) => scan.write(writer),
        }
    }
}

#[derive(Clone, Debug)]
pub enum ReadRequest {
    SingleHeader(ReadHeader),
    ClassScan(Classes),
    MultipleHeader(Vec<ReadHeader>),
}

impl ReadRequest {
    pub fn class_scan(scan: Classes) -> Self {
        Self::ClassScan(scan)
    }

    pub fn one_byte_range(variation: Variation, start: u8, stop: u8) -> Self {
        Self::SingleHeader(ReadHeader::one_byte_range(variation, start, stop))
    }

    pub fn two_byte_range(variation: Variation, start: u16, stop: u16) -> Self {
        Self::SingleHeader(ReadHeader::two_byte_range(variation, start, stop))
    }

    pub fn all_objects(variation: Variation) -> Self {
        Self::SingleHeader(ReadHeader::all_objects(variation))
    }

    pub fn multiple_headers(headers: &[ReadHeader]) -> Self {
        Self::MultipleHeader(headers.to_vec())
    }

    pub(crate) fn format(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
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

pub enum CommandHeader {
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

pub trait Command {
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

pub struct CommandHeaders {
    headers: Vec<CommandHeader>,
}

impl CommandHeaders {
    pub(crate) fn write(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
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

pub struct CommandBuilder {
    headers: Vec<CommandHeader>,
}

impl CommandBuilder {
    pub fn new() -> Self {
        Self {
            headers: Vec::new(),
        }
    }

    pub fn add_u8_header<C>(&mut self, command: C, index: u8)
    where
        C: Command,
    {
        self.headers.push(command.to_header_u8(index));
    }

    pub fn add_u16_header<C>(&mut self, command: C, index: u16)
    where
        C: Command,
    {
        self.headers.push(command.to_header_u16(index));
    }

    pub fn single_u8_header<C>(command: C, index: u8) -> CommandHeaders
    where
        C: Command,
    {
        let mut builder = Self::new();
        builder.add_u8_header(command, index);
        builder.build()
    }

    pub fn single_u16_header<C>(command: C, index: u16) -> CommandHeaders
    where
        C: Command,
    {
        let mut builder = Self::new();
        builder.add_u16_header(command, index);
        builder.build()
    }

    pub fn build(self) -> CommandHeaders {
        CommandHeaders {
            headers: self.headers,
        }
    }
}

impl Default for CommandBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandHeader {
    pub(crate) fn write(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
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
