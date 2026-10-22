pub(crate) mod attrs;
pub(crate) mod database;
pub(crate) mod event;
pub(crate) mod range;

/// whether a part of a response wrote everything it selected into the fragment
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum WriteProgress {
    /// everything selected was written
    Complete,
    /// the fragment filled first, and the rest stays selected for the next one
    Partial,
}

impl WriteProgress {
    /// write the next part of the response only if this part is complete
    pub(crate) fn then(self, next: impl FnOnce() -> Self) -> Self {
        match self {
            Self::Complete => next(),
            Self::Partial => Self::Partial,
        }
    }
}
