// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::util::buffer::Buffer;

/// Validated buffer size for use in configuration structs
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(feature = "serialization", serde(try_from = "usize"))]
pub struct BufferSize<const MIN_SIZE: usize = 249, const DEFAULT_SIZE: usize = 2048>(usize);

impl<const MIN_SIZE: usize, const DEFAULT_SIZE: usize> TryFrom<usize>
    for BufferSize<MIN_SIZE, DEFAULT_SIZE>
{
    type Error = BufferSizeError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// Error type returned for invalid buffer sizes
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BufferSizeError {
    /// provided size vs minimum size
    TooSmall(usize, usize),
}

impl<const MIN_SIZE: usize, const DEFAULT_SIZE: usize> BufferSize<MIN_SIZE, DEFAULT_SIZE> {
    /// minimum allowed buffer size
    pub const MIN: usize = MIN_SIZE;
    /// default buffer size
    ///
    /// [`BufferSize::default`] uses the larger of this value and [`Self::MIN`]
    pub const DEFAULT: usize = DEFAULT_SIZE;

    pub(crate) fn create_buffer(&self) -> Buffer {
        Buffer::new(self.0)
    }

    /// get the underlying value
    pub fn value(&self) -> usize {
        self.0
    }

    /// construct a [`BufferSize`] with the minimum value
    pub fn min() -> Self {
        Self(Self::MIN)
    }

    /// attempt to construct a [`BufferSize`]
    pub fn new(size: usize) -> Result<Self, BufferSizeError> {
        if size < Self::MIN {
            return Err(BufferSizeError::TooSmall(size, Self::MIN));
        }
        Ok(Self(size))
    }
}

impl<const MIN_SIZE: usize, const DEFAULT_SIZE: usize> Default
    for BufferSize<MIN_SIZE, DEFAULT_SIZE>
{
    /// Uses [`BufferSize::DEFAULT`], raised to [`BufferSize::MIN`] if it is smaller
    fn default() -> Self {
        Self(Self::DEFAULT.max(Self::MIN))
    }
}

impl std::fmt::Display for BufferSizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::TooSmall(provided_size, min_size) => write!(
                f,
                "provided size {provided_size} is less than the minimum allowed size of {min_size}",
            ),
        }
    }
}

impl std::error::Error for BufferSizeError {}

#[cfg(test)]
mod test {
    use super::BufferSize;

    #[test]
    fn default_is_never_below_min() {
        assert_eq!(BufferSize::<500, 100>::default().value(), 500);
        assert_eq!(BufferSize::<100, 500>::default().value(), 500);
        assert_eq!(BufferSize::<249, 2048>::default().value(), 2048);
    }

    #[test]
    #[cfg(feature = "serialization")]
    fn deserialization_enforces_min_size() {
        serde_json::from_str::<super::BufferSize>("248").unwrap_err();
        assert_eq!(
            249,
            serde_json::from_str::<super::BufferSize>("249").unwrap().0
        );
    }
}
