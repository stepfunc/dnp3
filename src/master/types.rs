use crate::app::format::write::HeaderWriter;
use crate::app::gen::enums::CommandStatus;
use crate::app::gen::variations::fixed::*;
use crate::app::gen::variations::prefixed::PrefixedVariation;
use crate::app::parse::count::CountSequence;
use crate::app::parse::parser::HeaderDetails;
use crate::app::parse::prefix::Prefix;
use crate::app::parse::traits::{FixedSize, FixedSizeVariation, Index};
use crate::util::cursor::WriteError;

#[derive(Copy, Clone)]
pub struct ClassScan {
    pub class1: bool,
    pub class2: bool,
    pub class3: bool,
    pub class0: bool,
}

impl ClassScan {
    pub fn new(class1: bool, class2: bool, class3: bool, class0: bool) -> Self {
        Self {
            class1,
            class2,
            class3,
            class0,
        }
    }

    pub fn class1() -> Self {
        Self::new(true, false, false, false)
    }

    pub fn class123() -> Self {
        Self::new(true, true, true, false)
    }

    pub fn integrity() -> Self {
        Self::new(true, true, true, true)
    }
}

pub enum PrefixedCommandHeader<I>
where
    I: Index + FixedSize,
{
    G12V1(Vec<(Group12Var1, I)>),
    G41V1(Vec<(Group41Var1, I)>),
    G41V2(Vec<(Group41Var2, I)>),
    G41V3(Vec<(Group41Var3, I)>),
    G41V4(Vec<(Group41Var4, I)>),
}

impl<I> PrefixedCommandHeader<I>
where
    I: Index + FixedSize,
{
    pub(crate) fn write(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        match self {
            PrefixedCommandHeader::G12V1(items) => writer.write_prefixed_header(items.iter()),
            PrefixedCommandHeader::G41V1(items) => writer.write_prefixed_header(items.iter()),
            PrefixedCommandHeader::G41V2(items) => writer.write_prefixed_header(items.iter()),
            PrefixedCommandHeader::G41V3(items) => writer.write_prefixed_header(items.iter()),
            PrefixedCommandHeader::G41V4(items) => writer.write_prefixed_header(items.iter()),
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
pub enum CommandError {
    // the outstation indicated that a command was not SUCCESS for the specified reason
    BadStatus(CommandStatus),
    // the number of headers in the response doesn't match the number in the request
    HeaderCountMismatch,
    // a header in the response doesn't match the request
    HeaderTypeMismatch,
    // the number of objects in one of the headers doesn't match the request
    ObjectCountMismatch,
    // a value in one of the objects in the response doesn't match the request
    ObjectValueMismatch,
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
    ) -> Result<(), CommandError>
    where
        V: FixedSizeVariation + HasCommandStatus,
        I: Index,
    {
        let mut received = seq.iter();

        for item in sent {
            match received.next() {
                None => return Err(CommandError::ObjectCountMismatch),
                Some(x) => {
                    if x.value.status() != CommandStatus::Success {
                        return Err(CommandError::BadStatus(x.value.status()));
                    }
                    if !x.equals(item) {
                        return Err(CommandError::ObjectValueMismatch);
                    }
                }
            }
        }

        if received.next().is_some() {
            return Err(CommandError::ObjectCountMismatch);
        }

        Ok(())
    }

    pub(crate) fn compare(&self, response: HeaderDetails) -> Result<(), CommandError> {
        match self {
            CommandHeader::U8(PrefixedCommandHeader::G12V1(items)) => match response {
                HeaderDetails::OneByteCountAndPrefix(_, PrefixedVariation::Group12Var1(seq)) => {
                    Self::compare_items(seq, items)
                }
                _ => Err(CommandError::HeaderTypeMismatch),
            },
            CommandHeader::U16(PrefixedCommandHeader::G12V1(items)) => match response {
                HeaderDetails::TwoByteCountAndPrefix(_, PrefixedVariation::Group12Var1(seq)) => {
                    Self::compare_items(seq, items)
                }
                _ => Err(CommandError::HeaderTypeMismatch),
            },
            CommandHeader::U8(PrefixedCommandHeader::G41V1(items)) => match response {
                HeaderDetails::OneByteCountAndPrefix(_, PrefixedVariation::Group41Var1(seq)) => {
                    Self::compare_items(seq, items)
                }
                _ => Err(CommandError::HeaderTypeMismatch),
            },
            CommandHeader::U16(PrefixedCommandHeader::G41V1(items)) => match response {
                HeaderDetails::TwoByteCountAndPrefix(_, PrefixedVariation::Group41Var1(seq)) => {
                    Self::compare_items(seq, items)
                }
                _ => Err(CommandError::HeaderTypeMismatch),
            },
            CommandHeader::U8(PrefixedCommandHeader::G41V2(items)) => match response {
                HeaderDetails::OneByteCountAndPrefix(_, PrefixedVariation::Group41Var2(seq)) => {
                    Self::compare_items(seq, items)
                }
                _ => Err(CommandError::HeaderTypeMismatch),
            },
            CommandHeader::U16(PrefixedCommandHeader::G41V2(items)) => match response {
                HeaderDetails::TwoByteCountAndPrefix(_, PrefixedVariation::Group41Var2(seq)) => {
                    Self::compare_items(seq, items)
                }
                _ => Err(CommandError::HeaderTypeMismatch),
            },
            CommandHeader::U8(PrefixedCommandHeader::G41V3(items)) => match response {
                HeaderDetails::OneByteCountAndPrefix(_, PrefixedVariation::Group41Var3(seq)) => {
                    Self::compare_items(seq, items)
                }
                _ => Err(CommandError::HeaderTypeMismatch),
            },
            CommandHeader::U16(PrefixedCommandHeader::G41V3(items)) => match response {
                HeaderDetails::TwoByteCountAndPrefix(_, PrefixedVariation::Group41Var3(seq)) => {
                    Self::compare_items(seq, items)
                }
                _ => Err(CommandError::HeaderTypeMismatch),
            },
            CommandHeader::U8(PrefixedCommandHeader::G41V4(items)) => match response {
                HeaderDetails::OneByteCountAndPrefix(_, PrefixedVariation::Group41Var4(seq)) => {
                    Self::compare_items(seq, items)
                }
                _ => Err(CommandError::HeaderTypeMismatch),
            },
            CommandHeader::U16(PrefixedCommandHeader::G41V4(items)) => match response {
                HeaderDetails::TwoByteCountAndPrefix(_, PrefixedVariation::Group41Var4(seq)) => {
                    Self::compare_items(seq, items)
                }
                _ => Err(CommandError::HeaderTypeMismatch),
            },
        }
    }
}
