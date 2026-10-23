// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use std::sync::atomic::Ordering;

#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct ParseOptions {
    pub(crate) parse_zero_length_strings: bool,
}

impl ParseOptions {
    pub(crate) fn parse_everything() -> Self {
        Self {
            parse_zero_length_strings: true,
        }
    }
}

static PARSE_ZERO_LENGTH_STRINGS: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

impl ParseOptions {
    pub(crate) fn get_static() -> Self {
        Self {
            parse_zero_length_strings: PARSE_ZERO_LENGTH_STRINGS.load(Ordering::Relaxed),
        }
    }

    pub(crate) fn parse_zero_length_strings(enabled: bool) {
        PARSE_ZERO_LENGTH_STRINGS.store(enabled, Ordering::Relaxed)
    }
}
