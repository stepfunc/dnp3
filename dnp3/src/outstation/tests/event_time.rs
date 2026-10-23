// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::measurement::*;
use crate::outstation::database::*;
use crate::outstation::tests::harness::*;
use crate::util::test_support::watchdog;

/// READ g2v2, all objects
const READ_G2V2: &[u8] = &[0xC0, 0x01, 0x02, 0x02, 0x06];
/// READ g11v2, all objects
const READ_G11V2: &[u8] = &[0xC0, 0x01, 0x0B, 0x02, 0x06];
/// READ g22v5, all objects
const READ_G22V5: &[u8] = &[0xC0, 0x01, 0x16, 0x05, 0x06];
/// READ g23v6, all objects
const READ_G23V6: &[u8] = &[0xC0, 0x01, 0x17, 0x06, 0x06];
/// READ g32v3, all objects
const READ_G32V3: &[u8] = &[0xC0, 0x01, 0x20, 0x03, 0x06];
/// READ g42v7, all objects
const READ_G42V7: &[u8] = &[0xC0, 0x01, 0x2A, 0x07, 0x06];

fn add_binary_inputs(db: &mut Database, e_var: EventBinaryInputVariation) {
    for index in [0, 1] {
        db.add(
            index,
            Some(EventClass::Class1),
            BinaryInputConfig::new(StaticBinaryInputVariation::Group1Var2, e_var),
        );
    }
}

fn update_binary(db: &mut Database, index: u16, value: bool, time: Time) {
    db.update(
        index,
        &BinaryInput::new(value, Flags::ONLINE, time),
        UpdateOptions::default(),
    );
}

async fn read_binary_events(
    e_var: EventBinaryInputVariation,
    events: &[(u16, bool, Time)],
    request: &[u8],
    response: &[u8],
) {
    let mut harness = new_harness(get_default_config());
    harness.handle.database.transaction(|db| {
        add_binary_inputs(db, e_var);
        for &(index, value, time) in events {
            update_binary(db, index, value, time);
        }
    });
    harness.test_request_response(request, response).await;
}

#[tokio::test]
async fn reports_synchronized_events_in_the_default_g2v2() {
    read_binary_events(
        EventBinaryInputVariation::Group2Var2,
        &[
            (0, true, Time::synchronized(1)),
            (1, true, Time::synchronized(2)),
        ],
        &read_class_1(0),
        &[
            // FIR, FIN, CON, seq 0, IIN1 restart
            0xE0, 0x81, 0x80, 0x00, // g2v2 (count of 2)
            0x02, 0x02, 0x28, 0x02, 0x00, // index 0 at 1
            0x00, 0x00, 0x81, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, // index 1 at 2
            0x01, 0x00, 0x81, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00,
        ],
    )
    .await;
}

#[tokio::test]
async fn reports_unsynchronized_events_for_the_default_g2v2_in_g2v3_after_g51v2() {
    read_binary_events(
        EventBinaryInputVariation::Group2Var2,
        &[
            (0, true, Time::unsynchronized(1)),
            (1, true, Time::unsynchronized(3)),
        ],
        &read_class_1(0),
        &[
            // FIR, FIN, CON, seq 0, IIN1 restart
            0xE0, 0x81, 0x80, 0x00, // g51v2 at 1
            0x33, 0x02, 0x07, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, // g2v3 (count of 2)
            0x02, 0x03, 0x28, 0x02, 0x00, // index 0 at +0, index 1 at +2
            0x00, 0x00, 0x81, 0x00, 0x00, 0x01, 0x00, 0x81, 0x02, 0x00,
        ],
    )
    .await;
}

#[tokio::test]
async fn reports_mixed_events_for_the_default_g2v2_in_order_of_occurrence() {
    read_binary_events(
        EventBinaryInputVariation::Group2Var2,
        &[
            (0, true, Time::synchronized(1)),
            (1, true, Time::unsynchronized(2)),
            (0, false, Time::synchronized(3)),
        ],
        &read_class_1(0),
        &[
            // FIR, FIN, CON, seq 0, IIN1 restart
            0xE0, 0x81, 0x80, 0x00, // g2v2 (count of 1), index 0 at 1
            0x02, 0x02, 0x28, 0x01, 0x00, 0x00, 0x00, 0x81, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
            // g51v2 at 2
            0x33, 0x02, 0x07, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00,
            // g2v3 (count of 1), index 1 at +0
            0x02, 0x03, 0x28, 0x01, 0x00, 0x01, 0x00, 0x81, 0x00, 0x00,
            // g2v2 (count of 1), index 0 at 3
            0x02, 0x02, 0x28, 0x01, 0x00, 0x00, 0x00, 0x01, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00,
        ],
    )
    .await;
}

