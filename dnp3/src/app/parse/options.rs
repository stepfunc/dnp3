use std::sync::atomic::Ordering;

pub(crate) struct ParseOptions {
    pub(crate) allow_zero_length_string: bool,
}

static ALLOW_ZERO_LENGTH_STRINGS: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

impl ParseOptions {
    pub(crate) fn get() -> Self {
        Self {
            allow_zero_length_string: ALLOW_ZERO_LENGTH_STRINGS.load(Ordering::Relaxed),
        }
    }

    pub(crate) fn enable_zero_length_strings() {
        ALLOW_ZERO_LENGTH_STRINGS.store(true, Ordering::Relaxed)
    }
}
