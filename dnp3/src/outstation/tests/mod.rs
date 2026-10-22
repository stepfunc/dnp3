pub(crate) mod harness;

/// device attributes (group 0)
mod attributes;
/// every control object and index width, handler statuses and the control limit
mod control_objects;
/// control functionality
mod controls;
/// freeze counters tests
mod freeze;
/// various IIN bit tests
mod iin;
/// the answer of each non-READ function to every header the parser accepts
mod non_read_headers;
/// encoding tests for octet strings
mod octet_strings;
/// reading g34
mod read_dead_band;
/// error IIN for reads of points that aren't installed
mod read_errors;
/// fragment boundaries between events, static data and attributes in READ and unsolicited responses
mod read_fragments;
/// READ headers the parser rejects or the database can't select, alone and in a deferred READ
mod read_headers;
/// READ header, selection and fragment limits
mod read_limits;
/// state machine for READ requests
mod read_states;
/// responses to repeated requests
mod repeats;
/// which frames and fragments are dropped or answered, over the real link and transport layers
mod request_filtering;
/// how a request is answered in idle and in each confirm wait
mod request_intake;
/// the bytes of the solicited and unsolicited responses sent again
mod response_slots;
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
}
