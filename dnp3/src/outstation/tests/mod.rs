pub(crate) mod harness;

/// respond/ignore addresses
mod addressing;
/// control functionality
mod controls;
/// freeze counters tests
mod freeze;
/// various IIN bit tests
mod iin;
/// encoding tests for octet strings
mod octet_strings;
/// reading g34
mod read_dead_band;
/// state machine for READ requests
mod read_states;
/// clear restart IIN + cold/warm restart
mod restart;
/// time synchronization
mod time;
/// unsolicited responses
mod unsolicited;
/// writing g34
mod write_dead_band;

/// test data for use in multiple tests
mod data {
    pub(crate) const DELAY_MEASURE: &[u8] = &[0xC0, 23];
    pub(crate) const RESPONSE_TIME_DELAY_FINE_ZERO: &[u8] =
        &[0xC0, 0x81, 0x80, 0x00, 0x34, 0x02, 0x07, 0x01, 0x00, 0x00];
}
