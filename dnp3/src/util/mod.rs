// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

pub(crate) mod bit;
pub(crate) mod bounded;
pub(crate) mod buffer;
pub(crate) mod channel;
pub(crate) mod deadline;
pub(crate) mod decode;
pub(crate) mod float;
pub(crate) mod lifecycle;
pub(crate) mod phys;
pub(crate) mod session;
pub(crate) mod shutdown;
pub(crate) mod skipped;
#[cfg(test)]
pub(crate) mod test_support;

/// signals a bad write operation
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct BadWrite;

impl From<scursor::WriteError> for BadWrite {
    fn from(_: scursor::WriteError) -> Self {
        Self
    }
}