#[tokio::test]
async fn reports_unsynchronized_events_for_a_read_of_g2v2_in_g2v3_after_g51v2() {
    read_binary_events(
        EventBinaryInputVariation::Group2Var1,
        &[
            (0, true, Time::unsynchronized(1)),
            (1, true, Time::synchronized(2)),
        ],
        READ_G2V2,
        &[
            // FIR, FIN, CON, seq 0, IIN1 restart
            0xE0, 0x81, 0x80, 0x00, // g51v2 at 1
            0x33, 0x02, 0x07, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
            // g2v3 (count of 1), index 0 at +0
            0x02, 0x03, 0x28, 0x01, 0x00, 0x00, 0x00, 0x81, 0x00, 0x00,
            // g2v2 (count of 1), index 1 at 2
            0x02, 0x02, 0x28, 0x01, 0x00, 0x01, 0x00, 0x81, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00,
        ],
    )
    .await;
}

#[tokio::test]
async fn reports_mixed_events_for_the_default_g4v2_in_order_of_occurrence() {
    let mut harness = new_harness(get_default_config());
    harness.handle.database.transaction(|db| {
        db.add(
            0,
            Some(EventClass::Class1),
            DoubleBitBinaryInputConfig::new(
                StaticDoubleBitBinaryInputVariation::Group3Var2,
                EventDoubleBitBinaryInputVariation::Group4Var2,
            ),
        );
        for (value, time) in [
            (DoubleBit::DeterminedOn, Time::unsynchronized(1)),
            (DoubleBit::DeterminedOff, Time::synchronized(2)),
        ] {
            db.update(
                0,
                &DoubleBitBinaryInput::new(value, Flags::ONLINE, time),
                UpdateOptions::default(),
            );
        }
    });
    harness
        .test_request_response(
            &read_class_1(0),
            &[
                // FIR, FIN, CON, seq 0, IIN1 restart
                0xE0, 0x81, 0x80, 0x00, // g51v2 at 1
                0x33, 0x02, 0x07, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
                // g4v3 (count of 1), index 0 determined on at +0
                0x04, 0x03, 0x28, 0x01, 0x00, 0x00, 0x00, 0x81, 0x00, 0x00,
                // g4v2 (count of 1), index 0 determined off at 2
                0x04, 0x02, 0x28, 0x01, 0x00, 0x00, 0x00, 0x41, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00,
            ],
        )
        .await;
}

async fn read_events<F>(configure: F, request: &[u8], response: &[u8])
where
    F: FnMut(&mut Database),
{
    watchdog(async {
        let mut harness = new_harness(get_default_config());
        harness.handle.database.transaction(configure);
        harness.test_request_response(request, response).await;
    })
    .await;
}

fn add_binary_output_status(db: &mut Database, e_var: EventBinaryOutputStatusVariation) {
    db.add(
        0,
        Some(EventClass::Class1),
        BinaryOutputStatusConfig::new(StaticBinaryOutputStatusVariation::Group10Var2, e_var),
    );
    db.update(
        0,
        &BinaryOutputStatus {
            value: true,
            flags: Flags::ONLINE,
            time: None,
        },
        UpdateOptions::default(),
    );
}

fn add_counter(db: &mut Database, e_var: EventCounterVariation) {
    db.add(
        0,
        Some(EventClass::Class1),
        CounterConfig::new(StaticCounterVariation::Group20Var1, e_var, 0),
    );
    db.update(
        0,
        &Counter {
            value: 7,
            flags: Flags::ONLINE,
            time: None,
        },
        UpdateOptions::default(),
    );
}

fn add_frozen_counter(db: &mut Database, e_var: EventFrozenCounterVariation) {
    db.add(
        0,
        Some(EventClass::Class1),
        FrozenCounterConfig::new(StaticFrozenCounterVariation::Group21Var1, e_var, 0),
    );
    db.update(
        0,
        &FrozenCounter {
            value: 7,
            flags: Flags::ONLINE,
            time: None,
        },
        UpdateOptions::default(),
    );
}

fn add_analog_input(db: &mut Database, e_var: EventAnalogInputVariation, times: &[Option<Time>]) {
    db.add(
        0,
        Some(EventClass::Class1),
        AnalogInputConfig::new(StaticAnalogInputVariation::Group30Var1, e_var, 0.0),
    );
    for (value, &time) in (5..).zip(times) {
        db.update(
            0,
            &AnalogInput {
                value: f64::from(value),
                flags: Flags::ONLINE,
                time,
            },
            UpdateOptions::default(),
        );
    }
}

