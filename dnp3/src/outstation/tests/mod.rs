// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

pub(crate) mod harness;

/// device attributes (group 0)
mod attributes;
/// every control object and index width, handler statuses and the control limit
mod control_objects;
/// control functionality
mod controls;
/// binary and double-bit event variations by the time quality of each event
mod event_time;
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
/// the confirm, select, unsolicited retry and keep-alive timers
mod timeouts;
/// unsolicited responses
mod unsolicited;
/// writing g34
mod write_dead_band;
