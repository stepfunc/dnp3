// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Request bytes from `CommandBuilder` and the comparison of each echo, for every command type
//! and index width
//!
//! The oracle is a hand encoding of each command; the outstation's parser reads the bytes back as
//! a second check.
//!
//! `commands_are_written_and_parsed_back` generates 0 to 11 commands of all five types, at either
//! index width and any index, sometimes finishing the header before a command. Floats are any bit
//! pattern, with NaN, the infinities and the zeros more likely; control codes include `Unknown`
//! values that alias a named one, and codes whose fields don't fit their bits.
//! `echo_matches_only_when_unchanged` generates 1 to 11 commands with codes that fit, and an echo
//! with at most one change: a byte of an object, a status, or a missing or repeated object or
//! header.
//!
//! Not generated, so left to the examples in this file: the single-header constructors
//! (`single_header_u8` and `single_header_u16`), `finish_header` twice in a row, an echo with more
//! than one change, and an echo with another command type or index width. An echo that doesn't
//! parse isn't compared here; the command tasks of `task_model.rs` fail on it.

use scursor::WriteCursor;

use crate::app::control::*;
use crate::app::format::write::HeaderWriter;
use crate::app::parse::options::ParseOptions;
use crate::app::parse::parser::HeaderCollection;
use crate::app::FunctionCode;
use crate::master::{CommandBuilder, CommandHeaders, CommandResponseError, CommandSupport};

/// The fields of a command object
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

#[derive(Copy, Clone, Debug, PartialEq)]
enum Width {
    One,
    Two,
}

const WIDTHS: [Width; 2] = [Width::One, Width::Two];

/// A command, and its encoding written out by hand, without its trailing status byte
type Example = (Value, &'static [u8]);

const LATCH_ON: ControlCode = ControlCode {
    tcc: TripCloseCode::Nul,
    clear: false,
    queue: false,
    op_type: OpType::LatchOn,
};

/// a command of each type
const COMMANDS: [Example; 5] = [
    (
        Value::G12V1 {
            code: LATCH_ON,
            count: 1,
            on_time: 100,
            off_time: 200,
        },
        &[0x03, 0x01, 100, 0, 0, 0, 200, 0, 0, 0],
    ),
    (Value::G41V1(-7), &[0xF9, 0xFF, 0xFF, 0xFF]),
    (Value::G41V2(513), &[0x01, 0x02]),
    (Value::G41V3(2.5), &[0x00, 0x00, 0x20, 0x40]),
    (Value::G41V4(2.5), &[0, 0, 0, 0, 0, 0, 0x04, 0x40]),
];

fn crob(code: ControlCode, count: u8, on_time: u32, off_time: u32) -> Group12Var1 {
    Group12Var1 {
        code,
        count,
        on_time,
        off_time,
        status: CommandStatus::Success,
    }
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
            } => add(builder, crob(code, count, on_time, off_time), width, index),
            Value::G41V1(x) => add(builder, Group41Var1::new(x), width, index),
            Value::G41V2(x) => add(builder, Group41Var2::new(x), width, index),
            Value::G41V3(x) => add(builder, Group41Var3::new(x), width, index),
            Value::G41V4(x) => add(builder, Group41Var4::new(x), width, index),
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
            Value::G12V1 {
                code,
                count,
                on_time,
                off_time,
            } => single(crob(code, count, on_time, off_time), width, index),
            Value::G41V1(x) => single(Group41Var1::new(x), width, index),
            Value::G41V2(x) => single(Group41Var2::new(x), width, index),
            Value::G41V3(x) => single(Group41Var3::new(x), width, index),
            Value::G41V4(x) => single(Group41Var4::new(x), width, index),
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

/// A header of objects as bytes: each object's index and value, then its status
type EncodedHeader = ([u8; 2], Width, Vec<(Vec<u8>, u8)>);

/// an object's index, then its encoded value
fn object(width: Width, index: u16, value: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    width.write(index, &mut bytes);
    bytes.extend_from_slice(value);
    bytes
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

/// a header of one command at each (index, status)
fn header((value, bytes): Example, width: Width, items: &[(u16, u8)]) -> EncodedHeader {
    let objects = items
        .iter()
        .map(|&(index, status)| (object(width, index, bytes), status))
        .collect();
    (value.group_var(), width, objects)
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
                write(&command.0.single(width, index)),
                to_bytes(&[header(command, width, &[(index, 0)])]),
                "{:?} {width:?}",
                command.0
            );
        }
    }
}

