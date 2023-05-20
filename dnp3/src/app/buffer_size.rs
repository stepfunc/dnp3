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
    fn default() -> Self {
        Self(Self::DEFAULT)
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

    #[test]
    #[cfg(feature = "serialization")]
    fn deserialization_enforces_min_size() {
        assert!(serde_json::from_str::<super::BufferSize>("248").is_err());
        assert_eq!(
            249,
            serde_json::from_str::<super::BufferSize>("249").unwrap().0
        );
    }
}
