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
