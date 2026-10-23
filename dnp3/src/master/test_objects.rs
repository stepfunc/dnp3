// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Object headers shared by the tests of how the master checks relative time and malformed
//! objects in a response

/// g2v3 with a count of 1 and a one-byte index: index 7, flags 0x01, relative time 0xFFFF
pub(crate) const RELATIVE_TIME_EVENT: &[u8] = &[0x02, 0x03, 0x17, 0x01, 0x07, 0x01, 0xFF, 0xFF];

/// g4v3 with a one-byte count and prefix, then g2v3 and g4v3 with a two-byte count and prefix,
/// each with one event at index 7 with flags 0x01 and relative time 0xFFFF
pub(crate) const OTHER_RELATIVE_TIME_EVENTS: [&[u8]; 3] = [
    &[0x04, 0x03, 0x17, 0x01, 0x07, 0x01, 0xFF, 0xFF],
    &[0x02, 0x03, 0x28, 0x01, 0x00, 0x07, 0x00, 0x01, 0xFF, 0xFF],
    &[0x04, 0x03, 0x28, 0x01, 0x00, 0x07, 0x00, 0x01, 0xFF, 0xFF],
];

/// g51v1 with a count of 1, time 2
pub(crate) const SYNCHRONIZED_CTO: &[u8] =
    &[0x33, 0x01, 0x07, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00];

/// g51v1 with a one-byte count of 0, g51v2 with a two-byte count of 0, then the same with a count
/// of 2 and times 2 and 3
pub(crate) const CTOS_WITH_A_COUNT_OTHER_THAN_ONE: [&[u8]; 4] = [
    &[0x33, 0x01, 0x07, 0x00],
    &[0x33, 0x02, 0x08, 0x00, 0x00],
    &[
        0x33, 0x01, 0x07, 0x02, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ],
    &[
        0x33, 0x02, 0x08, 0x02, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00,
        0x00, 0x00,
    ],
];

/// g32v2 with a count of 1 and a one-byte index, missing its flags and value
pub(crate) const TRUNCATED_ANALOG_EVENT: &[u8] = &[0x20, 0x02, 0x17, 0x01, 0x00];
