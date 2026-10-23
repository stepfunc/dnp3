// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::format::write::{start_request, start_response};
use crate::app::variations::{Group32Var2, Variation};
use crate::app::Sequence;
use crate::app::{ControlField, FunctionCode, Iin, Iin1, Iin2, ResponseFunction};
use crate::master::Classes;

use scursor::WriteCursor;
use sfio_tokio_mock_io::Event;

pub(crate) async fn startup_procedure(harness: &mut super::TestHarness, seq: &mut Sequence) {
    // Disable unsolicited
    assert_eq!(
        harness.io.next_event().await,
        Event::Write(disable_unsol_request(*seq))
    );
    harness
        .process_response(empty_response(seq.increment()))
        .await;

    // Integrity poll
    assert_eq!(
        harness.io.next_event().await,
        Event::Write(integrity_poll_request(*seq))
    );
    harness
        .process_response(empty_response(seq.increment()))
        .await;

    // Enable unsolicited
    assert_eq!(
        harness.io.next_event().await,
        Event::Write(enable_unsol_request(*seq))
    );
    harness
        .process_response(empty_response(seq.increment()))
        .await;
}

pub(crate) fn disable_unsol_request(seq: Sequence) -> Vec<u8> {
    // DISABLE_UNSOLICITED request
    let mut buffer = [0; 20];
    let mut cursor = WriteCursor::new(&mut buffer);
    let mut request = start_request(
        ControlField::request(seq),
        FunctionCode::DisableUnsolicited,
        &mut cursor,
    )
    .unwrap();

    request
        .write_all_objects_header(Variation::Group60Var2)
        .unwrap();
    request
        .write_all_objects_header(Variation::Group60Var3)
        .unwrap();
    request
        .write_all_objects_header(Variation::Group60Var4)
        .unwrap();

    cursor.written().to_vec()
}

pub(crate) fn class_scan_request(classes: Classes, seq: Sequence) -> Vec<u8> {
    // Integrity poll
    let mut buffer = [0; 20];
    let mut cursor = WriteCursor::new(&mut buffer);
    let mut request =
        start_request(ControlField::request(seq), FunctionCode::Read, &mut cursor).unwrap();

    if classes.class0 {
        request
            .write_all_objects_header(Variation::Group60Var1)
            .unwrap();
    }

    if classes.events.class1 {
        request
            .write_all_objects_header(Variation::Group60Var2)
            .unwrap();
    }

    if classes.events.class2 {
        request
            .write_all_objects_header(Variation::Group60Var3)
            .unwrap();
    }

    if classes.events.class3 {
        request
            .write_all_objects_header(Variation::Group60Var4)
            .unwrap();
    }

    cursor.written().to_vec()
}

pub(crate) fn integrity_poll_request(seq: Sequence) -> Vec<u8> {
    // Integrity poll
    let mut buffer = [0; 20];
    let mut cursor = WriteCursor::new(&mut buffer);
    let mut request =
        start_request(ControlField::request(seq), FunctionCode::Read, &mut cursor).unwrap();

    request.write_class1230().unwrap();

    cursor.written().to_vec()
}

pub(crate) fn enable_unsol_request(seq: Sequence) -> Vec<u8> {
    // ENABLE_UNSOLICITED request
    let mut buffer = [0; 20];
    let mut cursor = WriteCursor::new(&mut buffer);
    let mut request = start_request(
        ControlField::request(seq),
        FunctionCode::EnableUnsolicited,
        &mut cursor,
    )
    .unwrap();

    request
        .write_all_objects_header(Variation::Group60Var2)
        .unwrap();
    request
        .write_all_objects_header(Variation::Group60Var3)
        .unwrap();
    request
        .write_all_objects_header(Variation::Group60Var4)
        .unwrap();

    cursor.written().to_vec()
}

pub(crate) fn clear_restart_iin(seq: Sequence) -> Vec<u8> {
    // ENABLE_UNSOLICITED request
    let mut buffer = [0; 20];
    let mut cursor = WriteCursor::new(&mut buffer);
    let mut request =
        start_request(ControlField::request(seq), FunctionCode::Write, &mut cursor).unwrap();

    request.write_clear_restart().unwrap();

    cursor.written().to_vec()
}

pub(crate) fn empty_response(seq: Sequence) -> Vec<u8> {
    empty_response_custom_iin(seq, Iin::default())
}

pub(crate) fn empty_response_custom_iin(seq: Sequence, iin: Iin) -> Vec<u8> {
    let mut buffer = [0; 4];
    let mut cursor = WriteCursor::new(&mut buffer);
    start_response(
        ControlField::response(seq, true, true, false),
        ResponseFunction::Response,
        iin,
        &mut cursor,
    )
    .unwrap();
    cursor.written().to_vec()
}

/// The same response with the CON bit set
pub(crate) fn with_con(mut response: Vec<u8>) -> Vec<u8> {
    let control = ControlField::from(response[0]);
    response[0] = ControlField {
        con: true,
        ..control
    }
    .to_u8();
    response
}

pub(crate) fn solicited_confirm(seq: Sequence) -> Vec<u8> {
    let mut buffer = [0; 2];
    let mut cursor = WriteCursor::new(&mut buffer);
    start_request(
        ControlField::request(seq),
        FunctionCode::Confirm,
        &mut cursor,
    )
    .unwrap();
    cursor.written().to_vec()
}

// Unsolicited stuff

pub(crate) fn unsol_null(seq: Sequence, restart_iin: bool) -> Vec<u8> {
    let iin = if restart_iin {
        Iin::new(Iin1::new(0x80), Iin2::new(0x00))
    } else {
        Iin::default()
    };

    unsol_null_custom_iin(seq, iin)
}

pub(crate) fn unsol_null_custom_iin(seq: Sequence, iin: Iin) -> Vec<u8> {
    let mut buffer = [0; 4];
    let mut cursor = WriteCursor::new(&mut buffer);
    start_response(
        ControlField::unsolicited_response(seq),
        ResponseFunction::UnsolicitedResponse,
        iin,
        &mut cursor,
    )
    .unwrap();

    cursor.written().to_vec()
}

pub(crate) fn unsol_confirm(seq: Sequence) -> Vec<u8> {
    let mut buffer = [0; 2];
    let mut cursor = WriteCursor::new(&mut buffer);
    start_request(
        ControlField::unsolicited(seq),
        FunctionCode::Confirm,
        &mut cursor,
    )
    .unwrap();
    cursor.written().to_vec()
}

pub(crate) fn unsol_with_data(seq: Sequence, data: i16, restart_iin: bool) -> Vec<u8> {
    let iin = if restart_iin {
        Iin::new(Iin1::new(0x80), Iin2::new(0x00))
    } else {
        Iin::default()
    };

    let mut buffer = [0; 20];
    let mut cursor = WriteCursor::new(&mut buffer);
    let mut response = start_response(
        ControlField::unsolicited_response(seq),
        ResponseFunction::UnsolicitedResponse,
        iin,
        &mut cursor,
    )
    .unwrap();

    response
        .write_prefixed_items(
            [(
                Group32Var2 {
                    value: data,
                    flags: 0x00,
                },
                0u8,
            )]
            .iter(),
        )
        .unwrap();

    cursor.written().to_vec()
}
