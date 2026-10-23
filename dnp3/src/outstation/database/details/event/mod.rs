// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

/// api for interacting with a database
pub(crate) mod buffer;
/// custom list type for use with event buffer
pub(crate) mod list;
/// module level traits and impls
pub(crate) mod traits;
/// low-level types and functions for event writing
pub(crate) mod write_fn;
/// event writing
pub(crate) mod writer;

#[cfg(test)]
mod tests;
