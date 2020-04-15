use crate::app::format::write::HeaderWriter;
use crate::app::gen::enums::{CommandStatus, FunctionCode};
use crate::app::gen::variations::fixed::*;
use crate::app::gen::variations::prefixed::PrefixedVariation;
use crate::app::gen::variations::variation::Variation;
use crate::app::parse::count::CountSequence;
use crate::app::parse::parser::HeaderDetails;
use crate::app::parse::prefix::Prefix;
use crate::app::parse::traits::{FixedSizeVariation, Index};
use crate::master::handlers::RequestCompletionHandler;
use crate::master::runner::RequestError;
use crate::util::cursor::WriteError;

#[derive(Copy, Clone)]
pub struct EventClasses {
    pub class1: bool,
    pub class2: bool,
    pub class3: bool,
}

#[derive(Copy, Clone)]
pub struct Classes {
    pub class0: bool,
    pub events: EventClasses,
}

#[derive(Copy, Clone)]
pub struct RangeScan<T>
where
    T: Index,
{
    pub variation: Variation,
    pub start: T,
    pub stop: T,
}

impl EventClasses {
    pub fn new(class1: bool, class2: bool, class3: bool) -> Self {
        Self {
            class1,
            class2,
            class3,
        }
    }

    pub fn all() -> Self {
        Self {
            class1: true,
            class2: true,
            class3: true,
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

impl Classes {
    pub fn new(class0: bool, events: EventClasses) -> Self {
        Self { events, class0 }
    }

    pub fn events(events: EventClasses) -> Self {
        Self::new(false, events)
    }

    pub fn integrity() -> Self {
        Self::new(true, EventClasses::all())
    }

    pub(crate) fn write(self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        self.events.write(writer)?;
        if self.class0 {
            writer.write_all_objects_header(Variation::Group60Var1)?;
        }
        Ok(())
    }
}

impl<T> RangeScan<T>
where
    T: Index,
{
    pub fn new(variation: Variation, start: T, stop: T) -> Self {
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

#[derive(Copy, Clone)]
pub enum ReadRequest {
    ClassScan(Classes),
    Range8(RangeScan<u8>),
    Range16(RangeScan<u16>),
}

#[derive(Copy, Clone)]
pub enum BasicRequest {
    EnableUnsolicited(EventClasses),
    DisableUnsolicited(EventClasses),
}

impl ReadRequest {
    pub fn class_scan(scan: Classes) -> Self {
        ReadRequest::ClassScan(scan)
    }

    pub fn one_byte_range(variation: Variation, start: u8, stop: u8) -> Self {
        ReadRequest::Range8(RangeScan::new(variation, start, stop))
    }

    pub fn two_byte_range(variation: Variation, start: u16, stop: u16) -> Self {
        ReadRequest::Range16(RangeScan::new(variation, start, stop))
    }

    pub(crate) fn format(self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        match self {
            ReadRequest::ClassScan(classes) => classes.write(writer),
            ReadRequest::Range8(scan) => scan.write(writer),
            ReadRequest::Range16(scan) => scan.write(writer),
        }
    }
}

impl BasicRequest {
    pub(crate) fn format(self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        match self {
            BasicRequest::EnableUnsolicited(classes) => classes.write(writer),
            BasicRequest::DisableUnsolicited(classes) => classes.write(writer),
        }
    }

    pub(crate) fn function(self) -> FunctionCode {
        match self {
            BasicRequest::EnableUnsolicited(_) => FunctionCode::EnabledUnsolicited,
            BasicRequest::DisableUnsolicited(_) => FunctionCode::DisableUnsolicited,
        }
    }
}

pub enum PrefixedCommandHeader<I>
where
    I: Index,
{
    G12V1(Vec<(Group12Var1, I)>),
    G41V1(Vec<(Group41Var1, I)>),
    G41V2(Vec<(Group41Var2, I)>),
    G41V3(Vec<(Group41Var3, I)>),
    G41V4(Vec<(Group41Var4, I)>),
}

impl<I> PrefixedCommandHeader<I>
where
    I: Index,
{
    pub(crate) fn write(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        match self {
            PrefixedCommandHeader::G12V1(items) => writer.write_prefixed_items(items.iter()),
            PrefixedCommandHeader::G41V1(items) => writer.write_prefixed_items(items.iter()),
            PrefixedCommandHeader::G41V2(items) => writer.write_prefixed_items(items.iter()),
            PrefixedCommandHeader::G41V3(items) => writer.write_prefixed_items(items.iter()),
            PrefixedCommandHeader::G41V4(items) => writer.write_prefixed_items(items.iter()),
        }
    }
}

trait HasCommandStatus {
    fn status(&self) -> CommandStatus;
}

impl HasCommandStatus for Group12Var1 {
    fn status(&self) -> CommandStatus {
        self.status
    }
}

impl HasCommandStatus for Group41Var1 {
    fn status(&self) -> CommandStatus {
        self.status
    }
}

impl HasCommandStatus for Group41Var2 {
    fn status(&self) -> CommandStatus {
        self.status
    }
}

impl HasCommandStatus for Group41Var3 {
    fn status(&self) -> CommandStatus {
        self.status
    }
}

impl HasCommandStatus for Group41Var4 {
    fn status(&self) -> CommandStatus {
        self.status
    }
}

pub enum CommandHeader {
    U8(PrefixedCommandHeader<u8>),
    U16(PrefixedCommandHeader<u16>),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum CommandResponseError {
    /// the outstation indicated that a command was not SUCCESS for the specified reason
    BadStatus(CommandStatus),
    /// the number of headers in the response doesn't match the number in the request
    HeaderCountMismatch,
    /// a header in the response doesn't match the request
    HeaderTypeMismatch,
    /// the number of objects in one of the headers doesn't match the request
    ObjectCountMismatch,
    /// a value in one of the objects in the response doesn't match the request
    ObjectValueMismatch,
}

impl std::fmt::Display for CommandResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CommandResponseError::BadStatus(status) => write!(
                f,
                "command status value other than Success was returned: {:?}",
                status
            ),
            CommandResponseError::HeaderCountMismatch => f.write_str(
                "response did not contain the same number of object headers as the request",
            ),
            CommandResponseError::HeaderTypeMismatch => {
                f.write_str("response contained a header type different than the request")
            }
            CommandResponseError::ObjectCountMismatch => f.write_str(
                "response header does not have the same number of objects as the request",
            ),
            CommandResponseError::ObjectValueMismatch => f.write_str(
                "a value other than the status is different in the response than the request",
            ),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum CommandTaskError {
    Response(CommandResponseError),
    Task(RequestError),
}

impl std::fmt::Display for CommandTaskError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CommandTaskError::Response(x) => std::fmt::Display::fmt(x, f),
            CommandTaskError::Task(x) => std::fmt::Display::fmt(x, f),
        }
    }
}

pub trait CommandTaskHandler: RequestCompletionHandler {
    fn on_response(&mut self, result: Result<(), CommandResponseError>);
}

impl CommandHeader {
    pub(crate) fn write(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        match self {
            CommandHeader::U8(header) => header.write(writer),
            CommandHeader::U16(header) => header.write(writer),
        }
    }

    fn compare_items<V, I>(
        seq: CountSequence<'_, Prefix<I, V>>,
        sent: &[(V, I)],
    ) -> Result<(), CommandResponseError>
    where
        V: FixedSizeVariation + HasCommandStatus,
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
            CommandHeader::U8(PrefixedCommandHeader::G12V1(items)) => match response {
                HeaderDetails::OneByteCountAndPrefix(_, PrefixedVariation::Group12Var1(seq)) => {
                    Self::compare_items(seq, items)
                }
                _ => Err(CommandResponseError::HeaderTypeMismatch),
            },
            CommandHeader::U16(PrefixedCommandHeader::G12V1(items)) => match response {
                HeaderDetails::TwoByteCountAndPrefix(_, PrefixedVariation::Group12Var1(seq)) => {
                    Self::compare_items(seq, items)
                }
                _ => Err(CommandResponseError::HeaderTypeMismatch),
            },
            CommandHeader::U8(PrefixedCommandHeader::G41V1(items)) => match response {
                HeaderDetails::OneByteCountAndPrefix(_, PrefixedVariation::Group41Var1(seq)) => {
                    Self::compare_items(seq, items)
                }
                _ => Err(CommandResponseError::HeaderTypeMismatch),
            },
            CommandHeader::U16(PrefixedCommandHeader::G41V1(items)) => match response {
                HeaderDetails::TwoByteCountAndPrefix(_, PrefixedVariation::Group41Var1(seq)) => {
                    Self::compare_items(seq, items)
                }
                _ => Err(CommandResponseError::HeaderTypeMismatch),
            },
            CommandHeader::U8(PrefixedCommandHeader::G41V2(items)) => match response {
                HeaderDetails::OneByteCountAndPrefix(_, PrefixedVariation::Group41Var2(seq)) => {
                    Self::compare_items(seq, items)
                }
                _ => Err(CommandResponseError::HeaderTypeMismatch),
            },
            CommandHeader::U16(PrefixedCommandHeader::G41V2(items)) => match response {
                HeaderDetails::TwoByteCountAndPrefix(_, PrefixedVariation::Group41Var2(seq)) => {
                    Self::compare_items(seq, items)
                }
                _ => Err(CommandResponseError::HeaderTypeMismatch),
            },
            CommandHeader::U8(PrefixedCommandHeader::G41V3(items)) => match response {
                HeaderDetails::OneByteCountAndPrefix(_, PrefixedVariation::Group41Var3(seq)) => {
                    Self::compare_items(seq, items)
                }
                _ => Err(CommandResponseError::HeaderTypeMismatch),
            },
            CommandHeader::U16(PrefixedCommandHeader::G41V3(items)) => match response {
                HeaderDetails::TwoByteCountAndPrefix(_, PrefixedVariation::Group41Var3(seq)) => {
                    Self::compare_items(seq, items)
                }
                _ => Err(CommandResponseError::HeaderTypeMismatch),
            },
            CommandHeader::U8(PrefixedCommandHeader::G41V4(items)) => match response {
                HeaderDetails::OneByteCountAndPrefix(_, PrefixedVariation::Group41Var4(seq)) => {
                    Self::compare_items(seq, items)
                }
                _ => Err(CommandResponseError::HeaderTypeMismatch),
            },
            CommandHeader::U16(PrefixedCommandHeader::G41V4(items)) => match response {
                HeaderDetails::TwoByteCountAndPrefix(_, PrefixedVariation::Group41Var4(seq)) => {
                    Self::compare_items(seq, items)
                }
                _ => Err(CommandResponseError::HeaderTypeMismatch),
            },
        }
    }
}
