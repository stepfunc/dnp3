// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use std::fmt::Write;

const BYTES_PER_DECODE_LINE: usize = 18;

pub(crate) fn format_bytes(f: &mut std::fmt::Formatter, bytes: &[u8]) -> std::fmt::Result {
    for chunk in bytes.chunks(BYTES_PER_DECODE_LINE) {
        writeln!(f)?;
        let mut first = true;
        for byte in chunk {
            if !first {
                f.write_char(' ')?;
            }
            first = false;
            write!(f, "{byte:02X?}")?;
        }
    }
    Ok(())
}
