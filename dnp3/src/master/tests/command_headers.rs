// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Request bytes from `CommandBuilder` and the comparison of each echo, for every command type
//! and index width

use scursor::WriteCursor;

use crate::app::control::*;
use crate::app::format::write::HeaderWriter;
use crate::app::parse::options::ParseOptions;
use crate::app::parse::parser::HeaderCollection;
use crate::app::FunctionCode;
use crate::master::{CommandBuilder, CommandHeaders, CommandResponseError, CommandSupport};

#[derive(Copy, Clone, Debug)]
enum Command {
    G12V1,
    G41V1,
    G41V2,
    G41V3,
    G41V4,
}

const COMMANDS: [Command; 5] = [
    Command::G12V1,
    Command::G41V1,
    Command::G41V2,
    Command::G41V3,
    Command::G41V4,
];

#[derive(Copy, Clone, Debug, PartialEq)]
enum Width {
    One,
    Two,
}

const WIDTHS: [Width; 2] = [Width::One, Width::Two];

impl Command {
    fn crob() -> Group12Var1 {
        Group12Var1 {
            code: ControlCode::from_op_type(OpType::LatchOn),
            count: 1,
            on_time: 100,
            off_time: 200,
            status: CommandStatus::Success,
        }
    }

    fn group_var(self) -> [u8; 2] {
        match self {
            Command::G12V1 => [12, 1],
            Command::G41V1 => [41, 1],
            Command::G41V2 => [41, 2],
            Command::G41V3 => [41, 3],
            Command::G41V4 => [41, 4],
        }
    }

    /// the encoded command, without its trailing status byte
    fn value(self) -> &'static [u8] {
        match self {
            Command::G12V1 => &[0x03, 0x01, 100, 0, 0, 0, 200, 0, 0, 0],
            Command::G41V1 => &[0xF9, 0xFF, 0xFF, 0xFF],
            Command::G41V2 => &[0x01, 0x02],
            Command::G41V3 => &[0x00, 0x00, 0x20, 0x40],
            Command::G41V4 => &[0, 0, 0, 0, 0, 0, 0x04, 0x40],
        }
    }

    fn add(self, builder: &mut CommandBuilder, width: Width, index: u16) {
        fn add<T>(builder: &mut CommandBuilder, command: T, width: Width, index: u16)
        where
            CommandBuilder: CommandSupport<T>,
        {
            match width {
                Width::One => builder.add_u8(command, u8::try_from(index).unwrap()),
                Width::Two => builder.add_u16(command, index),
            }
        }

        match self {
            Command::G12V1 => add(builder, Self::crob(), width, index),
            Command::G41V1 => add(builder, Group41Var1::new(-7), width, index),
            Command::G41V2 => add(builder, Group41Var2::new(513), width, index),
            Command::G41V3 => add(builder, Group41Var3::new(2.5), width, index),
            Command::G41V4 => add(builder, Group41Var4::new(2.5), width, index),
        }
    }

    fn single(self, width: Width, index: u16) -> CommandHeaders {
        fn single<T>(command: T, width: Width, index: u16) -> CommandHeaders
        where
            CommandBuilder: CommandSupport<T>,
        {
            match width {
                Width::One => {
                    CommandBuilder::single_header_u8(command, u8::try_from(index).unwrap())
                }
                Width::Two => CommandBuilder::single_header_u16(command, index),
            }
        }

        match self {
            Command::G12V1 => single(Self::crob(), width, index),
            Command::G41V1 => single(Group41Var1::new(-7), width, index),
            Command::G41V2 => single(Group41Var2::new(513), width, index),
            Command::G41V3 => single(Group41Var3::new(2.5), width, index),
            Command::G41V4 => single(Group41Var4::new(2.5), width, index),
        }
    }
}

impl Width {
    fn qualifier(self) -> u8 {
        match self {
            Width::One => 0x17,
            Width::Two => 0x28,
        }
    }

    fn write(self, value: u16, out: &mut Vec<u8>) {
        match self {
            Width::One => out.push(u8::try_from(value).unwrap()),
            Width::Two => out.extend_from_slice(&value.to_le_bytes()),
        }
    }

    fn index(self) -> u16 {
        match self {
            Width::One => 3,
            Width::Two => 259,
        }
    }
}

/// expected bytes of a header: (command, width, [(index, status)])
type Header<'a> = (Command, Width, &'a [(u16, u8)]);