fn add_analog_output_status(db: &mut Database, e_var: EventAnalogOutputStatusVariation) {
    db.add(
        0,
        Some(EventClass::Class1),
        AnalogOutputStatusConfig::new(StaticAnalogOutputStatusVariation::Group40Var1, e_var, 0.0),
    );
    db.update(
        0,
        &AnalogOutputStatus {
            value: 2.5,
            flags: Flags::ONLINE,
            time: None,
        },
        UpdateOptions::default(),
    );
}

#[tokio::test]
async fn reports_events_without_time_for_the_default_g11v2_in_g11v1() {
    read_events(
        |db| add_binary_output_status(db, EventBinaryOutputStatusVariation::Group11Var2),
        &read_class_1(0),
        &[
            // FIR, FIN, CON, seq 0, IIN1 restart
            0xE0, 0x81, 0x80, 0x00, // g11v1 (count of 1), index 0 on
            0x0B, 0x01, 0x28, 0x01, 0x00, 0x00, 0x00, 0x81,
        ],
    )
    .await;
}

#[tokio::test]
async fn reports_events_without_time_for_a_read_of_g11v2_in_g11v1() {
    read_events(
        |db| add_binary_output_status(db, EventBinaryOutputStatusVariation::Group11Var1),
        READ_G11V2,
        &[
            // FIR, FIN, CON, seq 0, IIN1 restart
            0xE0, 0x81, 0x80, 0x00, // g11v1 (count of 1), index 0 on
            0x0B, 0x01, 0x28, 0x01, 0x00, 0x00, 0x00, 0x81,
        ],
    )
    .await;
}

#[tokio::test]
async fn reports_events_without_time_for_the_default_g22v5_in_g22v1() {
    read_events(
        |db| add_counter(db, EventCounterVariation::Group22Var5),
        &read_class_1(0),
        &[
            // FIR, FIN, CON, seq 0, IIN1 restart
            0xE0, 0x81, 0x80, 0x00, // g22v1 (count of 1), index 0 value 7
            0x16, 0x01, 0x28, 0x01, 0x00, 0x00, 0x00, 0x01, 0x07, 0x00, 0x00, 0x00,
        ],
    )
    .await;
}

#[tokio::test]
async fn reports_events_without_time_for_a_read_of_g22v5_in_g22v1() {
    read_events(
        |db| add_counter(db, EventCounterVariation::Group22Var1),
        READ_G22V5,
        &[
            // FIR, FIN, CON, seq 0, IIN1 restart
            0xE0, 0x81, 0x80, 0x00, // g22v1 (count of 1), index 0 value 7
            0x16, 0x01, 0x28, 0x01, 0x00, 0x00, 0x00, 0x01, 0x07, 0x00, 0x00, 0x00,
        ],
    )
    .await;
}

#[tokio::test]
async fn reports_events_without_time_for_a_read_of_g23v6_in_g23v2() {
    read_events(
        |db| add_frozen_counter(db, EventFrozenCounterVariation::Group23Var1),
        READ_G23V6,
        &[
            // FIR, FIN, CON, seq 0, IIN1 restart
            0xE0, 0x81, 0x80, 0x00, // g23v2 (count of 1), index 0 value 7
            0x17, 0x02, 0x28, 0x01, 0x00, 0x00, 0x00, 0x01, 0x07, 0x00,
        ],
    )
    .await;
}

#[tokio::test]
async fn reports_events_without_time_for_the_default_g32v3_in_g32v1() {
    read_events(
        |db| add_analog_input(db, EventAnalogInputVariation::Group32Var3, &[None]),
        &read_class_1(0),
        &[
            // FIR, FIN, CON, seq 0, IIN1 restart
            0xE0, 0x81, 0x80, 0x00, // g32v1 (count of 1), index 0 value 5
            0x20, 0x01, 0x28, 0x01, 0x00, 0x00, 0x00, 0x01, 0x05, 0x00, 0x00, 0x00,
        ],
    )
    .await;
}

#[tokio::test]
async fn reports_events_without_time_for_a_read_of_g32v3_in_g32v1() {
    read_events(
        |db| add_analog_input(db, EventAnalogInputVariation::Group32Var1, &[None]),
        READ_G32V3,
        &[
            // FIR, FIN, CON, seq 0, IIN1 restart
            0xE0, 0x81, 0x80, 0x00, // g32v1 (count of 1), index 0 value 5
            0x20, 0x01, 0x28, 0x01, 0x00, 0x00, 0x00, 0x01, 0x05, 0x00, 0x00, 0x00,
        ],
    )
    .await;
}

