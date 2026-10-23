// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Requests and responses that tests in several files send or expect

use std::ops::Range;

use crate::app::FunctionCode;

/// CONFIRM of a solicited response
pub(crate) const fn confirm(seq: u8) -> [u8; 2] {
    [0xC0 | seq, 0x00]
}

/// CONFIRM of an unsolicited response
pub(crate) const fn unsolicited_confirm(seq: u8) -> [u8; 2] {
    [0xD0 | seq, 0x00]
}

pub(crate) const fn record_current_time(seq: u8) -> [u8; 2] {
    [0xC0 | seq, 0x18]
}

pub(crate) const fn delay_measure(seq: u8) -> [u8; 2] {
    [0xC0 | seq, 0x17]
}

/// READ class 0
pub(crate) const fn read_class_0(seq: u8) -> [u8; 5] {
    [0xC0 | seq, 0x01, 60, 1, 0x06]
}

/// READ class 1
pub(crate) const fn read_class_1(seq: u8) -> [u8; 5] {
    [0xC0 | seq, 0x01, 60, 2, 0x06]
}

/// ENABLE_UNSOLICITED for classes 1, 2 and 3
pub(crate) const ENABLE_UNSOLICITED_SEQ_0: &[u8] = &[
    0xC0, 0x14, 0x3C, 0x02, 0x06, 0x3C, 0x03, 0x06, 0x3C, 0x04, 0x06,
];

/// ENABLE_UNSOLICITED for class 2 only
pub(crate) const ENABLE_UNSOLICITED_CLASS_2_SEQ_0: &[u8] = &[0xC0, 0x14, 0x3C, 0x03, 0x06];

/// bits of IIN1
pub(crate) mod iin1 {
    /// IIN1.0, all stations
    pub(crate) const BROADCAST: u8 = 0x01;
    /// IIN1.1, class 1 events
    pub(crate) const CLASS_1: u8 = 0x02;
    /// IIN1.2, class 2 events
    pub(crate) const CLASS_2: u8 = 0x04;
    /// IIN1.7, device restart
    pub(crate) const RESTART: u8 = 0x80;
}

/// bits of IIN2
pub(crate) mod iin2 {
    /// IIN2.0, function code not supported
    pub(crate) const NO_FUNC_CODE_SUPPORT: u8 = 0x01;
    /// IIN2.1, object unknown
    pub(crate) const OBJECT_UNKNOWN: u8 = 0x02;
    /// IIN2.2, parameter error
    pub(crate) const PARAMETER_ERROR: u8 = 0x04;
    /// IIN2.3, event buffer overflow
    pub(crate) const EVENT_BUFFER_OVERFLOW: u8 = 0x08;
}

/// the header of a solicited response with the application control field `control` (FIR, FIN,
/// CON and the sequence number)
pub(crate) const fn response_header(control: u8, iin1: u8, iin2: u8) -> [u8; 4] {
    [control, 0x81, iin1, iin2]
}

/// the header of a solicited response with FIR and FIN, and CON if `con`
pub(crate) const fn solicited(seq: u8, con: bool, iin1: u8, iin2: u8) -> [u8; 4] {
    let con = if con { 0x20 } else { 0x00 };
    response_header(0xC0 | con | seq, iin1, iin2)
}

/// a solicited response without objects or CON, whose IIN1 has only the restart bit
pub(crate) const fn empty_response(seq: u8, iin2: u8) -> [u8; 4] {
    solicited(seq, false, iin1::RESTART, iin2)
}

/// a response header followed by `objects`
pub(crate) fn with_objects(header: [u8; 4], objects: &[u8]) -> Vec<u8> {
    let mut bytes = header.to_vec();
    bytes.extend_from_slice(objects);
    bytes
}

/// DISABLE_UNSOLICITED of the classes 1, 2 and 3 that `classes` names
pub(crate) fn disable_unsolicited(seq: u8, classes: [bool; 3]) -> Vec<u8> {
    let mut bytes = vec![0xC0 | seq, 0x15];
    for (variation, named) in [2, 3, 4].into_iter().zip(classes) {
        if named {
            bytes.extend_from_slice(&[60, variation, 0x06]);
        }
    }
    bytes
}

/// a null unsolicited response with the restart IIN
pub(crate) const fn null_unsolicited(seq: u8) -> [u8; 4] {
    [0xF0 | seq, 0x82, 0x80, 0x00]
}

/// an unsolicited response with FIR, FIN and CON
pub(crate) fn unsolicited(seq: u8, iin1: u8, objects: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0xF0 | seq, 0x82, iin1, 0x00];
    bytes.extend_from_slice(objects);
    bytes
}

/// a request for g41v2 at `index` with value 513
pub(crate) fn g41v2_control(function: FunctionCode, seq: u8, index: u8) -> Vec<u8> {
    vec![
        0xC0 | seq,
        function.as_u8(),
        41,
        2,
        0x17,
        0x01,
        index,
        0x01,
        0x02,
        0x00,
    ]
}

/// g2v1 events for the indices, each with flags ONLINE and state true
#[expect(
    clippy::arithmetic_side_effects,
    reason = "tests pass ranges whose end is not below their start"
)]
pub(crate) fn g2v1_events(indices: Range<u16>) -> Vec<u8> {
    let count = indices.end - indices.start;
    let mut bytes = vec![0x02, 0x01, 0x28];
    bytes.extend(count.to_le_bytes());
    for index in indices {
        bytes.extend(index.to_le_bytes());
        bytes.push(0x81);
    }
    bytes
}

/// a visible string attribute in the default set
pub(crate) fn encode_string(variation: u8, value: &str) -> Vec<u8> {
    let mut bytes = vec![0x00, variation, 0x00, 0x00, 0x00, 0x01];
    bytes.push(value.len().try_into().unwrap());
    bytes.extend_from_slice(value.as_bytes());
    bytes
}
