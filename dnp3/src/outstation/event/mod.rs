/// api for interacting with a database
pub mod buffer;
/// custom list type for use with event buffer
pub(crate) mod list;
/// module level traits and impls
pub(crate) mod traits;
/// event variation enums
pub mod variations;
/// low-level types and functions for event writing
pub(crate) mod write_fn;
/// custom list type for use with event buffer
pub mod writer;