fn encode(headers: &[Header]) -> Vec<u8> {
    let mut out = Vec::new();
    for &(command, width, items) in headers {
        out.extend_from_slice(&command.group_var());
        out.push(width.qualifier());
        width.write(u16::try_from(items.len()).unwrap(), &mut out);
        for &(index, status) in items {
            width.write(index, &mut out);
            out.extend_from_slice(command.value());
            out.push(status);
        }
    }
    out
}

fn write(headers: &CommandHeaders) -> Vec<u8> {
    let mut buffer = [0u8; 256];
    let mut cursor = WriteCursor::new(&mut buffer);
    headers.write(&mut HeaderWriter::new(&mut cursor)).unwrap();
    cursor.written().to_vec()
}

fn parse(echo: &[u8]) -> HeaderCollection<'_> {
    HeaderCollection::parse(ParseOptions::default(), FunctionCode::Response, echo).unwrap()
}

fn compare(sent: &CommandHeaders, echo: &[u8]) -> Result<(), CommandResponseError> {
    sent.compare(parse(echo))
}

#[test]
fn single_header_of_each_command_at_each_width() {
    for command in COMMANDS {
        for width in WIDTHS {
            let index = width.index();
            assert_eq!(
                write(&command.single(width, index)),
                encode(&[(command, width, &[(index, 0)])]),
                "{command:?} {width:?}"
            );
        }
    }
}

#[test]
fn builder_merges_consecutive_commands_of_one_type_and_width() {
    for command in COMMANDS {
        for width in WIDTHS {
            let mut builder = CommandBuilder::new();
            command.add(&mut builder, width, 3);
            command.add(&mut builder, width, 4);
            command.add(&mut builder, width, 200);
            assert_eq!(
                write(&builder.build()),
                encode(&[(command, width, &[(3, 0), (4, 0), (200, 0)])]),
                "{command:?} {width:?}"
            );
        }
    }
}

#[test]
fn builder_starts_a_new_header_when_the_type_or_width_changes() {
    let mut builder = CommandBuilder::new();
    let adds = [
        (Command::G12V1, Width::One, 1),
        (Command::G12V1, Width::Two, 2),
        (Command::G12V1, Width::Two, 3),
        (Command::G41V1, Width::Two, 4),
        (Command::G41V1, Width::One, 5),
        (Command::G41V2, Width::One, 6),
        (Command::G41V2, Width::One, 7),
        (Command::G41V3, Width::Two, 8),
        (Command::G41V4, Width::One, 9),
        (Command::G41V4, Width::Two, 10),
        (Command::G41V3, Width::Two, 11),
        (Command::G12V1, Width::One, 12),
    ];
    for (command, width, index) in adds {
        command.add(&mut builder, width, index);
    }

    assert_eq!(
        write(&builder.build()),
        encode(&[
            (Command::G12V1, Width::One, &[(1, 0)]),
            (Command::G12V1, Width::Two, &[(2, 0), (3, 0)]),
            (Command::G41V1, Width::Two, &[(4, 0)]),
            (Command::G41V1, Width::One, &[(5, 0)]),
            (Command::G41V2, Width::One, &[(6, 0), (7, 0)]),
            (Command::G41V3, Width::Two, &[(8, 0)]),
            (Command::G41V4, Width::One, &[(9, 0)]),
            (Command::G41V4, Width::Two, &[(10, 0)]),
            (Command::G41V3, Width::Two, &[(11, 0)]),
            (Command::G12V1, Width::One, &[(12, 0)]),
        ])
    );
}

#[test]
fn finish_header_splits_commands_of_one_type_and_ignores_an_empty_header() {
    for command in COMMANDS {
        for width in WIDTHS {
            let mut builder = CommandBuilder::new();
            builder.finish_header();
            command.add(&mut builder, width, 1);
            builder.finish_header();
            builder.finish_header();
            command.add(&mut builder, width, 2);
            command.add(&mut builder, width, 3);
            builder.finish_header();
            assert_eq!(
                write(&builder.build()),
                encode(&[
                    (command, width, &[(1, 0)]),
                    (command, width, &[(2, 0), (3, 0)]),
                ]),
                "{command:?} {width:?}"
            );
        }
    }
}

#[test]
fn empty_builder_writes_no_headers() {
    assert_eq!(write(&CommandBuilder::new().build()), Vec::<u8>::new());
}