#[tokio::test]
async fn reports_events_without_time_for_the_default_g32v8_in_g32v6() {
    read_events(
        |db| add_analog_input(db, EventAnalogInputVariation::Group32Var8, &[None]),
        &read_class_1(0),
        &[
            // FIR, FIN, CON, seq 0, IIN1 restart
            0xE0, 0x81, 0x80, 0x00, // g32v6 (count of 1), index 0 value 5.0
            0x20, 0x06, 0x28, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x14, 0x40,
        ],
    )
    .await;
}

#[tokio::test]
async fn reports_events_without_time_for_the_default_g42v7_in_g42v5() {
    read_events(
        |db| add_analog_output_status(db, EventAnalogOutputStatusVariation::Group42Var7),
        &read_class_1(0),
        &[
            // FIR, FIN, CON, seq 0, IIN1 restart
            0xE0, 0x81, 0x80, 0x00, // g42v5 (count of 1), index 0 value 2.5
            0x2A, 0x05, 0x28, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x20, 0x40,
        ],
    )
    .await;
}

#[tokio::test]
async fn reports_events_without_time_for_a_read_of_g42v7_in_g42v5() {
    read_events(
        |db| add_analog_output_status(db, EventAnalogOutputStatusVariation::Group42Var1),
        READ_G42V7,
        &[
            // FIR, FIN, CON, seq 0, IIN1 restart
            0xE0, 0x81, 0x80, 0x00, // g42v5 (count of 1), index 0 value 2.5
            0x2A, 0x05, 0x28, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x20, 0x40,
        ],
    )
    .await;
}

#[tokio::test]
async fn reports_mixed_events_for_the_default_g32v3_in_order_of_occurrence() {
    read_events(
        |db| {
            add_analog_input(
                db,
                EventAnalogInputVariation::Group32Var3,
                &[Some(Time::synchronized(1)), None],
            )
        },
        &read_class_1(0),
        &[
            // FIR, FIN, CON, seq 0, IIN1 restart
            0xE0, 0x81, 0x80, 0x00, // g32v3 (count of 1), index 0 value 5 at 1
            0x20, 0x03, 0x28, 0x01, 0x00, 0x00, 0x00, 0x01, 0x05, 0x00, 0x00, 0x00, 0x01, 0x00,
            0x00, 0x00, 0x00, 0x00, // g32v1 (count of 1), index 0 value 6
            0x20, 0x01, 0x28, 0x01, 0x00, 0x00, 0x00, 0x01, 0x06, 0x00, 0x00, 0x00,
        ],
    )
    .await;
}

#[tokio::test]
async fn reports_unsynchronized_events_for_the_default_g32v3_in_g32v3_with_their_time() {
    read_events(
        |db| {
            add_analog_input(
                db,
                EventAnalogInputVariation::Group32Var3,
                &[Some(Time::unsynchronized(1))],
            )
        },
        &read_class_1(0),
        &[
            // FIR, FIN, CON, seq 0, IIN1 restart
            0xE0, 0x81, 0x80, 0x00, // g32v3 (count of 1), index 0 value 5 at 1
            0x20, 0x03, 0x28, 0x01, 0x00, 0x00, 0x00, 0x01, 0x05, 0x00, 0x00, 0x00, 0x01, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ],
    )
    .await;
}

#[tokio::test]
async fn reports_mixed_events_for_a_read_of_g32v3_in_order_of_occurrence() {
    read_events(
        |db| {
            add_analog_input(
                db,
                EventAnalogInputVariation::Group32Var1,
                &[None, Some(Time::synchronized(2)), None],
            )
        },
        READ_G32V3,
        &[
            // FIR, FIN, CON, seq 0, IIN1 restart
            0xE0, 0x81, 0x80, 0x00, // g32v1 (count of 1), index 0 value 5
            0x20, 0x01, 0x28, 0x01, 0x00, 0x00, 0x00, 0x01, 0x05, 0x00, 0x00, 0x00,
            // g32v3 (count of 1), index 0 value 6 at 2
            0x20, 0x03, 0x28, 0x01, 0x00, 0x00, 0x00, 0x01, 0x06, 0x00, 0x00, 0x00, 0x02, 0x00,
            0x00, 0x00, 0x00, 0x00, // g32v1 (count of 1), index 0 value 7
            0x20, 0x01, 0x28, 0x01, 0x00, 0x00, 0x00, 0x01, 0x07, 0x00, 0x00, 0x00,
        ],
    )
    .await;
}
