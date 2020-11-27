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

pub(crate) mod helpers;
