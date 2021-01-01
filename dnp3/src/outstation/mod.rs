/// configuration types
pub mod config;
/// database API to add/remove/update values
pub mod database;
/// user-facing traits used to receive dynamic callbacks from the outstation
pub mod traits;

/// functionality for processing control requests
pub(crate) mod control;
/// handling of deferred read requests
pub(crate) mod deferred;
/// outstation session
pub(crate) mod session;
/// async outstation task API that can be run on arbitrary I/O types
/// implementing `AsyncRead` + `AsyncWrite` + `Unpin`
pub(crate) mod task;

#[cfg(test)]
mod tests;
