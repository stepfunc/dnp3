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
