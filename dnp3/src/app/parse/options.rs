use std::sync::atomic::Ordering;

pub(crate) struct ParseOptions {
    pub(crate) parse_zero_length_strings: bool,
}

static PARSE_ZERO_LENGTH_STRINGS: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

impl ParseOptions {
    pub(crate) fn get() -> Self {
        Self {
            parse_zero_length_strings: PARSE_ZERO_LENGTH_STRINGS.load(Ordering::Relaxed),
        }
    }

    pub(crate) fn parse_zero_length_strings(enabled: bool) {
        PARSE_ZERO_LENGTH_STRINGS.store(enabled, Ordering::Relaxed)
    }
}
