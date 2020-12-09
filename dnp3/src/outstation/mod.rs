/// configuration types
pub mod config;
/// database API to add/remove/update values
pub mod database;
/// async outstation task API that can be run on arbitrary I/O types
/// implementing `AsyncRead` + `AsyncWrite` + `Unpin`
pub mod task;
/// user-facing traits used to receive dynamic callbacks from the outstation
pub mod traits;

/// functionality for processing control requests
pub(crate) mod control;
/// outstation session
pub(crate) mod session;

#[cfg(test)]
mod tests;
