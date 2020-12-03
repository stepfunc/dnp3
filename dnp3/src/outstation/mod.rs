/// user-facing database API to add/remove/update values
pub mod database;

/// async outstation task API that can be run on arbitrary I/O types
/// implementing `AsyncRead` + `AsyncWrite` + `Unpin`
pub mod task;
/// user-facing traits used to receive dynamic callbacks from the outstation
pub mod traits;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SelfAddressSupport {
    Enabled,
    Disabled,
}

impl SelfAddressSupport {
    pub(crate) fn is_enabled(&self) -> bool {
        *self == SelfAddressSupport::Enabled
    }
    pub(crate) fn is_disabled(&self) -> bool {
        !self.is_enabled()
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BroadcastAddressSupport {
    Enabled,
    Disabled,
}

impl BroadcastAddressSupport {
    pub(crate) fn is_enabled(&self) -> bool {
        *self == BroadcastAddressSupport::Enabled
    }
    pub(crate) fn is_disabled(&self) -> bool {
        !self.is_enabled()
    }
}

/// functionality for processing control requests
pub(crate) mod control;

#[cfg(test)]
mod tests;
