use crate::util::buffer::Buffer;

/// Validated buffer size for use in configuration structs
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serialization", derive(serde::Serialize, serde::Deserialize))]
pub struct BufferSize<const MIN_SIZE: usize = 249, const DEFAULT_SIZE: usize = 2048> {
    size: usize,
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
    pub const DEFAULT: usize = DEFAULT_SIZE;

    pub(crate) fn create_buffer(&self) -> Buffer {
        Buffer::new(self.size)
    }

    /// get the underlying value
    pub fn value(&self) -> usize {
        self.size
    }

    /// construct a [`BufferSize`] with the minimum value
    pub fn min() -> Self {
        Self { size: Self::MIN }
    }

    /// attempt to construct a [`BufferSize`]
    pub fn new(size: usize) -> Result<Self, BufferSizeError> {
        if size < Self::MIN {
            return Err(BufferSizeError::TooSmall(size, Self::MIN));
        }
        Ok(Self { size })
    }
}

impl<const MIN_SIZE: usize, const DEFAULT_SIZE: usize> Default
    for BufferSize<MIN_SIZE, DEFAULT_SIZE>
{
    fn default() -> Self {
        Self {
            size: Self::DEFAULT,
        }
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