fn two_of(command: Command, width: Width) -> CommandHeaders {
    let mut builder = CommandBuilder::new();
    command.add(&mut builder, width, 3);
    command.add(&mut builder, width, 4);
    builder.build()
}

#[test]
fn echo_of_each_command_at_each_width_is_compared_object_by_object() {
    for command in COMMANDS {
        for width in WIDTHS {
            let sent = two_of(command, width);
            let check = |items: &[(u16, u8)], expected| {
                assert_eq!(
                    compare(&sent, &encode(&[(command, width, items)])),
                    expected,
                    "{command:?} {width:?} {items:?}"
                );
            };

            check(&[(3, 0), (4, 0)], Ok(()));
            check(
                &[(3, 0), (4, 4)],
                Err(CommandResponseError::BadStatus(CommandStatus::NotSupported)),
            );
            check(
                &[(3, 200), (4, 0)],
                Err(CommandResponseError::BadStatus(CommandStatus::Unknown(200))),
            );
            // the status is checked before the value
            check(
                &[(3, 0), (5, 6)],
                Err(CommandResponseError::BadStatus(
                    CommandStatus::HardwareError,
                )),
            );
            check(
                &[(3, 0), (5, 0)],
                Err(CommandResponseError::ObjectValueMismatch),
            );
            check(&[(3, 0)], Err(CommandResponseError::ObjectCountMismatch));
            check(
                &[(3, 0), (4, 0), (5, 0)],
                Err(CommandResponseError::ObjectCountMismatch),
            );
        }
    }
}

