use crate::app::Sequence;
use crate::app::{Iin, Iin1, Iin2};
use crate::master::association::AssociationConfig;
use crate::master::request::EventClasses;
use crate::master::Classes;

use super::harness::create_association;
use super::harness::requests::*;

const BUFFER_OVERFLOW: Iin = Iin::new(Iin1::new(0x00), Iin2::new(0x08));
const CLASS_1_EVENTS: Iin = Iin::new(Iin1::new(0x02), Iin2::new(0x00));

#[tokio::test]
async fn auto_integrity_scan_on_buffer_overflow() {
    let mut config = AssociationConfig::default();
    config.auto_integrity_scan_on_buffer_overflow = true;
    let mut seq = Sequence::default();
    let mut harness = create_association(config).await;

    startup_procedure(&mut harness, &mut seq).await;

    // Unsolicited with IIN2.3 EVENT_BUFFER_OVERFLOW set
    harness
        .read_and_expect_write(
            unsol_null_custom_iin(seq, BUFFER_OVERFLOW),
            unsol_confirm(seq),
        )
        .await;

    // Send integrity poll
    harness
        .expect_write_and_respond(integrity_poll_request(seq), empty_response(seq.increment()))
        .await;

    assert_eq!(harness.io.pop_event(), None);
}

#[tokio::test]
async fn auto_integrity_scan_on_buffer_overflow_disabled() {
    let mut config = AssociationConfig::default();
    config.auto_integrity_scan_on_buffer_overflow = false;
    let mut seq = Sequence::default();
    let mut harness = create_association(config).await;

    startup_procedure(&mut harness, &mut seq).await;

    // Unsolicited with IIN2.3 EVENT_BUFFER_OVERFLOW set
    harness
        .read_and_expect_write(
            unsol_null_custom_iin(seq, BUFFER_OVERFLOW),
            unsol_confirm(seq),
        )
        .await;

    assert_eq!(harness.io.pop_event(), None);
}

#[tokio::test]
async fn auto_event_class_scan() {
    let mut config = AssociationConfig::default();
    config.event_scan_on_events_available = EventClasses::all();
    let mut seq = Sequence::default();
    let mut harness = create_association(config).await;

    startup_procedure(&mut harness, &mut seq).await;

    // notify that class 1 events are available
    harness
        .read_and_expect_write(
            unsol_null_custom_iin(seq, CLASS_1_EVENTS),
            unsol_confirm(seq),
        )
        .await;

    let class1 = Classes::new(false, EventClasses::new(true, false, false));
    harness
        .expect_write_and_respond(class_scan_request(class1, seq), empty_response(seq))
        .await;

    assert_eq!(harness.io.pop_event(), None);
}

#[tokio::test]
async fn auto_event_class_ignore_one_class_scan() {
    let mut config = AssociationConfig::default();
    config.event_scan_on_events_available = EventClasses::new(false, true, true);
    let mut seq = Sequence::default();
    let mut harness = create_association(config).await;

    startup_procedure(&mut harness, &mut seq).await;

    // Unsolicited with IIN2.3 EVENT_BUFFER_OVERFLOW set
    harness
        .read_and_expect_write(
            unsol_null_custom_iin(seq, CLASS_1_EVENTS),
            unsol_confirm(seq),
        )
        .await;
    // make sure this does not result in a write
    assert_eq!(harness.io.pop_event(), None);
}

#[tokio::test]
async fn auto_event_class_scan_disabled() {
    let mut config = AssociationConfig::default();
    config.event_scan_on_events_available = EventClasses::none();
    let mut seq = Sequence::default();
    let mut harness = create_association(config).await;

    startup_procedure(&mut harness, &mut seq).await;

    // Unsolicited with IIN2.3 EVENT_BUFFER_OVERFLOW set
    harness
        .read_and_expect_write(
            unsol_null_custom_iin(seq, CLASS_1_EVENTS),
            unsol_confirm(seq),
        )
        .await;
    // make sure this does not result in a write
    assert_eq!(harness.io.pop_event(), None);
}
