/// user-facing database API to add/remove/update values
pub mod database;
/// async outstation task API that can be run on arbitrary I/O types
/// implementing `AsyncRead` + `AsyncWrite` + `Unpin`
pub mod task;
