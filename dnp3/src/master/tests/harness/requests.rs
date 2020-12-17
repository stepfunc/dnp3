use crate::app::format::write::{start_request, start_response};
use crate::app::header::{Control, ResponseFunction, IIN, IIN1, IIN2};
use crate::app::sequence::Sequence;
use crate::prelude::master::*;
use crate::tokio::test::*;
use crate::util::cursor::WriteCursor;
use crate::util::task::RunError;
use std::future::Future;

pub(crate) fn startup_procedure<F: Future<Output = RunError>>(
    harness: &mut super::TestHarness<F>,
    seq: &mut Sequence,
) {
    // Disable unsolicited
    disable_unsol_request(&mut harness.io, *seq);
    empty_response(&mut harness.io, seq.increment());
    harness.assert_io();

    // Integrity poll
    integrity_poll_request(&mut harness.io, *seq);
    empty_response(&mut harness.io, seq.increment());
    harness.assert_io();

    // Enable unsolicited
    enable_unsol_request(&mut harness.io, *seq);
    empty_response(&mut harness.io, seq.increment());
    harness.assert_io();
}

pub(crate) fn disable_unsol_request(io: &mut io::Handle, seq: Sequence) {
    // DISABLE_UNSOLICITED request
    let mut buffer = [0; 20];
    let mut cursor = WriteCursor::new(&mut buffer);
    let mut request = start_request(
        Control::request(seq),
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

    io.write(cursor.written());
}

pub(crate) fn integrity_poll_request(io: &mut io::Handle, seq: Sequence) {
    // Integrity poll
    let mut buffer = [0; 20];
    let mut cursor = WriteCursor::new(&mut buffer);
    let mut request =
        start_request(Control::request(seq), FunctionCode::Read, &mut cursor).unwrap();

    request.write_class1230().unwrap();

    io.write(cursor.written());
}

pub(crate) fn enable_unsol_request(io: &mut io::Handle, seq: Sequence) {
    // ENABLE_UNSOLICITED request
    let mut buffer = [0; 20];
    let mut cursor = WriteCursor::new(&mut buffer);
    let mut request = start_request(
        Control::request(seq),
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

    io.write(cursor.written());
}

pub(crate) fn clear_restart_iin(io: &mut io::Handle, seq: Sequence) {
    // ENABLE_UNSOLICITED request
    let mut buffer = [0; 20];
    let mut cursor = WriteCursor::new(&mut buffer);
    let mut request =
        start_request(Control::request(seq), FunctionCode::Write, &mut cursor).unwrap();

    request.write_clear_restart().unwrap();

    io.write(cursor.written());
}

pub(crate) fn empty_response(io: &mut io::Handle, seq: Sequence) {
    empty_response_custom_iin(io, seq, IIN::default());
}

pub(crate) fn empty_response_custom_iin(io: &mut io::Handle, seq: Sequence, iin: IIN) {
    let mut buffer = [0; 4];
    let mut cursor = WriteCursor::new(&mut buffer);
    start_response(
        Control::response(seq, true, true, false),
        ResponseFunction::Response,
        iin,
        &mut cursor,
    )
    .unwrap();

    io.read(cursor.written());
}

// Unsolicited stuff

pub(crate) fn unsol_null(io: &mut io::Handle, seq: Sequence, restart_iin: bool) {
    let iin = if restart_iin {
        IIN::new(IIN1::new(0x80), IIN2::new(0x00))
    } else {
        IIN::default()
    };

    unsol_null_custom_iin(io, seq, iin);
}

pub(crate) fn unsol_null_custom_iin(io: &mut io::Handle, seq: Sequence, iin: IIN) {
    let mut buffer = [0; 4];
    let mut cursor = WriteCursor::new(&mut buffer);
    start_response(
        Control::unsolicited_response(seq),
        ResponseFunction::UnsolicitedResponse,
        iin,
        &mut cursor,
    )
    .unwrap();

    io.read(cursor.written());
}

pub(crate) fn unsol_confirm(io: &mut io::Handle, seq: Sequence) {
    let mut buffer = [0; 2];
    let mut cursor = WriteCursor::new(&mut buffer);
    start_request(
        Control::unsolicited(seq),
        FunctionCode::Confirm,
        &mut cursor,
    )
    .unwrap();

    io.write(cursor.written());
}

pub(crate) fn unsol_with_data(io: &mut io::Handle, seq: Sequence, data: i16, restart_iin: bool) {
    let iin = if restart_iin {
        IIN::new(IIN1::new(0x80), IIN2::new(0x00))
    } else {
        IIN::default()
    };

    let mut buffer = [0; 20];
    let mut cursor = WriteCursor::new(&mut buffer);
    let mut response = start_response(
        Control::unsolicited_response(seq),
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

    io.read(cursor.written());
}