#[test]
fn echo_with_another_command_type_or_width_is_a_header_type_mismatch() {
    for sent in COMMANDS {
        for sent_width in WIDTHS {
            for echoed in COMMANDS {
                for echoed_width in WIDTHS {
                    let expected =
                        if sent.group_var() == echoed.group_var() && sent_width == echoed_width {
                            Ok(())
                        } else {
                            Err(CommandResponseError::HeaderTypeMismatch)
                        };
                    assert_eq!(
                        compare(
                            &two_of(sent, sent_width),
                            &encode(&[(echoed, echoed_width, &[(3, 0), (4, 0)])])
                        ),
                        expected,
                        "{sent:?} {sent_width:?} {echoed:?} {echoed_width:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn echo_with_more_or_fewer_headers_is_a_header_count_mismatch() {
    let mut builder = CommandBuilder::new();
    Command::G12V1.add(&mut builder, Width::One, 3);
    Command::G41V2.add(&mut builder, Width::Two, 4);
    let sent = builder.build();

    let first: Header = (Command::G12V1, Width::One, &[(3, 0)]);
    let second: Header = (Command::G41V2, Width::Two, &[(4, 0)]);

    assert_eq!(compare(&sent, &encode(&[first, second])), Ok(()));
    assert_eq!(
        compare(&sent, &encode(&[first])),
        Err(CommandResponseError::HeaderCountMismatch)
    );
    assert_eq!(
        compare(&sent, &encode(&[first, second, second])),
        Err(CommandResponseError::HeaderCountMismatch)
    );
    // headers are compared in order, so a mismatch in the first is found before the missing second
    assert_eq!(
        compare(&sent, &encode(&[second])),
        Err(CommandResponseError::HeaderTypeMismatch)
    );
}

impl Command {
    /// encodings of the command that each differ from `value()` in one compared field
    fn other_values(self) -> Vec<Vec<u8>> {
        let crob = |offset: usize, byte: u8| {
            let mut value = Command::G12V1.value().to_vec();
            value[offset] = byte;
            value
        };
        match self {
            Command::G12V1 => vec![
                // op type LATCH_OFF
                crob(0, 0x04),
                // trip-close code CLOSE
                crob(0, 0x43),
                // clear bit
                crob(0, 0x23),
                // queue bit
                crob(0, 0x13),
                // count
                crob(1, 0x02),
                // on time
                crob(2, 101),
                // off time
                crob(6, 201),
            ],
            Command::G41V1 => vec![1i32.to_le_bytes().to_vec(), 2i32.to_le_bytes().to_vec()],
            Command::G41V2 => vec![1i16.to_le_bytes().to_vec(), 2i16.to_le_bytes().to_vec()],
            Command::G41V3 => vec![1.0f32.to_le_bytes().to_vec(), 2.0f32.to_le_bytes().to_vec()],
            Command::G41V4 => vec![1.0f64.to_le_bytes().to_vec(), 2.0f64.to_le_bytes().to_vec()],
        }
    }
}

/// echo of `two_of` whose second command is encoded as `value`, at the sent indices and with
/// success statuses
fn echo_with_second_value(command: Command, width: Width, value: &[u8]) -> Vec<u8> {
    let mut out = command.group_var().to_vec();
    out.push(width.qualifier());
    width.write(2, &mut out);
    width.write(3, &mut out);
    out.extend_from_slice(command.value());
    out.push(0);
    width.write(4, &mut out);
    out.extend_from_slice(value);
    out.push(0);
    out
}

#[test]
fn echo_with_a_different_value_at_the_same_index_is_an_object_value_mismatch() {
    for command in COMMANDS {
        for width in WIDTHS {
            let sent = two_of(command, width);
            assert_eq!(
                compare(
                    &sent,
                    &echo_with_second_value(command, width, command.value())
                ),
                Ok(()),
                "{command:?} {width:?}"
            );
            for value in command.other_values() {
                assert_eq!(
                    compare(&sent, &echo_with_second_value(command, width, &value)),
                    Err(CommandResponseError::ObjectValueMismatch),
                    "{command:?} {width:?} {value:?}"
                );
            }
        }
    }
}

mod property {
    use super::*;
    use crate::app::gen::prefixed::PrefixedVariation;
    use crate::app::parse::parser::ObjectHeader;
    use crate::app::parse::traits::Index;
    use crate::master::{BadEncoding, TaskError};

    use proptest::prelude::*;

    #[derive(Clone, Copy, Debug)]
    enum Value {
        G12V1 {
            code: ControlCode,
            count: u8,
            on_time: u32,
            off_time: u32,
        },
        G41V1(i32),
        G41V2(i16),
        G41V3(f32),
        G41V4(f64),
    }

    /// The fields of a command object, floats by their bits
    #[derive(Debug, PartialEq)]
    enum Fields {
        G12V1(ControlCode, u8, u32, u32, CommandStatus),
        G41V1(i32, CommandStatus),
        G41V2(i16, CommandStatus),
        G41V3(u32, CommandStatus),
        G41V4(u64, CommandStatus),
    }

    /// The byte a control code is written as, or `None` if its fields don't fit their bits
    fn code_byte(code: ControlCode) -> Option<u8> {
        let tcc = match code.tcc {
            TripCloseCode::Nul => 0x00,
            TripCloseCode::Close => 0x40,
            TripCloseCode::Trip => 0x80,
            TripCloseCode::Reserved => 0xC0,
            TripCloseCode::Unknown(_) => return None,
        };
        let op_type = match code.op_type {
            OpType::Nul => 0,
            OpType::PulseOn => 1,
            OpType::PulseOff => 2,
            OpType::LatchOn => 3,
            OpType::LatchOff => 4,
            OpType::Unknown(x @ 5..=15) => x,
            OpType::Unknown(_) => return None,
        };
        let clear = if code.clear { 0x20 } else { 0 };
        let queue = if code.queue { 0x10 } else { 0 };
        Some(tcc | clear | queue | op_type)
    }

    impl Value {
        fn group_var(self) -> [u8; 2] {
            match self {
                Value::G12V1 { .. } => [12, 1],
                Value::G41V1(_) => [41, 1],
                Value::G41V2(_) => [41, 2],
                Value::G41V3(_) => [41, 3],
                Value::G41V4(_) => [41, 4],
            }
        }

        /// the encoded object, without its status byte
        fn bytes(self) -> Vec<u8> {
            match self {
                Value::G12V1 {
                    code,
                    count,
                    on_time,
                    off_time,
                } => [
                    &[code_byte(code).unwrap(), count][..],
                    &on_time.to_le_bytes(),
                    &off_time.to_le_bytes(),
                ]
                .concat(),
                Value::G41V1(x) => x.to_le_bytes().to_vec(),
                Value::G41V2(x) => x.to_le_bytes().to_vec(),
                Value::G41V3(x) => x.to_bits().to_le_bytes().to_vec(),
                Value::G41V4(x) => x.to_bits().to_le_bytes().to_vec(),
            }
        }

        fn fields(self) -> Fields {
            let status = CommandStatus::Success;
            match self {
                Value::G12V1 {
                    code,
                    count,
                    on_time,
                    off_time,
                } => Fields::G12V1(code, count, on_time, off_time, status),
                Value::G41V1(x) => Fields::G41V1(x, status),
                Value::G41V2(x) => Fields::G41V2(x, status),
                Value::G41V3(x) => Fields::G41V3(x.to_bits(), status),
                Value::G41V4(x) => Fields::G41V4(x.to_bits(), status),
            }
        }

        fn add(self, builder: &mut CommandBuilder, width: Width, index: u16) {
            fn add<T>(builder: &mut CommandBuilder, command: T, width: Width, index: u16)
            where
                CommandBuilder: CommandSupport<T>,
            {
                match width {
                    Width::One => builder.add_u8(command, u8::try_from(index).unwrap()),
                    Width::Two => builder.add_u16(command, index),
                }
            }

            match self {
                Value::G12V1 {
                    code,
                    count,
                    on_time,
                    off_time,
                } => {
                    let crob = Group12Var1 {
                        code,
                        count,
                        on_time,
                        off_time,
                        status: CommandStatus::Success,
                    };
                    add(builder, crob, width, index)
                }
                Value::G41V1(x) => add(builder, Group41Var1::new(x), width, index),
                Value::G41V2(x) => add(builder, Group41Var2::new(x), width, index),
                Value::G41V3(x) => add(builder, Group41Var3::new(x), width, index),
                Value::G41V4(x) => add(builder, Group41Var4::new(x), width, index),
            }
        }
    }

    /// a command added to the builder, which first finishes the header it's building if `finish`
    #[derive(Clone, Debug)]
    struct Add {
        value: Value,
        width: Width,
        index: u16,
        finish: bool,
    }

    /// The headers the builder should make: consecutive commands of one type and index width
    /// share a header unless it was finished
    type ModelHeader = (Width, Vec<(u16, Value)>);

    fn model(adds: &[Add]) -> Vec<ModelHeader> {
        let mut headers: Vec<ModelHeader> = Vec::new();
        for add in adds {
            match headers.last_mut() {
                Some((width, items))
                    if !add.finish
                        && *width == add.width
                        && items[0].1.group_var() == add.value.group_var() =>
                {
                    items.push((add.index, add.value))
                }
                _ => headers.push((add.width, vec![(add.index, add.value)])),
            }
        }
        headers
    }

    /// A header of objects as bytes: each object's index and value, then its status
    type EncodedHeader = ([u8; 2], Width, Vec<(Vec<u8>, u8)>);

    fn encoded(headers: &[ModelHeader]) -> Vec<EncodedHeader> {
        headers
            .iter()
            .map(|(width, items)| {
                let objects = items
                    .iter()
                    .map(|(index, value)| {
                        let mut bytes = Vec::new();
                        width.write(*index, &mut bytes);
                        bytes.extend(value.bytes());
                        (bytes, 0)
                    })
                    .collect();
                (items[0].1.group_var(), *width, objects)
            })
            .collect()
    }

    fn to_bytes(headers: &[EncodedHeader]) -> Vec<u8> {
        let mut out = Vec::new();
        for (group_var, width, objects) in headers {
            out.extend_from_slice(group_var);
            out.push(width.qualifier());
            width.write(u16::try_from(objects.len()).unwrap(), &mut out);
            for (object, status) in objects {
                out.extend_from_slice(object);
                out.push(*status);
            }
        }
        out
    }

    fn prefixed<I: Index>(objects: PrefixedVariation<I>) -> Vec<(u16, Fields)> {
        match objects {
            PrefixedVariation::Group12Var1(seq) => seq
                .iter()
                .map(|x| {
                    let v = x.value;
                    let fields = Fields::G12V1(v.code, v.count, v.on_time, v.off_time, v.status);
                    (x.index.widen_to_u16(), fields)
                })
                .collect(),
            PrefixedVariation::Group41Var1(seq) => seq
                .iter()
                .map(|x| {
                    let fields = Fields::G41V1(x.value.value, x.value.status);
                    (x.index.widen_to_u16(), fields)
                })
                .collect(),
            PrefixedVariation::Group41Var2(seq) => seq
                .iter()
                .map(|x| {
                    let fields = Fields::G41V2(x.value.value, x.value.status);
                    (x.index.widen_to_u16(), fields)
                })
                .collect(),
            PrefixedVariation::Group41Var3(seq) => seq
                .iter()
                .map(|x| {
                    let fields = Fields::G41V3(x.value.value.to_bits(), x.value.status);
                    (x.index.widen_to_u16(), fields)
                })
                .collect(),
            PrefixedVariation::Group41Var4(seq) => seq
                .iter()
                .map(|x| {
                    let fields = Fields::G41V4(x.value.value.to_bits(), x.value.status);
                    (x.index.widen_to_u16(), fields)
                })
                .collect(),
            other => panic!("not a command: {:?}", other.variation()),
        }
    }

    /// the commands as the outstation parses them from a request
    fn parse_request(bytes: &[u8]) -> Vec<(Width, Vec<(u16, Fields)>)> {
        HeaderCollection::parse(ParseOptions::default(), FunctionCode::DirectOperate, bytes)
            .unwrap()
            .iter()
            .map(|header| match header {
                ObjectHeader::CountAndPrefix8(_, objects) => (Width::One, prefixed(objects)),
                ObjectHeader::CountAndPrefix16(_, objects) => (Width::Two, prefixed(objects)),
                other => panic!("not a command header: {other:?}"),
            })
            .collect()
    }

    /// every bit pattern, with NaN, the infinities and the zeros more likely
    fn float32() -> impl Strategy<Value = f32> {
        prop_oneof![
            3 => any::<u32>().prop_map(f32::from_bits),
            1 => prop::sample::select(vec![f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.0, -0.0]),
        ]
    }

    fn float64() -> impl Strategy<Value = f64> {
        prop_oneof![
            3 => any::<u64>().prop_map(f64::from_bits),
            1 => prop::sample::select(vec![f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -0.0]),
        ]
    }

    /// Control codes whose fields fit their bits, and with `invalid` weight, ones that don't:
    /// half of those are `Unknown` values that alias a named one, which a byte would parse as
    fn control_code(invalid: u32) -> impl Strategy<Value = ControlCode> {
        let tcc = prop_oneof![
            16 => (0..4u8).prop_map(TripCloseCode::from),
            invalid => (0..4u8).prop_map(TripCloseCode::Unknown),
            invalid => (4..=u8::MAX).prop_map(TripCloseCode::Unknown),
        ];
        let op_type = prop_oneof![
            16 => (0..16u8).prop_map(OpType::from),
            invalid => (0..=4u8).prop_map(OpType::Unknown),
            invalid => (16..=u8::MAX).prop_map(OpType::Unknown),
        ];
        (tcc, op_type, any::<bool>(), any::<bool>()).prop_map(|(tcc, op_type, clear, queue)| {
            ControlCode {
                tcc,
                clear,
                queue,
                op_type,
            }
        })
    }

    fn value(invalid_codes: u32) -> impl Strategy<Value = Value> {
        prop_oneof![
            (
                control_code(invalid_codes),
                any::<u8>(),
                any::<u32>(),
                any::<u32>()
            )
                .prop_map(|(code, count, on_time, off_time)| Value::G12V1 {
                    code,
                    count,
                    on_time,
                    off_time,
                }),
            any::<i32>().prop_map(Value::G41V1),
            any::<i16>().prop_map(Value::G41V2),
            float32().prop_map(Value::G41V3),
            float64().prop_map(Value::G41V4),
        ]
    }

    fn add(invalid_codes: u32) -> impl Strategy<Value = Add> {
        let width_and_index = prop_oneof![
            any::<u8>().prop_map(|index| (Width::One, u16::from(index))),
            any::<u16>().prop_map(|index| (Width::Two, index)),
        ];
        (
            value(invalid_codes),
            width_and_index,
            prop::bool::weighted(0.2),
        )
            .prop_map(|(value, (width, index), finish)| Add {
                value,
                width,
                index,
                finish,
            })
    }

    fn build(adds: &[Add]) -> CommandHeaders {
        let mut builder = CommandBuilder::new();
        for add in adds {
            if add.finish {
                builder.finish_header();
            }
            add.value.add(&mut builder, add.width, add.index);
        }
        builder.build()
    }

    /// One change an outstation could make to the echo of a request
    #[derive(Clone, Debug)]
    enum Change {
        /// XOR a byte of an object's index or value
        Object {
            object: prop::sample::Index,
            byte: prop::sample::Index,
            xor: u8,
        },
        /// a non-zero status for an object
        Status {
            object: prop::sample::Index,
            status: u8,
        },
        /// remove the last object of a header
        DropLastObject(prop::sample::Index),
        /// repeat the last object of a header
        RepeatLastObject(prop::sample::Index),
        DropLastHeader,
        RepeatLastHeader,
    }

    fn change() -> impl Strategy<Value = Change> {
        prop_oneof![
            (
                any::<prop::sample::Index>(),
                any::<prop::sample::Index>(),
                1..=u8::MAX
            )
                .prop_map(|(object, byte, xor)| Change::Object { object, byte, xor }),
            (any::<prop::sample::Index>(), 1..=u8::MAX)
                .prop_map(|(object, status)| Change::Status { object, status }),
            any::<prop::sample::Index>().prop_map(Change::DropLastObject),
            any::<prop::sample::Index>().prop_map(Change::RepeatLastObject),
            Just(Change::DropLastHeader),
            Just(Change::RepeatLastHeader),
        ]
    }

    /// the object at `object` among all the objects of the headers
    fn object_at<'a>(
        echo: &'a mut [EncodedHeader],
        object: &prop::sample::Index,
    ) -> &'a mut (Vec<u8>, u8) {
        let num_objects = echo.iter().map(|(_, _, objects)| objects.len()).sum();
        echo.iter_mut()
            .flat_map(|(_, _, objects)| objects.iter_mut())
            .nth(object.index(num_objects))
            .unwrap()
    }

    /// apply the change to the echo, and return the error the master reports for it
    fn apply(change: &Change, echo: &mut Vec<EncodedHeader>) -> CommandResponseError {
        match change {
            Change::Object { object, byte, xor } => {
                let (bytes, _) = object_at(echo, object);
                let byte = byte.index(bytes.len());
                bytes[byte] ^= xor;
                CommandResponseError::ObjectValueMismatch
            }
            Change::Status { object, status } => {
                object_at(echo, object).1 = *status;
                CommandResponseError::BadStatus(CommandStatus::from(*status))
            }
            Change::DropLastObject(header) => {
                let header = header.index(echo.len());
                echo[header].2.pop();
                CommandResponseError::ObjectCountMismatch
            }
            Change::RepeatLastObject(header) => {
                let header = header.index(echo.len());
                let objects = &mut echo[header].2;
                objects.push(objects.last().unwrap().clone());
                CommandResponseError::ObjectCountMismatch
            }
            Change::DropLastHeader => {
                echo.pop();
                CommandResponseError::HeaderCountMismatch
            }
            Change::RepeatLastHeader => {
                echo.push(echo.last().unwrap().clone());
                CommandResponseError::HeaderCountMismatch
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(512))]

        /// The builder groups commands into headers, the master writes them, and the outstation
        /// parses back the same commands, floats by their bits. A control code whose fields
        /// don't fit their bits makes the master write nothing.
        #[test]
        fn commands_are_written_and_parsed_back(adds in prop::collection::vec(add(2), 0..12)) {
            let headers = build(&adds);
            let mut buffer = [0; 1024];
            let mut cursor = WriteCursor::new(&mut buffer);
            cursor.write_u8(0xAA).unwrap();
            let result = headers.write(&mut HeaderWriter::new(&mut cursor));

            let invalid = adds.iter().find_map(|add| match add.value {
                Value::G12V1 { code, .. } if code_byte(code).is_none() => Some(code),
                _ => None,
            });
            if let Some(code) = invalid {
                prop_assert_eq!(
                    result,
                    Err(TaskError::BadEncoding(BadEncoding::ControlCode(code)))
                );
                prop_assert_eq!(cursor.written(), &[0xAA]);
                return Ok(());
            }

            result.unwrap();
            let model = model(&adds);
            let written = &cursor.written()[1..];
            prop_assert_eq!(written, to_bytes(&encoded(&model)));
            let expected: Vec<(Width, Vec<(u16, Fields)>)> = model
                .into_iter()
                .map(|(width, items)| {
                    let fields = items
                        .into_iter()
                        .map(|(index, value)| (index, value.fields()))
                        .collect();
                    (width, fields)
                })
                .collect();
            prop_assert_eq!(parse_request(written), expected);
        }

        /// An exact echo of the commands matches them, and one change to it is the error for
        /// that change
        #[test]
        fn echo_matches_only_when_unchanged(
            adds in prop::collection::vec(add(0), 1..12),
            change in prop::option::of(change()),
        ) {
            let headers = build(&adds);
            let mut echo = encoded(&model(&adds));
            let expected = match &change {
                None => Ok(()),
                Some(change) => Err(apply(change, &mut echo)),
            };
            prop_assert_eq!(compare(&headers, &to_bytes(&echo)), expected);
        }
    }
}
