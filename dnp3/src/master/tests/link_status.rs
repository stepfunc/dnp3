use super::harness::create_association;
use super::harness::requests::*;
use crate::app::sequence::Sequence;
use crate::prelude::master::*;

#[test]
fn master_startup_procedure() {
    let config = Configuration::default();
    let mut seq = Sequence::default();
    let mut harness = create_association(config);

    // Disable unsolicited
    disable_unsol_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.assert_io();

    // Integrity poll
    integrity_poll_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.assert_io();

    // Enable unsolicited
    enable_unsol_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.assert_io();
}
