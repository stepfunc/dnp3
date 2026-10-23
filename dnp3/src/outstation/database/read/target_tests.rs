// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Pins for what a READ of each variation selects, whatever its qualifier
//!
//! The READ header tests can't tell every target apart on the wire: with default configs a READ
//! of the any-variation (v0) of a type and one of its default variation give the same response,
//! and the frozen analog groups answer OBJECT_UNKNOWN whether they have a target or not, because
//! the database holds no frozen analog points. This table pins the target itself.

use crate::app::Variation;
use crate::outstation::database::variations::*;
use crate::outstation::database::EventClass;
use crate::outstation::target::Target;

use super::{EventTarget as E, ReadTarget, StaticTarget as S};

use ReadTarget::{Attr, Class0, Event, Static};

/// The groups whose variations are sized or numbered by the request, sampled below
const DYNAMIC_GROUPS: [u8; 3] = [0, 110, 111];

#[rustfmt::skip]
const ROWS: &[([u8; 2], Option<ReadTarget>)] = &[
    // binary input
    ([1, 0], Some(Static(S::Binary(None)))),
    ([1, 1], Some(Static(S::Binary(Some(StaticBinaryInputVariation::Group1Var1))))),
    ([1, 2], Some(Static(S::Binary(Some(StaticBinaryInputVariation::Group1Var2))))),
    ([2, 0], Some(Event(E::Binary(None)))),
    ([2, 1], Some(Event(E::Binary(Some(EventBinaryInputVariation::Group2Var1))))),
    ([2, 2], Some(Event(E::Binary(Some(EventBinaryInputVariation::Group2Var2))))),
    ([2, 3], Some(Event(E::Binary(Some(EventBinaryInputVariation::Group2Var3))))),
    // double-bit binary input
    ([3, 0], Some(Static(S::DoubleBitBinary(None)))),
    ([3, 1], Some(Static(S::DoubleBitBinary(Some(StaticDoubleBitBinaryInputVariation::Group3Var1))))),
    ([3, 2], Some(Static(S::DoubleBitBinary(Some(StaticDoubleBitBinaryInputVariation::Group3Var2))))),
    ([4, 0], Some(Event(E::DoubleBitBinary(None)))),
    ([4, 1], Some(Event(E::DoubleBitBinary(Some(EventDoubleBitBinaryInputVariation::Group4Var1))))),
    ([4, 2], Some(Event(E::DoubleBitBinary(Some(EventDoubleBitBinaryInputVariation::Group4Var2))))),
    ([4, 3], Some(Event(E::DoubleBitBinary(Some(EventDoubleBitBinaryInputVariation::Group4Var3))))),
    // binary output status
    ([10, 0], Some(Static(S::BinaryOutputStatus(None)))),
    ([10, 1], Some(Static(S::BinaryOutputStatus(Some(StaticBinaryOutputStatusVariation::Group10Var1))))),
    ([10, 2], Some(Static(S::BinaryOutputStatus(Some(StaticBinaryOutputStatusVariation::Group10Var2))))),
    ([11, 0], Some(Event(E::BinaryOutputStatus(None)))),
    ([11, 1], Some(Event(E::BinaryOutputStatus(Some(EventBinaryOutputStatusVariation::Group11Var1))))),
    ([11, 2], Some(Event(E::BinaryOutputStatus(Some(EventBinaryOutputStatusVariation::Group11Var2))))),
    // binary commands and their events are not read
    ([12, 1], None),
    ([13, 1], None),
    ([13, 2], None),
    // counter
    ([20, 0], Some(Static(S::Counter(None)))),
    ([20, 1], Some(Static(S::Counter(Some(StaticCounterVariation::Group20Var1))))),
    ([20, 2], Some(Static(S::Counter(Some(StaticCounterVariation::Group20Var2))))),
    ([20, 5], Some(Static(S::Counter(Some(StaticCounterVariation::Group20Var5))))),
    ([20, 6], Some(Static(S::Counter(Some(StaticCounterVariation::Group20Var6))))),
    ([22, 0], Some(Event(E::Counter(None)))),
    ([22, 1], Some(Event(E::Counter(Some(EventCounterVariation::Group22Var1))))),
    ([22, 2], Some(Event(E::Counter(Some(EventCounterVariation::Group22Var2))))),
    ([22, 5], Some(Event(E::Counter(Some(EventCounterVariation::Group22Var5))))),
    ([22, 6], Some(Event(E::Counter(Some(EventCounterVariation::Group22Var6))))),
    // frozen counter
    ([21, 0], Some(Static(S::FrozenCounter(None)))),
    ([21, 1], Some(Static(S::FrozenCounter(Some(StaticFrozenCounterVariation::Group21Var1))))),
    ([21, 2], Some(Static(S::FrozenCounter(Some(StaticFrozenCounterVariation::Group21Var2))))),
    ([21, 5], Some(Static(S::FrozenCounter(Some(StaticFrozenCounterVariation::Group21Var5))))),
    ([21, 6], Some(Static(S::FrozenCounter(Some(StaticFrozenCounterVariation::Group21Var6))))),
    ([21, 9], Some(Static(S::FrozenCounter(Some(StaticFrozenCounterVariation::Group21Var9))))),
    ([21, 10], Some(Static(S::FrozenCounter(Some(StaticFrozenCounterVariation::Group21Var10))))),
    ([23, 0], Some(Event(E::FrozenCounter(None)))),
    ([23, 1], Some(Event(E::FrozenCounter(Some(EventFrozenCounterVariation::Group23Var1))))),
    ([23, 2], Some(Event(E::FrozenCounter(Some(EventFrozenCounterVariation::Group23Var2))))),
    ([23, 5], Some(Event(E::FrozenCounter(Some(EventFrozenCounterVariation::Group23Var5))))),
    ([23, 6], Some(Event(E::FrozenCounter(Some(EventFrozenCounterVariation::Group23Var6))))),
    // analog input
    ([30, 0], Some(Static(S::Analog(None)))),
    ([30, 1], Some(Static(S::Analog(Some(StaticAnalogInputVariation::Group30Var1))))),
    ([30, 2], Some(Static(S::Analog(Some(StaticAnalogInputVariation::Group30Var2))))),
    ([30, 3], Some(Static(S::Analog(Some(StaticAnalogInputVariation::Group30Var3))))),
    ([30, 4], Some(Static(S::Analog(Some(StaticAnalogInputVariation::Group30Var4))))),
    ([30, 5], Some(Static(S::Analog(Some(StaticAnalogInputVariation::Group30Var5))))),
    ([30, 6], Some(Static(S::Analog(Some(StaticAnalogInputVariation::Group30Var6))))),
    ([32, 0], Some(Event(E::Analog(None)))),
    ([32, 1], Some(Event(E::Analog(Some(EventAnalogInputVariation::Group32Var1))))),
    ([32, 2], Some(Event(E::Analog(Some(EventAnalogInputVariation::Group32Var2))))),
    ([32, 3], Some(Event(E::Analog(Some(EventAnalogInputVariation::Group32Var3))))),
    ([32, 4], Some(Event(E::Analog(Some(EventAnalogInputVariation::Group32Var4))))),
    ([32, 5], Some(Event(E::Analog(Some(EventAnalogInputVariation::Group32Var5))))),
    ([32, 6], Some(Event(E::Analog(Some(EventAnalogInputVariation::Group32Var6))))),
    ([32, 7], Some(Event(E::Analog(Some(EventAnalogInputVariation::Group32Var7))))),
    ([32, 8], Some(Event(E::Analog(Some(EventAnalogInputVariation::Group32Var8))))),
    // frozen analog input
    ([31, 0], Some(Static(S::FrozenAnalog(None)))),
    ([31, 1], Some(Static(S::FrozenAnalog(Some(StaticFrozenAnalogInputVariation::Group31Var1))))),
    ([31, 2], Some(Static(S::FrozenAnalog(Some(StaticFrozenAnalogInputVariation::Group31Var2))))),
    ([31, 3], Some(Static(S::FrozenAnalog(Some(StaticFrozenAnalogInputVariation::Group31Var3))))),
    ([31, 4], Some(Static(S::FrozenAnalog(Some(StaticFrozenAnalogInputVariation::Group31Var4))))),
    ([31, 5], Some(Static(S::FrozenAnalog(Some(StaticFrozenAnalogInputVariation::Group31Var5))))),
    ([31, 6], Some(Static(S::FrozenAnalog(Some(StaticFrozenAnalogInputVariation::Group31Var6))))),
    ([31, 7], Some(Static(S::FrozenAnalog(Some(StaticFrozenAnalogInputVariation::Group31Var7))))),
    ([31, 8], Some(Static(S::FrozenAnalog(Some(StaticFrozenAnalogInputVariation::Group31Var8))))),
    ([33, 0], Some(Event(E::FrozenAnalog(None)))),
    ([33, 1], Some(Event(E::FrozenAnalog(Some(EventFrozenAnalogInputVariation::Group33Var1))))),
    ([33, 2], Some(Event(E::FrozenAnalog(Some(EventFrozenAnalogInputVariation::Group33Var2))))),
    ([33, 3], Some(Event(E::FrozenAnalog(Some(EventFrozenAnalogInputVariation::Group33Var3))))),
    ([33, 4], Some(Event(E::FrozenAnalog(Some(EventFrozenAnalogInputVariation::Group33Var4))))),
    ([33, 5], Some(Event(E::FrozenAnalog(Some(EventFrozenAnalogInputVariation::Group33Var5))))),
    ([33, 6], Some(Event(E::FrozenAnalog(Some(EventFrozenAnalogInputVariation::Group33Var6))))),
    ([33, 7], Some(Event(E::FrozenAnalog(Some(EventFrozenAnalogInputVariation::Group33Var7))))),
    ([33, 8], Some(Event(E::FrozenAnalog(Some(EventFrozenAnalogInputVariation::Group33Var8))))),
    // analog input reporting deadband, which has no events
    ([34, 0], Some(Static(S::AnalogInputDeadBand(None)))),
    ([34, 1], Some(Static(S::AnalogInputDeadBand(Some(AnalogInputDeadBandVariation::Group34Var1))))),
    ([34, 2], Some(Static(S::AnalogInputDeadBand(Some(AnalogInputDeadBandVariation::Group34Var2))))),
    ([34, 3], Some(Static(S::AnalogInputDeadBand(Some(AnalogInputDeadBandVariation::Group34Var3))))),
    // analog output status
    ([40, 0], Some(Static(S::AnalogOutputStatus(None)))),
    ([40, 1], Some(Static(S::AnalogOutputStatus(Some(StaticAnalogOutputStatusVariation::Group40Var1))))),
    ([40, 2], Some(Static(S::AnalogOutputStatus(Some(StaticAnalogOutputStatusVariation::Group40Var2))))),
    ([40, 3], Some(Static(S::AnalogOutputStatus(Some(StaticAnalogOutputStatusVariation::Group40Var3))))),
    ([40, 4], Some(Static(S::AnalogOutputStatus(Some(StaticAnalogOutputStatusVariation::Group40Var4))))),
    ([42, 0], Some(Event(E::AnalogOutputStatus(None)))),
    ([42, 1], Some(Event(E::AnalogOutputStatus(Some(EventAnalogOutputStatusVariation::Group42Var1))))),
    ([42, 2], Some(Event(E::AnalogOutputStatus(Some(EventAnalogOutputStatusVariation::Group42Var2))))),
    ([42, 3], Some(Event(E::AnalogOutputStatus(Some(EventAnalogOutputStatusVariation::Group42Var3))))),
    ([42, 4], Some(Event(E::AnalogOutputStatus(Some(EventAnalogOutputStatusVariation::Group42Var4))))),
    ([42, 5], Some(Event(E::AnalogOutputStatus(Some(EventAnalogOutputStatusVariation::Group42Var5))))),
    ([42, 6], Some(Event(E::AnalogOutputStatus(Some(EventAnalogOutputStatusVariation::Group42Var6))))),
    ([42, 7], Some(Event(E::AnalogOutputStatus(Some(EventAnalogOutputStatusVariation::Group42Var7))))),
    ([42, 8], Some(Event(E::AnalogOutputStatus(Some(EventAnalogOutputStatusVariation::Group42Var8))))),
    // analog commands and their events are not read
    ([41, 1], None),
    ([41, 2], None),
    ([41, 3], None),
    ([41, 4], None),
    ([43, 1], None),
    ([43, 2], None),
    ([43, 3], None),
    ([43, 4], None),
    ([43, 5], None),
    ([43, 6], None),
    ([43, 7], None),
    ([43, 8], None),
    // time and delay objects are not read
    ([50, 1], None),
    ([50, 2], None),
    ([50, 3], None),
    ([50, 4], None),
    ([51, 1], None),
    ([51, 2], None),
    ([52, 1], None),
    ([52, 2], None),
    // classes
    ([60, 1], Some(Class0)),
    ([60, 2], Some(Event(E::Class(EventClass::Class1)))),
    ([60, 3], Some(Event(E::Class(EventClass::Class2)))),
    ([60, 4], Some(Event(E::Class(EventClass::Class3)))),
    // file transfer, internal indications and unsigned integers are not read
    ([70, 2], None),
    ([70, 3], None),
    ([70, 4], None),
    ([70, 5], None),
    ([70, 6], None),
    ([70, 7], None),
    ([70, 8], None),
    ([80, 1], None),
    ([102, 0], None),
    ([102, 1], None),
];