#[test]
fn builder_starts_a_new_header_when_the_type_or_width_changes() {
    let [g12v1, g41v1, g41v2, g41v3, g41v4] = COMMANDS;
    let mut builder = CommandBuilder::new();
    let adds = [
        (g12v1, Width::One, 1),
        (g12v1, Width::Two, 2),
        (g12v1, Width::Two, 3),
        (g41v1, Width::Two, 4),
        (g41v1, Width::One, 5),
        (g41v2, Width::One, 6),
        (g41v2, Width::One, 7),
        (g41v3, Width::Two, 8),
        (g41v4, Width::One, 9),
        (g41v4, Width::Two, 10),
        (g41v3, Width::Two, 11),
        (g12v1, Width::One, 12),
    ];
    for ((value, _), width, index) in adds {
        value.add(&mut builder, width, index);
    }

    assert_eq!(
        write(&builder.build()),
        to_bytes(&[
            header(g12v1, Width::One, &[(1, 0)]),
            header(g12v1, Width::Two, &[(2, 0), (3, 0)]),
            header(g41v1, Width::Two, &[(4, 0)]),
            header(g41v1, Width::One, &[(5, 0)]),
            header(g41v2, Width::One, &[(6, 0), (7, 0)]),
            header(g41v3, Width::Two, &[(8, 0)]),
            header(g41v4, Width::One, &[(9, 0)]),
            header(g41v4, Width::Two, &[(10, 0)]),
            header(g41v3, Width::Two, &[(11, 0)]),
            header(g12v1, Width::One, &[(12, 0)]),
        ])
    );
}

#[test]
fn finish_header_splits_commands_of_one_type_and_ignores_an_empty_header() {
    for command in COMMANDS {
        for width in WIDTHS {
            let mut builder = CommandBuilder::new();
            builder.finish_header();
            command.0.add(&mut builder, width, 1);
            builder.finish_header();
            builder.finish_header();
            command.0.add(&mut builder, width, 2);
            command.0.add(&mut builder, width, 3);
            builder.finish_header();
            assert_eq!(
                write(&builder.build()),
                to_bytes(&[
                    header(command, width, &[(1, 0)]),
                    header(command, width, &[(2, 0), (3, 0)]),
                ]),
                "{:?} {width:?}",
                command.0
            );
        }
    }
}

fn two_of(value: Value, width: Width) -> CommandHeaders {
    let mut builder = CommandBuilder::new();
    value.add(&mut builder, width, 3);
    value.add(&mut builder, width, 4);
    builder.build()
}

#[test]
fn echo_status_is_checked_before_the_value() {
    for command in COMMANDS {
        for width in WIDTHS {
            assert_eq!(
                compare(
                    &two_of(command.0, width),
                    &to_bytes(&[header(command, width, &[(3, 0), (5, 6)])])
                ),
                Err(CommandResponseError::BadStatus(
                    CommandStatus::HardwareError
                )),
                "{:?} {width:?}",
                command.0
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
                    let expected = if sent.0.group_var() == echoed.0.group_var()
                        && sent_width == echoed_width
                    {
                        Ok(())
                    } else {
                        Err(CommandResponseError::HeaderTypeMismatch)
                    };
                    assert_eq!(
                        compare(
                            &two_of(sent.0, sent_width),
                            &to_bytes(&[header(echoed, echoed_width, &[(3, 0), (4, 0)])])
                        ),
                        expected,
                        "{:?} {sent_width:?} {:?} {echoed_width:?}",
                        sent.0,
                        echoed.0
                    );
                }
            }
        }
    }
}

/// The properties, and the examples among their cases that are kept as fixed cases
mod property {
    use super::*;
    use crate::app::gen::prefixed::PrefixedVariation;
    use crate::app::parse::parser::ObjectHeader;
    use crate::app::parse::traits::Index;
    use crate::master::{BadEncoding, TaskError};

