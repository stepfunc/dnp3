pub(crate) mod harness;

/// control functionality
mod controls;
/// various IIN bit tests
mod iin;
/// state machine for READ requests
mod read_states;
/// clear restart IIN + cold/warm restart
mod restart;
/// time synchronization
mod time;
/// unsolicited responses
mod unsolicited;

/// test data for use in multiple tests
mod data {
    pub(crate) const DELAY_MEASURE: &[u8] = &[0xC0, 23];
    pub(crate) const RESPONSE_TIME_DELAY_FINE_ZERO: &[u8] =
        &[0xC0, 0x81, 0x80, 0x00, 0x34, 0x02, 0x07, 0x01, 0x00, 0x00];
}