/// Samples of the groups whose variations are sized or numbered by the request
#[rustfmt::skip]
const SAMPLES: &[([u8; 2], Option<ReadTarget>)] = &[
    // device attributes: one attribute, all of them, the list of attribute variations
    ([0, 1], Some(Attr(1))),
    ([0, 254], Some(Attr(254))),
    ([0, 255], Some(Attr(255))),
    // octet strings are read with v0 only
    ([110, 0], Some(Static(S::OctetString))),
    ([110, 1], None),
    ([110, 255], None),
    ([111, 0], Some(Event(E::OctetString))),
    ([111, 1], None),
    ([111, 255], None),
];

fn check(rows: &[([u8; 2], Option<ReadTarget>)]) {
    for &([group, var], expected) in rows {
        let variation = Variation::lookup(group, var).expect("a variation of the table");
        assert_eq!(ReadTarget::get(variation), expected, "g{group}v{var}");
    }
}

#[test]
fn every_variation_has_the_read_target_of_its_table_row() {
    check(ROWS);
}

#[test]
fn sampled_variations_of_the_dynamic_groups_have_their_read_target() {
    check(SAMPLES);
}

#[test]
fn table_covers_every_variation_outside_the_dynamic_groups() {
    for variation in Variation::all() {
        let (group, var) = variation.to_group_and_var();
        if DYNAMIC_GROUPS.contains(&group) {
            continue;
        }
        assert!(
            ROWS.iter().any(|(gv, _)| *gv == [group, var]),
            "g{group}v{var}"
        );
    }
}

#[test]
fn table_has_one_row_per_variation() {
    for (i, (gv, _)) in ROWS.iter().enumerate() {
        assert!(
            ROWS.iter().skip(i).skip(1).all(|(other, _)| other != gv),
            "{gv:?}"
        );
    }
}