    use proptest::prelude::*;

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
    }

    /// a command added to the builder, which first finishes the header it's building if `finish`
    #[derive(Clone, Debug)]
    struct Add {
        value: Value,
        width: Width,
        index: u16,
        finish: bool,
    }

    impl Add {
        fn new(value: Value, width: Width, index: u16) -> Self {
            Self {
                value,
                width,
                index,
                finish: false,
            }
        }
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

    fn encoded(headers: &[ModelHeader]) -> Vec<EncodedHeader> {
        headers
            .iter()
            .map(|(width, items)| {
                let objects = items
                    .iter()
                    .map(|(index, value)| (object(*width, *index, &value.bytes()), 0))
                    .collect();
                (items[0].1.group_var(), *width, objects)
            })
            .collect()
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

    /// One change an outstation could make to the echo of a request. Each position is taken
    /// modulo the number of objects, bytes or headers it picks from
    #[derive(Clone, Debug)]
    enum Change {
        /// XOR a byte of an object's index or value
        Object {
            object: usize,
            byte: usize,
            xor: u8,
        },
        /// a non-zero status for an object
        Status {
            object: usize,
            status: u8,
        },
        /// remove the last object of a header
        DropLastObject(usize),
        /// repeat the last object of a header
        RepeatLastObject(usize),
        DropLastHeader,
        RepeatLastHeader,
    }

    fn change() -> impl Strategy<Value = Change> {
        prop_oneof![
            (any::<usize>(), any::<usize>(), 1..=u8::MAX)
                .prop_map(|(object, byte, xor)| Change::Object { object, byte, xor }),
            (any::<usize>(), 1..=u8::MAX)
                .prop_map(|(object, status)| Change::Status { object, status }),
            any::<usize>().prop_map(Change::DropLastObject),
            any::<usize>().prop_map(Change::RepeatLastObject),
            Just(Change::DropLastHeader),
            Just(Change::RepeatLastHeader),
        ]
    }

    /// the position among `len` items, wrapping around
    fn wrap(position: usize, len: usize) -> usize {
        position.checked_rem(len).unwrap()
    }

    /// the object at `object` among all the objects of the headers
    fn object_at(echo: &mut [EncodedHeader], object: usize) -> &mut (Vec<u8>, u8) {
        let num_objects = echo.iter().fold(0, |sum: usize, (_, _, objects)| {
            sum.saturating_add(objects.len())
        });
        echo.iter_mut()
            .flat_map(|(_, _, objects)| objects.iter_mut())
            .nth(wrap(object, num_objects))
            .unwrap()
    }

    /// apply the change to the echo, and return the error the master reports for it
    fn apply(change: &Change, echo: &mut Vec<EncodedHeader>) -> CommandResponseError {
        match *change {
            Change::Object { object, byte, xor } => {
                let (bytes, _) = object_at(echo, object);
                let byte = wrap(byte, bytes.len());
                bytes[byte] ^= xor;
                CommandResponseError::ObjectValueMismatch
            }
            Change::Status { object, status } => {
                object_at(echo, object).1 = status;
                CommandResponseError::BadStatus(CommandStatus::from(status))
            }
            Change::DropLastObject(header) => {
                let header = wrap(header, echo.len());
                echo[header].2.pop();
                CommandResponseError::ObjectCountMismatch
            }
            Change::RepeatLastObject(header) => {
                let header = wrap(header, echo.len());
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

    /// The builder groups commands into headers, the master writes them, and the outstation
    /// parses back the same commands, floats by their bits. A control code whose fields don't
    /// fit their bits makes the master write nothing.
    fn check_written_and_parsed_back(adds: &[Add]) {
        let headers = build(adds);
        let mut buffer = [0; 1024];
        let mut cursor = WriteCursor::new(&mut buffer);
        cursor.write_u8(0xAA).unwrap();
        let result = headers.write(&mut HeaderWriter::new(&mut cursor));

        let invalid = adds.iter().find_map(|add| match add.value {
            Value::G12V1 { code, .. } if code_byte(code).is_none() => Some(code),
            _ => None,
        });
        if let Some(code) = invalid {
            assert_eq!(
                result,
                Err(TaskError::BadEncoding(BadEncoding::ControlCode(code)))
            );
            assert_eq!(cursor.written(), &[0xAA]);
            return;
        }

        result.unwrap();
        let model = model(adds);
        let written = &cursor.written()[1..];
        assert_eq!(written, to_bytes(&encoded(&model)));
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
        assert_eq!(parse_request(written), expected);
    }

    type CompareResult = Result<(), CommandResponseError>;

    /// The master's comparison of the commands with their echo after the change if any, and the
    /// result for that change: a match for an exact echo
    fn compare_echo(adds: &[Add], change: Option<&Change>) -> (CompareResult, CompareResult) {
        let headers = build(adds);
        let mut echo = encoded(&model(adds));
        let expected = match change {
            None => Ok(()),
            Some(change) => Err(apply(change, &mut echo)),
        };
        (compare(&headers, &to_bytes(&echo)), expected)
    }

    /// the comparison of the echo with the change is the result for that change, and `expected`
    fn check_echo(adds: &[Add], change: Option<Change>, expected: CompareResult) {
        assert_eq!(
            compare_echo(adds, change.as_ref()),
            (expected, expected),
            "{adds:?} {change:?}"
        );
    }

    proptest! {
        #![proptest_config(crate::util::test_support::proptest_config(512))]

        #[test]
        fn commands_are_written_and_parsed_back(adds in prop::collection::vec(add(2), 0..12)) {
            check_written_and_parsed_back(&adds);
        }

        #[test]
        fn echo_matches_only_when_unchanged(
            adds in prop::collection::vec(add(0), 1..12),
            change in prop::option::of(change()),
        ) {
            let (result, expected) = compare_echo(&adds, change.as_ref());
            prop_assert_eq!(result, expected);
        }
    }

    /// commands of one type and width at each index, in one header
    fn adds(value: Value, width: Width, indices: &[u16]) -> Vec<Add> {
        indices
            .iter()
            .map(|&index| Add::new(value, width, index))
            .collect()
    }

    #[test]
    fn empty_builder_writes_no_headers() {
        check_written_and_parsed_back(&[]);
    }

    /// A control code whose fields don't fit their bits, in a header after a valid one, makes the
    /// master write nothing
    #[test]
    fn later_invalid_header_leaves_the_cursor_untouched() {
        let crob = |op_type| Value::G12V1 {
            code: ControlCode::from_op_type(op_type),
            count: 1,
            on_time: 1000,
            off_time: 1000,
        };
        let invalid = OpType::Unknown(0x11);
        assert_eq!(code_byte(ControlCode::from_op_type(invalid)), None);
        check_written_and_parsed_back(&[
            Add::new(crob(OpType::LatchOn), Width::One, 1),
            Add {
                finish: true,
                ..Add::new(crob(invalid), Width::One, 2)
            },
        ]);
    }

    #[test]
    fn builder_merges_consecutive_commands_of_one_type_and_width() {
        for (value, _) in COMMANDS {
            for width in WIDTHS {
                let adds = adds(value, width, &[3, 4, 200]);
                assert_eq!(model(&adds).len(), 1);
                check_written_and_parsed_back(&adds);
            }
        }
    }

    #[test]
    fn echo_of_each_command_at_each_width_is_compared_object_by_object() {
        for (value, _) in COMMANDS {
            for width in WIDTHS {
                let adds = adds(value, width, &[3, 4]);
                let check = |change, expected| check_echo(&adds, change, expected);

                check(None, Ok(()));
                check(
                    Some(Change::Status {
                        object: 1,
                        status: 4,
                    }),
                    Err(CommandResponseError::BadStatus(CommandStatus::NotSupported)),
                );
                check(
                    Some(Change::Status {
                        object: 0,
                        status: 200,
                    }),
                    Err(CommandResponseError::BadStatus(CommandStatus::Unknown(200))),
                );
                check(
                    Some(Change::DropLastObject(0)),
                    Err(CommandResponseError::ObjectCountMismatch),
                );
                check(
                    Some(Change::RepeatLastObject(0)),
                    Err(CommandResponseError::ObjectCountMismatch),
                );
            }
        }
    }

    #[test]
    fn echo_with_any_byte_of_an_object_changed_is_an_object_value_mismatch() {
        for (value, _) in COMMANDS {
            for width in WIDTHS {
                let adds = adds(value, width, &[3, 4]);
                let echo = encoded(&model(&adds));
                for (object, (bytes, _)) in echo[0].2.iter().enumerate() {
                    for byte in 0..bytes.len() {
                        // each field of a control code: op type, queue, clear and trip-close code
                        for xor in [0x01, 0x10, 0x20, 0x40, 0x80] {
                            check_echo(
                                &adds,
                                Some(Change::Object { object, byte, xor }),
                                Err(CommandResponseError::ObjectValueMismatch),
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn echo_with_more_or_fewer_headers_is_a_header_count_mismatch() {
        let [g12v1, _, g41v2, _, _] = COMMANDS;
        let adds = [
            Add::new(g12v1.0, Width::One, 3),
            Add::new(g41v2.0, Width::Two, 4),
        ];

        check_echo(&adds, None, Ok(()));
        check_echo(
            &adds,
            Some(Change::DropLastHeader),
            Err(CommandResponseError::HeaderCountMismatch),
        );
        check_echo(
            &adds,
            Some(Change::RepeatLastHeader),
            Err(CommandResponseError::HeaderCountMismatch),
        );
        // headers are compared in order, so a mismatch in the first is found before the missing
        // second
        assert_eq!(
            compare(
                &build(&adds),
                &to_bytes(&[header(g41v2, Width::Two, &[(4, 0)])])
            ),
            Err(CommandResponseError::HeaderTypeMismatch)
        );
    }
}
