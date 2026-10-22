use crate::app::measurement::*;
use crate::outstation::database::*;
use crate::outstation::tests::harness::*;

/// READ g60v2, all objects
const READ_CLASS_1: &[u8] = &[0xC0, 0x01, 0x3C, 0x02, 0x06];
/// READ g2v2, all objects
const READ_G2V2: &[u8] = &[0xC0, 0x01, 0x02, 0x02, 0x06];

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
        READ_CLASS_1,
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
        READ_CLASS_1,
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
        READ_CLASS_1,
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
            READ_CLASS_1,
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
