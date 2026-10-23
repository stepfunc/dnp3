// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::link::reader::LinkModes;
use crate::link::{EndpointAddress, LinkRole};
use crate::transport::Transport;

pub(crate) mod reader;
pub(crate) mod writer;

/// Passes whole fragments to and from the physical layer, with no link or transport framing
pub(crate) struct MockTransport;

impl Transport for MockTransport {
    type Reader = reader::MockReader;
    type Writer = writer::MockWriter;

    fn new(
        _: LinkModes,
        _: LinkRole,
        _: EndpointAddress,
        rx_buffer_size: usize,
    ) -> (Self::Reader, Self::Writer) {
        (reader::MockReader::new(rx_buffer_size), writer::MockWriter)
    }
}
