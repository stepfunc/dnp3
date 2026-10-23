// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

pub(crate) mod bit;
pub(crate) mod bytes;
pub(crate) mod count;
pub(crate) mod free_format;
#[cfg(test)]
pub(crate) mod header_identity_tests;
pub(crate) mod options;
pub(crate) mod parser;
pub(crate) mod prefix;
pub(crate) mod range;
pub(crate) mod traits;
