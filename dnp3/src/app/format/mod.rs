pub(crate) mod free_format;
pub(crate) mod write;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum WriteError {
    /// cursor error
    WriteError(scursor::WriteError),
    /// provided data would overflow a numeric representation
    Overflow,
}

impl From<scursor::WriteError> for WriteError {
    fn from(value: scursor::WriteError) -> Self {
        Self::WriteError(value)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Overflow;

pub(crate) fn to_u16<X: TryInto<u16>>(x: X) -> Result<u16, Overflow> {
    x.try_into().map_err(|_| Overflow)
}

impl From<Overflow> for WriteError {
    fn from(_: Overflow) -> Self {
        WriteError::Overflow
    }
}
